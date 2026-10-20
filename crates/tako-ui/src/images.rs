//! Imagens da interface: decodificadas fora da thread da UI, no tamanho em que aparecem, num LRU por bytes
//! (PRD §4: "decodificação sob demanda, LRU por bytes de GPU"). Durante o jogo o processo da UI termina
//! (ADR-0026), então não há o que liberar aqui além do próprio LRU.

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Converte uma URI de arte (`https://`, `steam-art://`, `ssmedia://`...) num arquivo local, baixando se
/// preciso. Roda nas threads de trabalho.
pub type Resolver = Arc<dyn Fn(&str) -> Option<PathBuf> + Send + Sync>;

struct Request {
    key: String,
    uri: String,
    max_w: u32,
    max_h: u32,
    /// Cópia de vidro da arte (`glass`): desfocada, saturada e com a grade de luminância.
    glass: bool,
    /// Ícone do jogo (`icon`): quadrado de `max_w` px, já com os cantos redondos e o contorno.
    icon: bool,
}

/// Grade de luminância da cópia de vidro (colunas × linhas sobre a arte inteira).
pub const GLASS_GRID: (u32, u32) = (16, 9);
/// Altura da cópia de vidro. Pequena de propósito: desfocada, não precisa de mais, e o desfoque fica barato.
const GLASS_HEIGHT: u32 = 240;
/// Desfoque na escala da cópia, para uma arte mostrada na altura de uma janela de ~900px: a borda da janela já
/// desfoca a arte em 42px e o vidro desfoca de novo em `glass-blur` (28px); juntos dão ~50px.
const GLASS_SIGMA: f32 = 50.0 * GLASS_HEIGHT as f32 / 900.0;
/// `glass-saturate` (140%).
const GLASS_SATURATE: f32 = 1.4;

/// Teto do LRU. Cabe a tela inteira (capas de 344x516 ≈ 0,7 MB, fundo na altura da janela ≈ 8–15 MB) e mais um
/// fundo para voltar rápido; o que está na tela nunca sai (ver `Lru::evict`), então o teto vale para o que está
/// fora dela. Cada imagem também ocupa uma textura do mesmo tamanho na GPU enquanto aparece.
const LIMIT: usize = 32 * 1024 * 1024;

struct Lru {
    /// Imagem, bytes e o último quadro (`frame`) em que foi pedida.
    map: HashMap<String, (Image, usize, u64)>,
    order: VecDeque<String>,
    bytes: usize,
    limit: usize,
    frame: u64,
}

impl Lru {
    fn new(limit: usize) -> Lru {
        Lru {
            map: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            limit,
            frame: 0,
        }
    }
    fn get(&mut self, key: &str) -> Option<Image> {
        let frame = self.frame;
        let (img, _, seen) = self.map.get_mut(key)?;
        *seen = frame;
        let img = img.clone();
        if let Some(pos) = self.order.iter().position(|k| k == key)
            && let Some(k) = self.order.remove(pos)
        {
            self.order.push_back(k);
        }
        Some(img)
    }
    fn insert(&mut self, key: String, img: Image, bytes: usize) {
        self.bytes += bytes;
        self.order.push_back(key.clone());
        self.map.insert(key, (img, bytes, self.frame));
        // Entre quadros: o que o último quadro mostrou continua protegido.
        self.evict(self.frame.saturating_sub(1));
    }
    /// Fim de um quadro: só o que ele pediu fica protegido.
    fn frame_done(&mut self) {
        self.evict(self.frame);
        self.frame += 1;
    }
    /// Tira as mais antigas até caber no teto, mas nunca as pedidas desde o quadro `keep_from`: se a tela
    /// mostra mais do que o teto, tirar uma delas só faria pedir de novo e decodificar em loop.
    fn evict(&mut self, keep_from: u64) {
        while self.bytes > self.limit {
            let Some(old) = self.order.front() else {
                break;
            };
            if self.map.get(old).is_some_and(|e| e.2 >= keep_from) {
                break;
            }
            if let Some(old) = self.order.pop_front()
                && let Some((_, b, _)) = self.map.remove(&old)
            {
                self.bytes -= b;
            }
        }
    }
}

/// Fila de pedidos servida do mais novo para o mais antigo: o jogo que a pessoa acabou de escolher passa à frente
/// dos que ela já deixou para trás (com a fila na ordem, a arte nova esperava a de todos os jogos anteriores).
#[derive(Clone, Default)]
struct Queue(Arc<(Mutex<Vec<Request>>, Condvar)>);

impl Queue {
    fn push(&self, req: Request) {
        let (list, ready) = &*self.0;
        list.lock().unwrap_or_else(|e| e.into_inner()).push(req);
        ready.notify_one();
    }
    fn pop(&self) -> Request {
        let (list, ready) = &*self.0;
        let mut list = list.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(req) = list.pop() {
                return req;
            }
            list = ready.wait(list).unwrap_or_else(|e| e.into_inner());
        }
    }
}

/// Por quanto tempo a troca de jogo segura a arte anterior esperando a nova (`is_pending`). Passou disso, a tela
/// mostra o jogo novo sem arte até ela chegar, em vez de ficar com o fundo e o logo de outro jogo.
pub const HOLD: Duration = Duration::from_millis(350);

/// Espera antes de pedir de novo uma imagem que falhou (rede fora, CDN lenta). Arte que não existe não custa rede
/// na nova tentativa: o `ArtCache` lembra a falta.
const RETRY: Duration = Duration::from_secs(30);

struct State {
    lru: Lru,
    /// Grade de luminância de cada cópia de vidro (pela mesma chave do LRU); poucos bytes por arte.
    lum: HashMap<String, Vec<f32>>,
    /// Pedido na fila e quando foi feito.
    pending: HashMap<String, Instant>,
    /// Falhou e quando: tenta de novo depois de `RETRY` (uma falha de rede na abertura apagava as artes até reiniciar).
    failed: HashMap<String, Instant>,
    tx: Option<Queue>,
    /// Fila só dos ícones da lista: centenas deles, muitos pela rede, não podem passar à frente da capa e do
    /// fundo que estão na tela (regressão da alpha.15).
    icon_tx: Option<Queue>,
    on_ready: Option<Box<dyn Fn()>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        lru: Lru::new(LIMIT),
        lum: HashMap::new(),
        pending: HashMap::new(),
        failed: HashMap::new(),
        tx: None,
        icon_tx: None,
        on_ready: None,
    });
}

/// Liga o carregador (na thread da UI). `on_ready` é chamado na thread da UI quando uma imagem chega.
pub fn init(resolver: Resolver, workers: usize, on_ready: impl Fn() + 'static) {
    let pool = |name: &str| {
        let queue = Queue::default();
        for _ in 0..workers.max(1) {
            let queue = queue.clone();
            let resolver = resolver.clone();
            std::thread::Builder::new()
                .name(name.into())
                .spawn(move || worker(queue, resolver))
                .ok();
        }
        queue
    };
    let (tx, icon_tx) = (pool("tako-images"), pool("tako-icons"));
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.tx = Some(tx);
        s.icon_tx = Some(icon_tx);
        s.on_ready = Some(Box::new(on_ready));
    });
}

/// Imagem pronta, ou `None` (e o pedido entra na fila). `max_w`/`max_h` em pixels físicos.
pub fn get(uri: &str, max_w: u32, max_h: u32) -> Option<Image> {
    request(uri, max_w, max_h, false, false)
}

/// Ícone do jogo (UX-005, `Game::shown_icon`): quadrado de `size_px` pixels físicos (28 na tela normal, 56 em
/// alta densidade), decodificado nesse tamanho e guardado já com os cantos redondos (raio 6/28 do lado) e o
/// contorno de `#ffffff1a`, para a lista só desenhar a imagem, sem recorte em tempo real. Aceita `.ico`, PNG,
/// JPG e WebP. `None` até ficar pronto (ou se a imagem não carrega): a lista desenha a inicial do nome.
pub fn icon(uri: &str, size_px: u32) -> Option<Image> {
    request(uri, size_px, size_px, false, true)
}

/// Cópia de vidro da arte do desktop (Liquid Glass sem desfoque em tempo real): desfocada e saturada uma vez,
/// em thread de fundo, com a luminância média em `GLASS_GRID` (linhas de cima para baixo). `None` até ficar pronta.
pub fn glass(uri: &str) -> Option<(Image, Vec<f32>)> {
    let img = request(uri, 0, GLASS_HEIGHT, true, false)?;
    let key = format!("glass|{uri}");
    let lum = STATE.with(|s| s.borrow().lum.get(&key).cloned())?;
    Some((img, lum))
}

fn request(uri: &str, max_w: u32, max_h: u32, glass: bool, icon: bool) -> Option<Image> {
    if uri.is_empty() {
        return None;
    }
    let key = if glass {
        format!("glass|{uri}")
    } else if icon {
        format!("icon{max_w}|{uri}")
    } else {
        format!("{max_w}x{max_h}|{uri}")
    };
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(img) = s.lru.get(&key) {
            return Some(img);
        }
        if s.pending.contains_key(&key) || s.failed.get(&key).is_some_and(|t| t.elapsed() < RETRY) {
            return None;
        }
        let request = Request {
            key: key.clone(),
            uri: uri.to_owned(),
            max_w,
            max_h,
            glass,
            icon,
        };
        let tx = if icon { &s.icon_tx } else { &s.tx };
        if let Some(tx) = tx.clone() {
            tx.push(request);
            s.pending.insert(key, Instant::now());
            if !icon {
                // Fim do `HOLD`: redesenha para a tela largar a arte anterior se esta ainda não chegou.
                slint::Timer::single_shot(HOLD, || {
                    STATE.with(|s| {
                        let cb = s.borrow_mut().on_ready.take();
                        if let Some(cb) = cb {
                            cb();
                            s.borrow_mut().on_ready = Some(cb);
                        }
                    });
                });
            }
        }
        None
    })
}

/// Luminância relativa (WCAG) média dos pixels visíveis de uma imagem já decodificada (o logo do jogo), medida
/// uma vez por URI. `None` se a imagem não tem pixels legíveis ou é toda transparente.
pub fn mean_luminance(uri: &str, img: &Image) -> Option<f32> {
    thread_local! {
        static SEEN: RefCell<HashMap<String, Option<f32>>> = RefCell::new(HashMap::new());
    }
    if let Some(known) = SEEN.with(|s| s.borrow().get(uri).copied()) {
        return known;
    }
    let buf = img.to_rgba8()?;
    let (mut sum, mut weight) = (0.0f64, 0.0f64);
    for p in buf.as_slice() {
        let a = f64::from(p.a) / 255.0;
        if a > 0.5 {
            sum += a * tako_core::presentation::Rgb::new(p.r, p.g, p.b).luminance();
            weight += a;
        }
    }
    let lum = (weight > 0.0).then(|| (sum / weight) as f32);
    SEEN.with(|s| s.borrow_mut().insert(uri.to_owned(), lum));
    lum
}

/// A imagem (em qualquer tamanho, ou a cópia de vidro) ainda está a caminho, pedida há menos de `HOLD`: depois disso a tela não espera mais por ela (ver `HOLD`).
pub fn is_pending(uri: &str) -> bool {
    let tail = format!("|{uri}");
    STATE.with(|s| {
        s.borrow()
            .pending
            .iter()
            .any(|(k, since)| k.ends_with(&tail) && since.elapsed() < HOLD)
    })
}

/// Chamar depois de montar cada quadro (todas as `get` da tela já feitas).
pub fn frame_done() {
    STATE.with(|s| s.borrow_mut().lru.frame_done());
}

/// Esquece falhas (ex.: depois de baixar novas artes).
pub fn retry_failed() {
    STATE.with(|s| s.borrow_mut().failed.clear());
}

pub fn clear() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.lru.map.clear();
        s.lru.order.clear();
        s.lru.bytes = 0;
        s.lum.clear();
    });
}

fn worker(queue: Queue, resolver: Resolver) {
    loop {
        let req = queue.pop();
        let buf = load(&req, &resolver);
        let key = req.key;
        let sent = slint::invoke_from_event_loop(move || {
            STATE.with(|s| {
                let mut s = s.borrow_mut();
                s.pending.remove(&key);
                match buf {
                    Some((buf, lum)) => {
                        let bytes = (buf.width() * buf.height() * 4) as usize;
                        if let Some(lum) = lum {
                            s.lum.insert(key.clone(), lum);
                        }
                        s.lru.insert(key, shared(buf), bytes);
                    }
                    None => {
                        s.failed.insert(key, Instant::now());
                    }
                }
            });
            // Fora do borrow: o callback pode pedir mais imagens.
            STATE.with(|s| {
                let cb = s.borrow_mut().on_ready.take();
                if let Some(cb) = cb {
                    cb();
                    s.borrow_mut().on_ready = Some(cb);
                }
            });
        });
        // Sem event loop (fechando o app, ou nos testes) a resposta se perde, mas a thread segue até a fila fechar:
        // parar aqui deixaria os pedidos seguintes pendentes para sempre.
        if sent.is_err() {
            continue;
        }
    }
}

/// Imagem com chave de cache própria: o femtovg sobe uma textura só por imagem, por mais `Image` do tema que a
/// mostrem. Sem chave (`Image::from_rgba8`), cada elemento sobe a sua: as ~90 faixas da borda da arte e cada painel
/// de vidro tinham uma cópia da arte desfocada na GPU, e o fundo nítido duas ou três.
pub(crate) fn shared(buf: SharedPixelBuffer<Rgba8Pixel>) -> Image {
    use i_slint_core::graphics::{ImageCacheKey, ImageInner, SharedImageBuffer};
    use std::sync::atomic::{AtomicUsize, Ordering};
    // `EmbeddedData` costuma ser o endereço de dados estáticos do binário; daqui para cima nenhum endereço real chega
    // (64 bits). Cada imagem nova ganha uma chave nova, então nunca há textura velha com a chave de outra.
    static NEXT: AtomicUsize = AtomicUsize::new(usize::MAX / 2);
    ImageInner::EmbeddedImage {
        cache_key: ImageCacheKey::EmbeddedData(NEXT.fetch_add(1, Ordering::Relaxed)),
        buffer: SharedImageBuffer::RGBA8(buf),
    }
    .into()
}

type Loaded = (SharedPixelBuffer<Rgba8Pixel>, Option<Vec<f32>>);

/// Lê e decodifica; nada aqui pode derrubar a UI (arquivos e downloads são dados não confiáveis).
fn load(req: &Request, resolver: &Resolver) -> Option<Loaded> {
    let path = local_path(&req.uri).or_else(|| resolver(&req.uri))?;
    let meta = std::fs::metadata(&path).ok()?;
    if meta.len() > 64 * 1024 * 1024 {
        return None;
    }
    let reader = image::ImageReader::open(&path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let mut decoder_limits = image::Limits::default();
    decoder_limits.max_image_width = Some(16384);
    decoder_limits.max_image_height = Some(16384);
    decoder_limits.max_alloc = Some(512 * 1024 * 1024);
    let mut reader = reader;
    reader.limits(decoder_limits);
    let img = reader.decode().ok()?;
    if req.glass {
        let (rgba, lum) = glass_copy(&img);
        let buf = SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height());
        return Some((buf, Some(lum)));
    }
    if req.icon {
        let rgba = round_icon(&img, req.max_w);
        return Some((
            SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height()),
            None,
        ));
    }
    let img = if img.width() > req.max_w || img.height() > req.max_h {
        img.resize(
            req.max_w.max(1),
            req.max_h.max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let rgba = img.to_rgba8();
    Some((
        SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height()),
        None,
    ))
}

/// Ícone quadrado de `size` px (recorte central do meio da imagem), com os cantos redondos (raio 6/28 do lado,
/// anti-serrilhado) e o contorno de 1 px a cada 28 de `#ffffff1a` por cima da borda. Fora do arredondado o pixel
/// fica transparente.
pub(crate) fn round_icon(img: &image::DynamicImage, size: u32) -> image::RgbaImage {
    let size = size.clamp(8, 256);
    let mut out = img
        .resize_to_fill(size, size, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    let half = size as f32 / 2.0;
    let radius = size as f32 * 6.0 / 28.0;
    let border = (size / 28).max(1) as f32;
    for (x, y, p) in out.enumerate_pixels_mut() {
        // Distância (negativa por dentro) até a borda do retângulo arredondado.
        let (dx, dy) = ((x as f32 + 0.5 - half).abs(), (y as f32 + 0.5 - half).abs());
        let (qx, qy) = (dx - (half - radius), dy - (half - radius));
        let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius;
        let inside = (0.5 - d).clamp(0.0, 1.0);
        let ring = (d + border + 0.5).clamp(0.0, 1.0) * (26.0 / 255.0);
        let pa = f32::from(p.0[3]) / 255.0;
        // O contorno (branco, alfa `ring`) por cima da imagem.
        let alpha = ring + pa * (1.0 - ring);
        for c in &mut p.0[..3] {
            *c = if alpha > 0.0 {
                ((255.0 * ring + f32::from(*c) * pa * (1.0 - ring)) / alpha).round() as u8
            } else {
                0
            };
        }
        p.0[3] = (alpha * inside * 255.0).round() as u8;
    }
    out
}

/// Reduz, satura e desfoca a arte; mede a luminância relativa (WCAG) de cada célula da grade.
pub(crate) fn glass_copy(img: &image::DynamicImage) -> (image::RgbaImage, Vec<f32>) {
    let h = GLASS_HEIGHT.min(img.height().max(1));
    let w = (u64::from(img.width()) * u64::from(h) / u64::from(img.height().max(1))).max(1) as u32;
    let mut small = img
        .resize_exact(w, h, image::imageops::FilterType::Triangle)
        .to_rgba8();
    for p in small.pixels_mut() {
        let [r, g, b, _] = p.0.map(f32::from);
        let gray = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let sat = |c: f32| (gray + (c - gray) * GLASS_SATURATE).clamp(0.0, 255.0) as u8;
        p.0 = [sat(r), sat(g), sat(b), 255];
    }
    let blurred = image::imageops::blur(&small, GLASS_SIGMA);
    let (cols, rows) = GLASS_GRID;
    let mut lum = Vec::with_capacity((cols * rows) as usize);
    for row in 0..rows {
        for col in 0..cols {
            let (x0, x1) = (
                col * w / cols,
                ((col + 1) * w / cols).max(col * w / cols + 1).min(w),
            );
            let (y0, y1) = (
                row * h / rows,
                ((row + 1) * h / rows).max(row * h / rows + 1).min(h),
            );
            let (mut sum, mut n) = ([0u64; 3], 0u64);
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = blurred.get_pixel(x, y).0;
                    for c in 0..3 {
                        sum[c] += u64::from(p[c]);
                    }
                    n += 1;
                }
            }
            let avg = |c: usize| (sum[c] / n.max(1)) as u8;
            lum.push(tako_core::presentation::Rgb::new(avg(0), avg(1), avg(2)).luminance() as f32);
        }
    }
    (blurred, lum)
}

/// `file:///c:/x.png`, `file:///home/x.png` ou caminho simples.
pub fn local_path(uri: &str) -> Option<PathBuf> {
    if let Some(rest) = uri.strip_prefix("file://") {
        let rest = percent_decode(rest);
        // file:///C:/x → C:/x no Windows; file:///home/x → /home/x.
        let p = if cfg!(windows) {
            rest.trim_start_matches('/').to_owned()
        } else {
            rest
        };
        return Some(PathBuf::from(p));
    }
    if uri.contains("://") {
        return None;
    }
    Some(PathBuf::from(uri))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    /// O jogo escolhido por último é servido primeiro: os pedidos dos jogos que a pessoa já passou ficam para depois.
    #[test]
    fn the_newest_request_is_served_first() {
        let q = Queue::default();
        for key in ["a", "b", "c"] {
            q.push(Request {
                key: key.into(),
                uri: key.into(),
                max_w: 1,
                max_h: 1,
                glass: false,
                icon: false,
            });
        }
        assert_eq!(q.pop().key, "c");
        assert_eq!(q.pop().key, "b");
    }

    /// Regressão da alpha.15: com 300 ícones na fila (cada um ~20 ms de rede), o fundo do jogo em destaque
    /// esperava atrás de todos (~4 s aqui; com a rede de verdade, minutos). Os ícones têm a fila deles.
    /// Sem event loop no teste, as threads seguem vivas (ver `worker`), então a espera é a da fila.
    #[test]
    fn icons_do_not_hold_back_the_art_on_screen() {
        let (seen, hero) = channel::<()>();
        let seen = Mutex::new(seen);
        let resolver: Resolver = Arc::new(move |uri: &str| {
            if uri.starts_with("icon") {
                std::thread::sleep(std::time::Duration::from_millis(20));
            } else if let Ok(tx) = seen.lock() {
                let _ = tx.send(());
            }
            None
        });
        init(resolver, 2, || {});
        for i in 0..300 {
            assert!(icon(&format!("icon://{i}"), 56).is_none());
        }
        let asked = std::time::Instant::now();
        assert!(get("hero://destaque", 1600, 900).is_none());
        let waited = hero.recv_timeout(std::time::Duration::from_secs(2));
        let ms = asked.elapsed().as_millis();
        eprintln!("fundo resolvido {ms} ms depois do pedido, com 300 ícones na fila");
        assert!(waited.is_ok() && ms < 500, "fundo esperou {ms} ms");
    }

    #[test]
    fn local_paths() {
        assert_eq!(local_path("https://x/y.png"), None);
        assert_eq!(local_path("steam-art://570/cover"), None);
        if !cfg!(windows) {
            assert_eq!(
                local_path("file:///home/a/b%20c.png"),
                Some(PathBuf::from("/home/a/b c.png"))
            );
        }
        assert_eq!(local_path("C:/x.png"), Some(PathBuf::from("C:/x.png")));
    }

    #[test]
    fn glass_copy_is_small_blurred_and_measures_light_and_dark_art() {
        // Metade de cima branca, de baixo preta: a linha de cima da grade passa do limite da tinta densa (0,18).
        let art = image::RgbaImage::from_fn(800, 450, |_, y| {
            if y < 225 {
                image::Rgba([255, 255, 255, 255])
            } else {
                image::Rgba([0, 0, 0, 255])
            }
        });
        let (copy, lum) = glass_copy(&image::DynamicImage::ImageRgba8(art));
        assert_eq!((copy.width(), copy.height()), (426, GLASS_HEIGHT));
        assert_eq!(lum.len(), (GLASS_GRID.0 * GLASS_GRID.1) as usize);
        assert!(lum[0] > 0.18 && lum[lum.len() - 1] < 0.18);
        // Desfocada: a borda entre o branco e o preto vira um degradê.
        let mid = copy.get_pixel(200, GLASS_HEIGHT / 2).0[0];
        assert!(mid > 20 && mid < 235, "{mid}");
    }

    fn load_icon(path: &std::path::Path, size: u32) -> image::RgbaImage {
        let req = Request {
            key: String::new(),
            uri: path.to_string_lossy().into_owned(),
            max_w: size,
            max_h: size,
            glass: false,
            icon: true,
        };
        let (buf, _) = load(&req, &(Arc::new(|_: &str| None) as Resolver)).unwrap();
        image::RgbaImage::from_raw(buf.width(), buf.height(), buf.as_bytes().to_vec()).unwrap()
    }

    #[test]
    fn icon_is_cached_round_with_a_faint_border_at_both_densities() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("icone.png");
        // Vermelho puro e opaco, maior que o tamanho de exibição e não quadrado: recorta no centro.
        image::RgbaImage::from_pixel(100, 80, image::Rgba([200, 0, 0, 255]))
            .save(&file)
            .unwrap();
        for size in [28u32, 56] {
            let icon = load_icon(&file, size);
            assert_eq!(icon.dimensions(), (size, size));
            let last = size - 1;
            // Os quatro cantos são transparentes; o miolo é a imagem intacta.
            for (x, y) in [(0, 0), (last, 0), (0, last), (last, last)] {
                assert_eq!(icon.get_pixel(x, y).0[3], 0, "canto {x},{y} a {size}px");
            }
            assert_eq!(icon.get_pixel(size / 2, size / 2).0, [200, 0, 0, 255]);
            // No meio de cada lado o contorno clareia o pixel (branco a 10 %), sem deixá-lo transparente.
            let edge = icon.get_pixel(size / 2, 0).0;
            assert_eq!(edge[3], 255);
            assert!(edge[1] > 15 && edge[1] < 40, "contorno {edge:?} a {size}px");
            // Logo depois do contorno a imagem volta ao normal (1 px a 28, 2 px a 56).
            let inner = icon.get_pixel(size / 2, size / 28 + 1).0;
            assert_eq!(inner, [200, 0, 0, 255]);
        }
    }

    #[test]
    fn icon_decodes_ico_and_survives_broken_files() {
        use image::ImageEncoder;
        let dir = tempfile::tempdir().unwrap();
        let ico = dir.path().join("jogo.ico");
        let mut bytes = Vec::new();
        image::codecs::ico::IcoEncoder::new(&mut bytes)
            .write_image(
                &[0, 128, 0, 255].repeat(32 * 32),
                32,
                32,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        std::fs::write(&ico, bytes).unwrap();
        assert_eq!(load_icon(&ico, 28).get_pixel(14, 14).0, [0, 128, 0, 255]);
        // Lixo, arquivo vazio e arquivo ausente: sem imagem, sem pânico.
        let req = |p: &std::path::Path| Request {
            key: String::new(),
            uri: p.to_string_lossy().into_owned(),
            max_w: 28,
            max_h: 28,
            glass: false,
            icon: true,
        };
        let none: Resolver = Arc::new(|_: &str| None);
        for (name, content) in [
            ("lixo.ico", &b"isto nao e uma imagem"[..]),
            ("vazio.png", b""),
        ] {
            let f = dir.path().join(name);
            std::fs::write(&f, content).unwrap();
            assert!(load(&req(&f), &none).is_none(), "{name}");
        }
        assert!(load(&req(&dir.path().join("nao-existe.png")), &none).is_none());
    }

    #[test]
    fn shared_images_get_their_own_cache_key() {
        use i_slint_core::graphics::{ImageCacheKey, ImageInner};
        let key = |img: &Image| match <&ImageInner>::from(img) {
            ImageInner::EmbeddedImage { cache_key, .. } => cache_key.clone(),
            _ => ImageCacheKey::Invalid,
        };
        let a = shared(SharedPixelBuffer::new(2, 2));
        let b = shared(SharedPixelBuffer::new(2, 2));
        assert!(matches!(key(&a), ImageCacheKey::EmbeddedData(_)));
        assert!(key(&a) != key(&b));
        // A mesma imagem passada adiante (LRU, tema) continua com a mesma chave: uma textura só.
        assert!(key(&a.clone()) == key(&a) && a.clone() == a);
        assert_eq!(a.to_rgba8().map(|p| p.width()), Some(2));
    }

    #[test]
    fn lru_evicts_by_bytes_but_keeps_what_is_on_screen() {
        let mut l = Lru::new(10);
        l.insert("a".into(), Image::default(), 6);
        l.frame_done();
        // "a" estava no último quadro: continua até um quadro terminar sem ela.
        l.insert("b".into(), Image::default(), 6);
        assert!(l.map.contains_key("a"));
        assert!(l.get("b").is_some());
        l.frame_done();
        assert!(!l.map.contains_key("a"));
        assert_eq!(l.bytes, 6);
        // A tela pede mais que o teto: nada do quadro sai.
        assert!(l.get("b").is_some());
        l.insert("c".into(), Image::default(), 6);
        assert!(l.get("c").is_some());
        l.frame_done();
        assert_eq!(l.bytes, 12);
        // Fora da tela de novo: volta ao teto.
        assert!(l.get("c").is_some());
        l.frame_done();
        assert_eq!((l.map.contains_key("b"), l.bytes), (false, 6));
    }
}
