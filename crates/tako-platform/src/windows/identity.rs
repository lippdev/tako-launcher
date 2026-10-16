//! Identidade de monitores e regras da topologia do Windows, sem chamadas ao SO (testadas em todo SO).
//! Port de `ConsoleMode.Runtime/Display/DisplayIdentity.cs` e das partes puras de `WindowsDisplayConfiguration.cs`.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use tako_core::devices::DisplayTopology;

/// O que o EDID diz do monitor: nome, número de série e resolução nativa (preferida).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EdidInfo {
    pub name: String,
    pub serial: String,
    pub preferred_width: u32,
    pub preferred_height: u32,
}

/// `\\?\DISPLAY#GSM5B7F#5&2d8e1c7&0&UID4352#{…}` → (`GSM5B7F`, `DISPLAY\GSM5B7F\5&2d8e1c7&0&UID4352`).
pub fn parse_device_path(device_path: &str) -> Option<(String, String)> {
    let path = device_path
        .strip_prefix(r"\\?\")
        .unwrap_or(device_path)
        .trim();
    let parts: Vec<&str> = path.split('#').collect();
    if parts.len() < 3 || parts[..3].iter().any(|p| p.is_empty()) {
        return None;
    }
    Some((
        parts[1].to_string(),
        format!(r"{}\{}\{}", parts[0], parts[1], parts[2]),
    ))
}

/// Id no formato do Console Mode: `MONITOR\GSM5B7F\{4d36e96e-…}\0004` (a última parte é a chave do driver).
pub fn monitor_id(hardware_id: &str, driver_key: Option<&str>) -> String {
    match driver_key {
        Some(key) if !hardware_id.trim().is_empty() && !key.trim().is_empty() => {
            format!(r"MONITOR\{hardware_id}\{key}")
        }
        _ => String::new(),
    }
}

/// Id estável de um alvo CCD: o caminho do dispositivo, ou adaptador:alvo se o driver não informar.
pub fn ccd_monitor_id(
    adapter_high: i32,
    adapter_low: u32,
    target_id: u32,
    device_path: Option<&str>,
) -> String {
    match device_path {
        Some(p) => p.to_string(),
        None => format!("{adapter_high:08X}{adapter_low:08X}:{target_id}"),
    }
}

/// Descritores de nome (0xFC) e série (0xFF) e a primeira temporização detalhada (o modo preferido).
pub fn parse_edid(edid: &[u8]) -> EdidInfo {
    let mut info = EdidInfo::default();
    if edid.len() < 128 {
        return info;
    }
    for offset in (54..=108).step_by(18) {
        let d = &edid[offset..offset + 18];
        if d[0] != 0 || d[1] != 0 {
            if info.preferred_width == 0 {
                info.preferred_width = u32::from(d[2]) | (u32::from(d[4] & 0xF0) << 4);
                info.preferred_height = u32::from(d[5]) | (u32::from(d[7] & 0xF0) << 4);
            }
            continue;
        }
        let text: String = d[5..]
            .iter()
            .take_while(|&&b| b != 0x0A && b != 0)
            .map(|&b| char::from(b))
            .collect();
        match d[3] {
            0xFC if info.name.is_empty() => info.name = text.trim().to_string(),
            0xFF if info.serial.is_empty() => info.serial = text.trim().to_string(),
            _ => {}
        }
    }
    info
}

/// Um monitor para [`assign_sources`]: fonte atual (se ativo), a última que usou e as candidatas.
pub struct SourceRequest<T, S> {
    pub target: T,
    pub active: Option<S>,
    pub preferred: Option<S>,
    pub candidates: Vec<S>,
}

/// Escolhe uma fonte de vídeo por monitor: os ativos mantêm a sua; cada inativo recebe a preferida se livre,
/// senão a primeira candidata livre. Dois monitores nunca dividem um `\\.\DISPLAYn`.
pub fn assign_sources<T: Eq + Hash + Clone, S: Eq + Hash + Copy>(
    requests: &[SourceRequest<T, S>],
) -> HashMap<T, S> {
    let mut result = HashMap::new();
    let mut used = HashSet::new();
    for r in requests {
        if let Some(a) = r.active {
            result.insert(r.target.clone(), a);
            used.insert(a);
        }
    }
    for r in requests.iter().filter(|r| r.active.is_none()) {
        let pick = r
            .preferred
            .filter(|p| r.candidates.contains(p) && !used.contains(p))
            .or_else(|| r.candidates.iter().copied().find(|c| !used.contains(c)));
        if let Some(s) = pick {
            result.insert(r.target.clone(), s);
            used.insert(s);
        }
    }
    result
}

/// Monitores do alvo que não estão iguais no estado atual (para a restauração monitor a monitor).
pub fn changed_ids(current: &DisplayTopology, target: &DisplayTopology) -> Vec<String> {
    target
        .monitors
        .iter()
        .filter(|m| current.find(&m.id).is_none_or(|c| !c.is_equivalent_to(m)))
        .map(|m| m.id.clone())
        .collect()
}

const BLOB_PREFIX: &str = "ccd1:";
const MAX_PATHS: usize = 64;
const MAX_MODES: usize = 256;

/// `ccd1:` + hex(contagem de caminhos u32 LE, contagem de modos u32 LE, bytes dos caminhos e modos).
pub fn encode_ccd(path_count: usize, mode_count: usize, raw: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(8 + raw.len());
    bytes.extend_from_slice(&(path_count as u32).to_le_bytes());
    bytes.extend_from_slice(&(mode_count as u32).to_le_bytes());
    bytes.extend_from_slice(raw);
    let mut s = String::with_capacity(BLOB_PREFIX.len() + bytes.len() * 2);
    s.push_str(BLOB_PREFIX);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Inverso de [`encode_ccd`]: `(caminhos, modos, bytes)`. Dado vindo das configurações: qualquer
/// tamanho ou contagem fora do esperado dá `None`.
pub fn decode_ccd(
    data: &str,
    path_size: usize,
    mode_size: usize,
) -> Option<(usize, usize, Vec<u8>)> {
    let hex = data.strip_prefix(BLOB_PREFIX)?;
    let limit = 8 + MAX_PATHS * path_size + MAX_MODES * mode_size;
    if hex.len() % 2 != 0 || hex.len() / 2 > limit || !hex.is_ascii() {
        return None;
    }
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
        .collect::<Option<_>>()?;
    let count = |i: usize| -> Option<usize> {
        Some(u32::from_le_bytes(bytes.get(i..i + 4)?.try_into().ok()?) as usize)
    };
    let (paths, modes) = (count(0)?, count(4)?);
    let valid = (1..=MAX_PATHS).contains(&paths)
        && modes <= MAX_MODES
        && bytes.len() == 8 + paths * path_size + modes * mode_size;
    valid.then(|| (paths, modes, bytes[8..].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::devices::MonitorState;

    #[test]
    fn device_path_gives_the_hardware_id_and_the_device_instance() {
        let (hw, instance) = parse_device_path(
            r"\\?\DISPLAY#GSM5B7F#5&2d8e1c7&0&UID4352#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}",
        )
        .unwrap();
        assert_eq!(hw, "GSM5B7F");
        assert_eq!(instance, r"DISPLAY\GSM5B7F\5&2d8e1c7&0&UID4352");
        assert_eq!(parse_device_path(""), None);
        assert_eq!(parse_device_path(r"\\?\DISPLAY#GSM5B7F"), None);
        assert_eq!(parse_device_path("DISPLAY##x"), None);
    }

    #[test]
    fn monitor_id_has_the_console_mode_format() {
        assert_eq!(
            monitor_id(
                "GSM5B7F",
                Some(r"{4d36e96e-e325-11ce-bfc1-08002be10318}\0004")
            ),
            r"MONITOR\GSM5B7F\{4d36e96e-e325-11ce-bfc1-08002be10318}\0004"
        );
        assert_eq!(monitor_id("GSM5B7F", None), "");
        assert_eq!(monitor_id(" ", Some("x")), "");
    }

    #[test]
    fn ccd_id_falls_back_to_adapter_and_target() {
        assert_eq!(ccd_monitor_id(0, 0x1A2B, 7, None), "0000000000001A2B:7");
        assert_eq!(
            ccd_monitor_id(1, 2, 3, Some(r"\\?\DISPLAY#X#Y#Z")),
            r"\\?\DISPLAY#X#Y#Z"
        );
    }

    fn write_text(edid: &mut [u8], offset: usize, tag: u8, text: &str) {
        edid[offset + 3] = tag;
        let bytes = format!("{text}\n").into_bytes();
        for i in 0..13 {
            edid[offset + 5 + i] = bytes.get(i).copied().unwrap_or(0x20);
        }
    }

    #[test]
    fn edid_gives_name_serial_and_native_resolution() {
        let mut edid = [0u8; 128];
        // Temporização detalhada 3840 × 2160 (o modo preferido).
        edid[54] = 0x08;
        edid[55] = 0xE8;
        edid[56] = 0x00;
        edid[58] = 0xF0;
        edid[59] = 0x70;
        edid[61] = 0x80;
        write_text(&mut edid, 72, 0xFC, "LG TV SSCR2");
        write_text(&mut edid, 90, 0xFF, "0x01010101");

        let info = parse_edid(&edid);
        assert_eq!(info.name, "LG TV SSCR2");
        assert_eq!(info.serial, "0x01010101");
        assert_eq!((info.preferred_width, info.preferred_height), (3840, 2160));
        assert_eq!(parse_edid(&[]), EdidInfo::default());
        assert_eq!(parse_edid(&edid[..127]), EdidInfo::default());
    }

    #[test]
    fn active_monitors_keep_their_source_and_inactive_ones_get_a_free_one() {
        let all = vec![0, 1, 2];
        let req = |t: &'static str, active, preferred| SourceRequest {
            target: t,
            active,
            preferred,
            candidates: all.clone(),
        };
        let assigned = assign_sources(&[
            req("desk", Some(0), None),
            req("tv", None, Some(0)), // quer a 0 (a última dele), mas a mesa está com ela
            req("side", None, Some(2)), // recebe a última dele
            req("projector", None, None),
        ]);
        assert_eq!(assigned["desk"], 0);
        assert_eq!(assigned["side"], 2);
        assert_eq!(assigned["tv"], 1);
        assert!(!assigned.contains_key("projector")); // acabaram as fontes
    }

    fn monitor(id: &str, width: u32) -> MonitorState {
        MonitorState {
            id: id.into(),
            name: id.into(),
            is_primary: id.eq_ignore_ascii_case("a"),
            x: 0,
            y: 0,
            width,
            height: 1080,
            refresh_hz: 60.0,
            rotation: 0,
        }
    }

    #[test]
    fn changed_ids_lists_new_and_different_monitors() {
        let current = DisplayTopology {
            monitors: vec![monitor("A", 1920), monitor("B", 1920)],
            platform_data: None,
        };
        let target = DisplayTopology {
            monitors: vec![monitor("a", 1920), monitor("B", 3840), monitor("C", 1920)],
            platform_data: None,
        };
        assert_eq!(changed_ids(&current, &target), vec!["B", "C"]);
    }

    #[test]
    fn ccd_blob_round_trips_and_rejects_bad_data() {
        let raw: Vec<u8> = (0..(2 * 3 + 4) as u8).collect();
        let blob = encode_ccd(2, 1, &raw);
        assert_eq!(decode_ccd(&blob, 3, 4), Some((2, 1, raw.clone())));
        for bad in [
            "",
            "ccd1:",
            "ccd1:zz",
            "ccd1:0",
            "ccd0:0200000001000000",
            &blob[..blob.len() - 2],       // falta um byte
            &format!("{blob}00"),          // sobra um byte
            &encode_ccd(0, 0, &[]),        // nenhum caminho
            &encode_ccd(65, 0, &[0; 195]), // caminhos demais
            "ccd1:é0",
        ] {
            assert_eq!(decode_ccd(bad, 3, 4), None, "{bad}");
        }
    }
}
