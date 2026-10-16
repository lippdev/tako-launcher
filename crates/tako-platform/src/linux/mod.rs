//! Implementações para Linux (PRD §5.3). A matriz de suporte por sessão está na ADR-0024.

mod audio;
mod displays;
mod input;
mod processes;

use crate::Platform;
use crate::unix::{run, session, text};
use std::sync::Arc;

pub fn platform(data_root: &std::path::Path) -> Platform {
    let session = session::detect(|k| std::env::var(k).ok());
    tracing::info!(?session, "sessão gráfica");
    let pointer = Arc::new(input::UinputPointer::default());
    Platform {
        displays: Arc::from(displays::for_session(session)),
        audio: Arc::new(audio::PipeWireAudio::default()),
        cursor: Arc::new(input::LinuxCursor::new(
            data_root.join("cursor-hidden"),
            matches!(session, session::Session::X11 | session::Session::Gamescope),
        )),
        pointer,
        processes: Arc::new(processes::LinuxProcesses),
        secrets: Arc::new(crate::secrets::OsSecrets),
        autostart: Arc::new(autostart()),
    }
}

fn autostart() -> run::FileAutostart {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| run::home().map(|h| h.join(".config")));
    // Num AppImage, o executável atual está numa montagem temporária; o arquivo certo é `$APPIMAGE`.
    let exe = std::env::var("APPIMAGE")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "tako".into());
    run::FileAutostart {
        path: config.map(|c| c.join("autostart/tako.desktop")),
        content: text::autostart_desktop_entry(&exe),
    }
}
