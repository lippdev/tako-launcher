//! Port de `SteamArtResolver.cs`: endereços das artes de um jogo da Steam (`steam-art://570/cover`,
//! `/hero`, `/logo`, `/icon`), do melhor ao pior: arquivos 2x informados pela API da loja, o caminho
//! antigo da CDN e, sem rede, o cache local da própria Steam (`appcache/librarycache`).

use super::STEAM_ART_SCHEME;
use super::http::{Http, HttpRequest, escape, parse_json};
use crate::i18n::{self, Lang};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const ASSET_BASE: &str = "https://shared.akamai.steamstatic.com/store_item_assets/";
const LEGACY_BASE: &str = "https://shared.akamai.steamstatic.com/store_item_assets/steam/apps/";

/// Onde buscar uma arte: na rede ou num arquivo do cache da Steam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtSource {
    Url(String),
    File(PathBuf),
}

pub struct SteamArtResolver {
    http: Arc<dyn Http>,
    /// Pasta da Steam (para o cache local); `None` sem Steam instalada.
    steam_root: Option<PathBuf>,
    // Uma consulta por jogo, guardada durante a sessão.
    assets: Mutex<HashMap<u32, Option<Value>>>,
}

/// `steam-art://570/cover` → (570, "cover").
pub fn parse_uri(uri: &str) -> Option<(u32, &str)> {
    let (scheme, rest) = uri.split_once("://")?;
    if !scheme.eq_ignore_ascii_case(STEAM_ART_SCHEME) {
        return None;
    }
    let (app, kind) = rest.split_once('/').unwrap_or((rest, ""));
    Some((app.parse().ok()?, kind.trim_matches('/')))
}

impl SteamArtResolver {
    pub fn new(http: Arc<dyn Http>, steam_root: Option<PathBuf>) -> Self {
        SteamArtResolver {
            http,
            steam_root,
            assets: Mutex::new(HashMap::new()),
        }
    }

    pub fn resolve(&self, uri: &str) -> Vec<ArtSource> {
        let Some((app, kind)) = parse_uri(uri) else {
            return Vec::new();
        };
        let assets = self.assets(app);
        let mut out = Vec::new();
        let from_api = |keys: &[&str], out: &mut Vec<ArtSource>| {
            let Some(a) = &assets else { return };
            let Some(format) = a.get("asset_url_format").and_then(Value::as_str) else {
                return;
            };
            for k in keys {
                if let Some(name) = a.get(*k).and_then(Value::as_str).filter(|n| !n.is_empty()) {
                    out.push(ArtSource::Url(format!(
                        "{ASSET_BASE}{}",
                        format.replace("${FILENAME}", name)
                    )));
                }
            }
        };
        match kind {
            "icon" => {
                if let Some(hash) = assets
                    .as_ref()
                    .and_then(|a| a.get("community_icon"))
                    .and_then(Value::as_str)
                    .filter(|h| !h.is_empty() && h.bytes().all(|b| b.is_ascii_alphanumeric()))
                {
                    out.push(ArtSource::Url(format!(
                        "https://cdn.akamai.steamstatic.com/steamcommunity/public/images/apps/{app}/{hash}.jpg"
                    )));
                }
                out.extend(self.local(app, &["icon.jpg", "icon.png"]));
            }
            "logo" => {
                from_api(&["library_logo_2x", "library_logo"], &mut out);
                out.push(ArtSource::Url(format!("{LEGACY_BASE}{app}/logo_2x.png")));
                out.push(ArtSource::Url(format!("{LEGACY_BASE}{app}/logo.png")));
                out.extend(self.local(app, &["logo.png"]));
            }
            "hero" => {
                from_api(&["library_hero_2x", "library_hero"], &mut out);
                out.push(ArtSource::Url(format!(
                    "{LEGACY_BASE}{app}/library_hero_2x.jpg"
                )));
                out.extend(self.local(app, &["library_hero.jpg"]));
            }
            _ => {
                from_api(&["library_capsule_2x", "library_capsule"], &mut out);
                out.push(ArtSource::Url(format!(
                    "{LEGACY_BASE}{app}/library_600x900_2x.jpg"
                )));
                out.extend(self.local(app, &["library_600x900.jpg", "library_capsule.jpg"]));
            }
        }
        out
    }

    /// Cache da Steam: formato novo (`librarycache/570/…/nome`) e antigo (`570_nome`).
    fn local(&self, app: u32, names: &[&str]) -> Vec<ArtSource> {
        let Some(root) = &self.steam_root else {
            return Vec::new();
        };
        let cache = root.join("appcache").join("librarycache");
        let folder = cache.join(app.to_string());
        let mut out = Vec::new();
        for name in names {
            find_named(&folder, name, 3, &mut out);
            let flat = cache.join(format!("{app}_{name}"));
            if flat.is_file() {
                out.push(ArtSource::File(flat));
            }
        }
        out
    }

    fn assets(&self, app: u32) -> Option<Value> {
        if let Some(known) = self.lock().get(&app) {
            return known.clone();
        }
        let fetched = self.fetch_assets(app);
        self.lock().insert(app, fetched.clone());
        fetched
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<u32, Option<Value>>> {
        self.assets.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn fetch_assets(&self, app: u32) -> Option<Value> {
        let (language, country) = match i18n::lang() {
            Lang::PtBr => ("brazilian", "BR"),
            Lang::EnUs => ("english", "US"),
        };
        let input = format!(
            r#"{{"ids":[{{"appid":{app}}}],"context":{{"language":"{language}","country_code":"{country}"}},"data_request":{{"include_assets":true}}}}"#
        );
        let url = format!(
            "https://api.steampowered.com/IStoreBrowseService/GetItems/v1/?input_json={}",
            escape(&input)
        );
        let r = match self.http.send(&HttpRequest::get(url)) {
            Ok(r) if r.is_success() => r,
            Ok(r) => {
                tracing::info!(
                    app,
                    status = r.status,
                    "Artes da Steam indisponíveis pela API"
                );
                return None;
            }
            Err(e) => {
                tracing::info!(app, "Artes da Steam indisponíveis pela API: {e}");
                return None;
            }
        };
        // A API respondeu: sem artes vira `Null`, para `answered` separar "não existe" de "sem rede".
        let assets = parse_json(&r.body).and_then(|v| {
            v.get("response")?
                .get("store_items")?
                .get(0)?
                .get("assets")
                .cloned()
        });
        Some(assets.unwrap_or(Value::Null))
    }

    /// A API da Steam respondeu sobre o `app` (com ou sem artes) nesta sessão.
    pub fn answered(&self, app: u32) -> bool {
        self.lock().get(&app).is_some_and(Option::is_some)
    }
}

fn find_named(dir: &Path, name: &str, depth: u32, out: &mut Vec<ArtSource>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        match e.file_type() {
            Ok(t) if t.is_dir() && depth > 0 => find_named(&p, name, depth - 1, out),
            Ok(t) if t.is_file() && e.file_name() == name => out.push(ArtSource::File(p)),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::test_support::FakeHttp;

    #[test]
    fn api_assets_come_first_then_legacy_cdn_then_local_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("appcache/librarycache");
        std::fs::create_dir_all(cache.join("570/abc")).unwrap();
        std::fs::write(cache.join("570/abc/library_600x900.jpg"), [1]).unwrap();
        std::fs::write(cache.join("570_library_hero.jpg"), [1]).unwrap();
        let http = Arc::new(FakeHttp::new());
        http.push(
            200,
            r#"{"response":{"store_items":[{"assets":{"asset_url_format":"steam/apps/570/${FILENAME}?t=1","library_capsule_2x":"cap2x.jpg","library_hero":"hero.jpg","community_icon":"deadbeef"}}]}}"#,
        );
        let r = SteamArtResolver::new(http.clone(), Some(dir.path().into()));
        assert_eq!(
            r.resolve("steam-art://570/cover"),
            [
                ArtSource::Url(format!("{ASSET_BASE}steam/apps/570/cap2x.jpg?t=1")),
                ArtSource::Url(format!("{LEGACY_BASE}570/library_600x900_2x.jpg")),
                ArtSource::File(cache.join("570/abc/library_600x900.jpg")),
            ]
        );
        let hero = r.resolve("steam-art://570/hero");
        assert_eq!(
            hero.last(),
            Some(&ArtSource::File(cache.join("570_library_hero.jpg")))
        );
        assert!(
            matches!(&r.resolve("steam-art://570/icon")[0], ArtSource::Url(u) if u.ends_with("/570/deadbeef.jpg"))
        );
        assert_eq!(http.sent().len(), 1); // uma consulta por jogo
        assert!(r.resolve("https://x/570/cover").is_empty());
        assert!(r.resolve("steam-art://abc/cover").is_empty());
    }

    #[test]
    fn broken_api_answers_fall_back_to_the_cdn() {
        let http = Arc::new(FakeHttp::with_fallback(200, "{\"response\":5}"));
        let r = SteamArtResolver::new(http, None);
        assert_eq!(
            r.resolve("steam-art://10/logo"),
            [
                ArtSource::Url(format!("{LEGACY_BASE}10/logo_2x.png")),
                ArtSource::Url(format!("{LEGACY_BASE}10/logo.png")),
            ]
        );
    }
}
