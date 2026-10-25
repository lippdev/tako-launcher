//! Arquivos do pacote e arte já presente no cache do núcleo. Nenhuma URL vira caminho de arquivo.

use crate::schemes::{self, ImageKind};
use image::{DynamicImage, ImageFormat, ImageReader};
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
        let image = decode_image(&bytes)?;
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

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

fn parse_png_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() >= 24 && data.starts_with(PNG_SIGNATURE) && &data[12..16] == b"IHDR" {
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        if width > 0 && height > 0 {
            return Some((width, height));
        }
    }
    None
}

/// Decodifica imagem com os limites seguros do Tako; se falhar, tenta leitura tolerante de ICO.
fn decode_image(bytes: &[u8]) -> Option<DynamicImage> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);

    if let Ok(mut reader) = ImageReader::new(Cursor::new(bytes)).with_guessed_format() {
        reader.limits(limits.clone());
        if let Ok(image) = reader.decode() {
            return Some(image);
        }
    }

    decode_ico_tolerant(bytes, &limits)
}

/// Decodificador tolerante para ICOs onde a decodificação normal falha (ex.: PNG 512x512
/// declarado como 256x256 e sem canal alfa em jogos Steam, WEB-71).
fn decode_ico_tolerant(bytes: &[u8], limits: &image::Limits) -> Option<DynamicImage> {
    if bytes.len() < 6 {
        return None;
    }
    let reserved = u16::from_le_bytes([bytes[0], bytes[1]]);
    let res_type = u16::from_le_bytes([bytes[2], bytes[3]]);
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;

    if reserved != 0 || (res_type != 1 && res_type != 2) || count == 0 {
        return None;
    }

    let dir_end = 6usize.checked_add(count.checked_mul(16)?)?;
    if bytes.len() < dir_end {
        return None;
    }

    struct Candidate {
        entry_header: [u8; 16],
        offset: usize,
        size: usize,
        width: u32,
        height: u32,
        bpp: u16,
        is_png: bool,
    }

    let mut candidates = Vec::with_capacity(count);

    for i in 0..count {
        let base = 6 + i * 16;
        let mut entry_header = [0u8; 16];
        entry_header.copy_from_slice(&bytes[base..base + 16]);

        let w_byte = entry_header[0];
        let h_byte = entry_header[1];
        let declared_width = if w_byte == 0 { 256 } else { u32::from(w_byte) };
        let declared_height = if h_byte == 0 { 256 } else { u32::from(h_byte) };
        let bpp = u16::from_le_bytes([entry_header[6], entry_header[7]]);
        let size = u32::from_le_bytes([
            entry_header[8],
            entry_header[9],
            entry_header[10],
            entry_header[11],
        ]) as usize;
        let offset = u32::from_le_bytes([
            entry_header[12],
            entry_header[13],
            entry_header[14],
            entry_header[15],
        ]) as usize;

        let end = match offset.checked_add(size) {
            Some(e) => e,
            None => continue,
        };
        if end > bytes.len() || size == 0 {
            continue;
        }

        let entry_data = &bytes[offset..end];
        let is_png = entry_data.starts_with(PNG_SIGNATURE);
        let (width, height) = if is_png {
            parse_png_dimensions(entry_data).unwrap_or((declared_width, declared_height))
        } else {
            (declared_width, declared_height)
        };

        candidates.push(Candidate {
            entry_header,
            offset,
            size,
            width,
            height,
            bpp,
            is_png,
        });
    }

    // Prefere a maior entrada válida (área maior, desempate por bpp e formato PNG)
    candidates.sort_by(|a, b| {
        let area_a = u64::from(a.width) * u64::from(a.height);
        let area_b = u64::from(b.width) * u64::from(b.height);
        area_b
            .cmp(&area_a)
            .then_with(|| b.bpp.cmp(&a.bpp))
            .then_with(|| b.is_png.cmp(&a.is_png))
    });

    for cand in candidates {
        let entry_data = &bytes[cand.offset..cand.offset + cand.size];
        if cand.is_png {
            // PNG decodificado diretamente pelo decodificador de PNG (com suas dimensões reais)
            let mut reader = ImageReader::new(Cursor::new(entry_data));
            reader.set_format(ImageFormat::Png);
            reader.limits(limits.clone());
            if let Ok(img) = reader.decode() {
                return Some(img);
            }
        } else {
            // BMP pelo caminho que já existe (IcoDecoder em entrada única sintetizada)
            let mut single_ico = Vec::with_capacity(22 + cand.size);
            single_ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
            let mut header = cand.entry_header;
            header[12..16].copy_from_slice(&22u32.to_le_bytes());
            single_ico.extend_from_slice(&header);
            single_ico.extend_from_slice(entry_data);

            let mut reader = ImageReader::new(Cursor::new(single_ico));
            reader.set_format(ImageFormat::Ico);
            reader.limits(limits.clone());
            if let Ok(img) = reader.decode() {
                return Some(img);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tako_core::metadata::http::{Http, HttpRequest, HttpResponse};

    struct Offline;
    impl Http for Offline {
        fn send(&self, _: &HttpRequest) -> Result<HttpResponse, String> {
            panic!("ícone local não deve usar rede");
        }
    }

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

    #[test]
    fn tolerant_ico_decodes_oversized_png_entry_and_ignores_dimension_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::at(dir.path());
        let store = Resources::new(dir.path().into(), &paths);

        // Gera PNG de 512x512 em RGB (como The Witcher 3 e CONTROL na Steam, sem canal alfa)
        let mut png_512 = Cursor::new(Vec::new());
        image::RgbImage::from_pixel(512, 512, image::Rgb([120, 40, 200]))
            .write_to(&mut png_512, ImageFormat::Png)
            .unwrap();
        let png_512_bytes = png_512.into_inner();

        // Gera entrada pequena válida (16x16)
        let mut small_png = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(16, 16, image::Rgba([20, 180, 50, 255]))
            .write_to(&mut small_png, ImageFormat::Png)
            .unwrap();
        let small_bytes = small_png.into_inner();

        // Monta o arquivo ICO com 2 entradas:
        // Entrada 0: 16x16 PNG válida
        // Entrada 1: declarada 256x256 (byte 0 no cabeçalho), com o PNG real de 512x512
        let mut ico = Vec::new();
        // Cabeçalho ICO (6 bytes): reservado (0), tipo (1), contagem (2)
        ico.extend_from_slice(&[0, 0, 1, 0, 2, 0]);

        let offset0 = 6 + 2 * 16;
        let offset1 = offset0 + small_bytes.len();

        // Entrada 0 (16x16)
        ico.push(16); // largura
        ico.push(16); // altura
        ico.push(0); // cores
        ico.push(0); // reservado
        ico.extend_from_slice(&1u16.to_le_bytes()); // planos
        ico.extend_from_slice(&32u16.to_le_bytes()); // bpp
        ico.extend_from_slice(&(small_bytes.len() as u32).to_le_bytes()); // tamanho
        ico.extend_from_slice(&(offset0 as u32).to_le_bytes()); // offset

        // Entrada 1 (declarada 256x256 via 0, 0, mas com o PNG de 512x512)
        ico.push(0); // 0 = 256
        ico.push(0); // 0 = 256
        ico.push(0); // cores
        ico.push(0); // reservado
        ico.extend_from_slice(&1u16.to_le_bytes()); // planos
        ico.extend_from_slice(&32u16.to_le_bytes()); // bpp
        ico.extend_from_slice(&(png_512_bytes.len() as u32).to_le_bytes()); // tamanho
        ico.extend_from_slice(&(offset1 as u32).to_le_bytes()); // offset

        // Dados
        ico.extend_from_slice(&small_bytes);
        ico.extend_from_slice(&png_512_bytes);

        // Confere que a decodificação padrão do crate `image` recusa o arquivo
        let std_decode = image::ImageReader::new(Cursor::new(&ico))
            .with_guessed_format()
            .unwrap()
            .decode();
        assert!(
            std_decode.is_err(),
            "decodificador padrão do crate image devia recusar o ICO incompatível"
        );

        // Confere que a decodificação tolerante decodifica com sucesso e prefere a maior entrada (512x512)
        let decoded = decode_image(&ico).expect("decodificação tolerante deve funcionar");
        assert_eq!((decoded.width(), decoded.height()), (512, 512));

        // Confere servindo pela infraestrutura de recursos e redimensionando para 32x32
        let cache = tako_core::metadata::art_cache::ArtCache::new(
            paths.art_cache(),
            Arc::new(Offline),
            tako_core::metadata::steam_art::SteamArtResolver::new(Arc::new(Offline), None),
            None,
        );
        let file = cache.import("file:///steam/mismatched.ico", &ico).unwrap();
        GameArt::new(&paths)
            .publish("steam:mismatched", ArtKind::Icon, &file)
            .unwrap();

        let resource = store.serve("tako-img://icon?id=steam%3Amismatched&w=32&h=32");
        assert_eq!(resource.status, 200);
        assert_eq!(resource.mime, "image/png");
        let result_img = image::load_from_memory(&resource.bytes).unwrap();
        assert_eq!((result_img.width(), result_img.height()), (32, 32));
    }

    #[test]
    fn tolerant_ico_falls_back_to_valid_bmp_entry() {
        let mut bmp_ico = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(32, 32, image::Rgba([10, 80, 160, 255]))
            .write_to(&mut bmp_ico, ImageFormat::Ico)
            .unwrap();
        let bmp_ico_bytes = bmp_ico.into_inner();

        assert!(bmp_ico_bytes.len() > 22);
        let bmp_data = &bmp_ico_bytes[22..];
        let entry_header = &bmp_ico_bytes[6..22];

        // Monta um ICO com 2 entradas:
        // Entrada 0: PNG corrompido de 256x256
        // Entrada 1: BMP válido de 32x32
        let mut ico = Vec::new();
        ico.extend_from_slice(&[0, 0, 1, 0, 2, 0]);

        let offset0 = 6 + 2 * 16;
        let corrupt_png = b"\x89PNG\r\n\x1a\ncorrupt_data_here!";
        let offset1 = offset0 + corrupt_png.len();

        // Entrada 0: declarada 256x256 (0, 0), mas PNG corrompido
        ico.push(0);
        ico.push(0);
        ico.push(0);
        ico.push(0);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(corrupt_png.len() as u32).to_le_bytes());
        ico.extend_from_slice(&(offset0 as u32).to_le_bytes());

        // Entrada 1: BMP 32x32 válido
        ico.extend_from_slice(&entry_header[0..12]);
        ico.extend_from_slice(&(offset1 as u32).to_le_bytes());

        ico.extend_from_slice(corrupt_png);
        ico.extend_from_slice(bmp_data);

        // A decodificação padrão falha por causa da entrada PNG corrompida selecionada como melhor
        assert!(image::load_from_memory(&ico).is_err());

        // A decodificação tolerante ignora o PNG corrompido e decodifica o BMP
        let decoded = decode_image(&ico).expect("deve fazer fallback para o BMP");
        assert_eq!((decoded.width(), decoded.height()), (32, 32));
    }

    #[test]
    fn tolerant_ico_truncated_and_garbage_returns_none_never_panics() {
        let limits = image::Limits::default();

        // Buffer vazio
        assert!(decode_image(&[]).is_none());
        assert!(decode_ico_tolerant(&[], &limits).is_none());

        // Menor que o cabeçalho de 6 bytes
        assert!(decode_image(&[0, 0, 1, 0]).is_none());
        assert!(decode_ico_tolerant(&[0, 0, 1, 0], &limits).is_none());

        // Tipo de recurso inválido
        assert!(decode_ico_tolerant(&[1, 0, 0, 0, 1, 0], &limits).is_none());
        assert!(decode_ico_tolerant(&[0, 0, 99, 0, 1, 0], &limits).is_none());

        // Contagem declara entradas mas buffer termina antes do fim do diretório
        assert!(decode_ico_tolerant(&[0, 0, 1, 0, 10, 0], &limits).is_none());

        // Diretório com offset além dos limites do buffer
        let mut truncated_entry = Vec::new();
        truncated_entry.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
        truncated_entry.extend_from_slice(&[16, 16, 0, 0, 1, 0, 32, 0]);
        truncated_entry.extend_from_slice(&100u32.to_le_bytes());
        truncated_entry.extend_from_slice(&500u32.to_le_bytes());
        assert!(decode_ico_tolerant(&truncated_entry, &limits).is_none());

        // Offset com overflow
        let mut overflow_entry = Vec::new();
        overflow_entry.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
        overflow_entry.extend_from_slice(&[16, 16, 0, 0, 1, 0, 32, 0]);
        overflow_entry.extend_from_slice(&u32::MAX.to_le_bytes());
        overflow_entry.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_ico_tolerant(&overflow_entry, &limits).is_none());

        // Dados completamente aleatórios / lixo
        let garbage = [0xDE, 0xAD, 0xBE, 0xEF].repeat(64);
        assert!(decode_image(&garbage).is_none());
        assert!(decode_ico_tolerant(&garbage, &limits).is_none());

        // Entrada PNG com assinatura válida mas payload corrompido
        let mut corrupt_png_ico = Vec::new();
        corrupt_png_ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
        corrupt_png_ico.extend_from_slice(&[0, 0, 0, 0, 1, 0, 32, 0]);
        corrupt_png_ico.extend_from_slice(&20u32.to_le_bytes());
        corrupt_png_ico.extend_from_slice(&22u32.to_le_bytes());
        corrupt_png_ico.extend_from_slice(b"\x89PNG\r\n\x1a\ncorrompido!!");
        assert!(decode_image(&corrupt_png_ico).is_none());
        assert!(decode_ico_tolerant(&corrupt_png_ico, &limits).is_none());
    }
}
