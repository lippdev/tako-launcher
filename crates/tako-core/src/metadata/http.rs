//! Rede e tempo para os provedores: HTTP bloqueante (`ureq`, sempre com tempo-limite e limite de
//! tamanho), relógio trocável nos testes, limite de vazão, backoff e cache de respostas em arquivo.

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

/// Relógio e espera; os testes trocam por um parado que só anota as esperas.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Respostas JSON acima disso são recusadas.
pub const MAX_JSON_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// "GET" ou "POST".
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: Option<String>,
    /// Corpo maior que isso vira erro.
    pub max_bytes: u64,
}

impl HttpRequest {
    pub fn get(url: impl Into<String>) -> Self {
        HttpRequest {
            method: "GET",
            url: url.into(),
            headers: Vec::new(),
            body: None,
            max_bytes: MAX_JSON_BYTES,
        }
    }

    pub fn post(url: impl Into<String>, body: impl Into<String>) -> Self {
        HttpRequest {
            method: "POST",
            body: Some(body.into()),
            ..Self::get(url)
        }
    }

    pub fn header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    /// `Retry-After` em segundos, quando o servidor manda.
    pub retry_after: Option<Duration>,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Transporte HTTP. `Err` = sem rede, tempo esgotado ou corpo grande demais.
pub trait Http: Send + Sync {
    fn send(&self, req: &HttpRequest) -> Result<HttpResponse, String>;
}

/// `ureq` com tempo-limite global; status HTTP de erro não viram `Err` (quem chama classifica).
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    pub fn new(timeout: Duration) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .http_status_as_error(false)
            .user_agent(concat!("Tako/", env!("CARGO_PKG_VERSION")))
            .build();
        UreqHttp {
            agent: ureq::Agent::new_with_config(config),
        }
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self::new(Duration::from_secs(20))
    }
}

impl Http for UreqHttp {
    fn send(&self, req: &HttpRequest) -> Result<HttpResponse, String> {
        let result = if req.method == "POST" {
            let mut b = self.agent.post(&req.url);
            for (k, v) in &req.headers {
                b = b.header(*k, v);
            }
            b.send(req.body.as_deref().unwrap_or_default())
        } else {
            let mut b = self.agent.get(&req.url);
            for (k, v) in &req.headers {
                b = b.header(*k, v);
            }
            b.call()
        };
        let mut resp = result.map_err(|e| e.to_string())?;
        let header = |name: &str| {
            resp.headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let content_type = header("content-type");
        let retry_after = header("retry-after")
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(Duration::from_secs);
        let status = resp.status().as_u16();
        let body = resp
            .body_mut()
            .with_config()
            .limit(req.max_bytes)
            .read_to_vec()
            .map_err(|e| e.to_string())?;
        Ok(HttpResponse {
            status,
            body,
            content_type,
            retry_after,
        })
    }
}

/// No máximo `permits` pedidos por janela. [`RateGate::reserve`] diz quanto esperar antes do próximo.
pub struct RateGate {
    permits: usize,
    window: chrono::Duration,
    slots: Mutex<VecDeque<DateTime<Utc>>>,
}

impl RateGate {
    pub fn new(permits: usize, window: Duration) -> Self {
        RateGate {
            permits: permits.max(1),
            window: chrono::Duration::from_std(window).unwrap_or(chrono::Duration::seconds(1)),
            slots: Mutex::new(VecDeque::new()),
        }
    }

    pub fn reserve(&self, now: DateTime<Utc>) -> Duration {
        let mut slots = self.slots.lock().unwrap_or_else(|e| e.into_inner());
        while slots.front().is_some_and(|s| *s <= now - self.window) {
            slots.pop_front();
        }
        let at = if slots.len() < self.permits {
            now
        } else {
            slots[slots.len() - self.permits] + self.window
        };
        slots.push_back(at);
        (at - now).to_std().unwrap_or_default()
    }
}

/// 1 s, 2 s, 4 s... (até 16 s) ou o que o servidor pedir (até 30 s).
pub fn backoff(attempt: u32, retry_after: Option<Duration>) -> Duration {
    match retry_after {
        Some(asked) if !asked.is_zero() => asked.min(Duration::from_secs(30)),
        _ => Duration::from_secs(1 << attempt.saturating_sub(1).min(4)),
    }
}

/// Cache de respostas (mesma consulta → mesma resposta).
pub trait ResponseCache: Send + Sync {
    fn read(&self, key: &str) -> Option<String>;
    fn write(&self, key: &str, json: &str);
}

/// Um arquivo por consulta (`<sha256>.json`), válido por `max_age`. Só respostas com sucesso.
pub struct FileCache {
    dir: PathBuf,
    max_age: Duration,
}

impl FileCache {
    pub fn new(dir: impl Into<PathBuf>, max_age: Duration) -> Self {
        FileCache {
            dir: dir.into(),
            max_age,
        }
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.dir
            .join(format!("{}.json", &hex(&Sha256::digest(key))[..32]))
    }
}

impl ResponseCache for FileCache {
    fn read(&self, key: &str) -> Option<String> {
        let path = self.path_for(key);
        let age = std::fs::metadata(&path).ok()?.modified().ok()?.elapsed();
        // Data no futuro (relógio mudou) conta como recente.
        if age.is_ok_and(|a| a >= self.max_age) {
            return None;
        }
        std::fs::read_to_string(path).ok()
    }

    fn write(&self, key: &str, json: &str) {
        // Sem cache, a consulta só se repete na próxima vez.
        let _ = std::fs::create_dir_all(&self.dir)
            .and_then(|()| std::fs::write(self.path_for(key), json));
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `Uri.EscapeDataString`: tudo fora de `A-Z a-z 0-9 - _ . ~` vira `%XX`.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Corpo `application/x-www-form-urlencoded`.
pub fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", escape(k), escape(v)))
        .collect::<Vec<_>>()
        .join("&")
}

pub fn parse_json(bytes: &[u8]) -> Option<serde_json::Value> {
    serde_json::from_slice(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::test_support::now;

    #[test]
    fn rate_gate_allows_four_per_second() {
        let gate = RateGate::new(4, Duration::from_secs(1));
        let t = now();
        for _ in 0..4 {
            assert_eq!(gate.reserve(t), Duration::ZERO);
        }
        assert_eq!(gate.reserve(t), Duration::from_secs(1));
        assert_eq!(
            gate.reserve(t + chrono::Duration::milliseconds(1500)),
            Duration::ZERO
        );
    }

    #[test]
    fn backoff_doubles_and_honors_retry_after() {
        assert_eq!(backoff(1, None), Duration::from_secs(1));
        assert_eq!(backoff(3, None), Duration::from_secs(4));
        assert_eq!(
            backoff(1, Some(Duration::from_secs(7))),
            Duration::from_secs(7)
        );
        assert_eq!(
            backoff(1, Some(Duration::from_secs(300))),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn file_cache_expires_after_its_max_age() {
        let dir = tempfile::tempdir().unwrap();
        let cache = FileCache::new(dir.path(), Duration::from_secs(3600));
        cache.write("games\nfields name;", "[1]");
        assert_eq!(cache.read("games\nfields name;").as_deref(), Some("[1]"));
        assert_eq!(cache.read("games\nfields id;"), None);
        let expired = FileCache::new(dir.path(), Duration::ZERO);
        assert_eq!(expired.read("games\nfields name;"), None);
    }

    #[test]
    fn escape_matches_uri_escape_data_string() {
        assert_eq!(
            escape("Lanterna Distante (USA).sfc"),
            "Lanterna%20Distante%20%28USA%29.sfc"
        );
        assert_eq!(escape("ç"), "%C3%A7");
    }
}
