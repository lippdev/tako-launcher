//! Regras locais da atualização que não dependem de interface (WEB-33): como o Tako foi instalado, para onde vai
//! a troca, as novidades da release e a execução do plano de aplicação. Servem o app Slint e a interface web.

use std::io;
use std::path::{Path, PathBuf};

use super::apply::{UpdateApplyPlan, mac_bundle_of};
use super::feed::Package;
use crate::paths::{DataPaths, Os};

/// Quantas novidades da release cabem no aviso, e o tamanho de cada uma.
const MAX_NOTES: usize = 3;
const MAX_NOTE_CHARS: usize = 150;

/// O tipo de instalação desta execução (instalada, portátil, AppImage…), que decide o arquivo da release.
pub fn detect_package(paths: &DataPaths) -> Package {
    let exe = std::env::current_exe().unwrap_or_default();
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
    let flatpak = std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists();
    Package::detect(
        Os::current(),
        paths.kind,
        &exe,
        appimage.as_deref(),
        flatpak,
    )
}

/// O que a troca substitui: o `.exe`, o `.AppImage`, a pasta do `.tar.gz` ou o bundle `.app`.
pub fn apply_target(package: Package, exe: &Path, appimage: Option<&Path>) -> PathBuf {
    match package {
        Package::MacApp => mac_bundle_of(exe).unwrap_or_else(|| exe.to_path_buf()),
        Package::LinuxAppImage => appimage.map_or_else(|| exe.to_path_buf(), Path::to_path_buf),
        Package::LinuxTarGz => exe.parent().map(Path::to_path_buf).unwrap_or_default(),
        _ => exe.to_path_buf(),
    }
}

/// PID do supervisor (definido por ele no ambiente da interface Slint); ausente na versão web.
fn supervisor_pid() -> Option<u32> {
    std::env::var("TAKO_SUPERVISOR_PID").ok()?.parse().ok()
}

/// Roda o plano de troca (instalador ou script) para o arquivo já baixado e conferido. O script espera este
/// processo (e o supervisor, se houver) sair: quem chama deve encerrar o Tako logo depois. `Ok(false)` = esta
/// instalação não se atualiza sozinha (`.deb`) e nada foi feito.
pub fn install(paths: &DataPaths, downloaded: &Path, relaunch: bool) -> io::Result<bool> {
    let exe = std::env::current_exe().unwrap_or_default();
    let appimage = std::env::var_os("APPIMAGE").map(PathBuf::from);
    let package = detect_package(paths);
    let target = apply_target(package, &exe, appimage.as_deref());
    let mut wait = vec![std::process::id()];
    wait.extend(supervisor_pid());
    let Some(plan) = UpdateApplyPlan::build(package, downloaded, &target, &wait, relaunch) else {
        return Ok(false);
    };
    if let Some((path, body)) = &plan.script {
        std::fs::write(path, body)?;
    }
    std::process::Command::new(&plan.program)
        .args(&plan.args)
        .spawn()?;
    Ok(true)
}

/// As primeiras novidades da release: só os itens de lista (`- ` ou `* `), em até três linhas curtas.
pub fn release_notes(notes: &str) -> Vec<String> {
    notes
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .map(|s| s.chars().take(MAX_NOTE_CHARS).collect())
        })
        .take(MAX_NOTES)
        .collect()
}

/// O script de troca deixa `failed` ao lado do download quando não conseguiu instalar (a versão antiga foi
/// restaurada). Lê e apaga o marcador: `true` uma vez só.
pub fn take_failed(dir: &Path) -> bool {
    std::fs::remove_file(dir.join("failed")).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_limit_and_ignore_headers() {
        assert_eq!(
            release_notes("# Changes\n- Um\n* Dois\n- Três\n- Quatro"),
            vec!["Um", "Dois", "Três"]
        );
        let long = format!("- {}", "a".repeat(400));
        assert_eq!(release_notes(&long)[0].chars().count(), MAX_NOTE_CHARS);
        assert!(release_notes("sem lista").is_empty());
    }

    #[test]
    fn failed_marker_is_consumed_once() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!take_failed(dir.path()));
        std::fs::write(dir.path().join("failed"), "").unwrap();
        assert!(take_failed(dir.path()));
        assert!(!take_failed(dir.path()));
    }

    #[test]
    fn the_target_follows_the_package() {
        let exe = Path::new("/Applications/Tako.app/Contents/MacOS/tako");
        assert_eq!(
            apply_target(Package::MacApp, exe, None),
            Path::new("/Applications/Tako.app")
        );
        assert_eq!(
            apply_target(
                Package::LinuxAppImage,
                Path::new("/tmp/.mount/tako"),
                Some(Path::new("/home/a/Tako.AppImage"))
            ),
            Path::new("/home/a/Tako.AppImage")
        );
        assert_eq!(
            apply_target(Package::LinuxTarGz, Path::new("/opt/tako/tako"), None),
            Path::new("/opt/tako")
        );
        assert_eq!(
            apply_target(Package::WindowsPortable, Path::new("D:/Tako.exe"), None),
            Path::new("D:/Tako.exe")
        );
    }
}
