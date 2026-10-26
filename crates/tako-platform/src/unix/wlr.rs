//! wlroots (Sway, Hyprland...): leitura de `wlr-randr --json` e argumentos para aplicar um estado.

use super::outputs::{Output, OutputMode};
use serde_json::Value;
use tako_core::devices::{HdrStatus, VrrStatus};

const MAX_JSON: usize = 4 << 20;

fn rotation(t: &str) -> u16 {
    match t {
        "90" | "flipped-90" => 90,
        "180" | "flipped-180" => 180,
        "270" | "flipped-270" => 270,
        _ => 0,
    }
}

pub fn parse(json: &str) -> Result<Vec<Output>, String> {
    if json.len() > MAX_JSON {
        return Err("saída do wlr-randr grande demais".into());
    }
    let root: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let outs = root.as_array().ok_or("esperava uma lista")?;
    Ok(outs
        .iter()
        .filter_map(|o| {
            let id = o.get("name")?.as_str()?.to_owned();
            let mut current = None;
            let modes = o
                .get("modes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|m| {
                    let width = u32::try_from(m.get("width")?.as_u64()?).ok()?;
                    let height = u32::try_from(m.get("height")?.as_u64()?).ok()?;
                    let refresh = m.get("refresh")?.as_f64()?;
                    let mode = OutputMode {
                        key: format!("{width}x{height}@{refresh:.3}Hz"),
                        width,
                        height,
                        refresh,
                    };
                    if m.get("current").and_then(Value::as_bool) == Some(true) {
                        current = Some(mode.clone());
                    }
                    Some(mode)
                })
                .collect();
            let pos = o.get("position").cloned().unwrap_or(Value::Null);
            let at = |k: &str| pos.get(k).and_then(Value::as_i64).unwrap_or(0) as i32;
            let name = ["make", "model"]
                .iter()
                .filter_map(|k| o.get(*k)?.as_str())
                .filter(|s| !s.is_empty() && *s != "Unknown")
                .collect::<Vec<_>>()
                .join(" ");
            Some(Output {
                name: if name.is_empty() { id.clone() } else { name },
                id,
                enabled: o.get("enabled").and_then(Value::as_bool).unwrap_or(false),
                // wlroots não tem tela principal.
                primary: false,
                x: at("x"),
                y: at("y"),
                current,
                modes,
                rotation: rotation(o.get("transform").and_then(Value::as_str).unwrap_or("")),
                scale: o.get("scale").and_then(Value::as_f64).unwrap_or(1.0),
                hdr: HdrStatus::Unsupported,
                // O protocolo só diz se está ligado, não se a tela aceita: desligado vale tentar.
                vrr: match o.get("adaptive_sync").and_then(Value::as_bool) {
                    Some(true) => VrrStatus::On,
                    Some(false) => VrrStatus::Off,
                    None => VrrStatus::Unsupported,
                },
            })
        })
        .collect())
}

pub fn vrr_args(id: &str, enabled: bool) -> Vec<String> {
    let state = if enabled { "enabled" } else { "disabled" };
    ["--output", id, "--adaptive-sync", state]
        .map(String::from)
        .to_vec()
}

pub fn args(desired: &[Output]) -> Vec<String> {
    let mut a = Vec::new();
    for o in desired {
        a.extend(["--output".into(), o.id.clone()]);
        match (&o.current, o.enabled) {
            (Some(m), true) => {
                let transform = match o.rotation {
                    90 | 180 | 270 => o.rotation.to_string(),
                    _ => "normal".into(),
                };
                a.extend([
                    "--on".into(),
                    "--mode".into(),
                    m.key.clone(),
                    "--pos".into(),
                    format!("{},{}", o.x, o.y),
                    "--transform".into(),
                    transform,
                ]);
            }
            _ => a.push("--off".into()),
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      {"name":"eDP-1","make":"BOE","model":"0x0BCA","enabled":true,"transform":"normal","scale":1.5,"adaptive_sync":false,
       "position":{"x":0,"y":0},
       "modes":[{"width":2256,"height":1504,"refresh":59.999,"preferred":true,"current":true},
                {"width":1920,"height":1200,"refresh":59.885,"preferred":false,"current":false}]},
      {"name":"HDMI-A-1","make":"Unknown","model":"","enabled":false,"transform":"90",
       "modes":[{"width":3840,"height":2160,"refresh":60.0}]},
      {"bad":true}
    ]"#;

    #[test]
    fn parses_outputs() {
        let o = parse(SAMPLE).unwrap();
        assert_eq!(o.len(), 2);
        assert_eq!(o[0].name, "BOE 0x0BCA");
        assert_eq!(o[0].current.as_ref().unwrap().key, "2256x1504@59.999Hz");
        assert_eq!(o[0].extent(), (1504, 1003));
        assert_eq!(o[1].name, "HDMI-A-1");
        assert!(o[1].current.is_none());
        assert_eq!(o[1].rotation, 90);
        assert_eq!(
            (o[0].vrr, o[1].vrr),
            (VrrStatus::Off, VrrStatus::Unsupported)
        );
        assert_eq!(
            vrr_args("eDP-1", true).join(" "),
            "--output eDP-1 --adaptive-sync enabled"
        );
        assert!(parse("{}").is_err());
        assert!(parse("[").is_err());
    }

    #[test]
    fn builds_args() {
        let mut o = parse(SAMPLE).unwrap();
        o[1].enabled = true;
        o[1].current = Some(o[1].modes[0].clone());
        o[1].x = 1504;
        o[0].enabled = false;
        assert_eq!(
            args(&o).join(" "),
            "--output eDP-1 --off --output HDMI-A-1 --on --mode 3840x2160@60.000Hz --pos 1504,0 --transform 90"
        );
    }
}
