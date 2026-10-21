//! Cache de artes no disco (port da parte de arquivos de `ArtService.cs`), agora com limite: quando
//! passa do total (1 GiB por padrão), apaga os arquivos usados há mais tempo. Aceita `https://`,
//! arquivos locais, `steam-art://` (vários candidatos) e `ssmedia://` (credenciais só na hora).

use super::cache::write_cache_file;
use super::http::{Http, HttpRequest, hex};
use super::screenscraper::{MEDIA_SCHEME, ScreenScraperClient};
use super::steam_art::{ArtSource, SteamArtResolver};
use super::{STEAM_ART_SCHEME, scheme_of};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

pub const DEFAULT_LIMIT: u64 = 1024 * 1024 * 1024;
/// Imagem maior que isso não é baixada.
pub const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;
/// Dias em que uma arte que a loja não tem (404, ícone ausente na API) não é pedida de novo.
const MISS_DAYS: u64 = 7;
/// Conteúdo do marcador de falta; marcador com outro conteúdo (versões antigas, gravadas quando o ícone da
/// Steam vinha de um endereço que dava 404) não vale e a arte é procurada de novo.
const MISS_MARK: &[u8] = b"2";

pub struct ArtCache {
    dir: PathBuf,
    limit: AtomicU64,
    http: Arc<dyn Http>,
    steam: SteamArtResolver,
    screenscraper: Option<Arc<ScreenScraperClient>>,
    // Uma limpeza por vez.
    evicting: Mutex<()>,
}

impl ArtCache {
    /// `dir` = `DataPaths::art_cache()`.
    pub fn new(
        dir: impl Into<PathBuf>,
        http: Arc<dyn Http>,
        steam: SteamArtResolver,
        screenscraper: Option<Arc<ScreenScraperClient>>,
    ) -> Self {
        ArtCache {
            dir: dir.into(),
            limit: AtomicU64::new(DEFAULT_LIMIT),
            http,
            steam,
            screenscraper,
            evicting: Mutex::new(()),
        }
    }

    pub fn limit(&self) -> u64 {
        self.limit.load(Ordering::Relaxed)
    }

    /// Novo limite em bytes; aplicado já.
    pub fn set_limit(&self, bytes: u64) {
        self.limit.store(bytes, Ordering::Relaxed);
        self.enforce_limit(None);
    }

    /// Arquivo local da imagem (baixa para o cache se preciso); `None` sem arte ou sem rede.
    /// Bloqueante: chamar fora da thread da UI.
    pub fn locate(&self, uri: &str) -> Option<PathBuf> {
        let scheme = scheme_of(uri);
        match scheme.as_deref() {
            None | Some("file") => return file_path(uri).filter(|p| p.is_file()),
            _ => {}
        }
        let cached = self.cache_path(uri);
        if cached.is_file() {
            touch(&cached);
            return Some(cached);
        }
        // Arte que a loja confirmou não ter (404/410, ou a API da Steam sem ela): não vai à rede por `MISS_DAYS`
        // dias. Só a rede é pulada: o cache local da Steam continua valendo.
        let mut miss = cached.clone().into_os_string();
        miss.push(".miss");
        let miss = PathBuf::from(miss);
        let fresh = std::fs::read(&miss).is_ok_and(|b| b == MISS_MARK)
            && std::fs::metadata(&miss)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age.as_secs() < MISS_DAYS * 24 * 3600);
        if fresh {
            if let Some(file) = self.steam.local_files(uri).into_iter().next() {
                return Some(file);
            }
            if let Some(app) = steam_app_from_icon_url(uri) {
                let fallback_uri = format!("{STEAM_ART_SCHEME}://{app}/icon");
                if let Some(fallback) = self.locate(&fallback_uri) {
                    let _ = std::fs::remove_file(&miss);
                    let _ = std::fs::copy(&fallback, &cached);
                    return Some(fallback);
                }
            }
            return None;
        }
        let transient = Cell::new(false);
        let found = match scheme.as_deref() {
            Some(MEDIA_SCHEME) => {
                let bytes = self
                    .screenscraper
                    .as_ref()?
                    .download_media(uri, MAX_IMAGE_BYTES)
                    .filter(|b| looks_like_image(b))?;
                // O ScreenScraper tem cota e credenciais: não marca falta.
                return self.store(&cached, &bytes);
            }
            // Artes da Steam: o cache vale para o endereço lógico, sem nova consulta à rede.
            Some(STEAM_ART_SCHEME) => {
                let found = self.steam.resolve(uri).into_iter().find_map(|c| match c {
                    ArtSource::File(p) => Some(p), // cache da própria Steam
                    ArtSource::Url(u) => self
                        .download(&u, &transient)
                        .and_then(|b| self.store(&cached, &b)),
                });
                let answered = super::steam_art::parse_uri(uri)
                    .is_some_and(|(app, _)| self.steam.answered(app));
                if !answered {
                    transient.set(true);
                }
                found
            }
            Some("https" | "http") => {
                let downloaded = self
                    .download(uri, &transient)
                    .and_then(|b| self.store(&cached, &b));
                if downloaded.is_some() {
                    downloaded
                } else if let Some(app) = steam_app_from_icon_url(uri) {
                    let fallback_uri = format!("{STEAM_ART_SCHEME}://{app}/icon");
                    let fallback = self.locate(&fallback_uri);
                    if let Some(ref path) = fallback {
                        transient.set(true);
                        let _ = std::fs::copy(path, &cached);
                    }
                    fallback
                } else {
                    None
                }
            }
            _ => return None,
        };
        if found.is_none() && !transient.get() {
            let _ = write_cache_file(&miss, MISS_MARK);
        }
        found
    }

    /// Nome no cache: hash do endereço lógico + extensão (`.img` para esquemas internos).
    pub fn cache_path(&self, uri: &str) -> PathBuf {
        let name = &hex(&Sha256::digest(uri))[..32];
        let ext = match scheme_of(uri).as_deref() {
            Some("https" | "http") => uri
                .split(['?', '#'])
                .next()
                .and_then(|p| p.rsplit('/').next())
                .and_then(|f| f.rsplit_once('.'))
                .map(|(_, e)| e.to_ascii_lowercase())
                .filter(|e| {
                    (1..=5).contains(&e.len()) && e.bytes().all(|b| b.is_ascii_alphanumeric())
                })
                .unwrap_or_else(|| "img".into()),
            _ => "img".into(),
        };
        self.dir.join(format!("{name}.{ext}"))
    }

    /// Tamanho do cache no disco (Configurações › Capas e metadados).
    pub fn disk_bytes(&self) -> u64 {
        self.files().iter().map(|(_, size, _)| size).sum()
    }

    /// "Limpar cache de imagens": apaga os baixados; devolve os bytes liberados.
    pub fn clear(&self) -> u64 {
        // Arquivo em uso agora: fica para a próxima limpeza.
        self.files()
            .into_iter()
            .filter(|(p, _, _)| std::fs::remove_file(p).is_ok())
            .map(|(_, size, _)| size)
            .sum()
    }

    /// `transient` vira `true` em toda falha que não seja 404 ou 410: só essas marcam falta.
    fn download(&self, url: &str, transient: &Cell<bool>) -> Option<Vec<u8>> {
        let mut req = HttpRequest::get(url);
        req.max_bytes = MAX_IMAGE_BYTES;
        let r = match self.http.send(&req) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("Arte indisponível: {e}");
                transient.set(true);
                return None;
            }
        };
        let type_ok = r.content_type.as_deref().is_none_or(|c| {
            let c = c.to_ascii_lowercase();
            c.starts_with("image/") || c.starts_with("application/octet-stream")
        });
        let ok = r.is_success() && type_ok && looks_like_image(&r.body);
        // Só 404 e 410 dizem que a arte não existe; 401, 403, 5xx ou um 200 em HTML (portal de Wi-Fi) podem passar.
        if !ok && !matches!(r.status, 404 | 410) {
            transient.set(true);
        }
        ok.then_some(r.body)
    }

    fn store(&self, path: &Path, bytes: &[u8]) -> Option<PathBuf> {
        if let Err(e) = write_cache_file(path, bytes) {
            tracing::warn!("Não foi possível gravar a arte no cache: {e}");
            return None;
        }
        self.enforce_limit(Some(path));
        Some(path.to_path_buf())
    }

    /// Acima do limite, apaga os usados há mais tempo (a data de modificação marca o último uso).
    fn enforce_limit(&self, keep: Option<&Path>) {
        let _one = self.evicting.lock().unwrap_or_else(|e| e.into_inner());
        let mut files = self.files();
        let mut total: u64 = files.iter().map(|(_, s, _)| s).sum();
        let limit = self.limit();
        if total <= limit {
            return;
        }
        files.sort_by_key(|(_, _, used)| *used);
        for (p, size, _) in files {
            if total <= limit {
                break;
            }
            if Some(p.as_path()) != keep && std::fs::remove_file(&p).is_ok() {
                total -= size;
            }
        }
    }

    /// (caminho, tamanho, último uso) dos arquivos do cache.
    fn files(&self) -> Vec<(PathBuf, u64, SystemTime)> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .filter_map(|e| {
                let m = e.metadata().ok().filter(|m| m.is_file())?;
                Some((
                    e.path(),
                    m.len(),
                    m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                ))
            })
            .collect()
    }
}

// Marca o uso (a data de acesso do disco não é confiável: `noatime`, `relatime`).
/// Appid da Steam quando o endereço é de um ícone da comunidade (`community_assets/images/apps/<appid>/...`).
fn steam_app_from_icon_url(url: &str) -> Option<u32> {
    if !url.contains("steamstatic.com") {
        return None;
    }
    let (_, tail) = url.split_once("/images/apps/")?;
    let (app, _) = tail.split_once('/')?;
    app.parse().ok()
}

fn touch(path: &Path) {
    if let Ok(f) = std::fs::File::options().write(true).open(path) {
        let _ = f.set_modified(SystemTime::now());
    }
}

/// PNG, JPEG, GIF, WebP ou BMP pela assinatura: texto de erro com status 200 não entra no cache.
pub fn looks_like_image(b: &[u8]) -> bool {
    b.starts_with(&[0x89, b'P', b'N', b'G'])
        || b.starts_with(&[0xFF, 0xD8, 0xFF])
        || b.starts_with(b"GIF8")
        || (b.len() >= 12 && b.starts_with(b"RIFF") && &b[8..12] == b"WEBP")
        || b.starts_with(b"BM")
}

/// `file:///C:/a%20b.png` ou um caminho comum → caminho.
pub fn file_path(uri: &str) -> Option<PathBuf> {
    let Some(rest) = uri.strip_prefix("file://") else {
        return (!uri.trim().is_empty()).then(|| PathBuf::from(uri));
    };
    let decoded = percent_decode(rest)?;
    // "/C:/x" (Windows) → "C:/x".
    let b = decoded.as_bytes();
    let path = if b.len() > 2 && b[0] == b'/' && b[2] == b':' && b[1].is_ascii_alphabetic() {
        &decoded[1..]
    } else {
        &decoded
    };
    Some(PathBuf::from(path))
}

fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let byte = (b[i] == b'%' && i + 2 < b.len())
            .then(|| std::str::from_utf8(&b[i + 1..i + 3]).ok())
            .flatten()
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        if let Some(byte) = byte {
            out.push(byte);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::http::HttpResponse;
    use crate::metadata::test_support::FakeHttp;

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 0];

    fn png_http() -> Arc<FakeHttp> {
        let mut http = FakeHttp::with_fallback(404, "");
        http.rule = Some(Box::new(|req| {
            req.url.contains("/ok").then(|| HttpResponse {
                status: 200,
                body: PNG.repeat(100),
                content_type: Some("image/png".into()),
                retry_after: None,
            })
        }));
        Arc::new(http)
    }

    fn cache(dir: &Path, http: Arc<FakeHttp>) -> ArtCache {
        ArtCache::new(dir, http.clone(), SteamArtResolver::new(http, None), None)
    }

    #[test]
    fn art_the_store_does_not_have_is_not_asked_again_for_a_week() {
        let dir = tempfile::tempdir().unwrap();
        // Epic (https) com 404: a segunda abertura não vai à rede.
        let http = Arc::new(FakeHttp::with_fallback(404, ""));
        let epic = "https://cdn.epic/icone.png";
        assert!(cache(dir.path(), http.clone()).locate(epic).is_none());
        assert!(cache(dir.path(), http.clone()).locate(epic).is_none());
        assert_eq!(http.sent().len(), 1);
        // Steam: a API respondeu sem `community_icon`; noutra sessão, nem a API é consultada.
        let http = Arc::new(FakeHttp::with_fallback(
            200,
            r#"{"response":{"store_items":[{"assets":{}}]}}"#,
        ));
        let steam = "steam-art://570/icon";
        assert!(cache(dir.path(), http.clone()).locate(steam).is_none());
        assert!(cache(dir.path(), http.clone()).locate(steam).is_none());
        assert_eq!(http.sent().len(), 1);
        // Passados 7 dias, pergunta de novo.
        let c = cache(dir.path(), http.clone());
        let old = SystemTime::now() - std::time::Duration::from_secs(8 * 24 * 3600);
        std::fs::File::options()
            .write(true)
            .open(miss_of(&c, steam))
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert!(c.locate(steam).is_none());
        assert_eq!(http.sent().len(), 2);
        // Marcador das versões antigas (vazio) não vale: pergunta de novo.
        std::fs::write(miss_of(&c, steam), b"").unwrap();
        assert!(cache(dir.path(), http.clone()).locate(steam).is_none());
        assert_eq!(http.sent().len(), 3);
        // 410 também marca; o marcador vai ao lado do arquivo, com a extensão dele.
        let http = Arc::new(FakeHttp::with_fallback(410, ""));
        let gone = "https://cdn.epic/sumiu.png";
        let c = cache(dir.path(), http.clone());
        assert!(c.locate(gone).is_none());
        assert!(miss_of(&c, gone).to_string_lossy().ends_with(".png.miss"));
        assert!(miss_of(&c, gone).is_file());
        // O resto pode passar e não marca: 5xx, 401, 403, a API da Steam fora do ar e 200 que não é imagem
        // (portal de Wi-Fi em HTML, ou corpo que não passa em `looks_like_image`).
        for (status, body, kind, uri) in [
            (503, "", "image/png", "https://cdn.epic/outro.png"),
            (503, "", "image/png", "steam-art://10/icon"),
            (401, "", "image/png", "https://cdn.epic/a.png"),
            (403, "", "image/png", "https://cdn.epic/b.png"),
            (
                200,
                "<html>login</html>",
                "text/html",
                "https://cdn.epic/c.png",
            ),
            (200, "NOMEDIA", "image/png", "https://cdn.epic/d.png"),
        ] {
            let mut fake = FakeHttp::new();
            let kind = kind.to_owned();
            let body = body.as_bytes().to_vec();
            fake.rule = Some(Box::new(move |_| {
                Some(HttpResponse {
                    status,
                    body: body.clone(),
                    content_type: Some(kind.clone()),
                    retry_after: None,
                })
            }));
            let http = Arc::new(fake);
            let c = cache(dir.path(), http.clone());
            assert!(c.locate(uri).is_none());
            assert!(c.locate(uri).is_none());
            assert!(!miss_of(&c, uri).exists(), "{status} {uri}");
        }
    }

    fn miss_of(c: &ArtCache, uri: &str) -> PathBuf {
        PathBuf::from(format!("{}.miss", c.cache_path(uri).display()))
    }

    #[test]
    fn a_miss_mark_does_not_hide_the_local_steam_cache() {
        let dir = tempfile::tempdir().unwrap();
        let steam_root = dir.path().join("steam");
        let library = steam_root.join("appcache/librarycache");
        std::fs::create_dir_all(&library).unwrap();
        let http = Arc::new(FakeHttp::with_fallback(404, ""));
        let c = ArtCache::new(
            dir.path().join("cache"),
            http.clone(),
            SteamArtResolver::new(http.clone(), Some(steam_root)),
            None,
        );
        let uri = "steam-art://570/icon";
        // Marcado como sem ícone; depois a Steam baixou o dela.
        write_cache_file(&miss_of(&c, uri), b"").unwrap();
        std::fs::write(library.join("570_icon.jpg"), [0xFF, 0xD8, 0xFF]).unwrap();
        assert_eq!(c.locate(uri), Some(library.join("570_icon.jpg")));
        assert!(http.sent().is_empty());
    }

    #[test]
    fn downloads_once_and_reuses_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let http = png_http();
        let c = cache(dir.path(), http.clone());
        let p = c.locate("https://cdn.example/ok/capa.PNG?x=1").unwrap();
        assert_eq!(p.extension().unwrap(), "png");
        assert_eq!(c.locate("https://cdn.example/ok/capa.PNG?x=1"), Some(p));
        assert_eq!(http.sent().len(), 1);
        assert_eq!(c.disk_bytes(), 1200);
        assert_eq!(c.clear(), 1200);
        assert_eq!(c.disk_bytes(), 0);
    }

    #[test]
    fn rejects_text_answers_and_bad_content_types() {
        let dir = tempfile::tempdir().unwrap();
        let c = cache(
            dir.path(),
            Arc::new(FakeHttp::with_fallback(200, "NOMEDIA")),
        );
        assert!(c.locate("https://cdn.example/a.png").is_none());
        let mut html = FakeHttp::new();
        html.rule = Some(Box::new(|_| {
            Some(HttpResponse {
                status: 200,
                body: PNG.to_vec(),
                content_type: Some("text/html".into()),
                retry_after: None,
            })
        }));
        assert!(
            cache(dir.path(), Arc::new(html))
                .locate("https://cdn.example/b.png")
                .is_none()
        );
        assert_eq!(cache(dir.path(), png_http()).disk_bytes(), 0);
    }

    #[test]
    fn steam_art_tries_candidates_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let http = png_http();
        // API sem artes; a CDN antiga responde 404: não há arte.
        let c = cache(dir.path(), http.clone());
        assert!(c.locate("steam-art://570/cover").is_none());
        let mut good = FakeHttp::with_fallback(404, "");
        good.rule = Some(Box::new(|req| {
            req.url
                .ends_with("library_600x900_2x.jpg")
                .then(|| HttpResponse {
                    status: 200,
                    body: PNG.to_vec(),
                    content_type: Some("image/jpeg".into()),
                    retry_after: None,
                })
        }));
        let good = Arc::new(good);
        let c = cache(dir.path(), good.clone());
        let p = c.locate("steam-art://570/cover").unwrap();
        assert_eq!(p.extension().unwrap(), "img");
        let before = good.sent().len();
        assert_eq!(c.locate("steam-art://570/cover"), Some(p));
        assert_eq!(good.sent().len(), before);
    }

    #[test]
    fn evicts_the_least_recently_used_files_over_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let c = cache(dir.path(), png_http());
        let a = c.locate("https://x/ok/a.png").unwrap();
        let old = SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options()
            .write(true)
            .open(&a)
            .unwrap()
            .set_modified(old)
            .unwrap();
        let b = c.locate("https://x/ok/b.png").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&b)
            .unwrap()
            .set_modified(old)
            .unwrap();
        // Usar "a" de novo o torna o mais recente.
        assert_eq!(c.locate("https://x/ok/a.png"), Some(a.clone()));
        c.set_limit(2500);
        let third = c.locate("https://x/ok/c.png").unwrap();
        assert!(a.exists() && third.exists());
        assert!(!b.exists());
        assert!(c.disk_bytes() <= 2500);
        // Um arquivo só maior que o limite fica (é o que vai aparecer).
        c.set_limit(10);
        let d = c.locate("https://x/ok/d.png").unwrap();
        assert!(d.exists());
    }

    #[test]
    fn local_files_and_file_uris() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("minha capa.png");
        std::fs::write(&f, PNG).unwrap();
        let c = cache(dir.path(), png_http());
        assert_eq!(c.locate(&f.to_string_lossy()), Some(f.clone()));
        let uri = format!("file://{}", f.to_string_lossy().replace(' ', "%20"));
        assert_eq!(c.locate(&uri), Some(f));
        assert_eq!(
            file_path("file:///C:/capas/a.png"),
            Some(PathBuf::from("C:/capas/a.png"))
        );
        assert!(c.locate("file:///nao/existe.png").is_none());
        assert!(c.locate("ssmedia://neoclone.screenscraper.fr/x").is_none()); // sem cliente
        assert_eq!(file_path("file:///a%2"), Some(PathBuf::from("/a%2")));
    }

    #[test]
    fn steam_icon_falls_back_to_api_when_primary_cdn_url_fails() {
        let dir = tempfile::tempdir().unwrap();
        let primary = "https://shared.akamai.steamstatic.com/community_assets/images/apps/3669870/badhash.jpg";
        let valid_icon_url = "https://shared.akamai.steamstatic.com/community_assets/images/apps/3669870/f9f541b67c42a9503467f44072ef9193c135ff33.jpg";
        let mut http = FakeHttp::with_fallback(404, "");
        http.rule = Some(Box::new(move |req| {
            if req.url.contains("IStoreBrowseService/GetItems") {
                Some(HttpResponse {
                    status: 200,
                    body: r#"{"response":{"store_items":[{"assets":{"community_icon":"f9f541b67c42a9503467f44072ef9193c135ff33"}}]}}"#.as_bytes().to_vec(),
                    content_type: Some("application/json".into()),
                    retry_after: None,
                })
            } else if req.url == valid_icon_url {
                Some(HttpResponse {
                    status: 200,
                    body: PNG.to_vec(),
                    content_type: Some("image/jpeg".into()),
                    retry_after: None,
                })
            } else {
                None
            }
        }));
        let http = Arc::new(http);
        let c = cache(dir.path(), http.clone());

        let p = c
            .locate(primary)
            .expect("deve encontrar o ícone pelo fallback da API");
        assert!(p.is_file());
        assert!(
            !miss_of(&c, primary).exists(),
            "não pode deixar marcador .miss permanente quando o fallback funciona"
        );

        write_cache_file(&miss_of(&c, primary), MISS_MARK).unwrap();
        std::fs::remove_file(c.cache_path(primary)).ok();
        let c2 = cache(dir.path(), http.clone());
        let p2 = c2
            .locate(primary)
            .expect("marcador .miss antigo não pode bloquear o fallback");
        assert!(p2.is_file());
        assert!(
            !miss_of(&c2, primary).exists(),
            "marcador .miss stale deve ser removido após sucesso"
        );
    }
}
