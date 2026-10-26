//! Pacotes MSIX/AppX sem WinRT (port de `GamingRoot`, `PackageIdentity` e `PackageAssets`): pastas de
//! jogos do app Xbox, nome da família, AUMID e as imagens do pacote.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Arquivo oculto que o app Xbox grava na raiz de cada unidade escolhida para jogos.
pub const GAMING_ROOT_FILE: &str = ".GamingRoot";
/// Pasta que o app Xbox usa quando o arquivo não existe ou não pôde ser lido.
pub const DEFAULT_GAMES_FOLDER: &str = "XboxGames";
const MAX_FOLDERS: usize = 64;

/// `.GamingRoot`: "RGBX", u32 (quantidade) e as pastas em UTF-16LE terminadas em zero. Vazio se o
/// arquivo não tiver o formato esperado.
pub fn parse_gaming_root(data: &[u8]) -> Vec<String> {
    if data.len() < 8 || &data[..4] != b"RGBX" {
        return Vec::new();
    }
    let declared = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let limit = if (1..=MAX_FOLDERS).contains(&declared) {
        declared
    } else {
        MAX_FOLDERS
    };
    let units: Vec<u16> = data[8..]
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    units
        .split(|&u| u == 0)
        .map(|s| String::from_utf16_lossy(s).trim().to_owned())
        .filter(|s| !s.is_empty())
        .take(limit)
        .collect()
}

/// Pastas de jogos de uma unidade (`drive_root`, ex.: `D:\`): as do arquivo, ou a padrão.
pub fn gaming_folders(drive_root: &Path, data: &[u8]) -> Vec<PathBuf> {
    let declared = parse_gaming_root(data);
    let names = if declared.is_empty() {
        vec![DEFAULT_GAMES_FOLDER.to_owned()]
    } else {
        declared
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for f in names {
        let p = if crate::windows_path::is_rooted(&f) {
            PathBuf::from(f)
        } else {
            drive_root.join(f.trim_start_matches(['\\', '/']))
        };
        if !out.iter().any(|o| {
            o.to_string_lossy()
                .eq_ignore_ascii_case(&p.to_string_lossy())
        }) {
            out.push(p);
        }
    }
    out
}

// Base32 do Windows para o id do publicador (sem i, l, o, u).
const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Id do publicador como o Windows calcula: SHA-256 do `Publisher` (UTF-16LE), primeiros 8 bytes,
/// 13 caracteres em base32. Ex.: "CN=Microsoft Corporation, …" → `8wekyb3d8bbwe`.
pub fn publisher_id(publisher: &str) -> String {
    let utf16: Vec<u8> = publisher
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let hash = Sha256::digest(&utf16);
    let bits = hash[..8]
        .iter()
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b));
    (0..13)
        .map(|i| {
            // 65 bits de 5 em 5 a partir do mais significativo; o último grupo tem 4 bits + o zero.
            let shift = 59 - i * 5;
            let v = if shift >= 0 {
                (bits >> shift) & 0x1F
            } else {
                (bits << 1) & 0x1F
            };
            ALPHABET[v as usize] as char
        })
        .collect()
}

pub fn family_name(name: &str, publisher: &str) -> String {
    format!("{name}_{}", publisher_id(publisher))
}

/// Família a partir do nome completo (`Nome_Versão_Arquitetura_Recurso_IdDoPublicador`).
pub fn family_from_full_name(full_name: &str) -> Option<String> {
    let parts: Vec<&str> = full_name.split('_').collect();
    (parts.len() == 5 && !parts[0].is_empty() && parts[4].len() == 13)
        .then(|| format!("{}_{}", parts[0], parts[4]))
}

/// Nome do pacote (antes do primeiro "_").
pub fn name_of(family_or_full_name: &str) -> &str {
    family_or_full_name
        .split('_')
        .next()
        .unwrap_or(family_or_full_name)
}

/// Alvo para abrir o app pelo shell, como o menu Iniciar (respeita a proteção do pacote).
pub fn launch_target(family: &str, app_id: &str) -> String {
    format!(r"shell:AppsFolder\{family}!{app_id}")
}

/// Texto para `SHLoadIndirectString` que resolve um `ms-resource:`; `None` se já é o texto final.
pub fn indirect_string(
    package_full_name: &str,
    package_name: &str,
    value: Option<&str>,
) -> Option<String> {
    const PREFIX: &str = "ms-resource:";
    let value = value?;
    let head = value.get(..PREFIX.len())?;
    if !head.eq_ignore_ascii_case(PREFIX) {
        return None;
    }
    let key = value[PREFIX.len()..].trim();
    let uri = if key.starts_with("//") {
        format!("ms-resource:{key}")
    } else if key.starts_with('/') {
        format!("ms-resource://{package_name}{key}")
    } else if key.contains('/') {
        format!("ms-resource://{package_name}/{key}")
    } else {
        format!("ms-resource://{package_name}/resources/{key}")
    };
    Some(format!("@{{{package_full_name}?{uri}}}"))
}

/// Entre os arquivos da pasta, o melhor para `declared` (ex.: `Logo.png`): o próprio, senão a
/// variação de maior escala sem alto contraste (`Logo.scale-200.png`).
pub fn pick_asset<'a>(declared: &str, files: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let (stem, ext) = match declared.rfind('.') {
        Some(i) if i > 0 => (&declared[..i], &declared[i..]),
        _ => (declared, ""),
    };
    let mut best: Option<(&str, i64)> = None;
    for file in files {
        if file.eq_ignore_ascii_case(declared) {
            return Some(file);
        }
        let lower = file.to_lowercase();
        let prefix = format!("{}.", stem.to_lowercase());
        if !lower.starts_with(&prefix)
            || !lower.ends_with(&ext.to_lowercase())
            || lower.len() <= prefix.len() + ext.len()
        {
            continue;
        }
        let Some(qualifiers) = lower.get(prefix.len()..lower.len() - ext.len()) else {
            continue;
        };
        let s = score(qualifiers);
        if best.is_none_or(|(_, b)| s > b) {
            best = Some((file, s));
        }
    }
    best.map(|(f, _)| f)
}

// Maior escala/tamanho primeiro; alto contraste e "unplated" por último.
fn score(qualifiers: &str) -> i64 {
    qualifiers
        .split(['_', '.'])
        .map(|part| {
            let (name, value) = part.split_once('-').unwrap_or((part, ""));
            let n: i64 = value.parse().unwrap_or(0);
            match name {
                "scale" | "targetsize" => n,
                "contrast" => -10_000,
                "altform" => -1_000,
                _ => 0,
            }
        })
        .sum()
}

/// Caminho real de uma imagem citada no manifesto, dentro da pasta do pacote; `None` se não existir.
pub fn resolve_asset(package_folder: &Path, declared: Option<&str>) -> Option<PathBuf> {
    let declared = declared.filter(|d| !d.trim().is_empty())?;
    if declared.contains("..")
        || crate::windows_path::is_rooted(declared)
        || declared.starts_with(['/', '\\'])
    {
        return None;
    }
    let parts: Vec<&str> = declared
        .split(['\\', '/'])
        .filter(|p| !p.is_empty())
        .collect();
    let (file, dirs) = parts.split_last()?;
    let folder = dirs
        .iter()
        .fold(package_folder.to_path_buf(), |p, d| p.join(d));
    let names: Vec<String> = std::fs::read_dir(&folder)
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    pick_asset(file, names.iter().map(String::as_str)).map(|f| folder.join(f))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const MICROSOFT: &str =
        "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US";
    const CONTOSO: &str = "CN=Contoso Games, O=Contoso, L=Seattle, S=Washington, C=US";
    const FABRIKAM: &str = "CN=Fabrikam, O=Fabrikam, C=US";

    pub fn gaming_root_bytes(folders: &[&str]) -> Vec<u8> {
        let mut b = b"RGBX".to_vec();
        b.extend((folders.len() as u32).to_le_bytes());
        for f in folders {
            b.extend(format!("{f}\0").encode_utf16().flat_map(u16::to_le_bytes));
        }
        b
    }

    #[test]
    fn publisher_id_matches_the_one_windows_computes() {
        assert_eq!(publisher_id(MICROSOFT), "8wekyb3d8bbwe");
        assert_eq!(
            family_name("Microsoft.GamingApp", MICROSOFT),
            "Microsoft.GamingApp_8wekyb3d8bbwe"
        );
        assert_eq!(publisher_id(CONTOSO).len(), 13);
        assert_ne!(publisher_id(CONTOSO), publisher_id(FABRIKAM));
    }

    #[test]
    fn full_names_give_the_family_and_launch_uses_the_aumid() {
        let family =
            family_from_full_name("Microsoft.GamingApp_2509.1001.27.0_x64__8wekyb3d8bbwe").unwrap();
        assert_eq!(family, "Microsoft.GamingApp_8wekyb3d8bbwe");
        assert_eq!(
            family_from_full_name("Microsoft.GamingApp_8wekyb3d8bbwe"),
            None
        );
        assert_eq!(name_of(&family), "Microsoft.GamingApp");
        assert_eq!(
            launch_target("Contoso.Racing_abc", "Game"),
            r"shell:AppsFolder\Contoso.Racing_abc!Game"
        );
    }

    #[test]
    fn resource_names_become_indirect_strings() {
        let f = |v| indirect_string("P_1_x64__abc", "Contoso.Racing", v);
        assert_eq!(
            f(Some("ms-resource:AppName")).unwrap(),
            "@{P_1_x64__abc?ms-resource://Contoso.Racing/resources/AppName}"
        );
        assert_eq!(
            f(Some("ms-resource:Strings/AppName")).unwrap(),
            "@{P_1_x64__abc?ms-resource://Contoso.Racing/Strings/AppName}"
        );
        assert_eq!(
            f(Some("ms-resource:/Strings/AppName")).unwrap(),
            "@{P_1_x64__abc?ms-resource://Contoso.Racing/Strings/AppName}"
        );
        assert_eq!(
            f(Some("ms-resource://Contoso.Racing/resources/AppName")).unwrap(),
            "@{P_1_x64__abc?ms-resource://Contoso.Racing/resources/AppName}"
        );
        assert_eq!(f(Some("Contoso Racing")), None);
        assert_eq!(f(Some("ç")), None);
        assert_eq!(f(None), None);
    }

    #[test]
    fn gaming_root_lists_the_folders_of_each_drive() {
        let drive = Path::new("/tmp/D");
        let data = gaming_root_bytes(&[r"\XboxGames", r"E:\Jogos Xbox"]);
        assert_eq!(parse_gaming_root(&data), [r"\XboxGames", r"E:\Jogos Xbox"]);
        assert_eq!(
            gaming_folders(drive, &data),
            [drive.join("XboxGames"), PathBuf::from(r"E:\Jogos Xbox")]
        );

        // Arquivo ausente ou estranho: a pasta padrão do app Xbox.
        assert!(parse_gaming_root(b"XXXX\x01\0\0\0").is_empty());
        assert!(parse_gaming_root(&[]).is_empty());
        assert_eq!(gaming_folders(drive, &[]), [drive.join("XboxGames")]);

        // Quantidade declarada maior que o conteúdo, sem o zero final: lê o que houver.
        let mut truncated = b"RGBX\x09\0\0\0".to_vec();
        truncated.extend("Jogos".encode_utf16().flat_map(u16::to_le_bytes));
        truncated.push(0x41); // byte solto no fim
        assert_eq!(parse_gaming_root(&truncated), ["Jogos"]);
    }

    #[test]
    fn package_assets_pick_the_largest_plain_variant() {
        let files = [
            "Logo.scale-100.png",
            "Logo.scale-200.png",
            "Logo.contrast-black_scale-400.png",
            "Logo.altform-unplated_scale-400.png",
            "Logo.jpg",
            "Icon.targetsize-48.png",
            "Icon.targetsize-256.png",
            "Exact.png",
            "Exact.scale-400.png",
            "LogoExtra.png",
        ];
        let pick = |d| pick_asset(d, files);
        assert_eq!(pick("Logo.png"), Some("Logo.scale-200.png"));
        assert_eq!(pick("logo.PNG"), Some("Logo.scale-200.png"));
        assert_eq!(pick("Icon.png"), Some("Icon.targetsize-256.png"));
        assert_eq!(pick("Exact.png"), Some("Exact.png"));
        assert_eq!(pick("Missing.png"), None);
        assert_eq!(pick("Ação.png"), None);
    }

    #[test]
    fn package_assets_resolve_inside_the_package_only() {
        let dir = tempfile::tempdir().unwrap();
        let package = dir.path().join("pkg");
        std::fs::create_dir_all(package.join("Assets")).unwrap();
        std::fs::write(package.join("Assets/Square150x150Logo.scale-200.png"), b"").unwrap();

        assert_eq!(
            resolve_asset(&package, Some(r"Assets\Square150x150Logo.png")),
            Some(
                package
                    .join("Assets")
                    .join("Square150x150Logo.scale-200.png")
            )
        );
        assert_eq!(resolve_asset(&package, Some(r"..\outro\Logo.png")), None);
        assert_eq!(resolve_asset(&package, Some(r"Nada\Logo.png")), None);
        assert_eq!(resolve_asset(&package, None), None);
    }
}
