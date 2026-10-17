//! Partes puras do cliente do HidHide (INPUT-005), compiladas e testadas em todo SO: listas `MULTI_SZ`, quais
//! dispositivos são de um controle e o marcador do que o Tako mudou (para desfazer só isso, inclusive depois de
//! uma queda).

use serde::{Deserialize, Serialize};

/// Limite de uma lista lida do driver (em `u16`): dado de fora, não confiável.
pub const MAX_LIST: usize = 256 * 1024;

/// Lista de textos UTF-16 separados por NUL e terminada por dois NUL.
pub fn encode(items: &[String]) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    for item in items.iter().filter(|i| !i.is_empty()) {
        out.extend(item.encode_utf16());
        out.push(0);
    }
    out.push(0);
    if out.len() == 1 {
        out.push(0);
    }
    out
}

pub fn decode(buf: &[u16]) -> Vec<String> {
    buf[..buf.len().min(MAX_LIST)]
        .split(|c| *c == 0)
        .filter(|s| !s.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

/// A instância (como a do `SetupDiGetDeviceInstanceId`) é deste controle? USB: `VID_054C&PID_0CE6`;
/// Bluetooth: `VID&0002054C_PID&0CE6`. Só HID e USB (as que o HidHide filtra).
pub fn is_pad_instance(instance: &str, vendor: u16, product: u16) -> bool {
    let id = instance.to_ascii_uppercase();
    if !(id.starts_with("HID\\") || id.starts_with("USB\\")) {
        return false;
    }
    let usb = format!("VID_{vendor:04X}&PID_{product:04X}");
    let bt = format!("VID&0002{vendor:04X}_PID&{product:04X}");
    id.contains(&usb) || id.contains(&bt)
}

/// Caminho do executável na notação de dispositivo que o driver usa (`\Device\HarddiskVolume3\...`), a partir
/// do caminho DOS e do dispositivo da unidade (o `QueryDosDevice` de `C:`).
pub fn device_path(dos: &str, drive_device: &str) -> Option<String> {
    let dos = dos.strip_prefix(r"\\?\").unwrap_or(dos);
    let rest = dos.get(2..)?;
    (dos.as_bytes().get(1) == Some(&b':') && rest.starts_with('\\'))
        .then(|| format!("{}{rest}", drive_device.trim_end_matches('\\')))
}

/// O que o Tako mudou no driver, gravado antes de mudar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    /// Instâncias que o Tako pôs na lista de escondidos (as que já estavam lá ficam de fora).
    #[serde(default)]
    pub hidden: Vec<String>,
    /// Programas que o Tako pôs na lista dos que continuam vendo.
    #[serde(default)]
    pub allowed: Vec<String>,
    /// O driver estava ligado antes do Tako ligar.
    #[serde(default)]
    pub was_active: bool,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.hidden.is_empty() && self.allowed.is_empty()
    }

    /// Marcador gravado; quebrado ou grande demais = nada a desfazer (com log).
    pub fn parse(text: &str) -> Changes {
        if text.len() > 64 * 1024 {
            return Changes::default();
        }
        serde_json::from_str(text)
            .map_err(|e| tracing::warn!("marcador do HidHide inválido: {e}"))
            .unwrap_or_default()
    }
}

/// Acrescenta `add` à lista; devolve a lista nova e o que entrou de fato.
pub fn add_to(list: &[String], add: &[String]) -> (Vec<String>, Vec<String>) {
    let mut out = list.to_vec();
    let mut added = Vec::new();
    for a in add {
        if !out.iter().any(|x| x.eq_ignore_ascii_case(a)) {
            out.push(a.clone());
            added.push(a.clone());
        }
    }
    (out, added)
}

/// Tira da lista só o que o Tako pôs.
pub fn remove_from(list: &[String], added: &[String]) -> Vec<String> {
    list.iter()
        .filter(|x| !added.iter().any(|a| a.eq_ignore_ascii_case(x)))
        .cloned()
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn multi_strings_round_trip() {
        let items = vec!["HID\\VID_054C&PID_0CE6\\1".to_owned(), "b".to_owned()];
        let buf = encode(&items);
        assert_eq!(&buf[buf.len() - 2..], &[0, 0]);
        assert_eq!(decode(&buf), items);
        assert_eq!(encode(&[]), [0, 0]);
        assert!(decode(&[0, 0]).is_empty());
        assert!(decode(&[]).is_empty());
    }

    #[test]
    fn pad_instances_over_usb_and_bluetooth() {
        let usb = r"HID\VID_054C&PID_0CE6&MI_03\7&2f3c&0&0000";
        let bt = r"HID\{00001124-0000-1000-8000-00805f9b34fb}_VID&0002054c_PID&0ce6\9&1&0";
        let xusb = r"USB\VID_045E&PID_0B12&MI_00\7&1&0&0000";
        assert!(is_pad_instance(usb, 0x054c, 0x0ce6));
        assert!(is_pad_instance(bt, 0x054c, 0x0ce6));
        assert!(is_pad_instance(xusb, 0x045e, 0x0b12));
        assert!(!is_pad_instance(usb, 0x045e, 0x0b12));
        assert!(!is_pad_instance(r"SWD\VID_054C&PID_0CE6", 0x054c, 0x0ce6));
    }

    #[test]
    fn exe_path_in_device_notation() {
        let dev = r"\Device\HarddiskVolume3";
        assert_eq!(
            device_path(r"C:\Tako\tako.exe", dev).as_deref(),
            Some(r"\Device\HarddiskVolume3\Tako\tako.exe")
        );
        assert_eq!(
            device_path(r"\\?\C:\Tako\tako.exe", dev).as_deref(),
            Some(r"\Device\HarddiskVolume3\Tako\tako.exe")
        );
        assert_eq!(device_path(r"\\server\share\tako.exe", dev), None);
        assert_eq!(device_path("C", dev), None);
    }

    #[test]
    fn only_what_tako_added_is_removed() {
        let user = vec!["HID\\MEU".to_owned()];
        let (list, added) = add_to(&user, &["HID\\TAKO".into(), "hid\\meu".into()]);
        assert_eq!(list, ["HID\\MEU", "HID\\TAKO"]);
        assert_eq!(added, ["HID\\TAKO"]);
        assert_eq!(remove_from(&list, &added), user);

        let c = Changes {
            hidden: added,
            allowed: vec![],
            was_active: true,
        };
        let text = serde_json::to_string(&c).unwrap();
        assert_eq!(Changes::parse(&text), c);
        assert!(Changes::parse("{lixo").is_empty());
        assert!(Changes::parse(&"x".repeat(70_000)).is_empty());
    }
}
