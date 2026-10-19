//! itch.io (Linux, SRC-002): jogos instalados lidos de `butler.db` do app oficial sem o itch aberto.
//!
//! O itch.io desktop utiliza o `butler` / `butlerd` como daemon e banco local SQLite em:
//! `~/.config/itch/db/butler.db` (nativo) ou `~/.var/app/io.itch.itch/config/itch/db/butler.db` (Flatpak).
//!
//! O esquema real é definido nos modelos do butler (github.com/itchio/butler, database/models/cave.go e install_location.go):
//! - `caves`: instâncias de jogos instalados (id, game_id, install_folder_name, install_location_id, custom_install_folder, verdict, installed_size, seconds_run, last_touched_at).
//! - `games`: metadados da loja (id, title, cover_url, still_cover_url, url).
//! - `install_locations`: locais base de instalação (id, path).
//!
//! ### Formas de abertura e pesquisa de suporte (SRC-002)
//! 1. **Executável instalado no disco:**
//!    - Manifesto explícito `.itch.toml` na raiz da pasta do jogo (ações de plataforma Linux ou universais).
//!    - Candidatos reportados pelo butler em `caves.verdict` (dash.Verdict), utilizado somente quando houver
//!      exatamente um candidato com sabor Linux.
//!
//!    Abrir o executável diretamente permite ao Tako acompanhar o processo (`GameTrackingStrategy::Process`),
//!    contar o tempo de sessão e fechar limpo sem intermediários.
//! 2. **Protocolo `itch://`:** Sintaxe oficial do app itch (github.com/itchio/itch e itchio/itch-setup, itch.io app docs):
//!    `itch://install?game_id=<id>&launch`. O app oficial abre e solicita confirmação do usuário antes da execução.
//!    Utilizado quando não há binário Linux único identificado, para jogos HTML5 ou quando o executável não é encontrado.

use crate::{LibraryScan, LibrarySource, read_limited};
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore, GameTrackingStrategy};

#[cfg(test)]
mod tests;

const ACCENT: &str = "#FA5C5C";

/// Interpreta o carimbo de data/hora retornado pelo SQLite (pode ser número inteiro ou texto ISO).
fn parse_timestamp(val: Option<rusqlite::types::Value>) -> Option<DateTime<Utc>> {
    match val? {
        rusqlite::types::Value::Integer(secs) if secs > 0 => DateTime::from_timestamp(secs, 0),
        rusqlite::types::Value::Text(s) => {
            let s = s.trim();
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|| {
                    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                        .ok()
                        .map(|ndt| ndt.and_utc())
                })
                .or_else(|| {
                    s.parse::<i64>()
                        .ok()
                        .filter(|&secs| secs > 0)
                        .and_then(|secs| DateTime::from_timestamp(secs, 0))
                })
        }
        _ => None,
    }
}

enum ManifestActionPath {
    Executable(String),
    Url,
}

struct ManifestAction {
    path: Option<ManifestActionPath>,
    platform: Option<String>,
}

/// Parser seguro para manifestos `.itch.toml` da especificação do itch.io.
/// Casa exatamente `path =` e `platform =`, suportando múltiplos blocos `[[actions]]`.
fn parse_itch_manifest_actions(text: &str) -> Vec<ManifestAction> {
    let mut actions = Vec::new();
    let mut current_action: Option<ManifestAction> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed == "[[actions]]" {
            if let Some(action) = current_action.take() {
                actions.push(action);
            }
            current_action = Some(ManifestAction {
                path: None,
                platform: None,
            });
            continue;
        }

        let action = current_action.get_or_insert(ManifestAction {
            path: None,
            platform: None,
        });

        // Casamento exato de path = ...
        if let Some(rest) = trimmed.strip_prefix("path") {
            let rest = rest.trim_start();
            if let Some(val) = rest.strip_prefix('=') {
                let raw_val = val.trim().trim_matches('"').trim_matches('\'').trim();
                if raw_val.starts_with("http://")
                    || raw_val.starts_with("https://")
                    || raw_val.starts_with("itch:")
                    || raw_val.contains("://")
                {
                    action.path = Some(ManifestActionPath::Url);
                } else {
                    action.path = Some(ManifestActionPath::Executable(raw_val.to_owned()));
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("platform") {
            let rest = rest.trim_start();
            if let Some(val) = rest.strip_prefix('=') {
                let p = val
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_ascii_lowercase();
                action.platform = Some(p);
            }
        }
    }

    if let Some(action) = current_action {
        actions.push(action);
    }

    actions
}

/// Avalia o `.itch.toml` na pasta do jogo.
/// Retorna `Some(Some(path))` quando encontra executável Linux válido.
/// Retorna `Some(None)` quando encontra ação Linux que é URL (HTML5) ou com caminho inválido (deve cair no protocolo).
/// Retorna `None` quando o manifesto não existe ou não define ação Linux.
fn resolve_manifest_executable(game_dir: &Path) -> Option<Option<PathBuf>> {
    let manifest = game_dir.join(".itch.toml");
    if !manifest.is_file() {
        return None;
    }
    let Ok(bytes) = read_limited(&manifest, 64 * 1024) else {
        return None;
    };
    let text = String::from_utf8_lossy(&bytes);
    let actions = parse_itch_manifest_actions(&text);

    // Prioriza ação explícita para Linux, ou ação genérica sem plataforma declarada
    let action = actions
        .iter()
        .find(|a| a.platform.as_deref() == Some("linux"))
        .or_else(|| actions.iter().find(|a| a.platform.is_none()))?;

    match &action.path {
        Some(ManifestActionPath::Url) => {
            // URL (HTML5): cai no protocolo
            Some(None)
        }
        Some(ManifestActionPath::Executable(path_str)) => {
            // Recusa caminho absoluto ou subida de diretório ".."
            if path_str.starts_with('/')
                || Path::new(path_str).is_absolute()
                || path_str.contains("..")
            {
                return Some(None);
            }

            let rel_path = path_str
                .split('/')
                .fold(PathBuf::new(), |acc, part| acc.join(part));
            let candidate = game_dir.join(&rel_path);

            // Canonicaliza e exige starts_with(game_dir)
            let (Ok(canonical_game_dir), Ok(canonical_target)) =
                (game_dir.canonicalize(), candidate.canonicalize())
            else {
                return Some(None);
            };

            if !canonical_target.starts_with(&canonical_game_dir) || !canonical_target.is_file() {
                return Some(None);
            }

            Some(Some(candidate))
        }
        None => None,
    }
}

#[derive(serde::Deserialize)]
struct DashVerdict {
    #[serde(rename = "basePath", default)]
    base_path: Option<String>,
    #[serde(default)]
    candidates: Vec<DashCandidate>,
}

#[derive(serde::Deserialize)]
struct DashCandidate {
    #[serde(default)]
    path: String,
    #[serde(default)]
    flavor: Option<String>,
    #[serde(rename = "linuxInfo")]
    linux_info: Option<serde_json::Value>,
}

/// Avalia os `candidates` do JSON em `caves.verdict`.
/// Usa apenas quando houver exatamente um candidato Linux.
fn resolve_verdict_executable(game_dir: &Path, verdict_str: &str) -> Option<PathBuf> {
    let verdict: DashVerdict = serde_json::from_str(verdict_str).ok()?;
    let linux_candidates: Vec<&DashCandidate> = verdict
        .candidates
        .iter()
        .filter(|c| c.flavor.as_deref() == Some("linux") || c.linux_info.is_some())
        .collect();

    if linux_candidates.len() != 1 {
        return None;
    }

    let candidate = linux_candidates[0];
    let rel_path = candidate
        .path
        .split('/')
        .fold(PathBuf::new(), |acc, part| acc.join(part));

    let base = verdict
        .base_path
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| game_dir.to_path_buf());

    let target = base.join(&rel_path);
    if target.is_file() { Some(target) } else { None }
}

/// Tenta encontrar o executável principal dentro da pasta do jogo no itch.io.
/// Consulta primeiro o `.itch.toml` e, em seguida, os `candidates` do `verdict` do butler.
pub fn find_executable(game_dir: &Path, verdict_json: Option<&str>) -> Option<PathBuf> {
    if !game_dir.is_dir() {
        return None;
    }

    // 1. Manifest `.itch.toml` explícito
    if let Some(manifest_result) = resolve_manifest_executable(game_dir) {
        return manifest_result;
    }

    // 2. Candidates em `caves.verdict` (apenas quando houver exatamente um candidato Linux)
    if let Some(v) = verdict_json.filter(|s| !s.trim().is_empty()) {
        return resolve_verdict_executable(game_dir, v);
    }

    None
}

/// Lê todos os jogos instalados registrados em `butler.db`.
pub fn read_games(db_path: &Path) -> Result<Vec<Game>, String> {
    let db = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;

    let _ = db.busy_timeout(Duration::from_millis(250));

    // Mapeamento de locais de instalação (`install_locations`)
    let mut locations: HashMap<String, PathBuf> = HashMap::new();
    if let Ok(mut stmt) = db.prepare("SELECT id, path FROM install_locations")
        && let Ok(rows) =
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
    {
        for (id, path) in rows.flatten() {
            locations.insert(id, PathBuf::from(path));
        }
    }

    // Pasta padrão de apps caso o local não esteja mapeado
    let default_apps_dir = db_path
        .parent()
        .and_then(Path::parent)
        .map(|p| p.join("apps"))
        .unwrap_or_else(|| PathBuf::from("apps"));

    let mut stmt = db
        .prepare(
            "SELECT
                c.id,
                c.game_id,
                g.title,
                g.cover_url,
                g.still_cover_url,
                g.url,
                c.install_folder_name,
                c.install_location_id,
                c.custom_install_folder,
                c.verdict,
                c.installed_size,
                c.seconds_run,
                c.last_touched_at
             FROM caves c
             LEFT JOIN games g ON c.game_id = g.id",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<String>>(9)?,
                r.get::<_, Option<i64>>(10)?,
                r.get::<_, Option<i64>>(11)?,
                r.get::<_, Option<rusqlite::types::Value>>(12)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut games = Vec::new();
    for row in rows {
        let Ok((
            _cave_id,
            game_id,
            title,
            cover_url,
            still_cover_url,
            _store_url,
            install_folder_name,
            install_location_id,
            custom_install_folder,
            verdict,
            installed_size,
            seconds_run,
            last_touched_at,
        )) = row
        else {
            continue;
        };

        let title = title
            .filter(|t| !t.trim().is_empty())
            .or_else(|| install_folder_name.clone())
            .unwrap_or_else(|| format!("Jogo itch {game_id}"));

        // Determina o diretório de instalação com base nos modelos do butler
        let game_dir = if let Some(custom) = custom_install_folder.filter(|s| !s.trim().is_empty())
        {
            PathBuf::from(custom)
        } else if let Some(folder_name) = install_folder_name.filter(|s| !s.trim().is_empty()) {
            let base = install_location_id
                .as_deref()
                .and_then(|id| locations.get(id))
                .cloned()
                .unwrap_or_else(|| default_apps_dir.clone());
            base.join(folder_name)
        } else {
            default_apps_dir.join(format!("game-{game_id}"))
        };

        // Resolução do alvo de abertura: executável direto com fallback para o protocolo oficial itch://
        let exe = find_executable(&game_dir, verdict.as_deref());
        let (launch_target, working_directory, tracking) = if let Some(target) = exe {
            let cwd = target
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .or_else(|| Some(game_dir.to_string_lossy().into_owned()));
            (
                Some(target.to_string_lossy().into_owned()),
                cwd,
                Some(GameTrackingStrategy::Process),
            )
        } else {
            (
                Some(format!("itch://install?game_id={game_id}&launch")),
                None,
                None,
            )
        };

        let cover = still_cover_url
            .or(cover_url)
            .filter(|c| !c.trim().is_empty());
        let size_bytes = installed_size.filter(|&s| s > 0).map(|s| s as u64);
        let play_time = Duration::from_secs(seconds_run.unwrap_or(0).max(0) as u64);
        let last_played_at = parse_timestamp(last_touched_at);

        let mut game = Game {
            id: format!("itch:{game_id}"),
            title,
            store: Some(GameStore::Itch),
            accent: ACCENT.to_owned(),
            cover_uri: cover,
            is_installed: true,
            launch_target,
            working_directory,
            tracking,
            install_path: Some(game_dir.to_string_lossy().into_owned()),
            store_product_id: Some(game_id.to_string()),
            size_bytes,
            size_label: size_bytes.and_then(crate::steam::size_label),
            play_time,
            last_played_at,
            ..Default::default()
        };
        game.has_fallback_art = game.cover_uri.is_some();
        games.push(game);
    }

    Ok(games)
}

/// Caminhos possíveis do banco `butler.db` no Linux (nativo e Flatpak).
#[cfg(any(unix, test))]
pub fn db_candidates(home: &Path, xdg_config: Option<PathBuf>) -> Vec<PathBuf> {
    let config = xdg_config
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    let mut paths = vec![
        config.join("itch/db/butler.db"),
        home.join(".var/app/io.itch.itch/config/itch/db/butler.db"),
    ];
    let host = home.join(".config/itch/db/butler.db");
    if !paths.contains(&host) {
        paths.push(host);
    }
    paths
}

/// Fonte de biblioteca do itch.io.
pub struct ItchSource {
    db_paths: Vec<PathBuf>,
}

impl ItchSource {
    pub fn new(db_paths: Vec<PathBuf>) -> Self {
        ItchSource { db_paths }
    }

    #[cfg(unix)]
    pub fn detect() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        ItchSource::new(db_candidates(
            &home,
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        ))
    }
}

impl LibrarySource for ItchSource {
    fn store(&self) -> GameStore {
        GameStore::Itch
    }

    fn scan(&self) -> LibraryScan {
        let existing: Vec<&PathBuf> = self.db_paths.iter().filter(|p| p.is_file()).collect();
        if existing.is_empty() {
            return LibraryScan::missing(t("sources_unix.itch_missing"));
        }

        let mut seen = HashSet::new();
        let mut games = Vec::new();
        for db_path in existing {
            match read_games(db_path) {
                Ok(list) => games.extend(list.into_iter().filter(|g| seen.insert(g.id.clone()))),
                Err(e) => {
                    tracing::warn!("butler.db do itch ilegível em {}: {e}", db_path.display())
                }
            }
        }

        games.sort_by_cached_key(|g| g.title.to_lowercase());
        LibraryScan {
            games,
            client_installed: true,
            notice: None,
        }
    }
}
