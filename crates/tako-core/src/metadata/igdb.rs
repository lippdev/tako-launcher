//! Port de `Igdb/IgdbAuth.cs`, `IgdbClient.cs`, `IgdbSettings.cs` e `IgdbProvider.cs` (ADR-0021):
//! chave da Twitch do próprio usuário, token `client_credentials`, cliente com fila, limite de 4/s,
//! backoff no 429, troca de token no 401 e cache por consulta.

use super::art::ArtCandidate;
use super::http::{Clock, Http, HttpRequest, RateGate, ResponseCache, backoff, form, parse_json};
use super::igdb_parse as q;
use super::{
    GameMetadata, MetadataLookup, MetadataMatch, MetadataProvider, SecretStore, external_ids,
    looks_like_key, sources,
};
use crate::i18n::t;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const TOKEN_ENDPOINT: &str = "https://id.twitch.tv/oauth2/token";
pub const BASE_URL: &str = "https://api.igdb.com/v4/";
const MAX_ATTEMPTS: u32 = 4;
const SECRET_KEY: &str = "metadata.igdb";

/// Por que a IGDB não respondeu; [`IgdbFailure::message`] vira texto para a tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgdbFailure {
    NotConfigured,
    InvalidClientId,
    InvalidSecret,
    Unauthorized,
    RateLimited,
    Network,
    Unavailable,
}

impl IgdbFailure {
    pub fn message(self) -> String {
        let key = match self {
            IgdbFailure::NotConfigured => "not_configured",
            IgdbFailure::InvalidClientId => "invalid_client_id",
            IgdbFailure::InvalidSecret => "invalid_secret",
            IgdbFailure::Unauthorized => "unauthorized",
            IgdbFailure::RateLimited => "rate_limited",
            IgdbFailure::Network => "network",
            IgdbFailure::Unavailable => "unavailable",
        };
        t(&format!("metadata.igdb.error.{key}"))
    }
}

/// Falha com o motivo e um detalhe técnico (para o log; nunca traz o segredo).
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{detail}")]
pub struct IgdbError {
    pub failure: IgdbFailure,
    pub detail: String,
}

fn err(failure: IgdbFailure, detail: impl Into<String>) -> IgdbError {
    IgdbError {
        failure,
        detail: detail.into(),
    }
}

/// IGDB ligada ou não e a chave da Twitch do próprio usuário.
#[derive(Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IgdbSettings {
    pub enabled: bool,
    pub client_id: String,
    pub client_secret: String,
}

impl std::fmt::Debug for IgdbSettings {
    /// Nunca mostra o segredo (logs, depuração).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IgdbSettings")
            .field("enabled", &self.enabled)
            .field("client_id", &self.client_id)
            .field("client_secret", &mask(&self.client_secret))
            .finish()
    }
}

pub(crate) fn mask(secret: &str) -> &'static str {
    if secret.is_empty() { "" } else { "***" }
}

impl IgdbSettings {
    pub fn has_key(&self) -> bool {
        !self.client_id.trim().is_empty() && !self.client_secret.trim().is_empty()
    }

    /// Ligada e com chave: só então o provedor consulta a IGDB.
    pub fn is_active(&self) -> bool {
        self.enabled && self.has_key()
    }

    /// O que falta ou parece errado na chave, antes de ir à rede; `None` = parece certo.
    pub fn problem(&self) -> Option<String> {
        let key = if self.client_id.is_empty() {
            "missing_client_id"
        } else if self.client_secret.is_empty() {
            "missing_client_secret"
        } else if !looks_like_key(&self.client_id) {
            "short_client_id"
        } else if !looks_like_key(&self.client_secret) {
            "short_client_secret"
        } else {
            return None;
        };
        Some(t(&format!("metadata.igdb.problem.{key}")))
    }

    /// Ilegível ou ausente = desligada.
    pub fn load(store: &dyn SecretStore) -> Self {
        store
            .get(SECRET_KEY)
            .and_then(|j| serde_json::from_str(&j).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, store: &dyn SecretStore) -> Result<(), String> {
        let clean = IgdbSettings {
            enabled: self.enabled,
            client_id: self.client_id.trim().into(),
            client_secret: self.client_secret.trim().into(),
        };
        store.set(
            SECRET_KEY,
            &serde_json::to_string(&clean).map_err(|e| e.to_string())?,
        )
    }
}

pub type SettingsFn<T> = Arc<dyn Fn() -> T + Send + Sync>;

struct Token {
    client_id: String,
    secret: String,
    token: String,
    expires: chrono::DateTime<chrono::Utc>,
}

/// Token de app da Twitch com a chave do usuário. Vale ~60 dias; é renovado 5 minutos antes de
/// vencer, quando a chave muda ou quando a IGDB o recusa. Fica só na memória.
pub struct TwitchAuth {
    http: Arc<dyn Http>,
    settings: SettingsFn<IgdbSettings>,
    clock: Arc<dyn Clock>,
    token: Mutex<Option<Token>>,
}

impl TwitchAuth {
    pub fn new(
        http: Arc<dyn Http>,
        settings: SettingsFn<IgdbSettings>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        TwitchAuth {
            http,
            settings,
            clock,
            token: Mutex::new(None),
        }
    }

    pub fn is_configured(&self) -> bool {
        (self.settings)().is_active()
    }

    /// (Client-ID, token).
    pub fn access(&self) -> Result<(String, String), IgdbError> {
        let s = (self.settings)();
        if !s.is_active() {
            return Err(err(
                IgdbFailure::NotConfigured,
                "IGDB desligada ou sem chave",
            ));
        }
        let mut guard = self.token.lock().unwrap_or_else(|e| e.into_inner());
        let now = self.clock.now();
        if let Some(t) = guard.as_ref()
            && t.client_id == s.client_id
            && t.secret == s.client_secret
            && now < t.expires - chrono::Duration::minutes(5)
        {
            return Ok((t.client_id.clone(), t.token.clone()));
        }
        let (token, lifetime) = request_token(self.http.as_ref(), &s.client_id, &s.client_secret)?;
        *guard = Some(Token {
            client_id: s.client_id.clone(),
            secret: s.client_secret.clone(),
            token: token.clone(),
            expires: now + chrono::Duration::seconds(lifetime),
        });
        Ok((s.client_id, token))
    }

    /// A IGDB recusou o token (401): o próximo pedido obtém outro.
    pub fn invalidate(&self) {
        *self.token.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// POST do `client_credentials`: (token, segundos de vida).
pub fn request_token(
    http: &dyn Http,
    client_id: &str,
    client_secret: &str,
) -> Result<(String, i64), IgdbError> {
    let body = form(&[
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "client_credentials"),
    ]);
    let req = HttpRequest::post(TOKEN_ENDPOINT, body)
        .header("Content-Type", "application/x-www-form-urlencoded");
    let r = http.send(&req).map_err(|e| {
        err(
            IgdbFailure::Network,
            format!("Sem conexão com a Twitch: {e}"),
        )
    })?;
    let text = r.text();
    if !r.is_success() {
        return Err(token_failure(r.status, &text));
    }
    let json = parse_json(&r.body)
        .ok_or_else(|| err(IgdbFailure::Unavailable, "Resposta inesperada da Twitch"))?;
    let token = json
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| err(IgdbFailure::Unavailable, "A Twitch não devolveu um token"))?;
    let seconds = json
        .get("expires_in")
        .and_then(Value::as_i64)
        .unwrap_or(3600);
    Ok((token.to_owned(), seconds))
}

/// A Twitch responde 400 "invalid client" (Client ID) e 403 "invalid client secret" (segredo).
pub fn token_failure(status: u16, body: &str) -> IgdbError {
    let secret = body.to_lowercase().contains("secret");
    let failure = match status {
        400 | 403 if secret => IgdbFailure::InvalidSecret,
        400 | 404 => IgdbFailure::InvalidClientId,
        401 | 403 => IgdbFailure::InvalidSecret,
        429 => IgdbFailure::RateLimited,
        _ => IgdbFailure::Unavailable,
    };
    err(failure, format!("Twitch respondeu {status}"))
}

/// Cliente da IGDB v4 (`POST {BASE_URL}{endpoint}`, corpo Apicalypse).
pub struct IgdbClient {
    http: Arc<dyn Http>,
    auth: TwitchAuth,
    clock: Arc<dyn Clock>,
    cache: Option<Box<dyn ResponseCache>>,
    gate: RateGate,
    queue: Mutex<()>,
    network: AtomicUsize,
}

impl IgdbClient {
    pub fn new(
        http: Arc<dyn Http>,
        settings: SettingsFn<IgdbSettings>,
        clock: Arc<dyn Clock>,
        cache: Option<Box<dyn ResponseCache>>,
    ) -> Self {
        IgdbClient {
            auth: TwitchAuth::new(http.clone(), settings, clock.clone()),
            http,
            clock,
            cache,
            gate: RateGate::new(4, Duration::from_secs(1)),
            queue: Mutex::new(()),
            network: AtomicUsize::new(0),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.auth.is_configured()
    }

    /// Consultas que chegaram à rede (sem contar o cache).
    pub fn network_requests(&self) -> usize {
        self.network.load(Ordering::SeqCst)
    }

    /// Resposta da consulta (um array JSON).
    pub fn query(&self, endpoint: &str, body: &str, use_cache: bool) -> Result<Value, IgdbError> {
        let key = format!("{endpoint}\n{body}");
        let cache = self.cache.as_deref().filter(|_| use_cache);
        if let Some(hit) = cache.and_then(|c| c.read(&key)).and_then(|j| as_array(&j)) {
            return Ok(hit);
        }
        let _one_at_a_time = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        let json = self.send(endpoint, body)?;
        let result = as_array(&json)
            .ok_or_else(|| err(IgdbFailure::Unavailable, "Resposta inesperada da IGDB"))?;
        if let Some(c) = cache {
            c.write(&key, &json);
        }
        Ok(result)
    }

    fn send(&self, endpoint: &str, body: &str) -> Result<String, IgdbError> {
        let mut refreshed = false;
        let mut attempt = 0;
        loop {
            attempt += 1;
            let (client_id, token) = self.auth.access()?;
            let wait = self.gate.reserve(self.clock.now());
            if !wait.is_zero() {
                self.clock.sleep(wait);
            }
            let req = HttpRequest::post(format!("{BASE_URL}{endpoint}"), body)
                .header("Client-ID", client_id)
                .header("Authorization", format!("Bearer {token}"))
                .header("Accept", "application/json")
                .header("Content-Type", "text/plain");
            self.network.fetch_add(1, Ordering::SeqCst);
            let r = self
                .http
                .send(&req)
                .map_err(|e| err(IgdbFailure::Network, format!("Sem conexão com a IGDB: {e}")))?;
            match r.status {
                200..=299 => return Ok(r.text()),
                401 if !refreshed => {
                    self.auth.invalidate(); // token vencido ou revogado: pede outro uma vez
                    refreshed = true;
                }
                401 | 403 => {
                    return Err(err(
                        IgdbFailure::Unauthorized,
                        format!("IGDB recusou o acesso ({})", r.status),
                    ));
                }
                429 if attempt < MAX_ATTEMPTS => self.clock.sleep(backoff(attempt, r.retry_after)),
                429 => return Err(err(IgdbFailure::RateLimited, "IGDB: limite de consultas")),
                s => return Err(err(IgdbFailure::Unavailable, format!("IGDB respondeu {s}"))),
            }
        }
    }

    /// Botão "Testar": token com a chave informada (ainda não salva) e uma consulta mínima, sem cache.
    pub fn test(
        http: Arc<dyn Http>,
        settings: &IgdbSettings,
        clock: Arc<dyn Clock>,
    ) -> Result<(), IgdbError> {
        let key = IgdbSettings {
            enabled: true,
            ..settings.clone()
        };
        IgdbClient::new(http, Arc::new(move || key.clone()), clock, None)
            .query("games", q::PING, false)
            .map(|_| ())
    }
}

fn as_array(json: &str) -> Option<Value> {
    serde_json::from_str::<Value>(json)
        .ok()
        .filter(Value::is_array)
}

const EXACT_ORDER: [&str; 4] = [
    external_ids::STEAM,
    external_ids::GOG,
    external_ids::EPIC,
    external_ids::MICROSOFT,
];

#[derive(Default)]
struct Known {
    external: HashMap<(String, String), i64>,
    misses: HashSet<(String, String)>,
    games: HashMap<i64, GameMetadata>,
    raw: HashMap<i64, Value>,
}

/// Provedor IGDB: casa pelos ids nas lojas (`external_games`), senão pelo nome filtrado pela
/// plataforma. Guarda os jogos na memória: buscar e depois detalhar não repete a consulta.
pub struct IgdbProvider {
    client: IgdbClient,
    clock: Arc<dyn Clock>,
    known: Mutex<Known>,
}

impl IgdbProvider {
    pub fn new(client: IgdbClient, clock: Arc<dyn Clock>) -> Self {
        IgdbProvider {
            client,
            clock,
            known: Mutex::new(Known::default()),
        }
    }

    pub fn client(&self) -> &IgdbClient {
        &self.client
    }

    fn known(&self) -> std::sync::MutexGuard<'_, Known> {
        self.known.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Capa, artes oficiais e capturas do jogo para o seletor (ADR-0023).
    pub fn images(&self, id: &str) -> Vec<ArtCandidate> {
        let Some(game_id) = self.parse_id(id) else {
            return Vec::new();
        };
        if !self.known().raw.contains_key(&game_id) {
            self.fetch_games(&[game_id]);
        }
        self.known()
            .raw
            .get(&game_id)
            .map(q::images)
            .unwrap_or_default()
    }

    fn parse_id(&self, id: &str) -> Option<i64> {
        self.is_enabled().then(|| id.parse().ok()).flatten()
    }

    /// Falha em segundo plano não interrompe nada: vai para o log e volta `None`.
    fn query(&self, endpoint: &str, body: &str) -> Option<Value> {
        match self.client.query(endpoint, body, true) {
            Ok(v) => Some(v),
            Err(e) if e.failure == IgdbFailure::NotConfigured => None,
            Err(e) => {
                tracing::warn!(endpoint, failure = ?e.failure, "IGDB: consulta falhou: {e}");
                None
            }
        }
    }

    fn resolve_external(&self, store: &str, uids: &[String]) {
        let Some(source) = q::external_source_id(store) else {
            return;
        };
        if uids.is_empty() {
            return;
        }
        // Falha: tenta de novo na próxima atualização.
        let Some(array) = self.query("external_games", &q::external_games(source, uids)) else {
            return;
        };
        let found = q::external_game_map(&array);
        let mut k = self.known();
        for uid in uids {
            let key = (store.to_owned(), uid.clone());
            match found.get(uid) {
                Some(id) => {
                    k.external.insert(key, *id);
                }
                None => {
                    k.misses.insert(key);
                }
            }
        }
    }

    fn fetch_games(&self, ids: &[i64]) {
        if let Some(Value::Array(items)) = self.query("games", &q::games_by_id(ids)) {
            for item in &items {
                self.remember(item);
            }
        }
    }

    fn remember(&self, item: &Value) {
        let Some(m) = q::game(item, self.clock.now()) else {
            return;
        };
        let Some(id) = m.source_ids.get(sources::IGDB).and_then(|i| i.parse().ok()) else {
            return;
        };
        let mut k = self.known();
        for (s, uid) in m.source_ids.iter().filter(|(s, _)| *s != sources::IGDB) {
            k.external.entry((s.clone(), uid.clone())).or_insert(id);
        }
        k.raw.insert(id, item.clone());
        k.games.insert(id, m);
    }
}

impl MetadataProvider for IgdbProvider {
    fn name(&self) -> &str {
        sources::IGDB
    }

    fn is_enabled(&self) -> bool {
        self.client.is_configured()
    }

    fn search(&self, lookup: &MetadataLookup) -> Vec<MetadataMatch> {
        if !self.is_enabled() {
            return Vec::new();
        }
        let mut results = Vec::new();
        if let Some(exact) = self.find_exact(lookup)
            && let Some(title) = self.fetch(&exact).and_then(|m| m.title)
        {
            let mut m = MetadataMatch::new(sources::IGDB, exact, title);
            m.is_exact = true;
            results.push(m);
        }
        let platforms = q::platform_ids(lookup.platform.as_deref());
        if let Some(Value::Array(items)) =
            self.query("games", &q::search_query(&lookup.title, &platforms, 10))
        {
            for item in &items {
                self.remember(item);
                if let Some(m) = q::match_of(item)
                    && !results.iter().any(|r: &MetadataMatch| r.id == m.id)
                {
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
        for store in EXACT_ORDER {
            let Some(uid) = lookup
                .external_ids
                .get(store)
                .filter(|u| !u.trim().is_empty())
            else {
                continue;
            };
            let key = (store.to_owned(), uid.clone());
            let unknown = {
                let k = self.known();
                !k.external.contains_key(&key) && !k.misses.contains(&key)
            };
            if unknown {
                self.resolve_external(store, std::slice::from_ref(uid));
            }
            if let Some(id) = self.known().external.get(&key) {
                return Some(id.to_string());
            }
        }
        None
    }

    fn fetch(&self, id: &str) -> Option<GameMetadata> {
        let game_id = self.parse_id(id)?;
        if !self.known().games.contains_key(&game_id) {
            self.fetch_games(&[game_id]);
        }
        let mut m = self.known().games.get(&game_id)?.clone();
        m.fetched_at = Some(self.clock.now());
        Some(m)
    }

    fn id_from_slug(&self, slug: &str) -> Option<String> {
        if !self.is_enabled() || slug.trim().is_empty() {
            return None;
        }
        let array = self.query("games", &q::by_slug(slug))?;
        array
            .as_array()?
            .iter()
            .find_map(|g| g.get("id").and_then(Value::as_i64))
            .map(|i| i.to_string())
    }

    /// Lote: uma consulta de `external_games` por loja e uma de `games` a cada 500 jogos.
    fn prefetch(&self, lookups: &[MetadataLookup]) {
        if !self.is_enabled() {
            return;
        }
        for store in EXACT_ORDER {
            let mut uids: Vec<String> = Vec::new();
            {
                let k = self.known();
                for uid in lookups.iter().filter_map(|l| l.external_ids.get(store)) {
                    let key = (store.to_owned(), uid.clone());
                    if !uid.trim().is_empty()
                        && !k.external.contains_key(&key)
                        && !k.misses.contains(&key)
                        && !uids.contains(uid)
                    {
                        uids.push(uid.clone());
                    }
                }
            }
            for chunk in uids.chunks(q::MAX_LIMIT / 2) {
                self.resolve_external(store, chunk);
            }
        }
        let missing: Vec<i64> = {
            let k = self.known();
            let mut v: Vec<i64> = k
                .external
                .values()
                .filter(|id| !k.games.contains_key(id))
                .copied()
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        for chunk in missing.chunks(q::MAX_LIMIT) {
            self.fetch_games(chunk);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::http::HttpResponse;
    use crate::metadata::test_support::{FakeHttp, TestClock, fixture, now};

    fn key() -> IgdbSettings {
        IgdbSettings {
            enabled: true,
            client_id: "abcdefghijklmnopqrstuvwxyz0123".into(),
            client_secret: "secretsecretsecretsecretsecret".into(),
        }
    }

    /// IGDB em fila; a Twitch responde tokens numerados.
    fn fake() -> Arc<FakeHttp> {
        let mut http = FakeHttp::new();
        let n = Arc::new(AtomicUsize::new(0));
        http.rule = Some(Box::new(move |req| {
            (req.url == TOKEN_ENDPOINT).then(|| HttpResponse {
                status: 200,
                body: format!(
                    r#"{{"access_token":"twitch-{}","expires_in":3600,"token_type":"bearer"}}"#,
                    n.fetch_add(1, Ordering::SeqCst) + 1
                )
                .into_bytes(),
                content_type: None,
                retry_after: None,
            })
        }));
        Arc::new(http)
    }

    fn igdb_requests(http: &FakeHttp) -> Vec<crate::metadata::test_support::Sent> {
        http.sent()
            .into_iter()
            .filter(|s| s.url != TOKEN_ENDPOINT)
            .collect()
    }

    struct MemoryCache(Mutex<HashMap<String, String>>);
    impl ResponseCache for MemoryCache {
        fn read(&self, key: &str) -> Option<String> {
            self.0.lock().unwrap().get(key).cloned()
        }
        fn write(&self, key: &str, json: &str) {
            self.0.lock().unwrap().insert(key.into(), json.into());
        }
    }

    fn client(
        http: &Arc<FakeHttp>,
        clock: &Arc<TestClock>,
        cache: Option<Box<dyn ResponseCache>>,
    ) -> IgdbClient {
        IgdbClient::new(http.clone(), Arc::new(key), clock.clone(), cache)
    }

    #[test]
    fn client_sends_headers_waits_on_429_and_caches() {
        let http = fake();
        http.push(429, "[]");
        http.push(200, &fixture("games-lanterna.json"));
        let clock = Arc::new(TestClock::at(now()));
        let c = client(&http, &clock, Some(Box::new(MemoryCache(Mutex::default()))));
        let first = c.query("games", "fields name; limit 1;", true).unwrap();
        let again = c.query("games", "fields name; limit 1;", true).unwrap();
        assert_eq!(first.as_array().unwrap().len(), 2);
        assert_eq!(again.as_array().unwrap().len(), 2);
        assert_eq!(c.network_requests(), 2);
        assert!(
            clock
                .sleeps
                .lock()
                .unwrap()
                .contains(&Duration::from_secs(1))
        );
        let last = igdb_requests(&http).pop().unwrap();
        assert_eq!(last.url, "https://api.igdb.com/v4/games");
        assert_eq!(last.header("Client-ID"), Some(key().client_id.as_str()));
        assert_eq!(last.header("Authorization"), Some("Bearer twitch-1"));
        assert_eq!(last.body.as_deref(), Some("fields name; limit 1;"));
    }

    #[test]
    fn client_renews_the_token_once_on_401_and_reports_failures() {
        let http = fake();
        http.push(401, "{}");
        http.push(200, "[]");
        let clock = Arc::new(TestClock::at(now()));
        let c = client(&http, &clock, None);
        c.query("games", q::PING, true).unwrap();
        assert_eq!(
            igdb_requests(&http).pop().unwrap().header("Authorization"),
            Some("Bearer twitch-2")
        );

        http.push(401, "{}");
        http.push(401, "{}");
        let denied = c.query("games", "fields id;", true).unwrap_err();
        assert_eq!(denied.failure, IgdbFailure::Unauthorized);

        for _ in 0..4 {
            http.push(429, "[]");
        }
        let limited = c.query("games", "fields slug;", true).unwrap_err();
        assert_eq!(limited.failure, IgdbFailure::RateLimited);

        http.push(200, "{\"não\":\"array\"}");
        assert_eq!(
            c.query("games", "x", true).unwrap_err().failure,
            IgdbFailure::Unavailable
        );
    }

    #[test]
    fn twitch_token_is_reused_and_renewed_before_expiring_or_when_the_key_changes() {
        let http = fake();
        let settings = Arc::new(Mutex::new(key()));
        let s = settings.clone();
        let clock = Arc::new(TestClock::at(now()));
        let auth = TwitchAuth::new(
            http.clone(),
            Arc::new(move || s.lock().unwrap().clone()),
            clock.clone(),
        );
        let first = auth.access().unwrap();
        let cached = auth.access().unwrap();
        clock.advance(chrono::Duration::seconds(3600));
        let renewed = auth.access().unwrap();
        settings.lock().unwrap().client_secret = "outrosegredooutrosegredooutros".into();
        let new_key = auth.access().unwrap();
        assert_eq!(first, (key().client_id, "twitch-1".to_owned()));
        assert_eq!(first, cached);
        assert_eq!(renewed.1, "twitch-2");
        assert_eq!(new_key.1, "twitch-3");
        let tokens: Vec<_> = http
            .sent()
            .into_iter()
            .filter(|s| s.url == TOKEN_ENDPOINT)
            .collect();
        assert_eq!(tokens.len(), 3);
        let body = tokens[0].body.clone().unwrap();
        assert!(body.contains("grant_type=client_credentials"));
        assert!(body.contains(&format!("client_id={}", key().client_id)));
    }

    #[test]
    fn twitch_errors_become_clear_reasons() {
        for (status, body, expected) in [
            (
                400,
                r#"{"status":400,"message":"invalid client"}"#,
                IgdbFailure::InvalidClientId,
            ),
            (
                403,
                r#"{"status":403,"message":"invalid client secret"}"#,
                IgdbFailure::InvalidSecret,
            ),
            (429, "{}", IgdbFailure::RateLimited),
            (502, "", IgdbFailure::Unavailable),
        ] {
            let f = token_failure(status, body).failure;
            assert_eq!(f, expected);
            assert!(!f.message().trim().is_empty());
        }
    }

    #[test]
    fn without_a_key_nothing_is_requested() {
        let http = fake();
        let off = IgdbSettings {
            enabled: false,
            ..key()
        };
        let clock = Arc::new(TestClock::at(now()));
        let c = IgdbClient::new(
            http.clone(),
            Arc::new(move || off.clone()),
            clock.clone(),
            None,
        );
        let auth_err = c.query("games", q::PING, true).unwrap_err();
        assert_eq!(auth_err.failure, IgdbFailure::NotConfigured);
        let p = IgdbProvider::new(c, clock);
        assert!(!p.is_enabled());
        assert!(p.search(&MetadataLookup::new("Lanterna")).is_empty());
        assert!(p.fetch("900001").is_none());
        assert!(http.sent().is_empty());
    }

    #[test]
    fn settings_are_stored_in_the_secret_store_and_validated() {
        #[derive(Default)]
        struct Mem(Mutex<HashMap<String, String>>);
        impl SecretStore for Mem {
            fn get(&self, k: &str) -> Option<String> {
                self.0.lock().unwrap().get(k).cloned()
            }
            fn set(&self, k: &str, v: &str) -> Result<(), String> {
                self.0.lock().unwrap().insert(k.into(), v.into());
                Ok(())
            }
        }
        let store = Mem::default();
        assert_eq!(IgdbSettings::load(&store), IgdbSettings::default());
        IgdbSettings {
            client_id: format!(" {} ", key().client_id),
            ..key()
        }
        .save(&store)
        .unwrap();
        let loaded = IgdbSettings::load(&store);
        assert_eq!(loaded, key());
        assert!(loaded.is_active());
        assert!(!format!("{loaded:?}").contains(&key().client_secret));
        store.set(SECRET_KEY, "corrompido").unwrap();
        assert_eq!(IgdbSettings::load(&store), IgdbSettings::default());

        assert_eq!(super::super::clean_key(" abc\n12 3\t"), "abc123");
        assert_eq!(key().problem(), None);
        let p = |s: IgdbSettings| s.problem().unwrap();
        assert_eq!(
            p(IgdbSettings {
                client_id: String::new(),
                ..key()
            }),
            t("metadata.igdb.problem.missing_client_id")
        );
        assert_eq!(
            p(IgdbSettings {
                client_secret: String::new(),
                ..key()
            }),
            t("metadata.igdb.problem.missing_client_secret")
        );
        assert_eq!(
            p(IgdbSettings {
                client_id: "abc123".into(),
                ..key()
            }),
            t("metadata.igdb.problem.short_client_id")
        );
        assert!(
            !IgdbSettings {
                enabled: false,
                ..key()
            }
            .is_active()
        );
    }

    fn provider(http: &Arc<FakeHttp>) -> IgdbProvider {
        let clock = Arc::new(TestClock::at(now()));
        IgdbProvider::new(client(http, &clock, None), clock)
    }

    fn steam(id: &str) -> MetadataLookup {
        MetadataLookup {
            external_ids: [("Steam".to_owned(), id.to_owned())].into(),
            ..MetadataLookup::new("Outro nome")
        }
    }

    #[test]
    fn provider_matches_store_ids_exactly_and_reuses_what_it_already_fetched() {
        let http = fake();
        http.push(200, &fixture("external-games.json"));
        http.push(200, &fixture("games-lanterna.json"));
        let p = provider(&http);
        let id = p.find_exact(&steam("4000001")).unwrap();
        let m = p.fetch(&id).unwrap();
        let gog = p.find_exact(&MetadataLookup {
            external_ids: [("GOG".to_owned(), "1400000001".to_owned())].into(),
            ..MetadataLookup::new("x")
        });
        assert_eq!(id, "900001");
        assert_eq!(m.title.as_deref(), Some("Lanterna Distante: Edição Brasil"));
        assert_eq!(gog.as_deref(), Some("900001")); // aprendido, sem nova consulta
        assert_eq!(p.client().network_requests(), 2);
        assert!(
            igdb_requests(&http)[0]
                .body
                .as_deref()
                .unwrap()
                .contains(r#"where external_game_source = 1 & uid = ("4000001")"#)
        );
    }

    #[test]
    fn provider_prefetches_the_library_in_batches() {
        let http = fake();
        http.push(200, &fixture("external-games.json"));
        http.push(200, &fixture("games-lanterna.json"));
        let p = provider(&http);
        let lookups = [steam("4000001"), steam("4000002"), steam("4000009")];
        p.prefetch(&lookups);
        for l in &lookups {
            p.find_exact(l);
        }
        assert!(p.fetch("900002").is_some());
        assert_eq!(p.client().network_requests(), 2);
        let sent = igdb_requests(&http);
        assert!(
            sent[0]
                .body
                .as_deref()
                .unwrap()
                .contains(r#"uid = ("4000001","4000002","4000009")"#)
        );
        assert!(
            sent[1]
                .body
                .as_deref()
                .unwrap()
                .contains("where id = (900001,900002)")
        );
    }

    #[test]
    fn provider_searches_by_title_and_platform_and_flags_exact_matches() {
        let http = fake();
        http.push(200, &fixture("games-lanterna.json"));
        let p = provider(&http);
        let r = p.search(&MetadataLookup {
            platform: Some("snes".into()),
            ..MetadataLookup::new("Lanterna Distante")
        });
        let ids: Vec<_> = r.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["900001", "900002"]);
        assert!(r.iter().all(|m| !m.is_exact));
        assert!(
            igdb_requests(&http)[0]
                .body
                .as_deref()
                .unwrap()
                .contains("platforms = (19,58)")
        );
        assert!(!p.images("900001").is_empty());
    }

    #[test]
    fn provider_failures_do_not_throw_during_background_updates() {
        let http = fake();
        http.push(500, "erro");
        assert!(
            provider(&http)
                .search(&MetadataLookup::new("Lanterna"))
                .is_empty()
        );
    }
}
