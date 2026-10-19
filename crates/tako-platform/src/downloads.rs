//! Pasta de downloads do usuário no SO (DL-001: o Tako grava em `<Downloads>/Tako/<título>`).

use std::path::PathBuf;

/// A pasta Downloads do usuário: a pasta conhecida no Windows (mesmo se mudada de lugar), `XDG_DOWNLOAD_DIR` no
/// Linux, `~/Downloads` no resto. Sem nenhuma, a pasta atual.
pub fn downloads_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(p) = known_folder() {
        return p;
    }
    #[cfg(target_os = "linux")]
    if let Some(p) = std::env::var_os("XDG_DOWNLOAD_DIR").filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    home.map_or_else(
        || PathBuf::from("."),
        |h| PathBuf::from(h).join("Downloads"),
    )
}

#[cfg(windows)]
fn known_folder() -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{FOLDERID_Downloads, KF_FLAG_DEFAULT, SHGetKnownFolderPath};
    // SAFETY: o GUID é uma constante estática; o PWSTR devolvido é do sistema e lido uma vez antes de ser
    // liberado com CoTaskMemFree, como a documentação pede.
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None).ok()?;
        let path = p.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(p.0 as *const _));
        path
    }
}
