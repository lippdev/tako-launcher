//! Bandeja do sistema (WEB-36, PRD D-10 e D-15).
//!
//! No Windows, cria o ícone na área de notificação com menu de contexto:
//! - Abrir o Tako;
//! - Central de Controle (se houver jogo aberto);
//! - Jogos recentes;
//! - Sair do Tako.
//!
//! Clique no ícone reabre a interface onde estava.

use std::sync::{Arc, Mutex};
use tako_core::i18n::Lang;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayAction {
    /// Abrir ou trazer para a frente a interface do Tako.
    Open,
    /// Abrir a Central de Controle.
    Central,
    /// Iniciar um jogo recente pelo identificador.
    Launch(String),
    /// Encerrar o aplicativo.
    Quit,
}

#[derive(Debug, Clone)]
pub struct TrayState {
    pub lang: Lang,
    pub game_active: bool,
    pub recent_games: Vec<TrayGame>,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            lang: Lang::PtBr,
            game_active: false,
            recent_games: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayGame {
    pub id: String,
    pub title: String,
}

pub struct TrayHandle {
    #[cfg(target_os = "windows")]
    _inner: crate::windows::tray::WindowsTray,
}

impl TrayHandle {
    /// Verdadeiro somente enquanto o ícone estiver registrado no sistema, inclusive após reinício do Explorer.
    pub fn is_available(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            self._inner.is_available()
        }
        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }
}

/// Cria o ícone na bandeja do sistema e conecta o menu às ações.
pub fn create_tray(
    state: Arc<Mutex<TrayState>>,
    on_action: impl Fn(TrayAction) + Send + 'static,
) -> Option<TrayHandle> {
    #[cfg(target_os = "windows")]
    {
        crate::windows::tray::WindowsTray::new(state, on_action).map(|_inner| TrayHandle { _inner })
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (state, on_action);
        None
    }
}
