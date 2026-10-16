//! Binário `tako`: inicialização, supervisor e ciclo de vida da sessão (PRD §5.2, ADR-0026).

use std::cell::RefCell;
use std::rc::Rc;
use tako_core::paths::DataPaths;
use tako_platform::Platform;
use tako_ui::app::{App, Host, UiOptions, build_ctx};
use tako_ui::shell::{Ctx, Effect};

/// Pedidos da interface tratados no próprio processo (sem supervisor).
struct LocalHost;

impl Host for LocalHost {
    fn handle(&mut self, effect: &Effect, ctx: &mut Ctx) -> bool {
        match effect {
            Effect::ExitToDesktop => return true,
            Effect::OpenUrl(u) => {
                let _ = ctx.platform.processes.launch(u, None, None);
            }
            Effect::OpenFolder(p) => {
                let _ = ctx.platform.processes.open_folder(p);
            }
            Effect::Launch(id) => {
                if let Some(g) = ctx.library.find(id).cloned()
                    && let Some(target) = &g.launch_target
                {
                    match ctx.platform.processes.launch(
                        target,
                        g.launch_arguments.as_deref(),
                        g.working_directory.as_deref(),
                    ) {
                        Ok(_) => ctx.toast(ctx.tf("session.opening", &[("game", &g.title)])),
                        Err(e) => ctx.toast(e.to_string()),
                    }
                }
            }
            other => tracing::info!("pedido ainda sem tratamento: {other:?}"),
        }
        false
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str| {
        args.iter()
            .position(|a| a == f)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if has("--version") {
        println!("tako {}", tako_ui::version());
        return;
    }
    let simulate = has("--simulate");
    let paths = match value("--data-dir") {
        Some(d) => DataPaths::at(d),
        None => DataPaths::detect(),
    };
    let _ = std::fs::create_dir_all(paths.logs());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let platform = if simulate {
        Platform::simulated()
    } else {
        Platform::native(&paths.root)
    };
    let ctx = build_ctx(paths, platform, simulate);
    let opts = UiOptions {
        simulate,
        ..Default::default()
    };
    let host: Rc<RefCell<dyn Host>> = Rc::new(RefCell::new(LocalHost));
    match App::new(ctx, &opts, host).and_then(|app| app.run()) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("tako: {e}");
            std::process::exit(1);
        }
    }
}
