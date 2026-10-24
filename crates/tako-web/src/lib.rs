//! Hospedeiro da interface web do Tako (PRD D-09, D-10; ADR-0035). Contrato em `docs/web/ponte.md`.
//!
//! Aqui fica o que não depende do motor: a ponte (`bridge`) e a validação dos esquemas próprios (`schemes`). O
//! hospedeiro CEF (WEB-02a) e a variante WebView2 (MIG-W01) usam estas peças e só cuidam de janela, processos e
//! transporte das strings.

pub mod bridge;
pub mod schemes;

#[cfg(any(all(windows, feature = "cef"), test))]
mod resources;
#[cfg(all(windows, feature = "cef"))]
mod windows_host;

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

/// Entrada optativa; sem `--web` o caminho Slint continua no binário.
pub fn run(paths: tako_core::paths::DataPaths) -> Result<(), String> {
    #[cfg(all(windows, feature = "cef"))]
    return windows_host::run(paths);
    #[cfg(not(all(windows, feature = "cef")))]
    {
        let _ = paths;
        Err("hospedeiro CEF disponível somente no Windows, com a feature cef, nesta etapa".into())
    }
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
