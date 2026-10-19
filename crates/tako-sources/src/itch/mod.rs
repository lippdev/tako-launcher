//! itch.io (Linux, SRC-002): jogos instalados lidos de `butler.db` do app oficial sem o itch aberto.
//!
//! O itch.io desktop utiliza o `butler` / `butlerd` como daemon e banco local SQLite em:
//! `~/.config/itch/db/butler.db` (nativo) ou `~/.var/app/io.itch.itch/config/itch/db/butler.db` (Flatpak).
//!
//! O banco guarda:
//! - `caves`: instâncias de jogos instalados (id, game_id, install_folder, install_location, installed_size, seconds_run, last_touched_at).
//! - `games`: metadados da loja (id, title, cover_url, still_cover_url, url).
//! - `install_locations`: locais base de instalação (id, path).
//!
//! ### Formas de abertura e pesquisa de suporte (SRC-002)
//! 1. **Executável instalado no disco:** O itch extrai os arquivos do jogo sem criptografia na pasta
//!    `<install_location>/<install_folder>`. Abrir o executável diretamente permite ao Tako acompanhar
//!    o processo com precisão (`GameTrackingStrategy::Process`), contar o tempo de sessão e fechar
//!    limpo sem nenhuma janela intermediária.
//! 2. **Protocolo `itch://`:** O app oficial registra os esquemas `itch://` e `itchio://`. A sintaxe
//!    oficial para abrir um jogo instalado é `itch://install?game_id=<id>&launch`. Porém, por segurança,
//!    o cliente do itch sempre exibe uma caixa modal de confirmação para o usuário aceitar antes de rodar
//!    o binário.
//! 3. **Estratégia do Tako:** Busca primeiro o executável na pasta instalada (lendo `.itch.toml` se existir
//!    ou inspecionando binários executáveis na pasta do jogo). Se encontrado, abre diretamente pelo
//!    executável; se não houver binário identificado (ex.: jogo HTML5 ou pasta movida), utiliza o fallback
//!    pelo protocolo `itch://install?game_id=<id>&launch`.

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

/// Tenta encontrar o executável principal dentro da pasta do jogo no itch.io.
pub fn find_executable(game_dir: &Path, title: Option<&str>) -> Option<PathBuf> {
    if !game_dir.is_dir() {
        return None;
    }

    // 1. Manifest `.itch.toml` explícito do desenvolvedor
    let manifest = game_dir.join(".itch.toml");
    if manifest.is_file()
        && let Ok(bytes) = read_limited(&manifest, 64 * 1024)
    {
        let text = String::from_utf8_lossy(&bytes);
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("path")
                && let Some((_, val)) = rest.split_once('=')
            {
                let path_val = val.trim().trim_matches('"').trim_matches('\'');
                let candidate = game_dir.join(path_val);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    // 2. Varredura rasa de arquivos executáveis
    let Ok(entries) = std::fs::read_dir(game_dir) else {
        return None;
    };

    let mut candidates: Vec<(PathBuf, i32)> = Vec::new();

    let title_lower = title.map(|t| t.to_lowercase()).unwrap_or_default();
    let folder_lower = game_dir
        .file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = p
                .file_name()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            // Ignora desinstaladores e instaladores auxiliares
            if name.contains("uninstall")
                || name.contains("setup")
                || name.contains("crashpad")
                || name.ends_with(".so")
                || name.ends_with(".dll")
            {
                continue;
            }

            let mut score = 0;
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");

            let is_known_ext = matches!(ext, "x86_64" | "x86" | "bin" | "sh");
            #[cfg(unix)]
            let is_exec = {
                use std::os::unix::fs::PermissionsExt;
                std::fs::metadata(&p).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
            };
            #[cfg(not(unix))]
            let is_exec = is_known_ext;

            if is_exec || is_known_ext {
                score += 5;
                if is_known_ext {
                    score += 5;
                }
                if !title_lower.is_empty() && name.contains(&title_lower) {
                    score += 20;
                }
                if !folder_lower.is_empty() && name.contains(&folder_lower) {
                    score += 15;
                }
                candidates.push((p, score));
            }
        }
    }

    candidates.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    candidates.into_iter().next().map(|(p, _)| p)
}

/// Lê todos os jogos instalados registrados em `butler.db`.
pub fn read_games(db_path: &Path) -> Result<Vec<Game>, String> {
    let db = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;

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
                c.install_folder,
                c.install_location,
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
                r.get::<_, Option<i64>>(8)?,
                r.get::<_, Option<i64>>(9)?,
                r.get::<_, Option<rusqlite::types::Value>>(10)?,
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
            install_folder,
            install_location,
            installed_size,
            seconds_run,
            last_touched_at,
        )) = row
        else {
            continue;
        };

        let title = title
            .filter(|t| !t.trim().is_empty())
            .or_else(|| install_folder.clone())
            .unwrap_or_else(|| format!("Jogo itch {game_id}"));

        // Determina o diretório de instalação
        let game_dir = if let Some(folder) = &install_folder {
            let folder_path = Path::new(folder);
            if folder_path.is_absolute() {
                folder_path.to_path_buf()
            } else {
                let base = install_location
                    .as_deref()
                    .and_then(|loc| locations.get(loc))
                    .cloned()
                    .or_else(|| {
                        install_location
                            .as_deref()
                            .filter(|l| l.starts_with('/'))
                            .map(PathBuf::from)
                    })
                    .unwrap_or_else(|| default_apps_dir.clone());
                base.join(folder)
            }
        } else {
            default_apps_dir.join(format!("game-{game_id}"))
        };

        // Resolução do alvo de abertura: executável direto com fallback para itch://
        let exe = find_executable(&game_dir, Some(&title));
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
