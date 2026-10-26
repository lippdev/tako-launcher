//! PipeWire/PulseAudio: leitura de `pactl -f json list sinks` e de `wpctl status`.

use serde_json::Value;

const MAX_TEXT: usize = 4 << 20;
/// Volume 100% do PulseAudio.
pub const PA_VOLUME_NORM: f64 = 65536.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Sink {
    /// `pactl`: nome do sink (estável); `wpctl`: id do objeto (muda entre sessões).
    pub id: String,
    pub description: String,
    /// 0.0 a 1.0 (acima de 100% é cortado).
    pub volume: f32,
    pub muted: bool,
    /// Só o `wpctl status` informa; com o `pactl` vem de `get-default-sink`.
    pub is_default: bool,
}

pub fn parse_pactl_sinks(json: &str) -> Result<Vec<Sink>, String> {
    if json.len() > MAX_TEXT {
        return Err("saída do pactl grande demais".into());
    }
    let root: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let sinks = root.as_array().ok_or("esperava uma lista")?;
    Ok(sinks
        .iter()
        .filter_map(|s| {
            let id = s.get("name")?.as_str()?.to_owned();
            let channels: Vec<f64> = s
                .get("volume")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|v| v.values())
                .filter_map(|c| c.get("value")?.as_f64())
                .collect();
            let volume = if channels.is_empty() {
                1.0
            } else {
                channels.iter().sum::<f64>() / channels.len() as f64 / PA_VOLUME_NORM
            };
            Some(Sink {
                description: s
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or(&id)
                    .to_owned(),
                id,
                volume: volume.clamp(0.0, 1.0) as f32,
                muted: s.get("mute").and_then(Value::as_bool).unwrap_or(false),
                is_default: false,
            })
        })
        .collect())
}

/// Sinks da seção "Audio" do `wpctl status` (linhas como `│  *   48. Nome   [vol: 0.40 MUTED]`).
pub fn parse_wpctl_status(text: &str) -> Vec<Sink> {
    let text = if text.len() > MAX_TEXT { "" } else { text };
    let mut out = Vec::new();
    let (mut in_audio, mut in_sinks) = (false, false);
    for line in text.lines() {
        if !line.starts_with([' ', '│', '├', '└']) && !line.trim().is_empty() {
            in_audio = line.trim() == "Audio";
            in_sinks = false;
            continue;
        }
        let item = line
            .trim_start_matches([' ', '│', '├', '└', '─'])
            .trim_end();
        if !in_audio {
            continue;
        }
        if item.ends_with(':') {
            in_sinks = item == "Sinks:";
            continue;
        }
        if !in_sinks || item.is_empty() {
            continue;
        }
        let (is_default, rest) = match item.strip_prefix('*') {
            Some(r) => (true, r.trim_start()),
            None => (false, item),
        };
        let Some((id, rest)) = rest.split_once('.') else {
            continue;
        };
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let (name, vol) = rest.split_once("[vol:").unwrap_or((rest, ""));
        let mut vol_parts = vol.trim_end_matches(']').split_whitespace();
        let volume = vol_parts
            .next()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.0);
        out.push(Sink {
            id: id.to_owned(),
            description: name.trim().to_owned(),
            volume: volume.clamp(0.0, 1.0),
            muted: vol_parts.any(|p| p == "MUTED"),
            is_default,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pactl_json() {
        let json = r#"[
          {"index":48,"name":"alsa_output.pci-0000_00_1f.3.analog-stereo","description":"Áudio interno",
           "mute":false,"volume":{"front-left":{"value":32768,"value_percent":"50%"},
                                  "front-right":{"value":32768,"value_percent":"50%"}}},
          {"index":50,"name":"alsa_output.hdmi","mute":true,
           "volume":{"mono":{"value":98304}}},
          {"index":51}
        ]"#;
        let s = parse_pactl_sinks(json).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].description, "Áudio interno");
        assert!((s[0].volume - 0.5).abs() < 1e-6 && !s[0].muted);
        assert_eq!(s[1].description, "alsa_output.hdmi");
        assert!(s[1].muted && s[1].volume == 1.0);
        assert!(parse_pactl_sinks("Sink #0").is_err());
        assert!(parse_pactl_sinks("{}").is_err());
    }

    #[test]
    fn parses_wpctl_status() {
        let text = "\
PipeWire 'pipewire-0' [1.0.5, user@host, cookie:1]
 └─ Clients:
        33. pipewire                            [1.0.5, user@host, pid:1]

Audio
 ├─ Devices:
 │      42. Built-in Audio                      [alsa]
 │
 ├─ Sinks:
 │  *   48. Built-in Audio Analog Stereo        [vol: 0.40]
 │      50. HDMI / DisplayPort 1                [vol: 1.00 MUTED]
 │      51. sem volume
 │
 ├─ Sink endpoints:
 │
 ├─ Sources:
 │  *   49. Built-in Audio Analog Stereo        [vol: 0.80]
 │

Video
 ├─ Sinks:
 │      70. Câmera                              [vol: 1.00]
";
        let s = parse_wpctl_status(text);
        assert_eq!(s.len(), 3, "{s:?}");
        assert_eq!(s[0].id, "48");
        assert!(s[0].is_default && (s[0].volume - 0.4).abs() < 1e-6);
        assert_eq!(s[1].description, "HDMI / DisplayPort 1");
        assert!(s[1].muted && !s[1].is_default);
        assert_eq!(s[2].description, "sem volume");
        assert!(parse_wpctl_status("lixo\n │ *  x. y").is_empty());
    }
}
