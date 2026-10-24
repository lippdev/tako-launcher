//! Arquivos do pacote e arte já presente no cache do núcleo. Nenhuma URL vira caminho de arquivo.

use crate::schemes::{self, ImageKind};
use image::{ImageFormat, ImageReader};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tako_core::library::Game;
use tako_core::metadata::art_cache::{ArtCache, MAX_IMAGE_BYTES};
use tako_core::metadata::http::UreqHttp;
use tako_core::metadata::steam_art::SteamArtResolver;
use tako_core::paths::DataPaths;

const MAX_LIBRARY_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_APP_FILE_BYTES: u64 = 32 * 1024 * 1024;

#[cfg(all(windows, feature = "cef"))]
pub const CSP: &str = "default-src tako-app:; img-src tako-app: tako-img: data:; style-src tako-app: 'unsafe-inline'; script-src tako-app:; connect-src 'none'; object-src 'none'; base-uri 'none'; frame-src 'none'; frame-ancestors 'none'";

pub struct Resource {
    pub status: i32,
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}

impl Resource {
    pub fn not_found() -> Self {
        Self {
            status: 404,
            mime: "text/plain",
            bytes: Vec::new(),
        }
    }
}

pub struct Resources {
    pub app_root: PathBuf,
    cache_root: PathBuf,
    cache: ArtCache,
    games: Vec<Game>,
}

impl Resources {
    pub fn new(app_root: PathBuf, paths: &DataPaths) -> Self {
        let http = Arc::new(UreqHttp::new(std::time::Duration::from_secs(2)));
        let cache_root = paths.art_cache();
        let cache = ArtCache::new(
            &cache_root,
            http.clone(),
            SteamArtResolver::new(http, None),
            None,
        );
        // A resolução compartilhada de arte passa para o núcleo na WEB-01d.
        let games = read_limited(
            &paths.cache().join("found-games.json"),
            MAX_LIBRARY_SNAPSHOT_BYTES,
        )
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
        Self {
            app_root,
            cache_root,
            cache,
            games,
        }
    }

    /// Fixture optativa copiada para o cache; nunca se serve diretamente o caminho da variável.
    pub fn install_test_art(&self, path: &Path) -> Result<(), String> {
        let bytes = read_limited(path, MAX_IMAGE_BYTES)
            .ok_or("fixture de arte ausente ou grande demais")?;
        std::fs::create_dir_all(&self.cache_root).map_err(|e| e.to_string())?;
        std::fs::write(self.cache.cache_path("tako-test://cover"), bytes).map_err(|e| e.to_string())
    }

    pub fn serve(&self, url: &str) -> Resource {
        self.load(url).unwrap_or_else(Resource::not_found)
    }

    fn load(&self, url: &str) -> Option<Resource> {
        if let Some(rest) = url.strip_prefix("tako-app://app") {
            if !rest.is_empty() && !rest.starts_with(['/', '?', '#']) {
                return None;
            }
            let path = rest.split(['?', '#']).next()?;
            let file = schemes::resolve_app_route(&self.app_root, path)?;
            let mime = match file
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or_default()
            {
                "html" => "text/html; charset=utf-8",
                "js" | "mjs" => "text/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "json" => "application/json",
                "webmanifest" => "application/manifest+json",
                "gif" => "image/gif",
                "avif" => "image/avif",
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "svg" => "image/svg+xml",
                "webp" => "image/webp",
                "woff2" => "font/woff2",
                "woff" => "font/woff",
                "ttf" => "font/ttf",
                "ico" => "image/x-icon",
                _ => "application/octet-stream",
            };
            return Some(Resource {
                status: 200,
                mime,
                bytes: read_limited(&file, MAX_APP_FILE_BYTES)?,
            });
        }
        let request = schemes::parse_image_url(url)?;
        let uri = if cfg!(debug_assertions)
            && request.game_id == "test:cover"
            && request.kind == ImageKind::Cover
        {
            "tako-test://cover".to_owned()
        } else if let Some(game) = self.games.iter().find(|game| game.id == request.game_id) {
            match request.kind {
                ImageKind::Cover => game.shown_cover(),
                ImageKind::Hero => game.shown_hero(),
                ImageKind::Logo => game.shown_logo(),
                ImageKind::Icon => game.shown_icon(),
            }?
            .to_owned()
        } else {
            let id = request.game_id.strip_prefix("steam:")?;
            if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let kind = match request.kind {
                ImageKind::Cover => "cover",
                ImageKind::Hero => "hero",
                ImageKind::Logo => "logo",
                ImageKind::Icon => "icon",
            };
            format!("steam-art://{id}/{kind}")
        };
        // cache_path só calcula um hash. locate poderia baixar ou aceitar file:// fora do cache.
        let file = self.cache.cache_path(&uri).canonicalize().ok()?;
        if !file.starts_with(self.cache_root.canonicalize().ok()?) {
            return None;
        }
        let bytes = read_limited(&file, MAX_IMAGE_BYTES)?;
        let mut reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(16384);
        limits.max_image_height = Some(16384);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let image = reader.decode().ok()?;
        let width = request.width.unwrap_or(image.width());
        let height = request.height.unwrap_or(image.height());
        if u64::from(width) * u64::from(height) > 16 * 1024 * 1024 {
            return None;
        }
        let image = image.thumbnail(width, height);
        let mut output = Cursor::new(Vec::new());
        image.write_to(&mut output, ImageFormat::Png).ok()?;
        Some(Resource {
            status: 200,
            mime: "image/png",
            bytes: output.into_inner(),
        })
    }
}

fn read_limited(path: &Path, limit: u64) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= limit).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_mime_includes_utf8_and_modern_assets() {
        let dir = tempfile::tempdir().unwrap();
        let store = Resources::new(dir.path().into(), &DataPaths::at(dir.path()));
        for (extension, mime) in [
            ("html", "text/html; charset=utf-8"),
            ("js", "text/javascript; charset=utf-8"),
            ("css", "text/css; charset=utf-8"),
            ("gif", "image/gif"),
            ("avif", "image/avif"),
            ("webmanifest", "application/manifest+json"),
        ] {
            std::fs::write(dir.path().join(format!("asset.{extension}")), "á漢字").unwrap();
            let resource = store.serve(&format!("tako-app://app/asset.{extension}"));
            assert_eq!(resource.status, 200);
            assert_eq!(resource.mime, mime);
        }
    }

    #[test]
    fn app_authority_routes_and_traversal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "ok").unwrap();
        let store = Resources::new(dir.path().into(), &DataPaths::at(dir.path()));
        assert_eq!(store.serve("tako-app://app/").status, 200);
        assert_eq!(store.serve("tako-app://app/library/steam-570").status, 200);
        assert_eq!(
            store.serve("tako-app://app/library/steam%3A570").status,
            200
        );
        for url in [
            "tako-app://app.evil/",
            "tako-app://app@evil/",
            "tako-app://app/../index.html",
            "tako-app://app/%2e%2e/secret",
            "tako-app://app/missing.js",
            "tako-img://cover?id=../secret",
        ] {
            assert_eq!(store.serve(url).status, 404, "{url}");
        }
    }

    #[test]
    fn core_cache_image_is_resized_and_unknown_ids_are_not_paths() {
        let dir = tempfile::tempdir().unwrap();
        let store = Resources::new(dir.path().into(), &DataPaths::at(dir.path()));
        let image_path = dir.path().join("fixture.png");
        image::RgbaImage::from_pixel(60, 90, image::Rgba([40, 100, 180, 255]))
            .save(&image_path)
            .unwrap();
        store.install_test_art(&image_path).unwrap();
        let response = store.serve("tako-img://cover?id=test%3Acover&w=20&h=30");
        assert_eq!(response.status, 200);
        assert_eq!(response.mime, "image/png");
        let image = image::load_from_memory(&response.bytes).unwrap();
        assert_eq!((image.width(), image.height()), (20, 30));
        assert_eq!(
            store
                .serve("tako-img://cover?id=file%3A%2F%2Fsecret")
                .status,
            404
        );
        std::fs::write(store.cache.cache_path("tako-test://cover"), b"broken").unwrap();
        assert_eq!(store.serve("tako-img://cover?id=test%3Acover").status, 404);
    }
}
