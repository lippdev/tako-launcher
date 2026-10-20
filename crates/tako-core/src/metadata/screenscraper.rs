//! Port de `ScreenScraper/ScreenScraperClient.cs`, `ScreenScraperSettings.cs` e
//! `ScreenScraperProvider.cs` (ADR-0022): ROMs pelo hash com a conta do próprio usuário. Respeita a
//! cota (pausa até a meia-noite de Paris), o limite de consultas simultâneas e nunca grava credenciais.

use super::art::ArtCandidate;
use super::http::{Clock, Http, HttpRequest, RateGate, ResponseCache, backoff, escape};
use super::igdb::{SettingsFn, mask};
use super::rom_hash::{RomHashCache, RomIdentity};
use super::screenscraper_parse::{self as p, EmulatedSystem, ScreenScraperAccount};
use super::{GameMetadata, MetadataLookup, MetadataMatch, MetadataProvider, SecretStore, sources};
use crate::i18n::t;
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Mídias guardadas sem credenciais: `ssmedia://host/caminho?…`.
pub const MEDIA_SCHEME: &str = "ssmedia";
pub const BASE_URL: &str = "https://api.screenscraper.fr/api2/";
pub const MAX_SEARCH_RESULTS: usize = 8;
const MAX_ATTEMPTS: u32 = 4;
const SPACING: Duration = Duration::from_millis(1200);
const SECRET_KEY: &str = "metadata.screenscraper";
const CREDENTIALS: [&str; 6] = [
    "devid",
    "devpassword",
    "softname",
    "ssid",
    "sspassword",
    "output",
];
const DISC_EXTENSIONS: [&str; 17] = [
    ".iso", ".cue", ".bin", ".chd", ".gdi", ".cdi", ".cso", ".rvz", ".wia", ".gcz", ".gcm",
    ".wbfs", ".pbp", ".mdf", ".ecm", ".m3u", ".xiso",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenScraperFailure {
    /// Esta versão do Tako não tem as credenciais do software.
    NoSoftwareAccess,
    NotConfigured,
    BadUser,
    BadSoftware,
    Blacklisted,
    ThreadLimit,
    QuotaExceeded,
    Closed,
    Network,
    Unavailable,
}

impl ScreenScraperFailure {
    pub const ALL: [ScreenScraperFailure; 10] = [
        Self::NoSoftwareAccess,
        Self::NotConfigured,
        Self::BadUser,
        Self::BadSoftware,
        Self::Blacklisted,
        Self::ThreadLimit,
        Self::QuotaExceeded,
        Self::Closed,
        Self::Network,
        Self::Unavailable,
    ];

    pub fn message(self) -> String {
        let key = match self {
            Self::NoSoftwareAccess => "no_software_access",
            Self::NotConfigured => "not_configured",
            Self::BadUser => "bad_user",
            Self::BadSoftware => "bad_software",
            Self::Blacklisted => "blacklisted",
            Self::ThreadLimit => "thread_limit",
            Self::QuotaExceeded => "quota_exceeded",
            Self::Closed => "closed",
            Self::Network => "network",
            Self::Unavailable => "unavailable",
        };
        t(&format!("metadata.screenscraper.error.{key}"))
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{detail}")]
pub struct ScreenScraperError {
    pub failure: ScreenScraperFailure,
    pub detail: String,
}

fn err(failure: ScreenScraperFailure, detail: impl Into<String>) -> ScreenScraperError {
    ScreenScraperError {
        failure,
        detail: detail.into(),
    }
}

/// ScreenScraper ligado ou não e a conta do próprio usuário.
#[derive(Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ScreenScraperSettings {
    pub enabled: bool,
    pub user: String,
    pub password: String,
    pub dev_id: String,
    pub dev_password: String,
}

impl std::fmt::Debug for ScreenScraperSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenScraperSettings")
            .field("enabled", &self.enabled)
            .field("user", &self.user)
            .field("password", &mask(&self.password))
            .field("dev_id", &self.dev_id)
            .field("dev_password", &mask(&self.dev_password))
            .finish()
    }
}

impl ScreenScraperSettings {
    pub fn has_account(&self) -> bool {
        !self.user.trim().is_empty() && !self.password.is_empty()
    }

    pub fn has_dev_credentials(&self) -> bool {
        !self.dev_id.trim().is_empty() && !self.dev_password.is_empty()
    }

    pub fn is_active(&self) -> bool {
        let dev_ok =
            ScreenScraperSoftware::from_build().is_available() || self.has_dev_credentials();
        self.enabled && self.has_account() && dev_ok
    }

    pub fn problem(&self) -> Option<String> {
        if self.user.trim().is_empty() {
            Some(t("metadata.screenscraper.problem.missing_user"))
        } else if self.password.is_empty() {
            Some(t("metadata.screenscraper.problem.missing_password"))
        } else if !ScreenScraperSoftware::from_build().is_available() {
            if self.dev_id.trim().is_empty() {
                Some(t("metadata.screenscraper.problem.missing_dev_id"))
            } else if self.dev_password.is_empty() {
                Some(t("metadata.screenscraper.problem.missing_dev_password"))
            } else {
                None
            }
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
        let clean = ScreenScraperSettings {
            user: self.user.trim().into(),
            dev_id: self.dev_id.trim().into(),
            ..self.clone()
        };
        store.set(
            SECRET_KEY,
            &serde_json::to_string(&clean).map_err(|e| e.to_string())?,
        )
    }
}

/// Credenciais do software que o ScreenScraper dá aos mantenedores. Nunca no repositório: o build
/// de release as recebe dos segredos do CI. Sem elas, o ScreenScraper fica indisponível com aviso.
#[derive(Clone, PartialEq, Eq)]
pub struct ScreenScraperSoftware {
    pub dev_id: String,
    pub dev_password: String,
    pub soft_name: String,
}

impl std::fmt::Debug for ScreenScraperSoftware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScreenScraperSoftware")
            .field("dev_id", &self.dev_id)
            .field("dev_password", &mask(&self.dev_password))
            .field("soft_name", &self.soft_name)
            .finish()
    }
}

impl ScreenScraperSoftware {
    /// Lidas na compilação (`TAKO_SCREENSCRAPER_DEV_ID`, `_DEV_PASSWORD`, `_SOFT_NAME`).
    pub fn from_build() -> Self {
        Self::from_values(
            option_env!("TAKO_SCREENSCRAPER_DEV_ID"),
            option_env!("TAKO_SCREENSCRAPER_DEV_PASSWORD"),
            option_env!("TAKO_SCREENSCRAPER_SOFT_NAME"),
        )
    }

    pub fn from_values(dev_id: Option<&str>, password: Option<&str>, soft: Option<&str>) -> Self {
        ScreenScraperSoftware {
            dev_id: dev_id.unwrap_or_default().trim().into(),
            dev_password: password.unwrap_or_default().trim().into(),
            soft_name: soft
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or("Tako")
                .into(),
        }
    }

    pub fn is_available(&self) -> bool {
        !self.dev_id.is_empty() && !self.dev_password.is_empty()
    }

    /// Usa as credenciais do build se existirem; se o build não as tiver, usa as do usuário.
    pub fn or_user(&self, s: &ScreenScraperSettings) -> Self {
        if self.is_available() {
            self.clone()
        } else {
            Self::from_values(Some(&s.dev_id), Some(&s.dev_password), None)
        }
    }
}

/// (host, caminho, pares da query) de `esquema://host/caminho?query`, só com host limpo.
fn split_url<'a>(url: &'a str, scheme: &str) -> Option<(String, &'a str, Vec<&'a str>)> {
    let (s, rest) = url.split_once("://")?;
    if !s.eq_ignore_ascii_case(scheme) {
        return None;
    }
    let rest = rest.split('#').next().unwrap_or_default();
    let (before_query, query) = rest.split_once('?').unwrap_or((rest, ""));
    let (host, path) = match before_query.find('/') {
        Some(i) => (&before_query[..i], &before_query[i..]),
        None => (before_query, "/"),
    };
    // Sem usuário, porta ou caracteres estranhos no host: credenciais só vão ao ScreenScraper.
    let host_ok = !host.is_empty()
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
    host_ok.then(|| {
        let pairs = query.split('&').filter(|p| !p.is_empty()).collect();
        (host.to_ascii_lowercase(), path, pairs)
    })
}

pub fn is_screenscraper_host(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    h == "screenscraper.fr" || h.ends_with(".screenscraper.fr")
}

fn without_credentials<'a>(pairs: &[&'a str]) -> Vec<&'a str> {
    pairs
        .iter()
        .filter(|p| {
            let k = p.split('=').next().unwrap_or_default();
            !CREDENTIALS.iter().any(|c| c.eq_ignore_ascii_case(k))
        })
        .copied()
        .collect()
}

/// URL da resposta → endereço guardado (sem credenciais); `None` se não for do ScreenScraper.
pub fn media_from_response(url: &str) -> Option<String> {
    let url = url.trim().replace(' ', "%20");
    let (host, path, pairs) = split_url(&url, "https").or_else(|| split_url(&url, "http"))?;
    if !is_screenscraper_host(&host) {
        return None;
    }
    let kept = without_credentials(&pairs);
    let query = if kept.is_empty() {
        String::new()
    } else {
        format!("?{}", kept.join("&"))
    };
    Some(format!("{MEDIA_SCHEME}://{host}{path}{query}"))
}

/// Endereço guardado → URL https com as credenciais, para baixar.
pub fn media_to_download(
    media: &str,
    software: &ScreenScraperSoftware,
    settings: &ScreenScraperSettings,
) -> Option<String> {
    let (host, path, pairs) = split_url(media, MEDIA_SCHEME)?;
    if !is_screenscraper_host(&host) {
        return None;
    }
    let mut parts = vec![
        format!("devid={}", escape(&software.dev_id)),
        format!("devpassword={}", escape(&software.dev_password)),
        format!("softname={}", escape(&software.soft_name)),
        format!("ssid={}", escape(&settings.user)),
        format!("sspassword={}", escape(&settings.password)),
    ];
    parts.extend(without_credentials(&pairs).iter().map(|p| p.to_string()));
    Some(format!("https://{host}{path}?{}", parts.join("&")))
}

/// Classifica pelo código HTTP e pelo começo do corpo (às vezes é texto de erro em francês).
/// `(None, false)` = sucesso; `(None, true)` = jogo não encontrado.
pub fn classify(status: u16, body: &str) -> (Option<ScreenScraperFailure>, bool) {
    use ScreenScraperFailure::*;
    let head: String = body.chars().take(512).collect::<String>().to_lowercase();
    if (200..300).contains(&status) && head.trim_start().starts_with('{') {
        return (None, false); // o texto de erro nunca vem em JSON
    }
    match status {
        430 | 431 => return (Some(QuotaExceeded), false),
        426 => return (Some(Blacklisted), false),
        429 => return (Some(ThreadLimit), false),
        401 | 423 => return (Some(Closed), false),
        _ => {}
    }
    let has = |s: &str| head.contains(s);
    if status == 404 || head.starts_with("erreur : rom") || has("non trouvée !") {
        return (None, true);
    }
    let f = if has("quota") {
        QuotaExceeded
    } else if has("blacklist") {
        Blacklisted
    } else if has("threads") {
        ThreadLimit
    } else if has("api fermé") || has("api totalement") || has("api closed") {
        Closed
    } else if has("identifiants utilisateur") || (has("erreur de login") && has("utilisateur")) {
        BadUser
    } else if status == 403 || has("identifiants développeur") || has("erreur de login") {
        BadSoftware
    } else {
        Unavailable
    };
    (Some(f), false)
}

/// Próxima meia-noite de Paris (+5 min), quando o ScreenScraper zera as cotas.
pub fn next_quota_reset(now: DateTime<Utc>) -> DateTime<Utc> {
    let offset = chrono::Duration::hours(paris_offset_hours(now));
    let local = (now + offset).date_naive();
    let midnight = local
        .succ_opt()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| Utc.from_utc_datetime(&d) - offset);
    midnight.unwrap_or(now + chrono::Duration::days(1)) + chrono::Duration::minutes(5)
}

// Horário de verão europeu: do último domingo de março ao último de outubro, às 01:00 UTC.
fn paris_offset_hours(now: DateTime<Utc>) -> i64 {
    let last_sunday = |month: u32| {
        let first_next = if month == 12 {
            NaiveDate::from_ymd_opt(now.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(now.year(), month + 1, 1)
        };
        let mut d = first_next.and_then(|d| d.pred_opt())?;
        while d.weekday() != Weekday::Sun {
            d = d.pred_opt()?;
        }
        d.and_hms_opt(1, 0, 0).map(|t| Utc.from_utc_datetime(&t))
    };
    match (last_sunday(3), last_sunday(10)) {
        (Some(start), Some(end)) if now >= start && now < end => 2,
        _ => 1,
    }
}

struct ClientState {
    rate: RateGate,
    max_threads: i64,
    pause: Option<(DateTime<Utc>, ScreenScraperSettings, ScreenScraperFailure)>,
    account: Option<ScreenScraperAccount>,
}

/// Cliente da API v2 (`GET {BASE_URL}{endpoint}`, JSON).
pub struct ScreenScraperClient {
    http: Arc<dyn Http>,
    software: ScreenScraperSoftware,
    settings: SettingsFn<ScreenScraperSettings>,
    clock: Arc<dyn Clock>,
    cache: Option<Box<dyn ResponseCache>>,
    state: Mutex<ClientState>,
    // shortcut: uma consulta por vez (o worker é uma thread só); `maxthreads` só aumenta a vazão.
    queue: Mutex<()>,
    network: AtomicUsize,
}

impl ScreenScraperClient {
    pub fn new(
        http: Arc<dyn Http>,
        software: ScreenScraperSoftware,
        settings: SettingsFn<ScreenScraperSettings>,
        clock: Arc<dyn Clock>,
        cache: Option<Box<dyn ResponseCache>>,
    ) -> Self {
        ScreenScraperClient {
            http,
            software,
            settings,
            clock,
            cache,
            state: Mutex::new(ClientState {
                rate: RateGate::new(1, SPACING),
                max_threads: 1,
                pause: None,
                account: None,
            }),
            queue: Mutex::new(()),
            network: AtomicUsize::new(0),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, ClientState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn has_software_access(&self) -> bool {
        self.software.or_user(&(self.settings)()).is_available()
    }

    pub fn is_configured(&self) -> bool {
        let settings = (self.settings)();
        self.software.or_user(&settings).is_available() && settings.is_active()
    }

    /// Pausado (cota, servidor fechado, sem rede): volta sozinho no prazo ou quando a conta muda.
    pub fn pause_reason(&self) -> Option<ScreenScraperFailure> {
        let current = (self.settings)();
        let now = self.clock.now();
        self.state()
            .pause
            .as_ref()
            .filter(|(until, s, _)| now < *until && *s == current)
            .map(|(_, _, f)| *f)
    }

    pub fn is_paused(&self) -> bool {
        self.pause_reason().is_some()
    }

    /// Última conta vista numa resposta.
    pub fn account(&self) -> Option<ScreenScraperAccount> {
        self.state().account.clone()
    }

    pub fn network_requests(&self) -> usize {
        self.network.load(Ordering::SeqCst)
    }

    /// `response` da consulta, ou `Ok(None)` quando o jogo não existe. `query` leva só os
    /// parâmetros da consulta: as credenciais entram aqui e nunca vão para o cache.
    pub fn get(
        &self,
        endpoint: &str,
        query: &[(&str, String)],
        use_cache: bool,
    ) -> Result<Option<Value>, ScreenScraperError> {
        let settings = (self.settings)();
        let software = self.software.or_user(&settings);
        if !software.is_available() {
            return Err(err(
                ScreenScraperFailure::NoSoftwareAccess,
                "Sem credenciais do software",
            ));
        }
        if !settings.has_account() {
            return Err(err(ScreenScraperFailure::NotConfigured, "Sem conta"));
        }
        if let Some(f) = self.pause_reason() {
            return Err(err(f, "ScreenScraper pausado"));
        }
        let mut sorted: Vec<&(&str, String)> = query.iter().collect();
        sorted.sort_by_key(|(k, _)| *k);
        let key = format!(
            "{endpoint}?{}",
            sorted
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&")
        );
        let cache = self.cache.as_deref().filter(|_| use_cache);
        if let Some(hit) = cache
            .and_then(|c| c.read(&key))
            .and_then(|j| p::envelope(&j))
        {
            return Ok(Some(hit));
        }
        let _one_at_a_time = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        let Some(json) = self.send(&self.build_url(endpoint, query, &settings), &settings)? else {
            return Ok(None);
        };
        let response = p::envelope(&json).ok_or_else(|| {
            self.pause(
                ScreenScraperFailure::Unavailable,
                chrono::Duration::minutes(5),
                &settings,
                "Resposta inesperada do ScreenScraper",
            )
        })?;
        self.observe(p::account(&response), &settings);
        if let Some(c) = cache {
            c.write(&key, &json);
        }
        Ok(Some(response))
    }

    /// Baixa uma mídia (`ssmedia://…`) com as credenciais; `None` quando não há acesso, a mídia não
    /// existe ou falhou (a interface usa a arte de reserva).
    pub fn download_media(&self, media: &str, max_bytes: u64) -> Option<Vec<u8>> {
        let settings = (self.settings)();
        let software = self.software.or_user(&settings);
        if !software.is_available() || self.is_paused() {
            return None;
        }
        let url = media_to_download(media, &software, &settings)?;
        let _one_at_a_time = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        self.wait_rate();
        self.network.fetch_add(1, Ordering::SeqCst);
        let mut req = HttpRequest::get(url);
        req.max_bytes = max_bytes;
        let r = self.http.send(&req).ok()?;
        // O ScreenScraper responde texto ("NOMEDIA") com 200 quando não tem a mídia.
        (r.is_success() && r.body.len() >= 256).then_some(r.body)
    }

    /// Botão "Testar": `ssuserInfos.php` com a conta informada (ainda não salva), sem cache.
    pub fn test(
        http: Arc<dyn Http>,
        software: ScreenScraperSoftware,
        settings: &ScreenScraperSettings,
        clock: Arc<dyn Clock>,
    ) -> Result<ScreenScraperAccount, ScreenScraperError> {
        let software = software.or_user(settings);
        let account = ScreenScraperSettings {
            enabled: true,
            ..settings.clone()
        };
        let client = ScreenScraperClient::new(
            http,
            software,
            Arc::new(move || account.clone()),
            clock,
            None,
        );
        client
            .get("ssuserInfos.php", &[], false)?
            .and_then(|r| p::account(&r))
            .ok_or_else(|| err(ScreenScraperFailure::BadUser, "Conta não reconhecida"))
    }

    /// Endereço completo: credenciais do software e do usuário, `output=json` e a consulta.
    pub fn build_url(
        &self,
        endpoint: &str,
        query: &[(&str, String)],
        settings: &ScreenScraperSettings,
    ) -> String {
        let software = self.software.or_user(settings);
        let mut all: Vec<(&str, &str)> = vec![
            ("devid", &software.dev_id),
            ("devpassword", &software.dev_password),
            ("softname", &software.soft_name),
            ("output", "json"),
            ("ssid", &settings.user),
            ("sspassword", &settings.password),
        ];
        all.extend(query.iter().map(|(k, v)| (*k, v.as_str())));
        let q = all
            .iter()
            .map(|(k, v)| format!("{}={}", escape(k), escape(v)))
            .collect::<Vec<_>>()
            .join("&");
        format!("{BASE_URL}{endpoint}?{q}")
    }

    fn send(
        &self,
        url: &str,
        settings: &ScreenScraperSettings,
    ) -> Result<Option<String>, ScreenScraperError> {
        use ScreenScraperFailure::*;
        let minutes = chrono::Duration::minutes;
        let mut attempt = 0;
        loop {
            attempt += 1;
            self.wait_rate();
            self.network.fetch_add(1, Ordering::SeqCst);
            let r = self.http.send(&HttpRequest::get(url)).map_err(|e| {
                self.pause(Network, minutes(2), settings, format!("Sem conexão: {e}"))
            })?;
            let body = r.text();
            let (failure, not_found) = classify(r.status, &body);
            if not_found {
                return Ok(None);
            }
            let status = r.status;
            match failure {
                None => return Ok(Some(body)),
                Some(ThreadLimit) if attempt < MAX_ATTEMPTS => {
                    self.clock.sleep(backoff(attempt, r.retry_after));
                }
                Some(ThreadLimit) => {
                    return Err(self.pause(ThreadLimit, minutes(1), settings, "Limite de threads"));
                }
                Some(QuotaExceeded) => {
                    let now = self.clock.now();
                    return Err(self.pause(
                        QuotaExceeded,
                        next_quota_reset(now) - now,
                        settings,
                        "Cota do dia esgotada",
                    ));
                }
                Some(Closed) => {
                    return Err(self.pause(
                        Closed,
                        minutes(15),
                        settings,
                        format!("Fechado ({status})"),
                    ));
                }
                // Não adianta insistir até a conta (ou o Tako) mudar.
                Some(f @ (BadUser | BadSoftware | Blacklisted)) => {
                    return Err(self.pause(
                        f,
                        chrono::Duration::hours(12),
                        settings,
                        format!("Acesso recusado ({status})"),
                    ));
                }
                Some(f) => {
                    return Err(self.pause(f, minutes(5), settings, format!("Respondeu {status}")));
                }
            }
        }
    }

    fn wait_rate(&self) {
        let wait = self.state().rate.reserve(self.clock.now());
        if !wait.is_zero() {
            self.clock.sleep(wait);
        }
    }

    fn pause(
        &self,
        failure: ScreenScraperFailure,
        duration: chrono::Duration,
        settings: &ScreenScraperSettings,
        detail: impl Into<String>,
    ) -> ScreenScraperError {
        let until = self.clock.now() + duration;
        self.state().pause = Some((until, settings.clone(), failure));
        err(failure, detail)
    }

    /// A conta vem em toda resposta: ajusta a vazão e pausa quando a cota do dia acaba.
    fn observe(&self, account: Option<ScreenScraperAccount>, settings: &ScreenScraperSettings) {
        let Some(account) = account else { return };
        let exhausted = account.is_exhausted();
        {
            let mut s = self.state();
            let threads = account.max_threads.clamp(1, 8);
            if threads != s.max_threads {
                s.max_threads = threads;
                s.rate = RateGate::new(threads as usize, SPACING);
            }
            s.account = Some(account);
        }
        if exhausted {
            let now = self.clock.now();
            self.pause(
                ScreenScraperFailure::QuotaExceeded,
                next_quota_reset(now) - now,
                settings,
                "Cota do dia esgotada",
            );
        }
    }
}

/// Provedor ScreenScraper: só ROMs. Casa pelo hash (`jeuInfos.php`), senão pelo nome no sistema.
pub struct ScreenScraperProvider {
    client: Arc<ScreenScraperClient>,
    hashes: RomHashCache,
    clock: Arc<dyn Clock>,
    games: Mutex<HashMap<String, GameMetadata>>,
    // ROM já consultada nesta execução → id do jogo, ou `None` quando o ScreenScraper não a conhece.
    roms: Mutex<HashMap<String, Option<String>>>,
}

impl ScreenScraperProvider {
    pub fn new(
        client: Arc<ScreenScraperClient>,
        hashes: RomHashCache,
        clock: Arc<dyn Clock>,
    ) -> Self {
        ScreenScraperProvider {
            client,
            hashes,
            clock,
            games: Mutex::new(HashMap::new()),
            roms: Mutex::new(HashMap::new()),
        }
    }

    pub fn client(&self) -> &Arc<ScreenScraperClient> {
        &self.client
    }

    /// Todas as mídias do jogo para o seletor de imagens (ADR-0023).
    pub fn media(&self, id: &str) -> Vec<ArtCandidate> {
        if !self.is_enabled() || self.is_paused() || id.trim().is_empty() {
            return Vec::new();
        }
        self.query("jeuInfos.php", &[("gameid", id.into())])
            .and_then(|r| r.get("jeu").map(p::media_candidates))
            .unwrap_or_default()
    }

    /// Parâmetros do `jeuInfos.php` para a ROM.
    pub fn rom_query(
        identity: &RomIdentity,
        system: &EmulatedSystem,
    ) -> Vec<(&'static str, String)> {
        let ext = p::extension_of(&identity.file_name);
        let mut q = vec![
            ("systemeid", system.screenscraper_id.to_string()),
            (
                "romtype",
                if DISC_EXTENSIONS.contains(&ext.as_str()) {
                    "iso"
                } else {
                    "rom"
                }
                .into(),
            ),
            ("romnom", identity.file_name.clone()),
            ("romtaille", identity.size.to_string()),
        ];
        for (k, v) in [
            ("crc", &identity.crc32),
            ("md5", &identity.md5),
            ("sha1", &identity.sha1),
        ] {
            if let Some(v) = v {
                q.push((k, v.clone()));
            }
        }
        q
    }

    /// Falha em segundo plano: vai para o log, o cliente pausa e volta `None`.
    fn query(&self, endpoint: &str, q: &[(&str, String)]) -> Option<Value> {
        match self.client.get(endpoint, q, true) {
            Ok(v) => v,
            Err(e)
                if matches!(
                    e.failure,
                    ScreenScraperFailure::NoSoftwareAccess | ScreenScraperFailure::NotConfigured
                ) =>
            {
                None
            }
            Err(e) => {
                tracing::warn!(endpoint, failure = ?e.failure, "ScreenScraper: consulta falhou: {e}");
                None
            }
        }
    }
}

impl MetadataProvider for ScreenScraperProvider {
    fn name(&self) -> &str {
        sources::SCREENSCRAPER
    }

    fn is_enabled(&self) -> bool {
        self.client.is_configured()
    }

    fn is_paused(&self) -> bool {
        self.client.is_paused()
    }

    fn search(&self, lookup: &MetadataLookup) -> Vec<MetadataMatch> {
        let Some(system) = p::system_by_key(lookup.platform.as_deref()) else {
            return Vec::new();
        };
        if !self.is_enabled() || self.is_paused() {
            return Vec::new();
        }
        let mut results = Vec::new();
        if let Some(id) = self.find_exact(lookup) {
            let known = self
                .games
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&id)
                .cloned();
            if let Some(g) = known {
                let mut m = MetadataMatch::new(
                    sources::SCREENSCRAPER,
                    id,
                    g.title.unwrap_or_else(|| lookup.title.clone()),
                );
                m.is_exact = true;
                m.year = g.release_date.map(|d| d.year());
                m.platforms = Some(system.name.into());
                results.push(m);
            }
        }
        let term = lookup.title.trim();
        if !term.is_empty()
            && let Some(r) = self.query(
                "jeuRecherche.php",
                &[
                    ("systemeid", system.screenscraper_id.to_string()),
                    ("recherche", term.into()),
                ],
            )
        {
            for m in p::search(&r).into_iter().take(MAX_SEARCH_RESULTS) {
                if !results.iter().any(|x: &MetadataMatch| x.id == m.id) {
                    results.push(m);
                }
            }
        }
        results
    }

    /// Pelo hash da ROM (com cache) e `jeuInfos.php`.
    fn find_exact(&self, lookup: &MetadataLookup) -> Option<String> {
        let system = p::system_by_key(lookup.platform.as_deref())?;
        let rom = lookup.rom_path.as_deref().filter(|r| !r.is_empty())?;
        if !self.is_enabled() || self.is_paused() {
            return None;
        }
        let meta = std::fs::metadata(rom).ok().filter(|m| m.is_file())?;
        let rom_key = RomHashCache::key_for(std::path::Path::new(rom), &meta, Some(system));
        if let Some(known) = self
            .roms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&rom_key)
        {
            return known.clone();
        }
        let identity = match self.hashes.get(std::path::Path::new(rom), Some(system)) {
            Ok(i) => i,
            Err(e) => {
                tracing::warn!("ScreenScraper: não foi possível ler a ROM: {e}");
                return None;
            }
        };
        let response = self.query("jeuInfos.php", &Self::rom_query(&identity, system));
        if self.is_paused() {
            return None; // falhou (cota, rede): não conta como "não conhece"
        }
        let metadata = response.and_then(|r| p::game_info(&r, self.clock.now()));
        let id = metadata
            .as_ref()
            .and_then(|m| m.source_ids.get(sources::SCREENSCRAPER).cloned());
        if let (Some(m), Some(id)) = (metadata, &id) {
            self.games
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id.clone(), m);
        }
        self.roms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(rom_key, id.clone());
        id
    }

    fn fetch(&self, id: &str) -> Option<GameMetadata> {
        if !self.is_enabled() || id.trim().is_empty() {
            return None;
        }
        let known = self
            .games
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned();
        let mut m = match known {
            Some(m) => m,
            None => {
                let r = self.query("jeuInfos.php", &[("gameid", id.into())])?;
                let m = p::game_info(&r, self.clock.now())?;
                self.games
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(id.into(), m.clone());
                m
            }
        };
        m.fetched_at = Some(self.clock.now());
        Some(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::library::{Game, GameStore};
    use crate::metadata::agent::MetadataAgent;
    use crate::metadata::agent::tests::FakeProvider;
    use crate::metadata::merge::{MetadataKind, combine};
    use crate::metadata::test_support::{FakeHttp, TestClock, fixture, now};
    use std::collections::BTreeMap;

    fn software() -> ScreenScraperSoftware {
        ScreenScraperSoftware::from_values(Some("dev"), Some("devpass"), Some("Tako"))
    }

    fn account() -> ScreenScraperSettings {
        ScreenScraperSettings {
            enabled: true,
            user: "jogador".into(),
            password: "senha secreta".into(),
            dev_id: "dev".into(),
            dev_password: "pass".into(),
        }
    }

    fn client_with(
        http: &Arc<FakeHttp>,
        clock: &Arc<TestClock>,
        settings: SettingsFn<ScreenScraperSettings>,
        cache: Option<Box<dyn ResponseCache>>,
    ) -> Arc<ScreenScraperClient> {
        Arc::new(ScreenScraperClient::new(
            http.clone(),
            software(),
            settings,
            clock.clone(),
            cache,
        ))
    }

    fn client(http: &Arc<FakeHttp>, clock: &Arc<TestClock>) -> Arc<ScreenScraperClient> {
        client_with(http, clock, Arc::new(account), None)
    }

    fn not_found() -> Arc<FakeHttp> {
        Arc::new(FakeHttp::with_fallback(
            404,
            "Erreur : Rom/Iso/Dossier non trouvée !",
        ))
    }

    fn write_rom(dir: &std::path::Path, name: &str, content: &str) -> String {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p.to_string_lossy().into_owned()
    }

    fn rom_game(path: &str) -> Game {
        Game {
            id: "manual:rom".into(),
            title: p::rom_title(path),
            store: Some(GameStore::Manual),
            platform: Some("snes".into()),
            rom_path: Some(path.into()),
            is_installed: true,
            ..Default::default()
        }
    }

    #[test]
    fn responses_are_classified_by_code_and_french_text() {
        use ScreenScraperFailure::*;
        for (status, body, expected, nf) in [
            (200, "{\"header\":{}}", None, false),
            (404, "Erreur : Rom/Iso/Dossier non trouvée !", None, true),
            (200, "Erreur : Rom/Iso/Dossier non trouvée !", None, true),
            (
                430,
                "Votre quota de scrape est dépassé pour aujourd'hui !",
                Some(QuotaExceeded),
                false,
            ),
            (431, "Trop de roms non trouvées", Some(QuotaExceeded), false),
            (
                429,
                "Le nombre de threads autorisés est atteint",
                Some(ThreadLimit),
                false,
            ),
            (401, "API fermé pour les non membres", Some(Closed), false),
            (423, "API totalement fermé", Some(Closed), false),
            (
                426,
                "Le logiciel de scrape utilisé a été blacklisté",
                Some(Blacklisted),
                false,
            ),
            (
                403,
                "Erreur de login : Vérifier vos identifiants développeur !",
                Some(BadSoftware),
                false,
            ),
            (
                200,
                "Erreur de login : Vérifier vos identifiants utilisateur !",
                Some(BadUser),
                false,
            ),
            (500, "erro", Some(Unavailable), false),
        ] {
            assert_eq!(classify(status, body), (expected, nf), "{status} {body}");
        }
    }

    #[test]
    fn error_messages_are_clear_and_distinct() {
        let mut m: Vec<String> = ScreenScraperFailure::ALL
            .iter()
            .map(|f| f.message())
            .collect();
        let n = m.len();
        m.sort();
        m.dedup();
        assert_eq!(m.len(), n);
    }

    #[test]
    fn media_urls_drop_credentials_and_only_go_back_to_screenscraper() {
        let stored = media_from_response("https://neoclone.screenscraper.fr/api2/mediaJeu.php?devid=D&devpassword=P&softname=S&ssid=U&sspassword=X&systemeid=4&jeuid=1&media=box-2D(us)").unwrap();
        assert_eq!(
            stored,
            "ssmedia://neoclone.screenscraper.fr/api2/mediaJeu.php?systemeid=4&jeuid=1&media=box-2D(us)"
        );
        let download = media_to_download(&stored, &software(), &account()).unwrap();
        assert!(download.starts_with("https://"));
        assert!(download.contains("devid=dev&devpassword=devpass&softname=Tako&ssid=jogador&sspassword=senha%20secreta&systemeid=4"));
        assert_eq!(media_from_response("https://exemplo.invalid/a.png"), None);
        assert_eq!(
            media_from_response("https://screenscraper.fr.exemplo.invalid/a.png"),
            None
        );
        assert_eq!(
            media_from_response("https://u@evil.invalid@x.screenscraper.fr/a.png"),
            None
        );
        assert_eq!(
            media_to_download("ssmedia://exemplo.invalid/a.png", &software(), &account()),
            None
        );
        assert_eq!(
            media_to_download(
                "https://neoclone.screenscraper.fr/a.png",
                &software(),
                &account()
            ),
            None
        );
    }

    #[test]
    fn software_credentials_come_only_from_the_build() {
        // `cargo test` não recebe os segredos do CI de release.
        if option_env!("TAKO_SCREENSCRAPER_DEV_ID").is_none() {
            assert!(!ScreenScraperSoftware::from_build().is_available());
        }
        let release = ScreenScraperSoftware::from_values(Some(" id "), Some("senha"), None);
        assert!(release.is_available());
        assert_eq!(release.dev_id, "id");
        assert_eq!(release.soft_name, "Tako");
        assert!(!format!("{release:?}").contains("senha"));
        assert!(!format!("{:?}", account()).contains("senha secreta"));
    }

    #[test]
    fn account_store_round_trips_and_validates() {
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
        assert_eq!(
            ScreenScraperSettings::load(&store),
            ScreenScraperSettings::default()
        );
        ScreenScraperSettings {
            user: " jogador ".into(),
            ..account()
        }
        .save(&store)
        .unwrap();
        let back = ScreenScraperSettings::load(&store);
        assert_eq!(back, account());
        assert!(back.is_active());
        assert_eq!(back.problem(), None);
        assert!(
            ScreenScraperSettings {
                user: String::new(),
                ..account()
            }
            .problem()
            .is_some()
        );
        assert!(
            ScreenScraperSettings {
                password: String::new(),
                ..account()
            }
            .problem()
            .is_some()
        );
        assert!(
            ScreenScraperSettings {
                dev_id: String::new(),
                ..account()
            }
            .problem()
            .is_some()
        );
        assert!(
            ScreenScraperSettings {
                dev_password: String::new(),
                ..account()
            }
            .problem()
            .is_some()
        );
        store.set(SECRET_KEY, "lixo").unwrap();
        assert_eq!(
            ScreenScraperSettings::load(&store),
            ScreenScraperSettings::default()
        );
    }

    #[test]
    fn without_software_credentials_nothing_goes_to_the_network() {
        let http = not_found();
        let clock = Arc::new(TestClock::at(now()));
        let no_dev = ScreenScraperSettings {
            dev_id: String::new(),
            dev_password: String::new(),
            ..account()
        };
        let no_dev_clone = no_dev.clone();
        let c = Arc::new(ScreenScraperClient::new(
            http.clone(),
            ScreenScraperSoftware::from_values(None, None, None),
            Arc::new(move || no_dev_clone.clone()),
            clock.clone(),
            None,
        ));
        assert!(!c.has_software_access());
        assert!(!c.is_configured());
        assert_eq!(
            c.get("jeuInfos.php", &[], true).unwrap_err().failure,
            ScreenScraperFailure::NoSoftwareAccess
        );
        let test = ScreenScraperClient::test(
            http.clone(),
            ScreenScraperSoftware::from_values(None, None, None),
            &no_dev,
            clock.clone(),
        );
        assert_eq!(
            test.unwrap_err().failure,
            ScreenScraperFailure::NoSoftwareAccess
        );
        assert!(http.sent().is_empty());
        assert!(
            !ScreenScraperProvider::new(c, RomHashCache::new(None), clock.clone()).is_enabled()
        );

        // Com as credenciais de dev informadas pelo usuário, o cliente funciona mesmo sem build
        let c_with_dev = Arc::new(ScreenScraperClient::new(
            http,
            ScreenScraperSoftware::from_values(None, None, None),
            Arc::new(account),
            clock,
            None,
        ));
        assert!(c_with_dev.has_software_access());
        assert!(c_with_dev.is_configured());
    }

    #[test]
    fn or_user_prefers_build_and_falls_back_to_user() {
        let build =
            ScreenScraperSoftware::from_values(Some("b_id"), Some("b_pass"), Some("CustomSoft"));
        let user_settings = ScreenScraperSettings {
            dev_id: "u_id".into(),
            dev_password: "u_pass".into(),
            ..Default::default()
        };
        let resolved_build = build.or_user(&user_settings);
        assert_eq!(resolved_build.dev_id, "b_id");
        assert_eq!(resolved_build.dev_password, "b_pass");
        assert_eq!(resolved_build.soft_name, "CustomSoft");

        let empty_build = ScreenScraperSoftware::from_values(None, None, None);
        assert!(!empty_build.is_available());
        let resolved_user = empty_build.or_user(&user_settings);
        assert_eq!(resolved_user.dev_id, "u_id");
        assert_eq!(resolved_user.dev_password, "u_pass");
        assert_eq!(resolved_user.soft_name, "Tako");
        assert!(resolved_user.is_available());

        let empty_settings = ScreenScraperSettings::default();
        let resolved_empty = empty_build.or_user(&empty_settings);
        assert!(!resolved_empty.is_available());
    }

    #[test]
    fn old_settings_without_dev_credentials_deserialize_cleanly() {
        let legacy_json = r#"{"enabled":true,"user":"jogador","password":"123"}"#;
        let s: ScreenScraperSettings = serde_json::from_str(legacy_json).unwrap();
        assert!(s.enabled);
        assert_eq!(s.user, "jogador");
        assert_eq!(s.password, "123");
        assert_eq!(s.dev_id, "");
        assert_eq!(s.dev_password, "");
    }

    #[test]
    fn debug_does_not_leak_dev_password() {
        let s = ScreenScraperSettings {
            enabled: true,
            user: "jogador".into(),
            password: "super_secret_user_pass".into(),
            dev_id: "my_dev_id".into(),
            dev_password: "super_secret_dev_pass".into(),
        };
        let debug_str = format!("{s:?}");
        assert!(!debug_str.contains("super_secret_user_pass"));
        assert!(!debug_str.contains("super_secret_dev_pass"));
        assert!(debug_str.contains("***"));
        assert!(debug_str.contains("my_dev_id"));
    }

    #[test]
    fn test_reports_level_and_daily_quota() {
        let http = not_found();
        http.push(200, &fixture("ssuserInfos.json"));
        let clock = Arc::new(TestClock::at(now()));
        let off = ScreenScraperSettings {
            enabled: false,
            ..account()
        };
        let a = ScreenScraperClient::test(http.clone(), software(), &off, clock.clone()).unwrap();
        assert_eq!(a.level, 1);
        assert_eq!(a.remaining_today(), 19980);
        let sent = http.sent();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].url.starts_with("https://api.screenscraper.fr/api2/ssuserInfos.php?devid=dev&devpassword=devpass&softname=Tako&output=json&ssid=jogador&sspassword=senha%20secreta"));

        http.push(
            200,
            "Erreur de login : Vérifier vos identifiants utilisateur !",
        );
        let bad = ScreenScraperClient::test(http, software(), &account(), clock).unwrap_err();
        assert_eq!(bad.failure, ScreenScraperFailure::BadUser);
    }

    struct MemoryCache(Mutex<BTreeMap<String, String>>);
    impl ResponseCache for MemoryCache {
        fn read(&self, key: &str) -> Option<String> {
            self.0.lock().unwrap().get(key).cloned()
        }
        fn write(&self, key: &str, json: &str) {
            self.0.lock().unwrap().insert(key.into(), json.into());
        }
    }

    #[test]
    fn client_retries_thread_limit_caches_without_credentials_and_learns_maxthreads() {
        let http = not_found();
        http.push(429, "Le nombre de threads autorisés est atteint");
        http.push(200, &fixture("jeuInfos-lanterna.json"));
        let clock = Arc::new(TestClock::at(now()));
        let cache = Arc::new(MemoryCache(Mutex::default()));
        struct Shared(Arc<MemoryCache>);
        impl ResponseCache for Shared {
            fn read(&self, k: &str) -> Option<String> {
                self.0.read(k)
            }
            fn write(&self, k: &str, j: &str) {
                self.0.write(k, j)
            }
        }
        let c = client_with(
            &http,
            &clock,
            Arc::new(account),
            Some(Box::new(Shared(cache.clone()))),
        );
        let q = [("gameid", "900077".to_owned())];
        assert!(c.get("jeuInfos.php", &q, true).unwrap().is_some());
        assert!(c.get("jeuInfos.php", &q, true).unwrap().is_some());
        assert_eq!(c.network_requests(), 2);
        assert!(
            clock
                .sleeps
                .lock()
                .unwrap()
                .contains(&Duration::from_secs(1))
        );
        let a = c.account().unwrap();
        assert_eq!(a.max_threads, 2);
        assert_eq!(a.remaining_today(), 19880);
        let keys: Vec<_> = cache.0.lock().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["jeuInfos.php?gameid=900077"]);
        assert!(!c.is_paused());
    }

    #[test]
    fn daily_quota_pauses_until_the_reset_or_an_account_change() {
        let http = not_found();
        http.push(430, "Votre quota de scrape est dépassé pour aujourd'hui !");
        let clock = Arc::new(TestClock::at(now()));
        let settings = Arc::new(Mutex::new(account()));
        let s = settings.clone();
        let c = client_with(
            &http,
            &clock,
            Arc::new(move || s.lock().unwrap().clone()),
            None,
        );
        let e = c
            .get("jeuInfos.php", &[("gameid", "1".into())], true)
            .unwrap_err();
        assert_eq!(e.failure, ScreenScraperFailure::QuotaExceeded);
        assert!(c.is_paused());
        assert_eq!(c.pause_reason(), Some(ScreenScraperFailure::QuotaExceeded));
        assert!(
            c.get("jeuInfos.php", &[("gameid", "2".into())], true)
                .is_err()
        );
        assert_eq!(http.sent().len(), 1);
        settings.lock().unwrap().user = "outra".into();
        assert!(!c.is_paused());
    }

    #[test]
    fn quota_resets_after_midnight_in_paris() {
        use chrono::Timelike;
        let reset = next_quota_reset(now());
        assert!(reset > now());
        assert!(reset - now() <= chrono::Duration::hours(25));
        assert_eq!(reset.minute(), 5);
        // 15/10/2026 é horário de verão (UTC+2): meia-noite de Paris = 22:00 UTC.
        assert_eq!(reset.hour(), 22);
        let winter = Utc
            .with_ymd_and_hms(2026, 12, 1, 12, 0, 0)
            .single()
            .unwrap();
        assert_eq!(next_quota_reset(winter).hour(), 23);
    }

    #[test]
    fn provider_matches_the_rom_by_hash_and_reuses_the_answer() {
        let dir = tempfile::tempdir().unwrap();
        let http = not_found();
        http.push(200, &fixture("jeuInfos-lanterna.json"));
        let clock = Arc::new(TestClock::at(now()));
        let p = ScreenScraperProvider::new(client(&http, &clock), RomHashCache::new(None), clock);
        let rom = write_rom(dir.path(), "Lanterna Distante (USA).sfc", "123456789");
        let lookup = MetadataLookup {
            platform: Some("snes".into()),
            rom_path: Some(rom),
            ..MetadataLookup::new("Lanterna Distante")
        };
        let id = p.find_exact(&lookup).unwrap();
        let m = p.fetch(&id).unwrap();
        assert_eq!(id, "900077");
        assert_eq!(m.title.as_deref(), Some("Lanterna Distante"));
        let sent = http.sent();
        assert_eq!(sent.len(), 1);
        assert!(sent[0].url.contains("/api2/jeuInfos.php?"));
        assert!(sent[0].url.contains("&systemeid=4&romtype=rom&romnom=Lanterna%20Distante%20%28USA%29.sfc&romtaille=9&crc=CBF43926&md5=25F9E794323B453885F5181F1B624D0B&sha1=F7C3BC1D808E04732ADF679965CCC34CA7AE3441"));

        assert_eq!(p.find_exact(&lookup).as_deref(), Some("900077"));
        assert_eq!(http.sent().len(), 1);

        assert_eq!(
            p.find_exact(&MetadataLookup {
                platform: None,
                ..lookup.clone()
            }),
            None
        );
        let gone = dir.path().join("sumiu.sfc").to_string_lossy().into_owned();
        assert_eq!(
            p.find_exact(&MetadataLookup {
                rom_path: Some(gone),
                ..lookup.clone()
            }),
            None
        );
        assert!(
            p.search(&MetadataLookup {
                platform: Some("pc".into()),
                ..lookup
            })
            .is_empty()
        );
        assert_eq!(http.sent().len(), 1);
    }

    #[test]
    fn rom_query_marks_disc_images_as_iso() {
        let q = ScreenScraperProvider::rom_query(
            &RomIdentity {
                file_name: "Lanterna.chd".into(),
                size: 700,
                ..Default::default()
            },
            p::system_by_key(Some("ps1")).unwrap(),
        );
        let pairs: Vec<_> = q.iter().map(|(k, v)| format!("{k}={v}")).collect();
        assert_eq!(
            pairs,
            [
                "systemeid=57",
                "romtype=iso",
                "romnom=Lanterna.chd",
                "romtaille=700"
            ]
        );
    }

    #[test]
    fn search_lists_the_hash_match_first_then_name_results() {
        let dir = tempfile::tempdir().unwrap();
        let http = not_found();
        http.push(404, "Erreur : Rom/Iso/Dossier non trouvée !");
        http.push(200, &fixture("jeuRecherche-lanterna.json"));
        let clock = Arc::new(TestClock::at(now()));
        let p = ScreenScraperProvider::new(client(&http, &clock), RomHashCache::new(None), clock);
        let lookup = MetadataLookup {
            platform: Some("snes".into()),
            rom_path: Some(write_rom(dir.path(), "lanterna.sfc", "x")),
            ..MetadataLookup::new("Lanterna")
        };
        let r = p.search(&lookup);
        let ids: Vec<_> = r.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["900077", "900078"]);
        assert!(r.iter().all(|m| !m.is_exact));
        let sent = http.sent();
        assert!(sent[1].url.contains("jeuRecherche.php?"));
        assert!(sent[1].url.contains("&systemeid=4&recherche=Lanterna"));
        assert!(!p.is_paused());
    }

    fn agent(providers: Vec<Arc<dyn MetadataProvider>>) -> MetadataAgent {
        let mut a = MetadataAgent::new(providers, Arc::new(TestClock::at(now())));
        a.lang = Some(Lang::PtBr);
        a
    }

    #[test]
    fn roms_use_screenscraper_first_and_igdb_fills_the_gaps() {
        let dir = tempfile::tempdir().unwrap();
        let http = not_found();
        http.push(200, &fixture("jeuInfos-lanterna.json"));
        let clock = Arc::new(TestClock::at(now()));
        let ss = Arc::new(ScreenScraperProvider::new(
            client(&http, &clock),
            RomHashCache::new(None),
            clock,
        ));
        let igdb_meta = GameMetadata {
            source_ids: [(sources::IGDB.to_owned(), "42".to_owned())].into(),
            title: Some("Distant Lantern".into()),
            description: Some("English summary".into()),
            description_language: Some("en".into()),
            themes: vec!["Fantasia".into()],
            franchise: Some("Lanterna".into()),
            cover_url: Some("https://images.igdb.com/capa.jpg".into()),
            ..GameMetadata::new(sources::IGDB)
        };
        let mut igdb = FakeProvider::new(sources::IGDB, Some(igdb_meta));
        igdb.results = vec![MetadataMatch {
            is_exact: true,
            ..MetadataMatch::new(sources::IGDB, "42", "Distant Lantern")
        }];
        let igdb = Arc::new(igdb);
        let a = agent(vec![igdb.clone(), ss]);
        let mut g = rom_game(&write_rom(
            dir.path(),
            "Lanterna Distante (USA).sfc",
            "123456789",
        ));
        let merged = a
            .update(&g, None, false, &BTreeMap::new())
            .metadata
            .unwrap();
        merged.apply_to(&mut g);
        assert_eq!(merged.source, "ScreenScraper + IGDB");
        assert!(g.description.starts_with("Um faroleiro"));
        assert_eq!(g.themes, ["Fantasia"]);
        assert_eq!(g.franchise.as_deref(), Some("Lanterna"));
        assert!(g.cover_uri.as_deref().unwrap().starts_with("ssmedia://"));
        assert_eq!(g.players.as_deref(), Some("1-2"));
        assert!(g.media.contains_key(crate::library::media::LOGO));
        assert!(!a.needs_update(&g, Some(&merged), false, &BTreeMap::new()));
        assert_eq!(igdb.searches(), 1); // a IGDB buscou com a plataforma da ROM
    }

    #[test]
    fn pc_games_never_spend_the_screenscraper_quota() {
        let http = not_found();
        let clock = Arc::new(TestClock::at(now()));
        let ss = Arc::new(ScreenScraperProvider::new(
            client(&http, &clock),
            RomHashCache::new(None),
            clock,
        ));
        let a = agent(vec![Arc::new(FakeProvider::new("Steam", None)), ss]);
        let g = Game {
            id: "epic:1".into(),
            title: "Lanterna".into(),
            store: Some(GameStore::Epic),
            ..Default::default()
        };
        let m = a
            .update(&g, None, false, &BTreeMap::new())
            .metadata
            .unwrap();
        assert_eq!(m.consulted, ["Steam"]);
        assert!(!a.needs_update(&g, Some(&m), false, &BTreeMap::new()));
        assert!(a.search(&g).is_empty());
        assert!(http.sent().is_empty());
    }

    #[test]
    fn exhausted_quota_keeps_roms_pending_and_their_old_data() {
        let dir = tempfile::tempdir().unwrap();
        let http = not_found();
        http.push(430, "Votre quota de scrape est dépassé pour aujourd'hui !");
        let clock = Arc::new(TestClock::at(now()));
        let ss = Arc::new(ScreenScraperProvider::new(
            client(&http, &clock),
            RomHashCache::new(None),
            clock,
        ));
        let a = agent(vec![
            Arc::new(FakeProvider::new(sources::IGDB, None)),
            ss.clone(),
        ]);
        let g = rom_game(&write_rom(dir.path(), "lanterna.sfc", "123456789"));
        let m = a
            .update(&g, None, false, &BTreeMap::new())
            .metadata
            .unwrap();
        assert!(ss.is_paused());
        assert_eq!(m.consulted, [sources::IGDB]);
        assert!(!a.needs_update(&g, Some(&m), false, &BTreeMap::new()));

        let old = GameMetadata {
            source_ids: [(sources::SCREENSCRAPER.to_owned(), "900077".to_owned())].into(),
            title: Some("Lanterna Distante".into()),
            players: Some("1-2".into()),
            ..GameMetadata::new(sources::SCREENSCRAPER)
        };
        let cached = combine(
            &[old],
            MetadataKind::Emulated,
            now(),
            Some(&[sources::SCREENSCRAPER.to_owned()]),
            Lang::PtBr,
        );
        let later = a
            .update(&g, Some(&cached), false, &BTreeMap::new())
            .metadata
            .unwrap();
        assert!(
            later
                .parts
                .iter()
                .any(|p| p.source == sources::SCREENSCRAPER)
        );
        assert_eq!(later.players.as_deref(), Some("1-2"));
        assert_eq!(http.sent().len(), 1);
    }
}
