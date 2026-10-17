//! AnyFSE (PLAT-007): projeto da comunidade que põe um launcher de terceiros como app inicial do modo Xbox (a tela
//! cheia de jogos do Windows 11). O Tako só detecta e ensina a configurar; não instala nem liga nada.

/// Página do projeto (instalador e instruções).
pub const PAGE: &str = "https://github.com/ashpynov/AnyFSE";

const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\AnyFSE";

/// Instalado pelo instalador oficial: a entrada de desinstalação ou a pasta em Arquivos de Programas.
pub fn installed() -> bool {
    if windows_registry::LOCAL_MACHINE.open(UNINSTALL_KEY).is_ok() {
        return true;
    }
    std::env::var_os("ProgramFiles")
        .is_some_and(|p| std::path::Path::new(&p).join("AnyFSE").is_dir())
}
