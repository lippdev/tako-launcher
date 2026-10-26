//! Identidade esparsa assinada para o Windows Full Screen Experience (FSE-002).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Enabled,
    Disabled,
    RecoveryRequired,
    Unavailable,
    Applying,
}

#[cfg(any(windows, test))]
fn parse_status(output: &str) -> Result<Status, String> {
    match output.trim() {
        "applying" => Ok(Status::Applying),
        "enabled" => Ok(Status::Enabled),
        "disabled" => Ok(Status::Disabled),
        "recovery" => Ok(Status::RecoveryRequired),
        "unavailable" => Ok(Status::Unavailable),
        _ => Err("Resposta inválida ao consultar o modo tela cheia".into()),
    }
}

/// Consulta conservadora: não presume disponibilidade apenas pela versão do Windows.
pub fn status() -> Result<Status, String> {
    #[cfg(windows)]
    return native::run("status").and_then(|s| parse_status(&s));
    #[cfg(not(windows))]
    Ok(Status::Unavailable)
}

/// Registra o Tako e preserva o estado anterior antes de alterar o Windows.
pub fn enable() -> Result<(), String> {
    change(true, native_run)
}

/// Restaura o estado anterior; o backup permanece disponível se a recuperação falhar.
pub fn disable() -> Result<(), String> {
    change(false, native_run)
}

fn change(enabled: bool, run: impl FnOnce(&str) -> Result<String, String>) -> Result<(), String> {
    run(if enabled { "enable" } else { "disable" }).map(|_| ())
}

fn native_run(action: &str) -> Result<String, String> {
    #[cfg(windows)]
    return native::run(action);
    #[cfg(not(windows))]
    {
        let _ = action;
        Err("Modo tela cheia não disponível neste sistema".into())
    }
}

#[cfg(windows)]
mod native {
    use super::{CERTIFICATE, INSTALLED, PORTABLE, package_variant};
    use std::{os::windows::process::CommandExt, process::Command};

    pub(super) fn run(action: &str) -> Result<String, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let layout = std::path::PathBuf::from(
            std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA indisponível")?,
        )
        .join("Tako")
        .join("GamingHome");
        std::fs::create_dir_all(&layout).map_err(|e| e.to_string())?;
        // A extração também fica protegida contra outra instância do launcher.
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(layout.join("launcher.lock"))
            .map_err(|e| e.to_string())?;
        if let Err(error) = lock.try_lock() {
            if action == "status" && matches!(error, std::fs::TryLockError::WouldBlock) {
                return Ok("applying".into());
            }
            return Err(error.to_string());
        }
        let script = layout.join("Setup.ps1");
        std::fs::write(&script, include_str!("fse.ps1")).map_err(|e| e.to_string())?;
        if action == "enable" {
            let portable = package_variant(exe.file_name().and_then(|s| s.to_str()).unwrap_or(""))?;
            let (msix, cer) = if portable {
                (PORTABLE, CERTIFICATE)
            } else {
                (INSTALLED, CERTIFICATE)
            };
            if msix.is_empty() || cer.is_empty() {
                return Err(
                    "Pacote FSE assinado ausente neste build; use uma distribuição Windows oficial"
                        .into(),
                );
            }
            std::fs::write(layout.join("Tako-GamingHome.msix"), msix).map_err(|e| e.to_string())?;
            std::fs::write(layout.join("Tako-GamingHome.cer"), cer).map_err(|e| e.to_string())?;
        }
        let powershell = std::path::PathBuf::from(
            std::env::var_os("SystemRoot").ok_or("SystemRoot indisponível")?,
        )
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let output = Command::new(powershell)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .arg("-Action")
            .arg(action)
            .arg("-Layout")
            .arg(&layout)
            .arg("-Executable")
            .arg(exe)
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[cfg(windows)]
include!(concat!(env!("OUT_DIR"), "/fse_payload.rs"));

#[cfg(any(windows, test))]
fn package_variant(filename: &str) -> Result<bool, String> {
    if filename.eq_ignore_ascii_case("Tako.exe") {
        Ok(false)
    } else if filename.eq_ignore_ascii_case("Tako-Portable-x64.exe") {
        Ok(true)
    } else {
        Err("Para ativar o FSE, mantenha o nome Tako.exe ou Tako-Portable-x64.exe".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_fail_closed() {
        assert_eq!(parse_status("enabled\r\n"), Ok(Status::Enabled));
        assert_eq!(parse_status("disabled"), Ok(Status::Disabled));
        assert_eq!(parse_status("applying"), Ok(Status::Applying));
        assert_eq!(parse_status("unavailable"), Ok(Status::Unavailable));
        assert!(parse_status("garbage").is_err());
    }

    #[test]
    fn commands_are_fixed_and_errors_propagate() {
        change(true, |action| {
            assert_eq!(action, "enable");
            Ok(String::new())
        })
        .expect("enable");
        assert_eq!(
            change(false, |action| {
                assert_eq!(action, "disable");
                Err("UAC cancelled".into())
            }),
            Err("UAC cancelled".into())
        );
    }

    #[test]
    fn identity_has_home_extension_and_development_catalog() {
        let manifest = include_str!("../../../packaging/fse/AppxManifest.xml");
        assert!(manifest.contains("windows.gamingApp"));
        assert!(manifest.contains("EntryPoint=\"Windows.FullTrustApplication\""));
        assert!(manifest.contains("uap18:RuntimeBehavior=\"win32App\""));
        assert!(manifest.contains("MinVersion=\"10.0.26100.8039\""));
        assert!(
            include_str!("../../../packaging/fse/CustomCapability.SCCD")
                .contains("<Catalog>FFFF</Catalog>")
        );
    }

    #[test]
    fn package_matches_actual_executable() {
        assert_eq!(package_variant("Tako.exe"), Ok(false));
        assert_eq!(package_variant("tako.EXE"), Ok(false));
        assert_eq!(package_variant("Tako-Portable-x64.exe"), Ok(true));
        assert!(package_variant("renamed.exe").is_err());
        assert!(package_variant("../Tako.exe").is_err());
    }
}
