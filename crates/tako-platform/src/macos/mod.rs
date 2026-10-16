//! Implementações para macOS (PRD §5.3). A matriz de suporte está na ADR-0024.

mod audio;
mod displays;
mod input;
mod processes;

use crate::Platform;
use crate::unix::outputs::OutputDisplays;
use crate::unix::{run, text};
use std::sync::Arc;

const LAUNCH_AGENT: &str = "dev.lipp.tako";

pub fn platform(data_root: &std::path::Path) -> Platform {
    let pointer = Arc::new(input::MacPointer::default());
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "tako".into());
    Platform {
        displays: Arc::new(OutputDisplays::new(displays::CoreGraphics)),
        audio: Arc::new(audio::CoreAudio),
        cursor: Arc::new(input::MacCursor::new(data_root.join("cursor-hidden"))),
        pointer,
        processes: Arc::new(processes::MacProcesses),
        secrets: Arc::new(crate::secrets::OsSecrets),
        autostart: Arc::new(run::FileAutostart {
            path: run::home().map(|h| h.join(format!("Library/LaunchAgents/{LAUNCH_AGENT}.plist"))),
            content: text::launch_agent_plist(LAUNCH_AGENT, &exe),
        }),
    }
}
