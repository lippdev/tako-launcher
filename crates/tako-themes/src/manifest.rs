//! `theme.toml`: leitura e validação do manifesto (ideias de `ThemeManifestValidator.cs`).

use tako_core::i18n::{t, tf};

use crate::Problem;

pub const MANIFEST_FILE: &str = "theme.toml";
pub const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
/// Versão da API de temas exposta pelo `tako-ui`; aceita a mesma versão maior.
pub const THEME_API_VERSION: u32 = 1;
pub const DEFAULT_ENTRY: &str = "main.slint";
pub const MAX_ID_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    /// Como escrito pelo autor ("1", "1.2").
    pub api: String,
    pub description: Option<String>,
    /// Arquivo `.slint` principal, relativo à raiz do tema.
    pub entry: String,
}

impl ThemeManifest {
    /// Valida tudo o que dá para ver só no texto; os arquivos do pacote são conferidos por quem chama.
    pub fn parse(text: &str) -> Result<ThemeManifest, Vec<Problem>> {
        let table: toml::Table = toml::from_str(text).map_err(|e| {
            vec![Problem::new(
                "manifest.invalid",
                tf("themes.manifest.invalid", &[("error", e.message())]),
            )]
        })?;
        let mut problems = Vec::new();
        let mut text_field = |key: &str| match table.get(key) {
            Some(toml::Value::String(s)) if !s.trim().is_empty() => Some(s.trim().to_owned()),
            _ => {
                problems.push(Problem::new(
                    "manifest.missing-field",
                    tf("themes.manifest.missing_field", &[("field", key)]),
                ));
                None
            }
        };
        let (id, name, version, author) = (
            text_field("id"),
            text_field("name"),
            text_field("version"),
            text_field("author"),
        );
        let api = match table.get("api") {
            Some(toml::Value::Integer(n)) => Some(n.to_string()),
            Some(toml::Value::String(s)) => Some(s.trim().to_owned()),
            _ => None,
        };
        match &api {
            None => problems.push(Problem::new(
                "manifest.missing-field",
                tf("themes.manifest.missing_field", &[("field", "api")]),
            )),
            Some(a) => match a.split('.').next().and_then(|m| m.parse::<u32>().ok()) {
                Some(THEME_API_VERSION) => {}
                Some(_) => problems.push(Problem::new(
                    "manifest.unsupported-api",
                    tf(
                        "themes.manifest.unsupported_api",
                        &[("api", a), ("supported", &THEME_API_VERSION.to_string())],
                    ),
                )),
                None => problems.push(Problem::new(
                    "manifest.invalid-api",
                    tf("themes.manifest.invalid_api", &[("api", a)]),
                )),
            },
        }
        if let Some(id) = &id
            && !is_safe_id(id)
        {
            problems.push(Problem::new(
                "manifest.invalid-id",
                tf(
                    "themes.manifest.invalid_id",
                    &[("id", id), ("max", &MAX_ID_LEN.to_string())],
                ),
            ));
        }
        let entry = match table.get("entry") {
            None => Some(DEFAULT_ENTRY.to_owned()),
            Some(toml::Value::String(s)) => match crate::package::normalize_entry(s) {
                Ok(Some(p)) if p.to_ascii_lowercase().ends_with(".slint") => Some(p),
                _ => None,
            },
            Some(_) => None,
        };
        if entry.is_none() {
            problems.push(Problem::new(
                "manifest.invalid-entry",
                t("themes.manifest.invalid_entry"),
            ));
        }
        let description = match table.get("description") {
            Some(toml::Value::String(s)) => Some(s.clone()),
            _ => None,
        };
        match (id, name, version, author, api, entry) {
            (Some(id), Some(name), Some(version), Some(author), Some(api), Some(entry))
                if problems.is_empty() =>
            {
                Ok(ThemeManifest {
                    id,
                    name,
                    version,
                    author,
                    api,
                    description,
                    entry,
                })
            }
            _ => Err(problems),
        }
    }
}

/// Port de `ThemeId.IsSafe`: seguro como nome de pasta em qualquer SO.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_ID_LEN
        && !id.starts_with('.')
        && !id.ends_with('.')
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
        && !is_device_name(id.split('.').next().unwrap_or(id))
}

/// CON, NUL, COM1… são dispositivos no Windows mesmo com extensão ("nul.txt").
pub fn is_device_name(stem: &str) -> bool {
    const NAMES: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = stem.trim_end();
    NAMES.iter().any(|n| n.eq_ignore_ascii_case(stem))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OK: &str = "id = \"autor.tema\"\nname = \"Tema de teste\"\nversion = \"1.0\"\nauthor = \"Autor\"\napi = 1\n";

    fn codes(text: &str) -> Vec<&'static str> {
        ThemeManifest::parse(text)
            .err()
            .unwrap_or_default()
            .iter()
            .map(|p| p.code)
            .collect()
    }

    #[test]
    fn reads_a_manifest_with_defaults() {
        let m = ThemeManifest::parse(OK).unwrap();
        assert_eq!(m.id, "autor.tema");
        assert_eq!(m.entry, "main.slint");
        assert_eq!(m.description, None);
        let m = ThemeManifest::parse(&format!(
            "{OK}description = \"Escuro\"\nentry = \"./ui//home.slint\"\nextra = 3\n"
        ))
        .unwrap();
        assert_eq!(m.entry, "ui/home.slint");
        assert_eq!(m.description.as_deref(), Some("Escuro"));
    }

    #[test]
    fn reports_every_missing_field() {
        let c = codes("descricao = \"nada\"\n");
        assert_eq!(
            c.iter().filter(|c| **c == "manifest.missing-field").count(),
            5
        );
    }

    #[test]
    fn checks_the_api_major_version() {
        for (api, code) in [
            ("2", Some("manifest.unsupported-api")),
            ("\"0.9\"", Some("manifest.unsupported-api")),
            ("\"dois\"", Some("manifest.invalid-api")),
            ("\"1.4\"", None),
            ("\"1\"", None),
        ] {
            let text = OK.replace("api = 1", &format!("api = {api}"));
            assert_eq!(codes(&text).first().copied(), code, "{api}");
        }
    }

    #[test]
    fn refuses_ids_that_are_unsafe_folder_names() {
        for id in [
            "../fora",
            "a/b",
            "C:",
            "CON",
            "nul.tema",
            ".oculto",
            "termina.",
            "com espaço",
            "",
        ] {
            let text = OK.replace("autor.tema", id);
            assert!(!codes(&text).is_empty(), "{id:?}");
        }
        assert!(!is_safe_id(&"a".repeat(129)));
        assert!(is_safe_id("Autor_Tema-2.0"));
    }

    #[test]
    fn refuses_bad_entries_and_broken_toml() {
        for entry in ["\"../x.slint\"", "\"main.png\"", "3", "\"/abs.slint\""] {
            assert_eq!(
                codes(&format!("{OK}entry = {entry}\n")),
                ["manifest.invalid-entry"]
            );
        }
        assert_eq!(codes("id = "), ["manifest.invalid"]);
        assert_eq!(codes("\u{0}"), ["manifest.invalid"]);
    }
}
