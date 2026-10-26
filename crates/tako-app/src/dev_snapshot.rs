//! `tako dev-snapshot` (WEB-DEV-01): a biblioteca real gravada em arquivos para o hospedeiro falso do navegador.
//!
//! O designer roda `npm run dev` e vê os próprios jogos, capas e tempos de jogo. As respostas saem das mesmas
//! chamadas da ponte ([`AppService::handle`]), no formato dos tipos ts-rs, e as imagens são copiadas do cache.
//!
//! A pasta de dados é só lida: banco, metadados e perfis são copiados para uma pasta temporária e o núcleo abre a
//! cópia, com a plataforma em memória (cofre vazio, nada de tela, áudio ou motor de downloads de verdade). Assim
//! nenhum segredo chega ao processo, e as configurações passam pelo mesmo filtro da ponte (`settings.get`).

use crate::protocol::{
    Call, GameRequest, LibraryListRequest, MAX_PAGE_LIMIT, ProfileSessionsRequest, Reply,
    SessionPeriod, SettingsGetRequest,
};
use crate::{AppService, Core};
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use tako_core::library::{ArtKind, Game, GameStore};
use tako_core::metadata::art_cache::MAX_IMAGE_BYTES;
use tako_core::metadata::game_art::{GameArt, read_limited};
use tako_core::paths::{self, DataPaths};
use tako_platform::Platform;
use tako_sources::{LibraryScan, LibrarySource};

/// Versão do formato do `snapshot.json`; o hospedeiro falso ignora um arquivo de outra versão.
pub const FORMAT_VERSION: u32 = 1;
/// Nome do arquivo de dados dentro da pasta de saída.
pub const SNAPSHOT_FILE: &str = "snapshot.json";
/// Subpasta das imagens copiadas, apagada e refeita a cada execução.
pub const IMAGES_DIR: &str = "images";

/// O que foi gravado, para a linha de comando mostrar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotReport {
    pub source: PathBuf,
    pub out: PathBuf,
    pub games: usize,
    pub images: usize,
}

/// A pasta de dados do Tako desta máquina: a que o executável detecta, se já tem banco; senão a instalada
/// (`%LOCALAPPDATA%\Tako`, `~/.local/share/tako`…). `cargo run` detectaria a portable ao lado de `target/`.
pub fn default_paths() -> DataPaths {
    let detected = DataPaths::detect();
    if detected.database().is_file() {
        return detected;
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."));
    paths::resolve(
        paths::Os::current(),
        &exe_dir,
        |k| std::env::var_os(k).map(PathBuf::from),
        |p| p.file_name().is_some_and(|n| n == paths::INSTALLED_MARKER),
    )
}

/// Lê a biblioteca de `source` (o perfil em uso) e grava `snapshot.json` e `images/` em `out`.
pub fn write(source: &DataPaths, out: &Path, version: &str) -> Result<SnapshotReport, String> {
    let active = tako_core::profiles::Profiles::load(&source.machine).paths(source);
    if !active.database().is_file() {
        return Err(format!(
            "não há biblioteca em {} (falta {})",
            active.root.display(),
            active.database().display()
        ));
    }
    let stage = Stage::new()?;
    let staged = stage.copy_from(&active)?;
    // O núcleo relê as lojas ao abrir; no lugar delas vai a última leitura gravada, para nada sumir nem mudar.
    let replay: Vec<Box<dyn LibrarySource>> = vec![Box::new(Replay(last_scan(
        &staged.cache().join(FOUND_FILE),
    )))];
    let core = Core::open_with_downloads(
        staged,
        Platform::simulated(),
        false,
        replay,
        || Box::new(tako_core::downloads::fake::MemoryDownloader::default()),
        stage.dir.join("downloads"),
    )
    .map_err(|e| format!("não foi possível abrir a cópia da biblioteca: {e}"))?;
    let mut app = AppService::with_core(version, core);
    wait_for_scan(&mut app)?;
    let mut snapshot = collect(&mut app)?;
    // O núcleo solta o banco antes de a pasta temporária ser apagada.
    drop(app);

    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let images = copy_images(&active, &snapshot.games, out)?;
    let count = images
        .values()
        .map(|v| v.as_object().map_or(0, Map::len))
        .sum();
    snapshot.root.insert("images".into(), Value::Object(images));
    let text = serde_json::to_vec_pretty(&Value::Object(snapshot.root))
        .map_err(|e| format!("snapshot: {e}"))?;
    let file = out.join(SNAPSHOT_FILE);
    let tmp = out.join(format!("{SNAPSHOT_FILE}.tmp"));
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, &file))
        .map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(SnapshotReport {
        source: active.root,
        out: out.to_path_buf(),
        games: snapshot.games.len(),
        images: count,
    })
}

/// A última leitura das lojas, que o núcleo grava em `cache/`.
const FOUND_FILE: &str = "found-games.json";
/// O mesmo teto do núcleo para esse arquivo.
const MAX_FOUND_BYTES: u64 = 64 * 1024 * 1024;
/// Quanto esperar a leitura de abertura (só a releitura do arquivo e a medição de pastas sem tamanho).
const SCAN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

fn last_scan(path: &Path) -> Vec<Game> {
    read_limited(path, MAX_FOUND_BYTES)
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Fonte que devolve a última leitura gravada, sem consultar loja, registro ou disco.
struct Replay(Vec<Game>);

impl LibrarySource for Replay {
    fn store(&self) -> GameStore {
        GameStore::Manual
    }

    fn scan(&self) -> LibraryScan {
        LibraryScan {
            games: self.0.clone(),
            client_installed: true,
            notice: None,
        }
    }
}

fn wait_for_scan(app: &mut AppService) -> Result<(), String> {
    let started = std::time::Instant::now();
    loop {
        if let Ok(Reply::LibrarySources(state)) = app.handle(Call::LibrarySources)
            && !state.scanning
        {
            return Ok(());
        }
        if started.elapsed() > SCAN_TIMEOUT {
            return Err("a leitura da biblioteca não terminou".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

struct Collected {
    root: Map<String, Value>,
    /// Ids dos jogos, na ordem de `library`.
    games: Vec<String>,
}

/// O `data` da resposta, exatamente como a ponte o entrega à interface.
fn data(reply: Reply) -> Value {
    match serde_json::to_value(reply) {
        Ok(Value::Object(mut wire)) => wire.remove("data").unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

/// Uma consulta opcional: sem resposta, o campo fica `null` e o hospedeiro falso usa o exemplo dele.
fn optional(app: &mut AppService, call: Call) -> Value {
    let name = serde_json::to_value(&call)
        .ok()
        .and_then(|v| v.get("name").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default();
    match app.handle(call) {
        Ok(reply) => data(reply),
        Err(e) => {
            tracing::warn!("{name}: {}", e.message);
            Value::Null
        }
    }
}

fn collect(app: &mut AppService) -> Result<Collected, String> {
    let mut rows = Vec::new();
    let mut ids = Vec::new();
    loop {
        let request = LibraryListRequest {
            offset: u32::try_from(rows.len()).map_err(|e| e.to_string())?,
            limit: MAX_PAGE_LIMIT,
            ..Default::default()
        };
        let Reply::LibraryList(page) = app
            .handle(Call::LibraryList(request))
            .map_err(|e| format!("library.list: {}", e.message))?
        else {
            return Err("library.list: resposta inesperada".into());
        };
        let got = page.rows.len();
        for row in page.rows {
            ids.push(row.id.clone());
            rows.push(serde_json::to_value(row).map_err(|e| e.to_string())?);
        }
        if got == 0 || rows.len() >= page.total {
            break;
        }
    }
    let mut details = Map::new();
    for id in &ids {
        let value = optional(
            app,
            Call::GameDetails(GameRequest {
                game_id: id.clone(),
            }),
        );
        if !value.is_null() {
            details.insert(id.clone(), value);
        }
    }
    let sessions = |app: &mut AppService, period| {
        optional(
            app,
            Call::ProfileSessions(ProfileSessionsRequest { period }),
        )
    };
    let root = json!({
        "version": FORMAT_VERSION,
        "createdAt": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "appInfo": optional(app, Call::AppInfo),
        "games": rows,
        "details": details,
        "collections": optional(app, Call::CollectionsList),
        "tags": optional(app, Call::TagsList),
        // O mesmo filtro da ponte: só a lista positiva de chaves públicas, nunca o cofre.
        "settings": optional(app, Call::SettingsGet(SettingsGetRequest { keys: None })),
        "profileSummary": optional(app, Call::ProfileSummary),
        "profileSessions": {
            "week": sessions(app, SessionPeriod::Week),
            "month": sessions(app, SessionPeriod::Month),
            "all": sessions(app, SessionPeriod::All),
        },
        "profileCollections": optional(app, Call::ProfileCollections),
        "profiles": optional(app, Call::ProfilesList),
    });
    let Value::Object(root) = root else {
        unreachable!("json! de objeto");
    };
    Ok(Collected { root, games: ids })
}

const KINDS: [(ArtKind, &str); 4] = [
    (ArtKind::Cover, "cover"),
    (ArtKind::Hero, "hero"),
    (ArtKind::Logo, "logo"),
    (ArtKind::Icon, "icon"),
];

/// Extensão pela assinatura; o que não é imagem conhecida não é copiado.
fn image_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"BM") {
        Some("bmp")
    } else if bytes.starts_with(&[0, 0, 1, 0]) {
        Some("ico")
    } else {
        None
    }
}

/// Só os arquivos que este comando gravou (`g<n>-<tipo>.<ext>`): uma `--out` errada não perde nada alheio.
fn is_snapshot_image(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('g') else {
        return false;
    };
    let Some((index, rest)) = rest.split_once('-') else {
        return false;
    };
    let Some((kind, ext)) = rest.split_once('.') else {
        return false;
    };
    !index.is_empty()
        && index.bytes().all(|b| b.is_ascii_digit())
        && KINDS.iter().any(|(_, k)| *k == kind)
        && matches!(ext, "png" | "jpg" | "gif" | "webp" | "bmp" | "ico")
}

/// Copia a arte publicada de cada jogo (a mesma que `tako-img://` serviria), sem tocar no cache.
fn copy_images(
    active: &DataPaths,
    games: &[String],
    out: &Path,
) -> Result<Map<String, Value>, String> {
    let dir = out.join(IMAGES_DIR);
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_str().is_some_and(is_snapshot_image) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let art = GameArt::new(active);
    let mut images = Map::new();
    for (index, id) in games.iter().enumerate() {
        let mut kinds = Map::new();
        for (kind, key) in KINDS {
            let Some(bytes) = art
                .peek(id, kind)
                .and_then(|file| read_limited(&file, MAX_IMAGE_BYTES))
            else {
                continue;
            };
            let Some(ext) = image_extension(&bytes) else {
                continue;
            };
            let name = format!("g{index}-{key}.{ext}");
            std::fs::write(dir.join(&name), &bytes).map_err(|e| format!("{name}: {e}"))?;
            kinds.insert(key.into(), Value::String(format!("{IMAGES_DIR}/{name}")));
        }
        if !kinds.is_empty() {
            images.insert(id.clone(), Value::Object(kinds));
        }
    }
    Ok(images)
}

/// Pasta temporária com a cópia do que o núcleo lê; apagada no fim, com ou sem erro.
struct Stage {
    dir: PathBuf,
}

impl Stage {
    fn new() -> Result<Self, String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir =
            std::env::temp_dir().join(format!("tako-dev-snapshot-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Self { dir })
    }

    /// Copia banco, biblioteca antiga, última leitura das lojas, metadados e perfis; devolve as pastas da cópia,
    /// com o perfil em uso no mesmo lugar relativo.
    fn copy_from(&self, active: &DataPaths) -> Result<DataPaths, String> {
        let relative = active
            .root
            .strip_prefix(&active.machine)
            .unwrap_or(Path::new(""));
        let staged = DataPaths {
            kind: active.kind,
            root: self.dir.join(relative),
            machine: self.dir.clone(),
        };
        copy_if_exists(
            &tako_core::profiles::Profiles::file(&active.machine),
            &tako_core::profiles::Profiles::file(&staged.machine),
        )?;
        copy_if_exists(
            &active.cache().join(FOUND_FILE),
            &staged.cache().join(FOUND_FILE),
        )?;
        copy_tree(&active.metadata_cache(), &staged.metadata_cache())?;
        for name in ["tako.db", "tako.db-wal", "tako.db-shm", "library.json"] {
            copy_if_exists(&active.root.join(name), &staged.root.join(name))?;
        }
        Ok(staged)
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.dir) {
            tracing::warn!("pasta temporária {} ficou: {e}", self.dir.display());
        }
    }
}

fn copy_if_exists(from: &Path, to: &Path) -> Result<(), String> {
    if !from.is_file() {
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("{}: {e}", from.display()))
}

/// Cópia recursiva só de arquivos e pastas comuns (links ficam de fora).
fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            copy_if_exists(&entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::persistence::{SettingsStore, TakoDatabase};

    #[test]
    fn snapshot_images_are_recognized_by_name() {
        assert!(is_snapshot_image("g0-cover.png"));
        assert!(is_snapshot_image("g12-icon.ico"));
        assert!(!is_snapshot_image("g-cover.png"));
        assert!(!is_snapshot_image("g1-banner.png"));
        assert!(!is_snapshot_image("g1-cover.exe"));
        assert!(!is_snapshot_image("notes.txt"));
    }

    #[test]
    fn image_extension_comes_from_the_signature() {
        assert_eq!(image_extension(b"\x89PNG\r\n\x1a\n"), Some("png"));
        assert_eq!(image_extension(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(image_extension(&[0, 0, 1, 0, 1, 0]), Some("ico"));
        assert_eq!(image_extension(b"<html>"), None);
    }

    /// A biblioteca vira o JSON da ponte; segredos do banco não saem e a pasta de dados fica como estava.
    #[test]
    fn writes_library_without_secrets_and_leaves_source_untouched() {
        let data = tempfile::tempdir().unwrap();
        let source = DataPaths::at(data.path());
        {
            let core =
                Core::open_with_sources(source.clone(), Platform::simulated(), false, Vec::new())
                    .unwrap();
            let mut app = AppService::with_core("test", core);
            app.reload_library(vec![tako_core::library::Game {
                id: "steam:570".into(),
                title: "Dota 2".into(),
                is_installed: true,
                ..Default::default()
            }])
            .unwrap();
        }
        let db = TakoDatabase::new(source.database());
        db.open().unwrap();
        let settings = SettingsStore::new(db);
        settings.set("ReducedMotion", "true").unwrap();
        settings.set("SteamGridDbApiKey", "segredo-123").unwrap();
        settings
            .set("metadata.igdb", "{\"client_secret\":\"x\"}")
            .unwrap();
        drop(settings);
        std::fs::create_dir_all(source.art_cache()).unwrap();
        let cover = source.art_cache().join("cover.img");
        std::fs::write(&cover, b"\x89PNG\r\n\x1a\nimagem").unwrap();
        GameArt::new(&source)
            .publish("steam:570", ArtKind::Cover, &cover)
            .unwrap();
        let before = listing(data.path());

        let out = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(out.path().join(IMAGES_DIR)).unwrap();
        std::fs::write(out.path().join(IMAGES_DIR).join("g9-cover.png"), b"old").unwrap();
        std::fs::write(out.path().join(IMAGES_DIR).join("alheio.txt"), b"x").unwrap();
        let report = write(&source, out.path(), "test").unwrap();

        assert_eq!(report.games, 1);
        let text = std::fs::read_to_string(out.path().join(SNAPSHOT_FILE)).unwrap();
        assert!(!text.contains("segredo-123"));
        assert!(!text.contains("SteamGridDbApiKey"));
        assert!(!text.contains("client_secret"));
        let json: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["version"], FORMAT_VERSION);
        assert_eq!(json["games"][0]["id"], "steam:570");
        assert_eq!(json["games"][0]["title"], "Dota 2");
        assert_eq!(json["details"]["steam:570"]["canPlay"], true);
        assert_eq!(json["settings"]["values"]["ReducedMotion"], "true");
        assert!(json["profileSessions"]["all"].is_object());
        assert_eq!(report.images, 1);
        assert_eq!(json["images"]["steam:570"]["cover"], "images/g0-cover.png");
        assert_eq!(
            std::fs::read(out.path().join("images/g0-cover.png")).unwrap(),
            b"\x89PNG\r\n\x1a\nimagem"
        );
        assert!(!out.path().join(IMAGES_DIR).join("g9-cover.png").exists());
        assert!(out.path().join(IMAGES_DIR).join("alheio.txt").exists());
        assert_eq!(listing(data.path()), before);
    }

    #[test]
    fn missing_library_is_an_error() {
        let data = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        assert!(write(&DataPaths::at(data.path()), out.path(), "test").is_err());
        assert!(!out.path().join(SNAPSHOT_FILE).exists());
    }

    /// Nome, tamanho e data de cada arquivo, para provar que nada mudou.
    fn listing(dir: &Path) -> Vec<(PathBuf, u64, std::time::SystemTime)> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d).unwrap().flatten() {
                let meta = entry.metadata().unwrap();
                if meta.is_dir() {
                    stack.push(entry.path());
                } else {
                    out.push((entry.path(), meta.len(), meta.modified().unwrap()));
                }
            }
        }
        out.sort();
        out
    }
}
