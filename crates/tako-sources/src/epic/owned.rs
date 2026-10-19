//! Jogos da conta Epic que não estão instalados (SRC-005), lidos offline do cache de catálogo da Epic Games
//! Launcher: `Data\Catalog\catcache.bin`, um JSON em base64 com os itens da biblioteca (mesma leitura do
//! rai-pal, `game_providers/epic_provider.rs`, e do DLSS Swapper, `EpicGamesStoreLibrary.cs`). Cada item é o
//! do catálogo da Epic: `id` (CatalogItemId), `namespace`, `title`, `categories[].path`, `releaseInfo[].appId`
//! (o AppName dos manifests) e `mainGameItem` nas DLCs.

use super::{ACCENT, escape_data};
use serde::Deserialize;
use std::path::Path;
use tako_core::library::{Game, GameStore};

/// O cache traz descrições e imagens de cada item: bibliotecas grandes passam dos 16 MB das outras lojas.
const MAX_CATCACHE: u64 = 64 << 20;

/// Jogo da conta, como o cache do catálogo informa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpicOwned {
    pub app_name: String,
    pub title: String,
    pub namespace: String,
    pub catalog_item_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    #[serde(default)]
    id: String,
    #[serde(default)]
    namespace: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    categories: Vec<Category>,
    #[serde(default)]
    release_info: Vec<Release>,
    main_game_item: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Category {
    #[serde(default)]
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Release {
    #[serde(default)]
    app_id: String,
}

/// Base64 padrão (o que a Epic grava), ignorando quebras de linha; `None` em qualquer outro caractere.
fn decode_base64(text: &[u8]) -> Option<Vec<u8>> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0u32);
    for &c in text.iter().filter(|c| !c.is_ascii_whitespace()) {
        if c == b'=' {
            break;
        }
        acc = (acc << 6) | u32::from(value(c)?);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// Itens do `catcache.bin` que são jogos da loja: fora DLCs (`mainGameItem`), apps, plugins e itens sem AppName.
/// `None` quando o arquivo não é o cache esperado.
pub fn parse(bytes: &[u8]) -> Option<Vec<EpicOwned>> {
    let json = decode_base64(bytes)?;
    let items: Vec<serde_json::Value> = serde_json::from_slice(&json).ok()?;
    let mut out: Vec<EpicOwned> = Vec::new();
    // Um item fora do formato não derruba os outros.
    for item in items.into_iter().filter_map(|v| Item::deserialize(v).ok()) {
        let is_game = item.categories.iter().any(|c| c.path == "games");
        let app_name = item
            .release_info
            .first()
            .map(|r| r.app_id.trim().to_owned())
            .unwrap_or_default();
        if !is_game
            || item.main_game_item.as_ref().is_some_and(|m| !m.is_null())
            || app_name.is_empty()
            || item.id.trim().is_empty()
            || item.namespace.trim().is_empty()
            || item.title.trim().is_empty()
            || out.iter().any(|o| o.app_name == app_name)
        {
            continue;
        }
        out.push(EpicOwned {
            app_name,
            title: item.title.trim().to_owned(),
            namespace: item.namespace.trim().to_owned(),
            catalog_item_id: item.id.trim().to_owned(),
        });
    }
    Some(out)
}

/// O cache no disco; ausente, grande demais ou quebrado = nenhum.
pub fn load(file: &Path) -> Vec<EpicOwned> {
    match crate::read_limited(file, MAX_CATCACHE) {
        Ok(bytes) => parse(&bytes).unwrap_or_else(|| {
            tracing::warn!("catcache.bin da Epic ilegível: {}", file.display());
            Vec::new()
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            tracing::warn!("não deu para ler {}: {e}", file.display());
            Vec::new()
        }
    }
}

/// `com.epicgames.launcher://apps/<Namespace>%3A<CatalogItemId>%3A<AppName>?action=install`.
pub fn install_uri(o: &EpicOwned) -> String {
    install_uri_of(&o.namespace, &o.catalog_item_id, &o.app_name)
}

pub(super) fn install_uri_of(namespace: &str, catalog_item_id: &str, app_name: &str) -> String {
    format!(
        "com.epicgames.launcher://apps/{}%3A{}%3A{}?action=install",
        escape_data(namespace),
        escape_data(catalog_item_id),
        escape_data(app_name)
    )
}

/// Jogo não instalado com o id do instalado (`epic:<AppName>`), para virar instalado no lugar.
pub fn to_game(o: &EpicOwned) -> Game {
    Game {
        id: format!("epic:{}", o.app_name),
        title: o.title.clone(),
        store: Some(GameStore::Epic),
        accent: ACCENT.to_owned(),
        is_installed: false,
        store_product_id: Some(o.catalog_item_id.clone()),
        install_uri: Some(install_uri(o)),
        ..Default::default()
    }
}

/// Os da conta que não estão em `installed`.
pub fn not_installed(owned: &[EpicOwned], installed: &[Game]) -> Vec<Game> {
    owned
        .iter()
        .map(to_game)
        .filter(|g| !installed.iter().any(|i| i.id == g.id))
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Como a Epic grava: o JSON do catálogo em base64 (aqui com quebras de linha, que também valem).
    pub(crate) fn catcache(json: &str) -> Vec<u8> {
        const ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = Vec::new();
        for (n, chunk) in json.as_bytes().chunks(3).enumerate() {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let v = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                out.push(if i <= chunk.len() {
                    ABC[(v >> (18 - 6 * i) & 63) as usize]
                } else {
                    b'='
                });
            }
            if n % 19 == 18 {
                out.push(b'\n');
            }
        }
        out
    }

    pub(crate) const CATALOG: &str = r#"[
      {"id":"a1b2c3d4e5f60718293a4b5c6d7e8f90","namespace":"farolns","title":"Farol do Fim do Mundo",
       "description":"Um farol.","keyImages":[{"type":"DieselGameBoxTall","url":"https://cdn1.epicgames.com/x.jpg","height":1600,"width":1200}],
       "categories":[{"path":"games"},{"path":"applications"}],
       "releaseInfo":[{"id":"r1","appId":"Farol","platform":["Windows"],"dateAdded":"2021-05-04T15:00:00.000Z"}]},
      {"id":"7e57ab1e0000000000000000000000ab","namespace":"mariscons","title":" Maré Alta ",
       "categories":[{"path":"games"},{"path":"applications"}],
       "releaseInfo":[{"id":"r2","appId":"MareAlta","platform":["Windows","Mac"]}]},
      {"id":"d1c0000000000000000000000000000d","namespace":"farolns","title":"Farol - Trilha Sonora",
       "categories":[{"path":"addons"},{"path":"addons/durable"}],
       "releaseInfo":[{"id":"r3","appId":"FarolOst"}],
       "mainGameItem":{"id":"a1b2c3d4e5f60718293a4b5c6d7e8f90","namespace":"farolns"}},
      {"id":"ue0000000000000000000000000000ee","namespace":"ue","title":"Unreal Engine",
       "categories":[{"path":"engines"}],"releaseInfo":[{"appId":"UE_5.3"}]},
      {"id":"nomeapp0000000000000000000000000","namespace":"x","title":"Sem AppName",
       "categories":[{"path":"games"}],"releaseInfo":[]},
      {"id":42,"title":"Fora do formato"}
    ]"#;

    #[test]
    fn reads_games_from_the_base64_catalog_and_skips_dlc_and_non_games() {
        let owned = parse(&catcache(CATALOG)).unwrap();
        assert_eq!(owned.len(), 2);
        assert_eq!(
            owned[0],
            EpicOwned {
                app_name: "Farol".into(),
                title: "Farol do Fim do Mundo".into(),
                namespace: "farolns".into(),
                catalog_item_id: "a1b2c3d4e5f60718293a4b5c6d7e8f90".into(),
            }
        );
        assert_eq!(owned[1].title, "Maré Alta");
    }

    #[test]
    fn broken_cache_is_nothing() {
        assert_eq!(parse(b"isto nao e base64!"), None);
        assert_eq!(parse(&catcache("{ quebrado")), None);
        assert_eq!(parse(&catcache(r#"{"games":[]}"#)), None);
        assert_eq!(parse(&catcache("[]")), Some(Vec::new()));
        assert!(load(Path::new("/nao/existe/catcache.bin")).is_empty());
    }

    #[test]
    fn base64_matches_the_standard_alphabet() {
        assert_eq!(decode_base64(b"VGFrbw==").unwrap(), b"Tako");
        assert_eq!(decode_base64(b"VGFr\r\nbyE=").unwrap(), b"Tako!");
        assert_eq!(decode_base64(b"").unwrap(), b"");
    }

    #[test]
    fn install_opens_the_epic_launcher_with_the_same_id_as_the_installed_game() {
        let owned = parse(&catcache(CATALOG)).unwrap();
        let g = to_game(&owned[1]);
        assert_eq!(g.id, "epic:MareAlta");
        assert!(!g.is_installed && g.launch_target.is_none());
        assert_eq!(
            g.install_uri.as_deref(),
            Some(
                "com.epicgames.launcher://apps/mariscons%3A7e57ab1e0000000000000000000000ab%3AMareAlta?action=install"
            )
        );
        let installed = Game {
            id: "epic:Farol".into(),
            is_installed: true,
            ..Default::default()
        };
        let rest = not_installed(&owned, &[installed]);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].id, "epic:MareAlta");
    }
}
