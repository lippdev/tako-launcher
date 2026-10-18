//! Imagens da interface: decodificadas fora da thread da UI, no tamanho em que aparecem, num LRU por bytes
//! (PRD §4: "decodificação sob demanda, LRU por bytes de GPU"). Durante o jogo o processo da UI termina
//! (ADR-0026), então não há o que liberar aqui além do próprio LRU.

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

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

/// Teto do LRU. Cabe a home inteira (capas de 300x450 ≈ 0,5 MB, fundo ≈ 5–8 MB) com folga para voltar
/// rápido; o que está na tela nunca sai (ver `Lru::evict`), então o teto vale para o que está fora dela.
const LIMIT: usize = 48 * 1024 * 1024;

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

struct State {
    lru: Lru,
    /// Grade de luminância de cada cópia de vidro (pela mesma chave do LRU); poucos bytes por arte.
    lum: HashMap<String, Vec<f32>>,
    pending: HashSet<String>,
    failed: HashSet<String>,
    tx: Option<Sender<Request>>,
    on_ready: Option<Box<dyn Fn()>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        lru: Lru::new(LIMIT),
        lum: HashMap::new(),
        pending: HashSet::new(),
        failed: HashSet::new(),
        tx: None,
        on_ready: None,
    });
}

/// Liga o carregador (na thread da UI). `on_ready` é chamado na thread da UI quando uma imagem chega.
pub fn init(resolver: Resolver, workers: usize, on_ready: impl Fn() + 'static) {
    let (tx, rx) = channel::<Request>();
    let rx = Arc::new(Mutex::new(rx));
    for _ in 0..workers.max(1) {
        let rx = rx.clone();
        let resolver = resolver.clone();
        std::thread::Builder::new()
            .name("tako-images".into())
            .spawn(move || worker(rx, resolver))
            .ok();
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.tx = Some(tx);
        s.on_ready = Some(Box::new(on_ready));
    });
}

/// Imagem pronta, ou `None` (e o pedido entra na fila). `max_w`/`max_h` em pixels físicos.
pub fn get(uri: &str, max_w: u32, max_h: u32) -> Option<Image> {
    request(uri, max_w, max_h, false)
}

/// Cópia de vidro da arte do desktop (Liquid Glass sem desfoque em tempo real): desfocada e saturada uma vez,
/// em thread de fundo, com a luminância média em `GLASS_GRID` (linhas de cima para baixo). `None` até ficar pronta.
pub fn glass(uri: &str) -> Option<(Image, Vec<f32>)> {
    let img = request(uri, 0, GLASS_HEIGHT, true)?;
    let key = format!("glass|{uri}");
    let lum = STATE.with(|s| s.borrow().lum.get(&key).cloned())?;
    Some((img, lum))
}

fn request(uri: &str, max_w: u32, max_h: u32, glass: bool) -> Option<Image> {
    if uri.is_empty() {
        return None;
    }
    let key = if glass {
        format!("glass|{uri}")
    } else {
        format!("{max_w}x{max_h}|{uri}")
    };
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(img) = s.lru.get(&key) {
            return Some(img);
        }
        if s.pending.contains(&key) || s.failed.contains(&key) {
            return None;
        }
        let request = Request {
            key: key.clone(),
            uri: uri.to_owned(),
            max_w,
            max_h,
            glass,
        };
        if let Some(tx) = &s.tx
            && tx.send(request).is_ok()
        {
            s.pending.insert(key);
        }
        None
    })
}

/// A imagem (em qualquer tamanho, ou a cópia de vidro) ainda está a caminho.
pub fn is_pending(uri: &str) -> bool {
    let tail = format!("|{uri}");
    STATE.with(|s| s.borrow().pending.iter().any(|k| k.ends_with(&tail)))
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

fn worker(rx: Arc<Mutex<Receiver<Request>>>, resolver: Resolver) {
    loop {
        let req = {
            let Ok(rx) = rx.lock() else { return };
            match rx.recv() {
                Ok(r) => r,
                Err(_) => return,
            }
        };
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
                        s.lru.insert(key, Image::from_rgba8(buf), bytes);
                    }
                    None => {
                        s.failed.insert(key);
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
        if sent.is_err() {
            return;
        }
    }
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
