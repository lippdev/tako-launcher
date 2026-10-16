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

/// Atalho de teclado global durante o jogo; o retorno mantém o atalho registrado até ser solto. `None` = sem
/// atalho (tecla sem equivalente, atalho em uso por outro app ou SO sem suporte).
/// shortcut: só Windows; Linux (portal GlobalShortcuts no Wayland, XGrabKey no X11) e macOS (CGEventTap com
/// Acessibilidade) ficam sem atalho de teclado durante o jogo, só o do controle. Fazer quando houver pedido.
pub fn global_hotkey(
    shortcut: &tako_core::input::shortcut::KeyShortcut,
    on_press: impl Fn() + Send + 'static,
) -> Option<Box<dyn std::any::Any>> {
    #[cfg(target_os = "windows")]
    {
        let vk = shortcut.virtual_key()?;
        windows::hotkey::register(shortcut.modifiers.0 as u32, vk, on_press)
            .map(|h| Box::new(h) as Box<dyn std::any::Any>)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (shortcut, on_press);
        None
    }
}

/// Chamar no início do `main`. No glibc o limiar de `mmap` sobe sozinho depois de cada bloco grande liberado
/// (até 32 MB); daí os buffers de decodificação das artes (vários MB, um por imagem) passam a vir do heap, que
/// fragmenta e não devolve a memória ao SO (medido: ~70 MB a mais em repouso na home). Fixar o limiar em 1 MB
/// mantém esses blocos em `mmap`, devolvidos no `free`. Windows e macOS já devolvem blocos grandes.
pub fn tune_allocator() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: `mallopt` só ajusta um parâmetro do alocador; chamado antes de criar threads.
    unsafe {
        libc::mallopt(libc::M_MMAP_THRESHOLD, 1024 * 1024);
    }
}

// Parte só é usada no Linux; nos outros SOs compila para os testes rodarem.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod unix;

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
