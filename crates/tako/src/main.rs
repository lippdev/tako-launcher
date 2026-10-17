//! Binário `tako`: inicialização, supervisor e ciclo de vida da sessão (PRD §5.2, ADR-0026).
//!
//! `tako` (versão publicada) = supervisor, que abre `tako --ui`; `tako --no-supervisor` = só a interface.
//! `--simulate` usa a biblioteca de demonstração e funções de plataforma em memória.

// Sem janela de terminal no Windows (supervisor e interface rodam em segundo plano). Builds de desenvolvimento
// mantêm o console para os logs.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod bench;
mod hosts;
mod protocol;
mod supervisor;

use std::cell::RefCell;
use std::io::BufReader;
use std::rc::Rc;
use std::time::Duration;
use tako_core::paths::DataPaths;
use tako_core::supervision::{ProcessRole, args};
use tako_platform::Platform;
use tako_ui::app::{App, Host, UiOptions, build_ctx};

fn value(all: &[String], flag: &str) -> Option<String> {
    all.iter()
        .position(|a| a == flag)
        .and_then(|i| all.get(i + 1))
        .cloned()
}

fn init_logs(paths: &DataPaths, role: &str) {
    use tracing_subscriber::prelude::*;
    let _ = std::fs::create_dir_all(paths.logs());
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok();
    let redactor = tako_core::diagnostics::LogRedactor::new(home.as_deref());
    let file = tako_core::diagnostics::file_layer(tako_core::diagnostics::DailyFile::new(
        paths.logs(),
        redactor,
    ));
    let stderr = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file)
        .with(stderr)
        .try_init();
    tracing::info!("tako {} ({role}) iniciando", tako_ui::version());
}

/// Definida no Tako reaberto por ele mesmo (saída do modo seguro): espera o anterior soltar a trava.
pub const RESTART_ENV: &str = "TAKO_RESTARTED";

/// Pedido do segundo Tako ao que já está aberto (o supervisor apaga ao atender).
pub const ACTIVATE_FILE: &str = "tako.activate";

/// Só um Tako por usuário: um segundo `tako` sai sem abrir outra janela.
fn single_instance(paths: &DataPaths) -> Option<std::fs::File> {
    let _ = std::fs::create_dir_all(&paths.root);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(paths.root.join("tako.lock"))
        .ok()?;
    // shortcut: o segundo Tako só sai; trazer a janela do primeiro para a frente fica para quando houver IPC nomeado.
    let tries = if std::env::var_os(RESTART_ENV).is_some() {
        50
    } else {
        1
    };
    for _ in 1..tries {
        if file.try_lock().is_ok() {
            return Some(file);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    file.try_lock().ok().map(|_| file)
}

fn main() {
    let started = std::time::Instant::now();
    tako_platform::tune_allocator();
    let all: Vec<String> = std::env::args().collect();
    let has = |f: &str| args::has(&all, f);
    if has("--bench") || has("--bench-supervisor") || has("--version") {
        tako_platform::attach_parent_console();
    }
    if has("--bench") {
        std::process::exit(bench::run(started, value(&all, "--data-dir")));
    }
    if has("--bench-supervisor") {
        std::process::exit(bench::supervisor_rss());
    }
    if has("--version") {
        println!("tako {}", tako_ui::version());
        return;
    }
    let simulate = has("--simulate");
    let paths = match value(&all, "--data-dir") {
        Some(d) => DataPaths::at(d),
        None => DataPaths::detect(),
    };
    let published = !cfg!(debug_assertions);
    let role = args::resolve_role(
        &all[1..],
        std::env::var(args::ENV_VAR).ok().as_deref(),
        published && !simulate,
    );
    let platform = if simulate {
        Platform::simulated()
    } else {
        Platform::native(&paths.root)
    };
    let supervised = args::pipe_name(&all[1..]).is_some();

    if role == ProcessRole::Supervisor {
        init_logs(&paths, "supervisor");
        let Some(_lock) = single_instance(&paths) else {
            tracing::info!("o Tako já está aberto; pedindo para ele vir para a frente");
            let _ = std::fs::write(paths.root.join(ACTIVATE_FILE), b"");
            return;
        };
        let code = supervisor::Supervisor::new(paths, platform, &all).run(has(args::SAFE_MODE));
        std::process::exit(code);
    }

    init_logs(&paths, "ui");
    let _lock = if supervised {
        None
    } else {
        match single_instance(&paths) {
            Some(l) => Some(l),
            None => {
                // shortcut: sem supervisor (desenvolvimento, --no-supervisor) ninguém atende o pedido de vir para
                // a frente; só a versão publicada (com supervisor) ativa a janela aberta.
                tracing::info!("o Tako já está aberto");
                return;
            }
        }
    };
    if !supervised && !simulate {
        tako_ui::session::recover_after_crash(&platform, &paths);
    }
    let mut ctx = build_ctx(paths, platform, simulate);
    ctx.controller_mouse = has("--controller-mouse");
    let after_game = value(&all, "--after-game").map(|id| {
        (
            id,
            Duration::from_secs(
                value(&all, "--played")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            ),
        )
    });
    let running =
        value(&all, "--running").map(|id| (id, value(&all, "--running-title").unwrap_or_default()));
    let opts = UiOptions {
        simulate,
        after_game,
        running,
        control_center: has("--control-center"),
        safe_mode: has(args::SAFE_MODE),
    };
    let host: Rc<RefCell<dyn Host>> = if supervised {
        Rc::new(RefCell::new(hosts::SupervisedHost::new()))
    } else {
        Rc::new(RefCell::new(hosts::LocalHost::default()))
    };
    let app = match App::new(ctx, &opts, host) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("tako: {e}");
            std::process::exit(1);
        }
    };
    if supervised {
        // Mensagens do supervisor (o jogo em segundo plano terminou).
        std::thread::spawn(|| {
            protocol::read_lines::<protocol::SupMsg>(BufReader::new(std::io::stdin()), |m| {
                let (game_id, played_secs) = match m {
                    protocol::SupMsg::GameEnded {
                        game_id,
                        played_secs,
                    } => (game_id, played_secs),
                    protocol::SupMsg::Activate => {
                        let _ = slint::invoke_from_event_loop(tako_ui::app::raise_current);
                        return;
                    }
                    protocol::SupMsg::GameShown => {
                        let _ = slint::invoke_from_event_loop(tako_ui::app::game_shown);
                        return;
                    }
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(shell) = tako_ui::tasks::shell() {
                        let mut s = shell.borrow_mut();
                        let played = Duration::from_secs(played_secs);
                        let _ = s.ctx.library.record_session(&game_id, played);
                        s.ctx.running = None;
                        s.ctx.welcome_back = Some((game_id, played));
                        s.ctx.dirty = true;
                    }
                });
            });
        });
    }
    if let Err(e) = app.run() {
        eprintln!("tako: {e}");
        std::process::exit(1);
    }
}
