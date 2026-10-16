//! Telas: port de `ConsoleMode.Runtime/Display/{DisplayControl,DisplayTopology}.cs`.

use super::DeviceResult;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HdrStatus {
    #[default]
    Unknown,
    Unsupported,
    Off,
    On,
}

/// Resolução e taxa. A chave textual (`3840x2160@120`) é a gravada nas configurações.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

impl DisplayMode {
    pub const fn new(width: u32, height: u32, refresh_hz: u32) -> Self {
        DisplayMode {
            width,
            height,
            refresh_hz,
        }
    }

    pub fn key(&self) -> String {
        format!("{}x{}@{}", self.width, self.height, self.refresh_hz)
    }

    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0 && self.refresh_hz > 0
    }

    pub fn parse(key: &str) -> Option<DisplayMode> {
        let (size, hz) = key.trim().split_once('@')?;
        let mut parts = size.split(['x', 'X', '×']);
        let (w, h) = (parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        let digits = |s: &str| {
            (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                .then(|| s.parse().ok())
                .flatten()
        };
        let mode = DisplayMode::new(digits(w)?, digits(h)?, digits(hz)?);
        mode.is_valid().then_some(mode)
    }

    /// Válidos, sem repetição, do maior para o menor (largura, altura, taxa).
    pub fn normalize(modes: impl IntoIterator<Item = DisplayMode>) -> Vec<DisplayMode> {
        let mut v: Vec<_> = modes.into_iter().filter(DisplayMode::is_valid).collect();
        v.sort_by_key(|m| std::cmp::Reverse((m.width, m.height, m.refresh_hz)));
        v.dedup();
        v
    }
}

impl fmt::Display for DisplayMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} × {} · {} Hz",
            self.width, self.height, self.refresh_hz
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectedDisplay {
    /// Id estável entre execuções (no Windows, o caminho do dispositivo; no Linux, o conector + EDID).
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub is_primary: bool,
    pub bounds: Option<ScreenBounds>,
    pub current_mode: Option<DisplayMode>,
    pub hdr: HdrStatus,
}

/// Estado de uma tela, para capturar antes do jogo e restaurar depois.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorState {
    pub id: String,
    pub name: String,
    pub is_primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Taxa exata (ex.: 59.94).
    pub refresh_hz: f64,
    /// 0, 90, 180 ou 270.
    pub rotation: u16,
}

impl MonitorState {
    pub const REFRESH_TOLERANCE_HZ: f64 = 0.05;

    pub fn has_same_mode(&self, other: &MonitorState) -> bool {
        self.width == other.width
            && self.height == other.height
            && (self.refresh_hz - other.refresh_hz).abs() <= Self::REFRESH_TOLERANCE_HZ
    }

    pub fn is_equivalent_to(&self, other: &MonitorState) -> bool {
        self.id.eq_ignore_ascii_case(&other.id)
            && self.is_primary == other.is_primary
            && self.x == other.x
            && self.y == other.y
            && self.has_same_mode(other)
            && self.rotation == other.rotation
    }
}

/// Topologia completa. `platform_data` guarda o que só o SO entende (no Windows, os caminhos CCD),
/// para a restauração ser exata.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DisplayTopology {
    pub monitors: Vec<MonitorState>,
    pub platform_data: Option<String>,
}

impl DisplayTopology {
    pub fn primary(&self) -> Option<&MonitorState> {
        self.monitors.iter().find(|m| m.is_primary)
    }

    pub fn find(&self, id: &str) -> Option<&MonitorState> {
        self.monitors.iter().find(|m| m.id.eq_ignore_ascii_case(id))
    }

    pub fn is_equivalent_to(&self, other: &DisplayTopology) -> bool {
        self.monitors.len() == other.monitors.len()
            && self
                .monitors
                .iter()
                .all(|m| other.find(&m.id).is_some_and(|o| m.is_equivalent_to(o)))
    }
}

/// Troca de resolução e taxa, ligar/desligar telas e HDR (PRD §5.3).
pub trait Displays: Send + Sync {
    fn list(&self) -> DeviceResult<Vec<ConnectedDisplay>>;
    fn modes(&self, display_id: &str) -> DeviceResult<Vec<DisplayMode>>;
    fn set_mode(&self, display_id: &str, mode: DisplayMode) -> DeviceResult<()>;
    /// Liga só estas telas (as outras são desligadas) e torna `primary` a principal.
    fn set_active(&self, active_ids: &[String], primary: &str) -> DeviceResult<()>;
    fn set_hdr(&self, display_id: &str, enabled: bool) -> DeviceResult<()>;
    fn capture(&self) -> DeviceResult<DisplayTopology>;
    fn apply(&self, topology: &DisplayTopology) -> DeviceResult<()>;
    /// Sessão gamescope e afins: a resolução é da sessão; o Tako só informa.
    fn is_managed_by_session(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_formats_mode_keys() {
        let m = DisplayMode::parse("3840x2160@120").expect("válido");
        assert_eq!(m, DisplayMode::new(3840, 2160, 120));
        assert_eq!(m.key(), "3840x2160@120");
        assert_eq!(
            DisplayMode::parse(" 1920×1080@60 "),
            Some(DisplayMode::new(1920, 1080, 60))
        );
        for bad in [
            "",
            "1920x1080",
            "1920x1080@0",
            "axb@60",
            "1x2x3@60",
            "-1x2@3",
            "1920x1080@+60",
        ] {
            assert_eq!(DisplayMode::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn normalize_sorts_desc_and_dedups() {
        let v = DisplayMode::normalize([
            DisplayMode::new(1920, 1080, 60),
            DisplayMode::new(3840, 2160, 60),
            DisplayMode::new(1920, 1080, 144),
            DisplayMode::new(1920, 1080, 60),
            DisplayMode::new(0, 0, 60),
        ]);
        assert_eq!(
            v,
            vec![
                DisplayMode::new(3840, 2160, 60),
                DisplayMode::new(1920, 1080, 144),
                DisplayMode::new(1920, 1080, 60)
            ]
        );
    }

    #[test]
    fn refresh_tolerance() {
        let a = MonitorState {
            id: "A".into(),
            name: "a".into(),
            is_primary: true,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            refresh_hz: 59.94,
            rotation: 0,
        };
        let b = MonitorState {
            refresh_hz: 59.98,
            id: "a".into(),
            ..a.clone()
        };
        assert!(a.is_equivalent_to(&b));
        let c = MonitorState {
            refresh_hz: 60.5,
            ..a.clone()
        };
        assert!(!a.is_equivalent_to(&c));
    }
}
