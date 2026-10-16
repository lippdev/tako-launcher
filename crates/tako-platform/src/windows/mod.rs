//! Implementações para Windows (PRD §5.3): telas pela CCD, áudio pelo Core Audio, cursor por `SetSystemCursor`
//! e `SendInput`, processos pelo Shell e Toolhelp, abrir ao entrar pelo registro.

use crate::Platform;
use std::sync::Arc;

mod audio;
mod autostart;
mod display;
pub mod dpapi;
pub mod hotkey;
mod pointer;
mod process;

pub fn platform(data_root: &std::path::Path) -> Platform {
    let pointer = Arc::new(pointer::WindowsPointer::new(data_root));
    Platform {
        displays: Arc::new(display::WindowsDisplays::default()),
        audio: Arc::new(audio::WindowsAudio::default()),
        cursor: pointer.clone(),
        pointer,
        processes: Arc::new(process::WindowsProcesses::default()),
        secrets: Arc::new(crate::secrets::OsSecrets),
        autostart: Arc::new(autostart::WindowsAutostart),
    }
}
