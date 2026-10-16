//! GNOME Wayland: o estado de `org.gnome.Mutter.DisplayConfig.GetCurrentState` já decodificado
//! (a parte D-Bus fica em `linux::displays`) e a montagem do `ApplyMonitorsConfig`.

use super::outputs::{Output, OutputMode};
use tako_core::devices::HdrStatus;

#[derive(Debug, Clone, PartialEq)]
pub struct MutterMode {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
    pub preferred_scale: f64,
    pub scales: Vec<f64>,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutterMonitor {
    pub connector: String,
    pub display_name: String,
    pub modes: Vec<MutterMode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutterLogical {
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: u32,
    pub primary: bool,
    pub connectors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MutterState {
    pub serial: u32,
    pub monitors: Vec<MutterMonitor>,
    pub logical: Vec<MutterLogical>,
    /// `layout-mode` 2 (físico): posições em pixels reais, sem dividir pela escala.
    pub physical_layout: bool,
}

/// Um monitor lógico do `ApplyMonitorsConfig`: (x, y, escala, transform, principal, [(conector, modo)]).
pub type LogicalConfig = (i32, i32, f64, u32, bool, Vec<(String, String)>);

fn mode(m: &MutterMode) -> OutputMode {
    OutputMode {
        key: m.id.clone(),
        width: m.width,
        height: m.height,
        refresh: m.refresh,
    }
}

pub fn outputs(state: &MutterState) -> Vec<Output> {
    state
        .monitors
        .iter()
        .map(|mon| {
            let lm = state
                .logical
                .iter()
                .find(|l| l.connectors.contains(&mon.connector));
            let current = mon.modes.iter().find(|m| m.current);
            Output {
                id: mon.connector.clone(),
                name: if mon.display_name.is_empty() {
                    mon.connector.clone()
                } else {
                    mon.display_name.clone()
                },
                enabled: lm.is_some() && current.is_some(),
                primary: lm.is_some_and(|l| l.primary),
                x: lm.map_or(0, |l| l.x),
                y: lm.map_or(0, |l| l.y),
                current: current.map(mode),
                modes: mon.modes.iter().map(mode).collect(),
                rotation: lm.map_or(0, |l| (l.transform % 4) as u16 * 90),
                scale: match (state.physical_layout, lm) {
                    (true, _) => 1.0,
                    (false, Some(l)) => l.scale,
                    (false, None) => current.map_or(1.0, |m| m.preferred_scale),
                },
                hdr: HdrStatus::Unsupported,
            }
        })
        .collect()
}

/// Um monitor lógico por tela ligada. A escala atual é mantida se o novo modo a aceitar; senão,
/// vale a preferida do modo. shortcut: espelhamento vira telas separadas; refazer se pedirem.
pub fn logical_config(state: &MutterState, desired: &[Output]) -> Vec<LogicalConfig> {
    desired
        .iter()
        .filter(|o| o.enabled)
        .filter_map(|o| {
            let key = &o.current.as_ref()?.key;
            let mon = state.monitors.iter().find(|m| m.connector == o.id)?;
            let m = mon.modes.iter().find(|m| &m.id == key)?;
            let current_scale = state
                .logical
                .iter()
                .find(|l| l.connectors.contains(&o.id))
                .map_or(m.preferred_scale, |l| l.scale);
            let scale = if m.scales.iter().any(|s| (s - current_scale).abs() < 1e-4) {
                current_scale
            } else {
                m.preferred_scale
            };
            Some((
                o.x,
                o.y,
                scale,
                u32::from(o.rotation / 90),
                o.primary,
                vec![(o.id.clone(), m.id.clone())],
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(id: &str, w: u32, h: u32, hz: f64, scales: &[f64], current: bool) -> MutterMode {
        MutterMode {
            id: id.into(),
            width: w,
            height: h,
            refresh: hz,
            preferred_scale: scales[0],
            scales: scales.to_vec(),
            current,
        }
    }

    fn state() -> MutterState {
        MutterState {
            serial: 7,
            monitors: vec![
                MutterMonitor {
                    connector: "eDP-1".into(),
                    display_name: "Built-in display".into(),
                    modes: vec![
                        mm("2880x1800@120", 2880, 1800, 120.0, &[2.0, 1.0, 1.5], true),
                        mm("1920x1200@60", 1920, 1200, 60.0, &[1.0], false),
                    ],
                },
                MutterMonitor {
                    connector: "HDMI-1".into(),
                    display_name: String::new(),
                    modes: vec![mm("3840x2160@60", 3840, 2160, 60.0, &[2.0, 1.0], false)],
                },
            ],
            logical: vec![MutterLogical {
                x: 0,
                y: 0,
                scale: 1.5,
                transform: 0,
                primary: true,
                connectors: vec!["eDP-1".into()],
            }],
            physical_layout: false,
        }
    }

    #[test]
    fn maps_state_to_outputs() {
        let o = outputs(&state());
        assert!(o[0].enabled && o[0].primary);
        assert_eq!(o[0].name, "Built-in display");
        assert_eq!(o[0].extent(), (1920, 1200));
        assert!(!o[1].enabled && o[1].current.is_none());
        assert_eq!(o[1].name, "HDMI-1");
        let mut s = state();
        s.physical_layout = true;
        assert_eq!(outputs(&s)[0].extent(), (2880, 1800));
    }

    #[test]
    fn keeps_scale_when_supported() {
        let s = state();
        let mut o = outputs(&s);
        assert_eq!(
            logical_config(&s, &o),
            vec![(
                0,
                0,
                1.5,
                0,
                true,
                vec![("eDP-1".into(), "2880x1800@120".into())]
            )]
        );
        o[0].current = Some(mode(&s.monitors[0].modes[1]));
        o[1].enabled = true;
        o[1].current = Some(mode(&s.monitors[1].modes[0]));
        o[1].x = 1920;
        o[1].rotation = 270;
        let c = logical_config(&s, &o);
        assert_eq!(c[0].2, 1.0);
        assert_eq!((c[1].0, c[1].2, c[1].3), (1920, 2.0, 3));
    }
}
