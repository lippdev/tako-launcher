//! Identidade local de desenvolvimento para o Windows Full Screen Experience (FSE-001).

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
    use std::{io::Cursor, os::windows::process::CommandExt, process::Command};

    pub(super) fn run(action: &str) -> Result<String, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let layout = std::path::PathBuf::from(
            std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA indisponível")?,
        )
        .join("Tako")
        .join("GamingHome");
        std::fs::create_dir_all(&layout).map_err(|e| e.to_string())?;
        // Protect extraction too: two launcher instances must not replace a live manifest.
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
        if action == "enable" && !layout.join("previous.clixml").exists() {
            let filename = exe
                .file_name()
                .ok_or("Executável inválido")?
                .to_string_lossy();
            let escaped = super::xml_escape(&filename);
            std::fs::write(
                layout.join("AppxManifest.xml"),
                super::MANIFEST.replace("__EXECUTABLE__", &escaped),
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(layout.join("CustomCapability.SCCD"), super::SCCD)
                .map_err(|e| e.to_string())?;
            std::fs::create_dir_all(layout.join("Public")).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(layout.join("Assets")).map_err(|e| e.to_string())?;
            let logo = image::load_from_memory(include_bytes!("../../tako-ui/theme/tako-logo.png"))
                .map_err(|e| e.to_string())?;
            for (name, size) in [
                ("StoreLogo", 50),
                ("Square150x150Logo", 150),
                ("Square44x44Logo", 44),
            ] {
                let mut png = Cursor::new(Vec::new());
                logo.resize_exact(size, size, image::imageops::FilterType::Lanczos3)
                    .write_to(&mut png, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
                std::fs::write(
                    layout.join("Assets").join(format!("{name}.png")),
                    png.into_inner(),
                )
                .map_err(|e| e.to_string())?;
            }
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

#[cfg(any(windows, test))]
const MANIFEST: &str = include_str!("../../../packaging/fse/AppxManifest.xml");
#[cfg(any(windows, test))]
const SCCD: &str = include_str!("../../../packaging/fse/CustomCapability.SCCD");
#[cfg(any(windows, test))]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
        assert!(MANIFEST.contains("windows.gamingApp"));
        assert!(!MANIFEST.contains("EntryPoint="));
        assert!(MANIFEST.contains("uap10:Parameters=\"--big-picture\""));
        assert!(MANIFEST.contains("<uap10:AllowExternalContent>true"));
        assert!(SCCD.contains("<Catalog>FFFF</Catalog>"));
        assert!(SCCD.contains("Microsoft.appCategory.gamingHome_8wekyb3d8bbwe"));
        assert_eq!(xml_escape("a&\"'<>.exe"), "a&amp;&quot;&apos;&lt;&gt;.exe");
    }
}
