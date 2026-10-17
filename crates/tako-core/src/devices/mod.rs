//! Contratos das funções de plataforma (PRD D-03 e §5.3) e os tipos que elas trocam.
//!
//! Os traits ficam no core para a sessão orquestrar sem conhecer o SO (dependências apontam para dentro);
//! as implementações por SO ficam em `tako-platform`, e [`fake`] tem versões em memória para testes.

pub mod audio;
pub mod display;
pub mod fake;
pub mod pointer;
pub mod process;

pub use audio::{AudioDevice, AudioOutputs, AudioState};
pub use display::{
    ConnectedDisplay, DisplayMode, DisplayTopology, Displays, HdrStatus, MonitorState, ScreenBounds,
};
pub use pointer::{CursorControl, MouseButton, PointerInjector};
pub use process::{GameWindow, ProcessEntry, ProcessLauncher};

/// Erro de uma função de plataforma. A mensagem vai para o log e, resumida, para o usuário.
#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    /// O SO ou o compositor não oferece a função (ex.: esconder o cursor sobre o jogo no Wayland).
    #[error("não suportado: {0}")]
    Unsupported(String),
    /// Falta permissão (ex.: `/dev/uinput`, Acessibilidade no macOS).
    #[error("sem permissão: {0}")]
    PermissionDenied(String),
    #[error("dispositivo não encontrado: {0}")]
    NotFound(String),
    #[error("{0}")]
    Failed(String),
}

pub type DeviceResult<T> = Result<T, DeviceError>;
