//! KDE Wayland: leitura de `kscreen-doctor -j` e argumentos para aplicar um estado.

use super::outputs::{Output, OutputMode};
use serde_json::Value;
use tako_core::devices::{HdrStatus, VrrStatus};

const MAX_JSON: usize = 4 << 20;

fn rotation(code: i64) -> u16 {
    match code {
        2 => 90,
        4 => 180,
        8 => 270,
        _ => 0,
    }
}

fn rotation_name(deg: u16) -> &'static str {
    match deg {
        90 => "left",
        180 => "inverted",
        270 => "right",
        _ => "none",
    }
}

fn int(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}

/// Saídas conectadas. `primary` (Plasma 5) ou `priority == 1` (Plasma 6) marca a principal.
pub fn parse(json: &str) -> Result<Vec<Output>, String> {
    if json.len() > MAX_JSON {
        return Err("saída do kscreen-doctor grande demais".into());
    }
    let root: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let outs = root
        .get("outputs")
        .and_then(Value::as_array)
        .ok_or("sem \"outputs\"")?;
    Ok(outs
        .iter()
        .filter(|o| o.get("connected").and_then(Value::as_bool) != Some(false))
        .filter_map(|o| {
            let id = o.get("name")?.as_str()?.to_owned();
            let modes: Vec<OutputMode> = o
                .get("modes")
                .and_then(Value::as_array)
                .map(|ms| {
                    ms.iter()
                        .filter_map(|m| {
                            let size = m.get("size")?;
                            Some(OutputMode {
                                key: m.get("name")?.as_str()?.to_owned(),
                                width: u32::try_from(int(size, "width")).ok()?,
                                height: u32::try_from(int(size, "height")).ok()?,
                                refresh: m.get("refreshRate")?.as_f64()?,
                            })
                            .filter(|m| m.width > 0 && m.height > 0)
                        })
                        .collect()
                })
                .unwrap_or_default();
            let current_id = o.get("currentModeId").map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            });
            let current = current_id.and_then(|cid| {
                o.get("modes")?
                    .as_array()?
                    .iter()
                    .find(|m| m.get("id").and_then(Value::as_str) == Some(&cid))
                    .and_then(|m| m.get("name")?.as_str())
                    .and_then(|name| modes.iter().find(|m| m.key == name).cloned())
            });
            let enabled = o.get("enabled").and_then(Value::as_bool).unwrap_or(false);
            let pos = o.get("pos").cloned().unwrap_or(Value::Null);
            Some(Output {
                name: id.clone(),
                id,
                enabled,
                primary: o.get("primary").and_then(Value::as_bool) == Some(true)
                    || int(o, "priority") == 1,
                x: int(&pos, "x") as i32,
                y: int(&pos, "y") as i32,
                current,
                modes,
                rotation: rotation(int(o, "rotation")),
                scale: o.get("scale").and_then(Value::as_f64).unwrap_or(1.0),
                hdr: match o.get("hdr").and_then(Value::as_bool) {
                    Some(true) => HdrStatus::On,
                    Some(false) => HdrStatus::Off,
                    None => HdrStatus::Unsupported,
                },
                // Só vem quando a tela aceita VRR: 0 = nunca, 1 = sempre, 2 = automático (jogos em tela cheia).
                vrr: match o.get("vrrPolicy").and_then(Value::as_i64) {
                    Some(0) => VrrStatus::Off,
                    Some(_) => VrrStatus::On,
                    None => VrrStatus::Unsupported,
                },
            })
        })
        .collect())
}

/// Argumento de `kscreen-doctor` para ligar (automático: só em tela cheia, como o Plasma) ou desligar o VRR.
pub fn vrr_arg(id: &str, enabled: bool) -> String {
    let policy = if enabled { "automatic" } else { "never" };
    format!("output.{id}.vrrpolicy.{policy}")
}

/// Argumentos de `kscreen-doctor` para o estado inteiro (aplicado de uma vez).
pub fn args(desired: &[Output]) -> Vec<String> {
    let mut a = Vec::new();
    for o in desired {
        let n = &o.id;
        match (&o.current, o.enabled) {
            (Some(m), true) => {
                a.push(format!("output.{n}.enable"));
                a.push(format!("output.{n}.mode.{}", m.key));
                a.push(format!("output.{n}.position.{},{}", o.x, o.y));
                a.push(format!("output.{n}.rotation.{}", rotation_name(o.rotation)));
                if o.primary {
                    a.push(format!("output.{n}.primary"));
                }
            }
            _ => a.push(format!("output.{n}.disable")),
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"outputs":[
      {"id":1,"name":"DP-1","enabled":true,"connected":true,"priority":1,"pos":{"x":0,"y":0},
       "rotation":1,"scale":1.25,"currentModeId":"2","hdr":false,"vrrPolicy":2,
       "modes":[{"id":"1","name":"2560x1440@60","refreshRate":59.95,"size":{"width":2560,"height":1440}},
                {"id":"2","name":"2560x1440@144","refreshRate":143.91,"size":{"width":2560,"height":1440}}]},
      {"id":2,"name":"HDMI-A-1","enabled":false,"connected":true,"priority":0,"pos":{"x":2048,"y":0},
       "rotation":2,"currentModeId":"","vrrPolicy":0,
       "modes":[{"id":"5","name":"3840x2160@60","refreshRate":60.0,"size":{"width":3840,"height":2160}},
                {"id":"6","name":"broken","size":{"width":0,"height":0}}]},
      {"id":3,"name":"DP-2","enabled":false,"connected":false,"modes":[]}
    ]}"#;

    #[test]
    fn parses_outputs() {
        let o = parse(SAMPLE).unwrap();
        assert_eq!(o.len(), 2);
        assert!(o[0].enabled && o[0].primary);
        assert_eq!(o[0].current.as_ref().unwrap().key, "2560x1440@144");
        assert_eq!(o[0].scale, 1.25);
        assert_eq!(o[0].hdr, HdrStatus::Off);
        assert_eq!(o[1].modes.len(), 1);
        assert!(o[1].current.is_none() && !o[1].primary);
        assert_eq!(o[1].rotation, 90);
        assert_eq!(o[1].hdr, HdrStatus::Unsupported);
        assert_eq!((o[0].vrr, o[1].vrr), (VrrStatus::On, VrrStatus::Off));
        assert_eq!(vrr_arg("DP-1", true), "output.DP-1.vrrpolicy.automatic");
        assert_eq!(vrr_arg("DP-1", false), "output.DP-1.vrrpolicy.never");
    }

    #[test]
    fn plasma5_primary_flag() {
        let o = parse(r#"{"outputs":[{"name":"eDP-1","enabled":true,"primary":true}]}"#).unwrap();
        assert!(o[0].primary && o[0].modes.is_empty());
        assert_eq!(o[0].vrr, VrrStatus::Unsupported);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("").is_err());
        assert!(parse("{}").is_err());
        assert!(
            parse(r#"{"outputs":[1,"x",{"name":3}]}"#)
                .unwrap()
                .is_empty()
        );
        assert!(parse(&" ".repeat(MAX_JSON + 1)).is_err());
    }

    #[test]
    fn builds_args() {
        let mut o = parse(SAMPLE).unwrap();
        o[0].primary = false;
        o[1].enabled = true;
        o[1].primary = true;
        o[1].current = Some(o[1].modes[0].clone());
        o[0].enabled = false;
        assert_eq!(
            args(&o),
            [
                "output.DP-1.disable",
                "output.HDMI-A-1.enable",
                "output.HDMI-A-1.mode.3840x2160@60",
                "output.HDMI-A-1.position.2048,0",
                "output.HDMI-A-1.rotation.left",
                "output.HDMI-A-1.primary",
            ]
        );
    }
}
