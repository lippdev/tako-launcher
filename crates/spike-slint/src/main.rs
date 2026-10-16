//! Spike MIG-001: mede se Slint (via slint-interpreter, tema carregado em tempo de execução)
//! cumpre as metas da PRD §8.1 com uma grade de 1.000 capas.
//!
//! Uso: `spike-slint [--bench] [--theme caminho/main.slint]`.
//! Com `--bench`, navega sozinho pela grade, simula uma sessão de jogo e imprime JSON com os números.

use slint_interpreter::{Compiler, ComponentHandle, Struct, Value};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::time::{Duration, Instant};

const GAMES: usize = 1000;
const COLS: usize = 8;
const COVER_W: u32 = 300;
const COVER_H: u32 = 450;
/// Limite do cache de capas decodificadas (bytes de RGBA), como o LRU por bytes do PRD §4.
const CACHE_BYTES: usize = 96 * 1024 * 1024;

/// Simula decodificação sob demanda: gera a "capa" do jogo i e guarda num LRU por bytes.
#[derive(Default)]
struct CoverCache {
    map: HashMap<usize, slint::Image>,
    order: VecDeque<usize>,
    bytes: usize,
}

impl CoverCache {
    fn get(&mut self, i: usize) -> slint::Image {
        if let Some(img) = self.map.get(&i) {
            return img.clone();
        }
        let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(COVER_W, COVER_H);
        let hue = (i * 37 % 360) as f32;
        for (n, px) in buf.make_mut_slice().iter_mut().enumerate() {
            let y = (n as u32 / COVER_W) as f32 / COVER_H as f32;
            let (r, g, b) = hsv(hue, 0.6, 0.35 + 0.5 * y);
            *px = slint::Rgba8Pixel { r, g, b, a: 255 };
        }
        let img = slint::Image::from_rgba8(buf);
        self.bytes += (COVER_W * COVER_H * 4) as usize;
        self.map.insert(i, img.clone());
        self.order.push_back(i);
        while self.bytes > CACHE_BYTES {
            let Some(old) = self.order.pop_front() else {
                break;
            };
            self.map.remove(&old);
            self.bytes -= (COVER_W * COVER_H * 4) as usize;
        }
        img
    }
    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.bytes = 0;
    }
}

fn hsv(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

/// Modelo de linhas: só diz onde cada linha começa; as capas vêm pelo callback `cover-image`.
struct Rows {
    notify: slint::ModelNotify,
}

impl slint::Model for Rows {
    type Data = Value;
    fn row_count(&self) -> usize {
        GAMES.div_ceil(COLS)
    }
    fn row_data(&self, row: usize) -> Option<Value> {
        if row >= self.row_count() {
            return None;
        }
        let mut r = Struct::default();
        r.set_field("first".into(), Value::Number((row * COLS) as f64));
        r.set_field(
            "count".into(),
            Value::Number((GAMES - row * COLS).min(COLS) as f64),
        );
        Some(Value::Struct(r))
    }
    fn model_tracker(&self) -> &dyn slint::ModelTracker {
        &self.notify
    }
}

fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

fn percentile(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() as f64 - 1.0) * p).round() as usize]
}

#[derive(Default)]
struct Probe {
    frames: Vec<Instant>,
    first_frame: Option<Instant>,
    pending_input: Option<Instant>,
    input_latency_ms: Vec<f64>,
}

fn main() {
    let started = Instant::now();
    let args: Vec<String> = std::env::args().collect();
    let bench = args.iter().any(|a| a == "--bench");
    let theme = args
        .iter()
        .position(|a| a == "--theme")
        .and_then(|i| args.get(i + 1).cloned())
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/theme/main.slint").to_owned());

    let compile_start = Instant::now();
    let result = spin_on::spin_on(Compiler::default().build_from_path(&theme));
    let diags: Vec<_> = result.diagnostics().collect();
    for d in &diags {
        eprintln!("{d}");
    }
    let Some(def) = result.component("App") else {
        eprintln!("tema inválido: componente App não encontrado");
        std::process::exit(2);
    };
    let compile_ms = compile_start.elapsed().as_secs_f64() * 1000.0;
    let app = def.create().expect("criar componente");

    let cache = Rc::new(RefCell::new(CoverCache::default()));
    let rows = Rc::new(Rows {
        notify: Default::default(),
    });
    app.set_property(
        "rows",
        Value::Model(slint::ModelRc::from(
            rows.clone() as Rc<dyn slint::Model<Data = Value>>
        )),
    )
    .expect("rows");

    {
        let cache = cache.clone();
        app.set_callback("cover-image", move |args| match args.first() {
            Some(Value::Number(i)) => Value::Image(cache.borrow_mut().get(*i as usize)),
            _ => Value::Image(Default::default()),
        })
        .expect("cover-image");
    }

    let focus = Rc::new(RefCell::new((0usize, 0usize)));
    let probe = Rc::new(RefCell::new(Probe::default()));

    let nav = {
        let app = app.as_weak();
        let focus = focus.clone();
        let probe = probe.clone();
        move |dir: &str| {
            let Some(app) = app.upgrade() else { return };
            let (mut r, mut c) = *focus.borrow();
            let last_row = GAMES.div_ceil(COLS) - 1;
            match dir {
                "up" => r = r.saturating_sub(1),
                "down" => r = (r + 1).min(last_row),
                "left" => c = c.saturating_sub(1),
                "right" => c = (c + 1).min(COLS - 1),
                _ => {}
            }
            let row_len = (GAMES - r * COLS).min(COLS);
            c = c.min(row_len - 1);
            *focus.borrow_mut() = (r, c);
            probe
                .borrow_mut()
                .pending_input
                .get_or_insert(Instant::now());
            app.set_property("focus-row", Value::Number(r as f64)).ok();
            app.set_property("focus-col", Value::Number(c as f64)).ok();
        }
    };
    let nav = Rc::new(nav);
    {
        let nav = nav.clone();
        app.set_callback("navigate", move |args| {
            if let Some(Value::String(s)) = args.first() {
                nav(s.as_str());
            }
            Value::Void
        })
        .expect("navigate");
    }

    {
        let probe = probe.clone();
        app.window()
            .set_rendering_notifier(move |state, _| {
                if let slint::RenderingState::AfterRendering = state {
                    let now = Instant::now();
                    let mut p = probe.borrow_mut();
                    p.first_frame.get_or_insert(now);
                    p.frames.push(now);
                    if let Some(t) = p.pending_input.take() {
                        p.input_latency_ms.push((now - t).as_secs_f64() * 1000.0);
                    }
                }
            })
            .ok();
    }

    // Controle: SDL3 numa thread própria; eventos vão para o loop do Slint.
    spawn_gamepad_thread(app.as_weak());

    if bench {
        run_bench(
            &app,
            nav.clone(),
            probe.clone(),
            cache.clone(),
            started,
            compile_ms,
        );
    }
    // Esconder a janela durante o jogo não pode encerrar o loop.
    app.show().expect("show");
    slint::run_event_loop_until_quit().expect("loop");
}

fn spawn_gamepad_thread(app: slint::Weak<slint_interpreter::ComponentInstance>) {
    std::thread::spawn(move || {
        let Ok(sdl) = sdl3::init() else { return };
        let Ok(gp) = sdl.gamepad() else { return };
        let Ok(mut pump) = sdl.event_pump() else {
            return;
        };
        let mut open = Vec::new();
        loop {
            // Espera por evento: sem polling ativo (PRD §8.1, CPU durante o jogo).
            let ev = pump.wait_event();
            use sdl3::event::Event;
            match ev {
                Event::GamepadAdded { which, .. } => {
                    if let Ok(g) = gp.open(which) {
                        open.push(g);
                    }
                }
                Event::GamepadButtonDown { button, .. } => {
                    use sdl3::gamepad::Button;
                    let dir = match button {
                        Button::DPadUp => "up",
                        Button::DPadDown => "down",
                        Button::DPadLeft => "left",
                        Button::DPadRight => "right",
                        _ => continue,
                    };
                    let app = app.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(app) = app.upgrade() {
                            app.invoke("navigate", &[Value::String(dir.into())]).ok();
                        }
                    })
                    .ok();
                }
                Event::Quit { .. } => break,
                _ => {}
            }
        }
    });
}

fn run_bench(
    app: &slint_interpreter::ComponentInstance,
    nav: Rc<dyn Fn(&str)>,
    probe: Rc<RefCell<Probe>>,
    cache: Rc<RefCell<CoverCache>>,
    started: Instant,
    compile_ms: f64,
) {
    let app_weak = app.as_weak();
    let step = Rc::new(RefCell::new(0usize));
    let timer = Rc::new(slint::Timer::default());
    let t2 = timer.clone();
    let idle_rss = Rc::new(RefCell::new(0u64));
    let nav_start = Rc::new(RefCell::new(None::<Instant>));
    // 60 passos por segundo: desce a grade inteira e volta, com zigue-zague nas colunas.
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
        let mut s = step.borrow_mut();
        *s += 1;
        let n = *s;
        let rows = GAMES.div_ceil(COLS);
        if n == 1 {
            *nav_start.borrow_mut() = Some(Instant::now());
            probe.borrow_mut().frames.clear();
        }
        if n <= rows * 2 {
            nav(if n < rows { "down" } else { "up" });
            if n.is_multiple_of(3) {
                nav(if n.is_multiple_of(2) { "right" } else { "left" });
            }
            return;
        }
        if n == rows * 2 + 1 {
            *idle_rss.borrow_mut() = rss_kb();
            return;
        }
        if n == rows * 2 + 2 {
            // Simula o jogo: esconde a janela e libera texturas e caches.
            if let Some(app) = app_weak.upgrade() {
                app.hide().ok();
                app.set_property("rows", Value::Model(slint::ModelRc::default())).ok();
            }
            cache.borrow_mut().clear();
            return;
        }
        if n == rows * 2 + 30 {
            // Devolve ao SO a memória já liberada pelo alocador (glibc não faz isso sozinho).
            #[cfg(target_os = "linux")]
            unsafe {
                libc::malloc_trim(0);
            }
            return;
        }
        if n < rows * 2 + 2 + 60 {
            return; // ~1 s parado
        }
        t2.stop();
        let p = probe.borrow();
        let nav_s = nav_start.borrow().map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0);
        let mut deltas: Vec<f64> = p.frames.windows(2).map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0).collect();
        let frames = p.frames.len();
        let mut lat = p.input_latency_ms.clone();
        let first_frame_ms = p.first_frame.map(|t| (t - started).as_secs_f64() * 1000.0).unwrap_or(0.0);
        println!(
            "{{\"compile_ms\":{compile_ms:.1},\"first_frame_ms\":{first_frame_ms:.1},\"frames\":{frames},\"avg_fps\":{:.1},\"frame_ms_p95\":{:.2},\"input_latency_ms_p95\":{:.2},\"rss_idle_mb\":{:.1},\"rss_playing_mb\":{:.1}}}",
            frames as f64 / nav_s,
            percentile(&mut deltas, 0.95),
            percentile(&mut lat, 0.95),
            *idle_rss.borrow() as f64 / 1024.0,
            rss_kb() as f64 / 1024.0,
        );
        slint::quit_event_loop().ok();
    });
    std::mem::forget(timer);
}
