//! Arquivos do pacote e arte já presente no cache do núcleo. Nenhuma URL vira caminho de arquivo.

use crate::schemes::{self, ImageKind};
use image::{ImageFormat, ImageReader};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use tako_core::library::ArtKind;
use tako_core::metadata::art_cache::MAX_IMAGE_BYTES;
use tako_core::metadata::game_art::GameArt;
use tako_core::paths::DataPaths;

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
    art: GameArt,
}

impl Resources {
    pub fn new(app_root: PathBuf, paths: &DataPaths) -> Self {
        Self {
            app_root,
            cache_root: paths.art_cache(),
            art: GameArt::new(paths),
        }
    }

    /// Fixture optativa copiada para o cache; nunca se serve diretamente o caminho da variável.
    pub fn install_test_art(&self, path: &Path) -> Result<(), String> {
        let bytes = read_limited(path, MAX_IMAGE_BYTES)
            .ok_or("fixture de arte ausente ou grande demais")?;
        std::fs::create_dir_all(&self.cache_root).map_err(|e| e.to_string())?;
        let file = self.cache_root.join("test-cover.img");
        std::fs::write(&file, bytes).map_err(|e| e.to_string())?;
        self.art
            .publish("test:cover", ArtKind::Cover, &file)
            .map(|_| ())
            .map_err(|e| e.to_string())
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
        let kind = match request.kind {
            ImageKind::Cover => ArtKind::Cover,
            ImageKind::Hero => ArtKind::Hero,
            ImageKind::Logo => ArtKind::Logo,
            ImageKind::Icon => ArtKind::Icon,
        };
        let file = self.art.resolve(&request.game_id, kind)?;
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
        std::fs::write(store.cache_root.join("test-cover.img"), b"broken").unwrap();
        assert_eq!(store.serve("tako-img://cover?id=test%3Acover").status, 404);
    }

    #[test]
    fn running_host_reads_new_core_art_without_reopening_or_library_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let store = Resources::new(dir.path().into(), &paths);
        let url = "tako-img://cover?id=gog%3Anew&v=1&w=20&h=30";
        assert_eq!(store.serve(url).status, 404);
        std::fs::create_dir_all(paths.art_cache()).unwrap();
        let file = paths.art_cache().join("new.png");
        image::RgbaImage::from_pixel(40, 60, image::Rgba([1, 2, 3, 255]))
            .save(&file)
            .unwrap();
        GameArt::new(&paths)
            .publish("gog:new", ArtKind::Cover, &file)
            .unwrap();
        assert_eq!(store.serve(url).status, 200);
        assert!(!paths.cache().join("found-games.json").exists());
    }

    #[test]
    fn imported_local_ico_is_decoded_to_png() {
        use std::sync::Arc;
        use tako_core::metadata::http::{Http, HttpRequest, HttpResponse};
        struct Offline;
        impl Http for Offline {
            fn send(&self, _: &HttpRequest) -> Result<HttpResponse, String> {
                panic!("ícone local não deve usar rede");
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let store = Resources::new(dir.path().into(), &paths);
        let mut bytes = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(32, 32, image::Rgba([30, 100, 180, 255]))
            .write_to(&mut bytes, ImageFormat::Ico)
            .unwrap();
        let http = Arc::new(Offline);
        let cache = tako_core::metadata::art_cache::ArtCache::new(
            paths.art_cache(),
            http.clone(),
            tako_core::metadata::steam_art::SteamArtResolver::new(http, None),
            None,
        );
        let file = cache
            .import("file:///steam/icon.ico", bytes.get_ref())
            .unwrap();
        GameArt::new(&paths)
            .publish("steam:1", ArtKind::Icon, &file)
            .unwrap();
        let resource = store.serve("tako-img://icon?id=steam%3A1&w=16&h=16&v=2");
        assert_eq!(resource.status, 200);
        assert_eq!(resource.mime, "image/png");
        let image = image::load_from_memory(&resource.bytes).unwrap();
        assert_eq!((image.width(), image.height()), (16, 16));
    }
}
