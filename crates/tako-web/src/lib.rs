//! Hospedeiro da interface web do Tako (PRD D-09, D-10; ADR-0035). Contrato em `docs/web/ponte.md`.
//!
//! Aqui fica o que não depende do motor: a ponte (`bridge`), a validação dos esquemas próprios (`schemes`) e o
//! transporte entre processos (`ipc`). O hospedeiro CEF (WEB-02a) e a variante WebView2 (MIG-W01) usam estas
//! peças e só cuidam de janela, processos e transporte das strings.
//!
//! Dois processos (WEB-02b, D-10): `tako --web` é o núcleo (`core_process`, sem CEF) e abre `tako --web-ui`,
//! a interface descartável (`windows_host`, com o CEF). Detalhes em `docs/web/hospedeiro.md`.

pub mod bridge;
pub mod ipc;
pub mod schemes;

#[cfg(any(all(windows, feature = "cef"), test))]
mod core_process;
#[cfg(any(all(windows, feature = "cef"), test))]
mod resources;
// Nos testes sem CEF só `dev_origin` é usado.
#[cfg(any(all(windows, feature = "cef"), test))]
#[cfg_attr(not(all(windows, feature = "cef")), allow(dead_code))]
mod target;
#[cfg(all(windows, feature = "cef"))]
mod windows_host;

pub use ipc::UI_ARG;

/// Executa subprocessos CEF antes de logs, SDL, trava de instância ou supervisor.
pub fn execute_subprocess() -> Option<i32> {
    #[cfg(all(windows, feature = "cef"))]
    if std::env::args_os().any(|arg| arg.to_str().is_some_and(|arg| arg.starts_with("--type="))) {
        if let Err(error) = std::env::current_exe()
            .map_err(|e| e.to_string())
            .and_then(|exe| check_runtime(&exe))
        {
            eprintln!("Tako: {error}");
            return Some(1);
        }
        return Some(windows_host::execute_subprocess());
    }
    None
}

#[cfg(any(all(windows, feature = "cef"), test))]
fn check_runtime(exe: &std::path::Path) -> Result<(), String> {
    let runtime = exe.with_file_name("libcef.dll");
    if runtime.is_file() {
        return Ok(());
    }
    Err(format!(
        "runtime CEF ausente: {}; prepare com scripts/cef-setup.ps1 e recompile com --features web",
        runtime.display()
    ))
}

#[cfg(not(all(windows, feature = "cef")))]
const UNAVAILABLE: &str =
    "hospedeiro CEF disponível somente no Windows, com a feature cef, nesta etapa";

/// Processo do núcleo (`tako --web`): abre o `AppService` e a interface em processo próprio. Não carrega
/// a `libcef.dll` (carga adiada pelo `/DELAYLOAD`; aqui só se confere que o arquivo existe). `data_dir` é o
/// `--data-dir` recebido, repassado ao filho. Sem `--web` o caminho Slint continua no binário.
pub fn run(paths: tako_core::paths::DataPaths, data_dir: Option<String>) -> Result<(), String> {
    #[cfg(all(windows, feature = "cef"))]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        check_runtime(&exe)?;
        target::resolve()?;
        let mut args = vec![UI_ARG.to_owned()];
        if let Some(dir) = data_dir {
            args.extend(["--data-dir".to_owned(), dir]);
        }
        tracing::info!(pid = std::process::id(), "núcleo do --web sem CEF");
        let version = env!("CARGO_PKG_VERSION");
        // O segundo `tako --web` (ou o Slint) deixa o pedido de ativação nesta pasta (WEB-34).
        let activation_root = paths.root.clone();
        core_process::run(
            core_process::ExeLauncher { exe, args },
            move || {
                let platform = tako_platform::Platform::native(&paths.machine);
                tako_app::Core::open(paths, platform, false)
                    .map(|core| tako_app::AppService::with_core(version, core))
                    .map_err(|e| e.to_string())
            },
            core_process::Timings::default(),
            |control| {
                core_process::spawn_pads(control.clone());
                core_process::watch_activation(activation_root, control.clone());
                if let Some(delay) = recycle_delay() {
                    std::thread::spawn(move || recycle(&control, delay));
                }
            },
        )
    }
    #[cfg(not(all(windows, feature = "cef")))]
    {
        let _ = (paths, data_dir);
        Err(UNAVAILABLE.into())
    }
}

/// Processo da interface (`tako --web-ui`): CEF e janela. Só o núcleo o abre, com pipes na entrada e na saída.
pub fn run_ui(paths: tako_core::paths::DataPaths) -> Result<(), String> {
    if std::env::var_os(ipc::CORE_PID_ENV).is_none() {
        return Err(format!("{UI_ARG} só é aberto pelo tako --web"));
    }
    #[cfg(all(windows, feature = "cef"))]
    return windows_host::run(paths);
    #[cfg(not(all(windows, feature = "cef")))]
    {
        let _ = paths;
        Err(UNAVAILABLE.into())
    }
}

/// `TAKO_WEB_RECYCLE_MS` (só debug, 100 a 300000): o núcleo fecha a interface esse tempo depois de abrir e a
/// reabre o mesmo tempo depois, uma vez, pelo mesmo caminho que a sessão de jogo usa. Para testar e medir.
#[cfg(all(windows, feature = "cef"))]
fn recycle(control: &std::sync::mpsc::Sender<core_process::Command>, delay: std::time::Duration) {
    std::thread::sleep(delay);
    tracing::info!("TAKO_WEB_RECYCLE_MS: fechando a interface");
    let _ = control.send(core_process::Command::Hide);
    std::thread::sleep(delay);
    tracing::info!("TAKO_WEB_RECYCLE_MS: reabrindo a interface");
    let _ = control.send(core_process::Command::Show);
}

#[cfg(all(windows, feature = "cef"))]
fn recycle_delay() -> Option<std::time::Duration> {
    if !cfg!(debug_assertions) {
        return None;
    }
    std::env::var("TAKO_WEB_RECYCLE_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| (100..=300_000).contains(v))
        .map(std::time::Duration::from_millis)
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_runtime_is_an_error_before_loading_cef() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("tako.exe");
        assert!(
            super::check_runtime(&exe)
                .unwrap_err()
                .contains("runtime CEF ausente")
        );
        std::fs::write(dir.path().join("libcef.dll"), []).unwrap();
        assert!(super::check_runtime(&exe).is_ok());
    }
}
