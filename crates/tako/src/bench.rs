//! Benchmark (MIG-041, PRD §8.1): abre a interface de verdade com 1.000 jogos, navega pela grade, mede a
//! primeira tela, os quadros e a resposta ao teclado, e a memória em repouso; depois mede a memória do
//! supervisor sozinho (o que fica durante o jogo, ADR-0026). Imprime uma linha JSON.
//! Números do CI só mostram tendência: nada aqui reprova a build, só travar ou não abrir.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tako_core::library::{Game, GameStore};
use tako_core::paths::DataPaths;
use tako_platform::Platform;
use tako_ui::app::{App, Host, HostAction, UiOptions, build_ctx};
use tako_ui::shell::{Cmd, Ctx, Effect};

struct NoHost;
impl Host for NoHost {
    fn handle(&mut self, _: &Effect, _: &mut Ctx) -> HostAction {
        HostAction::Stay
    }
}

pub fn rss_kb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let s = std::fs::read_to_string("/proc/self/status").ok()?;
        s.lines()
            .find(|l| l.starts_with("VmRSS:"))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn games(n: usize) -> Vec<Game> {
    let stores = [
        GameStore::Steam,
        GameStore::Epic,
        GameStore::Gog,
        GameStore::Manual,
    ];
    (0..n)
        .map(|i| Game {
            id: format!("bench:{i}"),
            title: format!("Jogo {i:04}"),
            store: Some(stores[i % stores.len()]),
            accent: format!(
                "#{:02X}{:02X}{:02X}",
                (i * 37) % 200 + 30,
                (i * 53) % 200 + 30,
                (i * 71) % 200 + 30
            ),
            is_installed: true,
            ..Default::default()
        })
        .collect()
}

#[derive(Default)]
struct Probe {
    frames: Vec<Instant>,
    first_frame: Option<Instant>,
    pending: Option<Instant>,
    latency_ms: Vec<f64>,
}

fn p95(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() as f64 - 1.0) * 0.95).round() as usize]
}

pub fn run(started: Instant, data_dir: Option<String>) -> i32 {
    let dir = data_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("tako-bench"));
    let paths = DataPaths::at(&dir);
    let ctx = build_ctx(paths, Platform::simulated(), true);
    if let Ok(mut f) = ctx.found.lock() {
        *f = games(1000);
    }
    let mut ctx = ctx;
    ctx.library.refresh();
    let host: Rc<RefCell<dyn Host>> = Rc::new(RefCell::new(NoHost));
    let app = match App::new(
        ctx,
        &UiOptions {
            simulate: true,
            // As metas de memória e de quadros (PERF-001) são medidas na tela cheia.
            mode: tako_ui::mode::UiMode::BigPicture,
            ..Default::default()
        },
        host,
    ) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("bench: {e}");
            return 1;
        }
    };
    let probe = Rc::new(RefCell::new(Probe::default()));
    {
        let probe = probe.clone();
        use slint::ComponentHandle;
        let _ = app
            .ui
            .borrow()
            .window()
            .set_rendering_notifier(move |state, _| {
                if let slint::RenderingState::AfterRendering = state {
                    let now = Instant::now();
                    let mut p = probe.borrow_mut();
                    p.first_frame.get_or_insert(now);
                    p.frames.push(now);
                    if let Some(t) = p.pending.take() {
                        p.latency_ms.push((now - t).as_secs_f64() * 1000.0);
                    }
                }
            });
    }
    // Biblioteca inteira, navegando a ~30 passos por segundo (repetição do direcional).
    app.shell.borrow_mut().open_route("library:all");
    let step = Rc::new(RefCell::new(0usize));
    let nav_start = Rc::new(RefCell::new(None::<Instant>));
    let idle = Rc::new(RefCell::new(None::<u64>));
    let timer = slint::Timer::default();
    let app2 = app.clone();
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(33), move || {
        let n = {
            let mut s = step.borrow_mut();
            *s += 1;
            *s
        };
        if n == 30 {
            *nav_start.borrow_mut() = Some(Instant::now());
            probe.borrow_mut().frames.clear();
        }
        if (30..330).contains(&n) {
            let cmd = if n < 230 { Cmd::Down } else if n % 2 == 0 { Cmd::Right } else { Cmd::Up };
            probe.borrow_mut().pending.get_or_insert(Instant::now());
            app2.shell.borrow_mut().handle(cmd);
            app2.redraw();
            return;
        }
        if n == 360 {
            *idle.borrow_mut() = rss_kb();
            return;
        }
        if n == 361 {
            let p = probe.borrow();
            let secs = nav_start.borrow().map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0);
            let mut deltas: Vec<f64> = p.frames.windows(2).map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0).collect();
            let mut lat = p.latency_ms.clone();
            let first = p.first_frame.map(|t| (t - started).as_secs_f64() * 1000.0).unwrap_or(0.0);
            let json = format!(
                "{{\"games\":1000,\"first_frame_ms\":{first:.0},\"avg_fps\":{:.1},\"frame_ms_p95\":{:.1},\"input_latency_ms_p95\":{:.1},\"rss_idle_mb\":{}}}",
                p.frames.len() as f64 / secs,
                p95(&mut deltas),
                p95(&mut lat),
                idle.borrow().map(|k| format!("{:.1}", k as f64 / 1024.0)).unwrap_or("null".into()),
            );
            println!("{json}");
            let _ = slint::quit_event_loop();
        }
    });
    std::mem::forget(timer);
    match app.run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("bench: {e}");
            1
        }
    }
}

/// Memória do supervisor durante o jogo: o mesmo binário, sem janela, acompanhando um jogo simulado.
pub fn supervisor_rss() -> i32 {
    let platform = Platform::simulated();
    let req = tako_ui::session::TrackRequest {
        game_id: "bench:0".into(),
        title: "bench".into(),
        root_pid: Some(1),
        strategy: tako_core::library::GameTrackingStrategy::Process,
        folder: None,
        steam_app_id: None,
        launched_at: chrono::Utc::now(),
        hide_cursor: false,
        controller_mouse: false,
        restore: false,
        discord: false,
        after_command: None,
        store: None,
        close_tools: Vec::new(),
    };
    let mut running = match tako_ui::session::Running::new(req) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("bench: {e}");
            return 1;
        }
    };
    let pads = tako_ui::gamepad::Pads::open().ok();
    for _ in 0..3 {
        let _ = running.poll(&platform);
        std::thread::sleep(Duration::from_millis(200));
    }
    drop(pads);
    println!(
        "{{\"supervisor_rss_mb\":{}}}",
        rss_kb()
            .map(|k| format!("{:.1}", k as f64 / 1024.0))
            .unwrap_or("null".into())
    );
    0
}
