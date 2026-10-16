//! Implementações por SO dos contratos de `tako_core::devices` (PRD §5.3).
//!
//! [`Platform::native`] monta as implementações do SO atual; [`Platform::simulated`] usa as falsas do core
//! (CI, testes de UI e `--simulate`).

use std::sync::Arc;
use tako_core::devices::{
    AudioOutputs, CursorControl, Displays, PointerInjector, ProcessLauncher,
    fake::{FakeAudio, FakeDisplays, FakePointer, FakeProcesses},
};

pub mod secrets;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
// Partes puras do Windows, compiladas e testadas em todo SO.
#[path = "windows/identity.rs"]
pub mod windows_identity;
#[path = "windows/startup.rs"]
pub mod windows_startup;

/// Conjunto de funções de plataforma usado pela sessão e pela UI.
#[derive(Clone)]
pub struct Platform {
    pub displays: Arc<dyn Displays>,
    pub audio: Arc<dyn AudioOutputs>,
    pub cursor: Arc<dyn CursorControl>,
    pub pointer: Arc<dyn PointerInjector>,
    pub processes: Arc<dyn ProcessLauncher>,
    pub secrets: Arc<dyn secrets::Secrets>,
    pub autostart: Arc<dyn Autostart>,
}

/// Abrir o Tako ao entrar no sistema.
pub trait Autostart: Send + Sync {
    fn is_enabled(&self) -> bool;
    fn set_enabled(&self, enabled: bool) -> tako_core::devices::DeviceResult<()>;
}

#[derive(Default)]
struct NoAutostart(std::sync::Mutex<bool>);

impl Autostart for NoAutostart {
    fn is_enabled(&self) -> bool {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn set_enabled(&self, enabled: bool) -> tako_core::devices::DeviceResult<()> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = enabled;
        Ok(())
    }
}

impl Platform {
    pub fn simulated() -> Platform {
        let pointer = Arc::new(FakePointer::default());
        Platform {
            displays: Arc::new(FakeDisplays::desk()),
            audio: Arc::new(FakeAudio::desk()),
            cursor: pointer.clone(),
            pointer,
            processes: Arc::new(FakeProcesses::default()),
            secrets: Arc::new(secrets::MemorySecrets::default()),
            autostart: Arc::new(NoAutostart::default()),
        }
    }

    /// Implementações do SO atual. `data_root` guarda marcadores de recuperação (cursor escondido).
    pub fn native(data_root: &std::path::Path) -> Platform {
        #[cfg(target_os = "windows")]
        return windows::platform(data_root);
        #[cfg(target_os = "linux")]
        return linux::platform(data_root);
        #[cfg(target_os = "macos")]
        return macos::platform(data_root);
        #[allow(unreachable_code)]
        {
            let _ = data_root;
            Platform::simulated()
        }
    }
}
