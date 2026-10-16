//! Port de `SteamGridDb/*` (ADR-0023): capas, fundos, logos e ícones da comunidade com a chave do
//! próprio usuário. Como fonte automática só completa a capa e o fundo de quem ficou sem.

use super::art::ArtCandidate;
use super::http::{Clock, Http, HttpRequest, RateGate, ResponseCache, backoff, escape};
use super::igdb::{SettingsFn, mask};
use super::{
    GameMetadata, MetadataLookup, MetadataMatch, MetadataProvider, SecretStore, external_ids,
    looks_like_key, sources,
};
use crate::i18n::t;
use crate::library::ArtKind;
use chrono::{DateTime, Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const BASE_URL: &str = "https://www.steamgriddb.com/api/v2/";
pub const VERTICAL_COVER: [&str; 3] = ["600x900", "342x482", "660x930"];
pub const HORIZONTAL_COVER: [&str; 2] = ["920x430", "460x215"];
const MAX_ATTEMPTS: u32 = 4;
const SECRET_KEY: &str = "metadata.steamgriddb";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteamGridDbFailure {
    NotConfigured,
    Unauthorized,
    RateLimited,
    Network,
    Unavailable,
}

impl SteamGridDbFailure {
    pub fn message(self) -> String {
        let key = match self {
            Self::NotConfigured => "not_configured",
            Self::Unauthorized => "unauthorized",
            Self::RateLimited => "rate_limited",
            Self::Network => "network",
            Self::Unavailable => "unavailable",
        };
        t(&format!("metadata.steamgriddb.error.{key}"))
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{detail}")]
pub struct SteamGridDbError {
    pub failure: SteamGridDbFailure,
    pub detail: String,
}

fn err(failure: SteamGridDbFailure, detail: impl Into<String>) -> SteamGridDbError {
    SteamGridDbError {
        failure,
        detail: detail.into(),
    }
}

/// Ligado ou não, a chave do usuário, "usar quando faltar capa" e "mostrar animadas".
#[derive(Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SteamGridDbSettings {
    pub enabled: bool,
    pub api_key: String,
    /// Desligado por padrão.
    pub auto_fill: bool,
    pub animated: bool,
}

impl std::fmt::Debug for SteamGridDbSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SteamGridDbSettings")
            .field("enabled", &self.enabled)
            .field("api_key", &mask(&self.api_key))
            .field("auto_fill", &self.auto_fill)
            .field("animated", &self.animated)
            .finish()
    }
}

impl SteamGridDbSettings {
    pub fn has_key(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    pub fn is_active(&self) -> bool {
        self.enabled && self.has_key()
    }

    /// `None` = parece certa (32 letras e números).
    pub fn problem(&self) -> Option<String> {
        if self.api_key.is_empty() {
            Some(t("metadata.steamgriddb.problem.missing_key"))
        } else if !looks_like_key(&self.api_key) {
            Some(t("metadata.steamgriddb.problem.short_key"))
        } else {
            None
        }
    }

    pub fn load(store: &dyn SecretStore) -> Self {
        store
            .get(SECRET_KEY)
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, store: &dyn SecretStore) -> Result<(), String> {
        let clean = SteamGridDbSettings {
            api_key: self.api_key.trim().into(),
            ..self.clone()
        };
        store.set(
            SECRET_KEY,
            &serde_json::to_string(&clean).map_err(|e| e.to_string())?,
        )
    }
}

/// Filtro do seletor. NSFW e humor ficam sempre de fora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamGridDbFilter {
    pub kind: ArtKind,
    pub horizontal: bool,
    /// `None` = todos.
    pub style: Option<String>,
    pub animated: bool,
}

impl SteamGridDbFilter {
    pub fn new(kind: ArtKind) -> Self {
        SteamGridDbFilter {
            kind,
            horizontal: false,
            style: None,
            animated: false,
        }
    }
}

/// Estilos que a API aceita por tipo de imagem.
pub fn styles(kind: ArtKind) -> &'static [&'static str] {
    match kind {
        ArtKind::Cover => &["alternate", "blurred", "white_logo", "material", "no_logo"],
        ArtKind::Hero => &["alternate", "blurred", "material"],
        ArtKind::Logo => &["official", "white", "black", "custom"],
        ArtKind::Icon => &["official", "custom"],
    }
}

/// Nome do estilo para a tela.
pub fn style_label(style: Option<&str>) -> String {
    match style {
        None => t("metadata.steamgriddb.style.all"),
        Some(
            s @ ("alternate" | "blurred" | "white_logo" | "material" | "no_logo" | "official"
            | "white" | "black" | "custom"),
        ) => t(&format!("metadata.steamgriddb.style.{s}")),
        Some(s) => s.to_owned(),
    }
}

pub fn autocomplete(term: &str) -> String {
    format!("search/autocomplete/{}", escape(term.trim()))
}

pub fn by_steam_app_id(app_id: u32) -> String {
    format!("games/steam/{app_id}")
}

pub fn by_id(game_id: &str) -> String {
    format!("games/id/{}", escape(game_id))
}

/// Imagens do jogo com o filtro (`grids`, `heroes`, `logos`, `icons`).
pub fn images_path(game_id: &str, filter: &SteamGridDbFilter, page: u32) -> String {
    let endpoint = match filter.kind {
        ArtKind::Cover => "grids",
        ArtKind::Hero => "heroes",
        ArtKind::Logo => "logos",
        ArtKind::Icon => "icons",
    };
    let mut q = Vec::new();
    if filter.kind == ArtKind::Cover {
        let sizes: &[&str] = if filter.horizontal {
            &HORIZONTAL_COVER
        } else {
            &VERTICAL_COVER
        };
        q.push(format!("dimensions={}", sizes.join(",")));
    }
    if let Some(s) = filter
        .style
        .as_deref()
        .filter(|s| styles(filter.kind).contains(s))
    {
        q.push(format!("styles={s}"));
    }
    q.push(
        if filter.animated {
            "types=static,animated"
        } else {
            "types=static"
        }
        .into(),
    );
    q.push("nsfw=false".into());
    q.push("humor=false".into());
    if page > 0 {
        q.push(format!("page={page}"));
    }
    format!("{endpoint}/game/{}?{}", escape(game_id), q.join("&"))
}

fn size_of(c: &ArtCandidate) -> String {
    match (c.width, c.height) {
        (Some(w), Some(h)) => format!("{w}x{h}"),
        _ => String::new(),
    }
}

/// Aplica o filtro (a API pode ignorar algum parâmetro); a mais votada primeiro.
pub fn apply(candidates: &[ArtCandidate], filter: &SteamGridDbFilter) -> Vec<ArtCandidate> {
    let sizes: &[&str] = if filter.horizontal {
        &HORIZONTAL_COVER
    } else {
        &VERTICAL_COVER
    };
    let mut v: Vec<ArtCandidate> = candidates
        .iter()
        .filter(|c| c.kind == Some(filter.kind) && !c.is_nsfw && !c.is_humor)
        .filter(|c| filter.animated || !c.is_animated)
        .filter(|c| filter.style.is_none() || c.style == filter.style)
        .filter(|c| {
            filter.kind != ArtKind::Cover
                || c.width.is_none()
                || sizes.contains(&size_of(c).as_str())
        })
        .cloned()
        .collect();
    v.sort_by_key(|c| std::cmp::Reverse(c.score));
    v
}

/// Escolha automática: estática, sem NSFW/humor; capa prefere 600×900, depois a mais votada.
pub fn best_automatic(candidates: &[ArtCandidate], kind: ArtKind) -> Option<ArtCandidate> {
    let mut allowed = apply(candidates, &SteamGridDbFilter::new(kind));
    allowed.sort_by_key(|c| !(kind == ArtKind::Cover && size_of(c) == "600x900"));
    allowed.into_iter().next()
}

fn text(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_owned)
}

fn https(v: Option<String>) -> Option<String> {
    v.filter(|u| u.starts_with("https://"))
}

/// `search/autocomplete` (lista) ou `games/…` (um jogo) → resultados.
pub fn parse_games(data: &Value) -> Vec<MetadataMatch> {
    let items: Vec<&Value> = match data {
        Value::Array(a) => a.iter().collect(),
        Value::Object(_) => vec![data],
        _ => Vec::new(),
    };
    items
        .into_iter()
        .filter_map(|i| {
            let mut m = MetadataMatch::new(
                sources::STEAMGRIDDB,
                i.get("id")?.as_i64()?.to_string(),
                text(i, "name")?,
            );
            m.year = i
                .get("release_date")
                .and_then(Value::as_i64)
                .filter(|s| *s > 0)
                .and_then(|s| DateTime::from_timestamp(s, 0))
                .map(|d| d.year());
            m.platforms = i.get("types").and_then(Value::as_array).map(|types| {
                let mut labels: Vec<&str> = Vec::new();
                for l in types.iter().filter_map(Value::as_str).map(platform_label) {
                    if !labels.contains(&l) {
                        labels.push(l);
                    }
                }
                labels.join(", ")
            });
            Some(m)
        })
        .collect()
}

fn platform_label(t: &str) -> &str {
    match t {
        "steam" => "Steam",
        "origin" => "EA",
        "egs" => "Epic",
        "bnet" => "Battle.net",
        "uplay" => "Ubisoft",
        "flashpoint" => "Flashpoint",
        "eshop" => "Nintendo",
        other => other,
    }
}

/// Lista de imagens → candidatos com autor, estilo e tamanho (só https).
pub fn parse_images(data: &Value, kind: ArtKind) -> Vec<ArtCandidate> {
    let Some(items) = data.as_array() else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|i| {
            let url = https(text(i, "url"))?;
            let mime = text(i, "mime");
            let style = text(i, "style");
            let dim = |k| {
                i.get(k)
                    .and_then(Value::as_u64)
                    .and_then(|n| u32::try_from(n).ok())
            };
            Some(ArtCandidate {
                thumbnail: https(text(i, "thumb")),
                width: dim("width"),
                height: dim("height"),
                author: i.get("author").and_then(|a| text(a, "name")),
                score: i.get("score").and_then(Value::as_i64).unwrap_or(0),
                is_nsfw: i.get("nsfw").and_then(Value::as_bool) == Some(true),
                is_humor: i.get("humor").and_then(Value::as_bool) == Some(true),
                is_animated: matches!(mime.as_deref(), Some("image/webp" | "image/apng"))
                    || url.to_lowercase().ends_with(".webp"),
                label: Some(style_label(style.as_deref())),
                style,
                ..ArtCandidate::new(sources::STEAMGRIDDB, kind, url)
            })
        })
        .collect()
}

/// `{"success":true,"data":…}` → `data`.
pub fn data(json: &str) -> Option<Value> {
    let mut root: Value = serde_json::from_str(json).ok()?;
    if root.get("success").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    root.get_mut("data").map(Value::take)
}

/// Cliente da API v2 (`GET {BASE_URL}{caminho}`, `Authorization: Bearer`). No máximo 2 por segundo.
pub struct SteamGridDbClient {
    http: Arc<dyn Http>,
    settings: SettingsFn<SteamGridDbSettings>,
    clock: Arc<dyn Clock>,
    cache: Option<Box<dyn ResponseCache>>,
    gate: RateGate,
    queue: Mutex<()>,
    last_failure: Mutex<Option<SteamGridDbFailure>>,
    network: AtomicUsize,
}

impl SteamGridDbClient {
    pub fn new(
        http: Arc<dyn Http>,
        settings: SettingsFn<SteamGridDbSettings>,
        clock: Arc<dyn Clock>,
        cache: Option<Box<dyn ResponseCache>>,
    ) -> Self {
        SteamGridDbClient {
            http,
            settings,
            clock,
            cache,
            gate: RateGate::new(2, Duration::from_secs(1)),
            queue: Mutex::new(()),
            last_failure: Mutex::new(None),
            network: AtomicUsize::new(0),
        }
    }

    pub fn settings(&self) -> SteamGridDbSettings {
        (self.settings)()
    }

    pub fn is_configured(&self) -> bool {
        self.settings().is_active()
    }

    /// Última falha de rede ou de chave; limpa no próximo sucesso.
    pub fn last_failure(&self) -> Option<SteamGridDbFailure> {
        *self.last_failure.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn network_requests(&self) -> usize {
        self.network.load(Ordering::SeqCst)
    }

    /// O `data` da resposta; `Ok(None)` quando o SteamGridDB não conhece (404).
    pub fn get(&self, path: &str, use_cache: bool) -> Result<Option<Value>, SteamGridDbError> {
        let key = self.settings();
        if !key.has_key() {
            return Err(err(
                SteamGridDbFailure::NotConfigured,
                "SteamGridDB sem chave",
            ));
        }
        let cache = self.cache.as_deref().filter(|_| use_cache);
        if let Some(hit) = cache.and_then(|c| c.read(path)).and_then(|j| data(&j)) {
            return Ok(Some(hit));
        }
        let _one_at_a_time = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        let result = self.send(path, key.api_key.trim());
        *self.last_failure.lock().unwrap_or_else(|e| e.into_inner()) =
            result.as_ref().err().map(|e| e.failure);
        let Some(json) = result? else {
            return Ok(None);
        };
        let d = data(&json)
            .ok_or_else(|| err(SteamGridDbFailure::Unavailable, "Resposta inesperada"))?;
        if let Some(c) = cache {
            c.write(path, &json);
        }
        Ok(Some(d))
    }

    fn send(&self, path: &str, api_key: &str) -> Result<Option<String>, SteamGridDbError> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let wait = self.gate.reserve(self.clock.now());
            if !wait.is_zero() {
                self.clock.sleep(wait);
            }
            let req = HttpRequest::get(format!("{BASE_URL}{path}"))
                .header("Authorization", format!("Bearer {api_key}"))
                .header("Accept", "application/json");
            self.network.fetch_add(1, Ordering::SeqCst);
            let r = self
                .http
                .send(&req)
                .map_err(|e| err(SteamGridDbFailure::Network, format!("Sem conexão: {e}")))?;
            match r.status {
                200..=299 => return Ok(Some(r.text())),
                404 => return Ok(None),
                401 | 403 => {
                    return Err(err(
                        SteamGridDbFailure::Unauthorized,
                        format!("Chave recusada ({})", r.status),
                    ));
                }
                429 if attempt < MAX_ATTEMPTS => self.clock.sleep(backoff(attempt, r.retry_after)),
                429 => return Err(err(SteamGridDbFailure::RateLimited, "Limite de consultas")),
                s => {
                    return Err(err(
                        SteamGridDbFailure::Unavailable,
                        format!("Respondeu {s}"),
                    ));
                }
            }
        }
    }

    /// Botão "Testar": uma busca mínima com a chave informada, sem cache.
    pub fn test(
        http: Arc<dyn Http>,
        settings: &SteamGridDbSettings,
        clock: Arc<dyn Clock>,
    ) -> Result<(), SteamGridDbError> {
        let s = settings.clone();
        SteamGridDbClient::new(http, Arc::new(move || s.clone()), clock, None)
            .get(&autocomplete("portal"), false)
            .map(|_| ())
    }
}

/// Provedor SteamGridDB: pelo appid da Steam (`games/steam/{appid}`), senão pelo nome.
pub struct SteamGridDbProvider {
    client: SteamGridDbClient,
    clock: Arc<dyn Clock>,
    by_steam: Mutex<HashMap<u32, Option<String>>>,
}

impl SteamGridDbProvider {
    pub fn new(client: SteamGridDbClient, clock: Arc<dyn Clock>) -> Self {
        SteamGridDbProvider {
            client,
            clock,
            by_steam: Mutex::new(HashMap::new()),
        }
    }

    pub fn client(&self) -> &SteamGridDbClient {
        &self.client
    }

    fn query(&self, path: &str) -> Option<Value> {
        match self.client.get(path, true) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(failure = ?e.failure, "SteamGridDB: consulta falhou: {e}");
                None
            }
        }
    }

    /// Imagens do jogo com o filtro do seletor.
    pub fn images(
        &self,
        game_id: &str,
        filter: &SteamGridDbFilter,
        page: u32,
    ) -> Vec<ArtCandidate> {
        self.query(&images_path(game_id, filter, page))
            .map(|d| apply(&parse_images(&d, filter.kind), filter))
            .unwrap_or_default()
    }

    /// Busca livre por nome (campo de busca do seletor).
    pub fn search_term(&self, term: &str) -> Vec<MetadataMatch> {
        if !self.is_enabled() || term.trim().is_empty() {
            return Vec::new();
        }
        self.query(&autocomplete(term))
            .map(|d| parse_games(&d))
            .unwrap_or_default()
    }
}

impl MetadataProvider for SteamGridDbProvider {
    fn name(&self) -> &str {
        sources::STEAMGRIDDB
    }

    fn is_enabled(&self) -> bool {
        self.client.is_configured()
    }

    fn art_fallback(&self) -> Option<bool> {
        Some(self.client.settings().auto_fill)
    }

    fn search(&self, lookup: &MetadataLookup) -> Vec<MetadataMatch> {
        if !self.is_enabled() || lookup.title.trim().is_empty() {
            return Vec::new();
        }
        let mut results = Vec::new();
        if let Some(exact) = self.find_exact(lookup)
            && let Some(d) = self.query(&by_id(&exact))
        {
            results.extend(parse_games(&d).into_iter().map(|m| MetadataMatch {
                is_exact: true,
                ..m
            }));
        }
        if let Some(d) = self.query(&autocomplete(&lookup.title)) {
            for m in parse_games(&d) {
                if !results.iter().any(|r: &MetadataMatch| r.id == m.id) {
                    results.push(m);
                }
            }
        }
        results
    }

    fn find_exact(&self, lookup: &MetadataLookup) -> Option<String> {
        if !self.is_enabled() {
            return None;
        }
        let app: u32 = lookup.external_ids.get(external_ids::STEAM)?.parse().ok()?;
        if let Some(known) = self
            .by_steam
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&app)
        {
            return known.clone();
        }
        let id = self
            .query(&by_steam_app_id(app))
            .and_then(|d| parse_games(&d).into_iter().next())
            .map(|m| m.id);
        self.by_steam
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(app, id.clone());
        id
    }

    /// Nome do jogo e a melhor capa vertical e o melhor fundo estáticos.
    fn fetch(&self, id: &str) -> Option<GameMetadata> {
        if !self.is_enabled() || id.trim().is_empty() {
            return None;
        }
        let game = parse_games(&self.query(&by_id(id))?).into_iter().next()?;
        let best = |kind| best_automatic(&self.images(id, &SteamGridDbFilter::new(kind), 0), kind);
        Some(GameMetadata {
            fetched_at: Some(self.clock.now()),
            source_ids: [(sources::STEAMGRIDDB.to_owned(), id.to_owned())].into(),
            title: Some(game.title),
            release_date: game.year.and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1)),
            cover_url: best(ArtKind::Cover).map(|c| c.url),
            background_url: best(ArtKind::Hero).map(|c| c.url),
            ..GameMetadata::new(sources::STEAMGRIDDB)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::test_support::{FakeHttp, TestClock, now};

    const GRIDS: &str = r#"{"success":true,"data":[
      {"id":1,"score":3,"style":"alternate","width":600,"height":900,"nsfw":false,"humor":false,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/a.png","thumb":"https://cdn2.steamgriddb.com/thumb/a.jpg","author":{"name":"Ana"}},
      {"id":2,"score":9,"style":"blurred","width":342,"height":482,"nsfw":false,"humor":false,"mime":"image/jpeg","url":"https://cdn2.steamgriddb.com/grid/b.jpg","thumb":"https://cdn2.steamgriddb.com/thumb/b.jpg","author":{"name":"Bia"}},
      {"id":3,"score":20,"style":"alternate","width":600,"height":900,"nsfw":true,"humor":false,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/c.png","author":{"name":"Caio"}},
      {"id":4,"score":15,"style":"material","width":920,"height":430,"nsfw":false,"humor":false,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/d.png","author":{"name":"Davi"}},
      {"id":5,"score":30,"style":"alternate","width":600,"height":900,"nsfw":false,"humor":false,"mime":"image/webp","url":"https://cdn2.steamgriddb.com/grid/e.webp","thumb":"https://cdn2.steamgriddb.com/thumb/e.webp","author":{"name":"Eva"}},
      {"id":6,"score":50,"style":"alternate","width":600,"height":900,"nsfw":false,"humor":true,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/f.png","author":{"name":"Fred"}},
      {"id":7,"score":1,"url":"http://inseguro.example/x.png"}
    ]}"#;

    fn images() -> Vec<ArtCandidate> {
        parse_images(&data(GRIDS).unwrap(), ArtKind::Cover)
    }

    fn names(v: &[ArtCandidate]) -> Vec<&str> {
        v.iter()
            .map(|c| c.url.rsplit('/').next().unwrap())
            .collect()
    }

    #[test]
    fn image_paths_carry_the_filter_for_each_kind() {
        let f = SteamGridDbFilter::new;
        assert_eq!(
            images_path("42", &f(ArtKind::Cover), 0),
            "grids/game/42?dimensions=600x900,342x482,660x930&types=static&nsfw=false&humor=false"
        );
        let wide = SteamGridDbFilter {
            horizontal: true,
            style: Some("material".into()),
            animated: true,
            ..f(ArtKind::Cover)
        };
        assert_eq!(
            images_path("42", &wide, 0),
            "grids/game/42?dimensions=920x430,460x215&styles=material&types=static,animated&nsfw=false&humor=false"
        );
        let hero = SteamGridDbFilter {
            style: Some("no_logo".into()),
            ..f(ArtKind::Hero)
        };
        assert_eq!(
            images_path("42", &hero, 0),
            "heroes/game/42?types=static&nsfw=false&humor=false"
        );
        let logo = SteamGridDbFilter {
            style: Some("white".into()),
            ..f(ArtKind::Logo)
        };
        assert!(images_path("42", &logo, 0).starts_with("logos/game/42?styles=white&"));
        assert!(images_path("42", &f(ArtKind::Icon), 2).starts_with("icons/game/42?"));
        assert!(images_path("42", &f(ArtKind::Icon), 2).ends_with("&page=2"));
        assert_eq!(by_steam_app_id(570), "games/steam/570");
        assert_eq!(
            autocomplete(" Lanterna Distante "),
            "search/autocomplete/Lanterna%20Distante"
        );
    }

    #[test]
    fn parser_reads_author_size_style_and_animation_and_drops_insecure_urls() {
        let imgs = images();
        assert_eq!(imgs.len(), 6);
        let first = &imgs[0];
        assert_eq!(first.author.as_deref(), Some("Ana"));
        assert_eq!(first.width, Some(600));
        assert_eq!(first.style.as_deref(), Some("alternate"));
        assert_eq!(first.preview(), "https://cdn2.steamgriddb.com/thumb/a.jpg");
        assert_eq!(
            first.caption(),
            format!(
                "{} · 600 × 900 · {}",
                style_label(Some("alternate")),
                crate::i18n::tf("metadata.art.by", &[("author", "Ana")])
            )
        );
        assert!(
            imgs.iter()
                .find(|i| i.url.ends_with("e.webp"))
                .unwrap()
                .is_animated
        );
        assert!(data(r#"{"success":false,"errors":["Unauthorized"]}"#).is_none());
        assert!(data("<html>").is_none());
        assert!(parse_images(&serde_json::json!({"x":1}), ArtKind::Cover).is_empty());
    }

    #[test]
    fn filter_keeps_vertical_static_safe_images_by_score_and_auto_choice_prefers_600x900() {
        let imgs = images();
        let f = SteamGridDbFilter::new(ArtKind::Cover);
        assert_eq!(names(&apply(&imgs, &f)), ["b.jpg", "a.png"]);
        let animated = SteamGridDbFilter {
            animated: true,
            ..f.clone()
        };
        assert_eq!(
            names(&apply(&imgs, &animated)),
            ["e.webp", "b.jpg", "a.png"]
        );
        let wide = SteamGridDbFilter {
            horizontal: true,
            ..f.clone()
        };
        assert_eq!(names(&apply(&imgs, &wide)), ["d.png"]);
        let alt = SteamGridDbFilter {
            style: Some("alternate".into()),
            ..f
        };
        assert_eq!(names(&apply(&imgs, &alt)), ["a.png"]);
        assert!(
            best_automatic(&imgs, ArtKind::Cover)
                .unwrap()
                .url
                .ends_with("a.png")
        );
        assert!(best_automatic(&[], ArtKind::Hero).is_none());
    }

    #[test]
    fn games_come_from_autocomplete_and_steam_lookup() {
        let list = parse_games(&data(r#"{"success":true,"data":[{"id":5249,"name":"Lanterna Distante","release_date":1717200000,"types":["steam","egs"],"verified":true},{"id":7,"name":"Lanterna 2"}]}"#).unwrap());
        let single = parse_games(
            &data(r#"{"success":true,"data":{"id":5249,"name":"Lanterna Distante"}}"#).unwrap(),
        );
        let ids: Vec<_> = list.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["5249", "7"]);
        assert_eq!(list[0].year, Some(2024));
        assert_eq!(list[0].platforms.as_deref(), Some("Steam, Epic"));
        assert_eq!(list[0].source, "SteamGridDB");
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].title, "Lanterna Distante");
    }

    #[test]
    fn settings_validate_the_key_and_never_print_it() {
        let s = |k: &str| SteamGridDbSettings {
            enabled: true,
            api_key: k.into(),
            ..Default::default()
        };
        assert_eq!(
            s("").problem(),
            Some(t("metadata.steamgriddb.problem.missing_key"))
        );
        assert_eq!(
            s("abc").problem(),
            Some(t("metadata.steamgriddb.problem.short_key"))
        );
        let ok = s("0123456789abcdef0123456789abcdef");
        assert_eq!(ok.problem(), None);
        assert!(ok.is_active());
        assert!(!ok.auto_fill);
        assert!(!format!("{ok:?}").contains("0123456789abcdef"));
    }

    #[test]
    fn provider_finds_by_appid_and_picks_the_best_static_art() {
        let http = Arc::new(FakeHttp::with_fallback(404, ""));
        http.push(
            200,
            r#"{"success":true,"data":{"id":5249,"name":"Lanterna Distante"}}"#,
        );
        http.push(200, r#"{"success":true,"data":{"id":5249,"name":"Lanterna Distante","release_date":1717200000}}"#);
        http.push(200, GRIDS);
        http.push(200, r#"{"success":true,"data":[]}"#);
        let settings = SteamGridDbSettings {
            enabled: true,
            api_key: "0123456789abcdef0123456789abcdef".into(),
            auto_fill: true,
            animated: false,
        };
        let clock = Arc::new(TestClock::at(now()));
        let p = SteamGridDbProvider::new(
            SteamGridDbClient::new(
                http.clone(),
                Arc::new(move || settings.clone()),
                clock.clone(),
                None,
            ),
            clock,
        );
        assert_eq!(p.art_fallback(), Some(true));
        let lookup = MetadataLookup {
            external_ids: [("Steam".to_owned(), "570".to_owned())].into(),
            ..MetadataLookup::new("x")
        };
        assert_eq!(p.find_exact(&lookup).as_deref(), Some("5249"));
        let m = p.fetch("5249").unwrap();
        assert!(m.cover_url.unwrap().ends_with("a.png"));
        assert_eq!(m.background_url, None);
        let sent = http.sent();
        assert_eq!(
            sent[0].url,
            "https://www.steamgriddb.com/api/v2/games/steam/570"
        );
        assert_eq!(
            sent[0].header("Authorization"),
            Some("Bearer 0123456789abcdef0123456789abcdef")
        );
        assert_eq!(p.find_exact(&lookup).as_deref(), Some("5249"));
        assert_eq!(p.client().network_requests(), 4);
        http.push(401, "{}");
        assert!(p.search_term("outro").is_empty());
        assert_eq!(
            p.client().last_failure(),
            Some(SteamGridDbFailure::Unauthorized)
        );
    }
}
