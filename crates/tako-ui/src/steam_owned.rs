//! Jogos da conta Steam (SRC-005): busca a lista na Web API com a chave do usuário e grava o cache que a fonte
//! da Steam lê. Sem rede ou com a chave errada, fica o cache anterior.

use std::path::{Path, PathBuf};
use std::time::Duration;
use tako_core::metadata::http::{Http, HttpRequest, UreqHttp};
use tako_core::paths::DataPaths;
use tako_sources::steam::owned;

/// Nome da chave no cofre do SO.
pub const SECRET: &str = "steam-web-api-key";
/// Ler a API de novo só depois disso (a biblioteca é relida bem mais vezes).
const FRESH: Duration = Duration::from_secs(60 * 60);

pub fn cache(paths: &DataPaths) -> PathBuf {
    paths.cache().join("steam-owned.json")
}

fn is_fresh(cache: &Path) -> bool {
    std::fs::metadata(cache)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < FRESH)
}

/// Atualiza o cache se ele tiver mais de 1 hora (`force`: agora). Bloqueante: rodar fora da thread da UI.
pub fn refresh(key: &str, cache: &Path, force: bool) -> Result<(), String> {
    if !force && is_fresh(cache) {
        return Ok(());
    }
    let root = tako_sources::steam::find_root().ok_or("Steam não encontrada")?;
    let account = owned::account_id(&root).ok_or("nenhuma conta da Steam neste PC")?;
    let http = UreqHttp::new(Duration::from_secs(20));
    // A chave vai no endereço: nada daqui pode ir para o log sem tirar a chave.
    let hide = |e: String| e.replace(key, "***");
    let resp = http
        .send(&HttpRequest::get(owned::url(key, owned::steam_id(account))))
        .map_err(hide)?;
    if !resp.is_success() {
        return Err(format!("Web API da Steam respondeu {}", resp.status));
    }
    let text = resp.text();
    owned::parse(&text).ok_or("resposta inesperada da Web API da Steam")?;
    if let Some(dir) = cache.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = cache.with_extension("json.tmp");
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, cache))
        .map_err(|e| e.to_string())
}
