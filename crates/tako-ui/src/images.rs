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
}

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
    pending: HashSet<String>,
    failed: HashSet<String>,
    tx: Option<Sender<Request>>,
    on_ready: Option<Box<dyn Fn()>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        lru: Lru::new(LIMIT),
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
    if uri.is_empty() {
        return None;
    }
    let key = format!("{max_w}x{max_h}|{uri}");
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
        };
        if let Some(tx) = &s.tx
            && tx.send(request).is_ok()
        {
            s.pending.insert(key);
        }
        None
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
                    Some(buf) => {
                        let bytes = (buf.width() * buf.height() * 4) as usize;
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

/// Lê e decodifica; nada aqui pode derrubar a UI (arquivos e downloads são dados não confiáveis).
fn load(req: &Request, resolver: &Resolver) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
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
    Some(SharedPixelBuffer::clone_from_slice(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
    ))
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
