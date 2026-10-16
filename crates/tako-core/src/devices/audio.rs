//! Saída de áudio: port de `ConsoleMode.Runtime/Audio/AudioState.cs`, sem os papéis do Windows
//! (a implementação do Windows aplica o padrão nos três papéis).

use super::DeviceResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioLevel {
    pub device_id: String,
    /// 0.0 a 1.0.
    pub volume: f32,
    pub muted: bool,
}

/// Saídas, a padrão e os níveis: o que é capturado antes do jogo e restaurado depois.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AudioState {
    pub devices: Vec<AudioDevice>,
    pub default_id: Option<String>,
    pub levels: Vec<AudioLevel>,
}

impl AudioState {
    pub const VOLUME_TOLERANCE: f32 = 0.005;

    pub fn find(&self, id: &str) -> Option<&AudioDevice> {
        self.devices.iter().find(|d| d.id.eq_ignore_ascii_case(id))
    }

    pub fn level_of(&self, id: &str) -> Option<&AudioLevel> {
        self.levels
            .iter()
            .find(|l| l.device_id.eq_ignore_ascii_case(id))
    }

    pub fn is_equivalent_to(&self, other: &AudioState) -> bool {
        let same_default = match (&self.default_id, &other.default_id) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            (None, None) => true,
            _ => false,
        };
        same_default
            && self.levels.iter().all(|l| {
                other.level_of(&l.device_id).is_none_or(|o| {
                    o.muted == l.muted && (o.volume - l.volume).abs() <= Self::VOLUME_TOLERANCE
                })
            })
    }
}

/// Troca da saída padrão, volume e mudo (PRD §5.3).
pub trait AudioOutputs: Send + Sync {
    /// Falso quando o SO não deixa trocar a saída padrão (a UI explica).
    fn can_set_default(&self) -> bool;
    fn capture(&self) -> DeviceResult<AudioState>;
    fn set_default(&self, device_id: &str) -> DeviceResult<()>;
    fn set_volume(&self, device_id: &str, volume: f32) -> DeviceResult<()>;
    fn set_mute(&self, device_id: &str, muted: bool) -> DeviceResult<()>;
}
