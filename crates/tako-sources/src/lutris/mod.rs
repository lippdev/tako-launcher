//! Lutris (Linux, MIG-026): jogos instalados lidos do banco `pga.db` sem o Lutris aberto.
//!
//! Abre pelo protocolo `lutris:rungameid/<id>` (o id numérico do banco, sem ambiguidade de slug).
//! Capa e banner vêm de `coverart/<slug>.jpg|png` e `banners/<slug>.jpg|png` na pasta de dados.

use crate::{LibraryScan, LibrarySource, file_uri};
use chrono::DateTime;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore, media};

#[cfg(test)]
mod tests;

const ACCENT: &str = "#E8812F";

/// O slug vira nome de arquivo: nada de `/` nem `..`.
fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug.len() <= 200
        && !slug.starts_with('.')
        && slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

fn first_file(dir: &Path, names: &[String]) -> Option<String> {
    names
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
        .map(|p| file_uri(&p))
}

/// Jogos instalados (`installed = 1`) de uma pasta de dados do Lutris (a que tem `pga.db`).
pub fn read_games(data_dir: &Path) -> Result<Vec<Game>, String> {
    let db = Connection::open_with_flags(
        data_dir.join("pga.db"),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, name, slug, directory, playtime, lastplayed, installed_at
             FROM games WHERE installed = 1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                // Colunas sem tipo rígido no SQLite: um valor estranho vira "sem dado", não erro.
                r.get::<_, f64>(4).unwrap_or(0.0),
                r.get::<_, i64>(5).ok(),
                r.get::<_, i64>(6).ok(),
            ))
        })
        .map_err(|e| e.to_string())?;

    // Ícones ficam fora da pasta do Lutris: `<dados do usuário>/icons/hicolor/128x128/apps`.
    let icons = data_dir
        .parent()
        .map(|p| p.join("icons/hicolor/128x128/apps"));
    let mut games = Vec::new();
    for row in rows {
        let Ok((id, name, slug, directory, playtime, lastplayed, installed_at)) = row else {
            continue;
        };
        let Some(slug) = slug.filter(|s| valid_slug(s)) else {
            continue;
        };
        let art = [format!("{slug}.jpg"), format!("{slug}.png")];
        let mut game = Game {
            id: format!("lutris:{slug}"),
            title: name
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| slug.clone()),
            store: Some(GameStore::Lutris),
            accent: ACCENT.to_owned(),
            cover_uri: first_file(&data_dir.join("coverart"), &art),
            hero_uri: first_file(&data_dir.join("banners"), &art),
            is_installed: true,
            launch_target: Some(format!("lutris:rungameid/{id}")),
            install_path: directory.filter(|d| !d.trim().is_empty()),
            store_product_id: Some(id.to_string()),
            // Lutris guarda horas (REAL).
            play_time: Duration::try_from_secs_f64(playtime * 3600.0).unwrap_or_default(),
            last_played_at: lastplayed
                .filter(|s| *s > 0)
                .and_then(|s| DateTime::from_timestamp(s, 0)),
            added_at: installed_at
                .filter(|s| *s > 0)
                .and_then(|s| DateTime::from_timestamp(s, 0)),
            ..Default::default()
        };
        game.has_fallback_art = game.cover_uri.is_some();
        if let Some(icon) = icons
            .as_deref()
            .and_then(|d| first_file(d, &[format!("lutris_{slug}.png")]))
        {
            game.media.insert(media::ICON.to_owned(), icon);
        }
        games.push(game);
    }
    Ok(games)
}

/// Fonte do Lutris: junta a instalação nativa e a Flatpak.
pub struct LutrisSource {
    data_dirs: Vec<PathBuf>,
}

impl LutrisSource {
    pub fn new(data_dirs: Vec<PathBuf>) -> Self {
        LutrisSource { data_dirs }
    }

    // shortcut: ignora `pga_path` do lutris.conf (banco em outro lugar); ler o conf se alguém pedir.
    #[cfg(unix)]
    pub fn detect() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".local/share"));
        let mut dirs = vec![
            data.join("lutris"),
            home.join(".var/app/net.lutris.Lutris/data/lutris"),
        ];
        // Num Flatpak o XDG_DATA_HOME é o da sandbox; o Lutris nativo fica na pasta padrão do usuário.
        let host = home.join(".local/share/lutris");
        if !dirs.contains(&host) {
            dirs.push(host);
        }
        LutrisSource::new(dirs)
    }
}

impl LibrarySource for LutrisSource {
    fn store(&self) -> GameStore {
        GameStore::Lutris
    }

    fn scan(&self) -> LibraryScan {
        let dirs: Vec<&PathBuf> = self
            .data_dirs
            .iter()
            .filter(|d| d.join("pga.db").is_file())
            .collect();
        if dirs.is_empty() {
            return LibraryScan::missing(t("sources_unix.lutris_missing"));
        }
        let mut seen = HashSet::new();
        let mut games = Vec::new();
        for dir in dirs {
            match read_games(dir) {
                Ok(list) => games.extend(list.into_iter().filter(|g| seen.insert(g.id.clone()))),
                Err(e) => tracing::warn!("pga.db do Lutris ilegível em {}: {e}", dir.display()),
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
