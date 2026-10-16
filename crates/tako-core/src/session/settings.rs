//! Configurações por jogo (port de `Session/GameSettings.cs`). O resolvedor (global + jogo) é do MIG-024.

use serde::{Deserialize, Serialize};

/// Ajustes de um jogo. `None` = "Padrão" (usa o do Tako). Gravado em `game_settings`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GameSettingsOverride {
    pub use_game_screen: Option<bool>,
    /// Chave do modo (`1920x1080@60`) ou [`GameSettingsOverride::KEEP_CURRENT_MODE`].
    pub screen_mode: Option<String>,
    pub hdr: Option<bool>,
    /// Id da saída de áudio ou [`GameSettingsOverride::KEEP_AUDIO`].
    pub audio_device_id: Option<String>,
    pub audio_device_name: Option<String>,
    /// Fora do escopo do Rust (FPS é do Console Mode), mas preservado para não perder dados das alphas.
    pub fps_limit: Option<i32>,
    pub show_fps: Option<bool>,
    pub extra_arguments: Option<String>,
    /// Pasta de trabalho do jogo (PRD §6); vazio = a do executável.
    pub working_directory: Option<String>,
}

impl GameSettingsOverride {
    /// "Manter a resolução atual".
    pub const KEEP_CURRENT_MODE: &'static str = "";
    /// "Não alterar o áudio".
    pub const KEEP_AUDIO: &'static str = "";

    pub fn changed_count(&self) -> usize {
        [
            self.use_game_screen.is_some(),
            self.screen_mode.is_some(),
            self.hdr.is_some(),
            self.audio_device_id.is_some(),
            self.extra_arguments.as_deref().is_some_and(|a| !a.trim().is_empty()),
            self.working_directory.as_deref().is_some_and(|a| !a.trim().is_empty()),
        ]
        .into_iter()
        .filter(|b| *b)
        .count()
    }

    /// FPS não conta: não aparece mais na interface.
    pub fn is_default(&self) -> bool {
        self.changed_count() == 0
    }
}
