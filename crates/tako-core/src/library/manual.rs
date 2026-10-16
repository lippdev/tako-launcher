//! Port de `Library/ManualGame.cs`: jogo adicionado pelo usuário e estado do usuário por jogo.

use super::{Game, GameStore, GameTrackingStrategy, PlayStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Jogo adicionado pelo usuário: executável, atalho, script, `.desktop`, AppImage, `.app` ou URI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManualGameRecord {
    pub id: String,
    pub title: String,
    /// Caminho do que abre o jogo, ou URI (`steam://...`).
    pub path: String,
    pub arguments: Option<String>,
    pub working_directory: Option<String>,
    /// Como reconhecer o fim do jogo; `None` = automático (processo iniciado).
    pub track_by: Option<GameTrackingStrategy>,
    /// Imagem local escolhida como capa (2:3).
    pub cover_path: Option<String>,
    /// Imagem local de fundo; sem ela, usa a capa.
    pub hero_path: Option<String>,
    pub added_at: DateTime<Utc>,
    /// Sistema da ROM ("snes", "ps2"): o arquivo é uma ROM, não um executável. `None` = jogo de PC.
    pub platform: Option<String>,
}

/// Estado do usuário por jogo (qualquer loja): favorito, status, última vez aberto pelo launcher e tempo registrado.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameStateRecord {
    pub is_favorite: bool,
    #[serde(default)]
    pub play_status: Option<PlayStatus>,
    pub last_played_at: Option<DateTime<Utc>>,
    pub play_minutes: f64,
    /// Histórico (SESSION-002), da mais antiga para a mais recente; só as últimas `MAX_SESSIONS`.
    #[serde(default)]
    pub sessions: Vec<PlaySession>,
}

/// Uma vez que o jogo rodou pelo launcher.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlaySession {
    pub started_at: DateTime<Utc>,
    pub duration: Duration,
}

impl ManualGameRecord {
    pub fn create(title: &str, path: &str, now: DateTime<Utc>) -> ManualGameRecord {
        ManualGameRecord {
            id: format!("manual:{}", uuid::Uuid::new_v4().simple()),
            title: title.to_owned(),
            path: path.to_owned(),
            arguments: None,
            working_directory: parent_dir(path),
            track_by: None,
            cover_path: None,
            hero_path: None,
            added_at: now,
            platform: None,
        }
    }

    /// ROM ("Adicionar jogo"): o título já vem limpo do nome do arquivo (quem chama usa o `RomTitle` dos metadados).
    pub fn create_rom(
        title: &str,
        path: &str,
        platform: &str,
        now: DateTime<Utc>,
    ) -> ManualGameRecord {
        ManualGameRecord {
            platform: Some(platform.to_owned()),
            ..Self::create(title, path, now)
        }
    }

    /// Lê o disco (existência do arquivo e das imagens).
    pub fn to_game(&self) -> Game {
        let is_uri = is_uri(&self.path);
        let cover = self
            .cover_path
            .as_deref()
            .filter(|p| Path::new(p).is_file());
        let hero = self
            .hero_path
            .as_deref()
            .filter(|p| Path::new(p).is_file())
            .or(cover);
        Game {
            id: self.id.clone(),
            title: self.title.clone(),
            store: Some(GameStore::Manual),
            accent: "#2B3A55".into(),
            cover_uri: cover.map(file_uri),
            hero_uri: hero.map(file_uri),
            is_installed: is_uri || launchable_exists(&self.path),
            added_at: Some(self.added_at),
            launch_target: Some(self.path.clone()),
            launch_arguments: self.arguments.clone(),
            working_directory: self.working_directory.clone(),
            install_path: if is_uri { None } else { parent_dir(&self.path) },
            tracking: self.track_by,
            platform: self.platform.clone(),
            rom_path: if self.platform.is_none() || is_uri {
                None
            } else {
                Some(self.path.clone())
            },
            ..Game::default()
        }
    }
}

/// URI de outro esquema que não `file:` (`steam://`, `com.epicgames.launcher://`). Esquema de uma letra é unidade
/// do Windows (`C:\`).
pub fn is_uri(path: &str) -> bool {
    let Some((scheme, _)) = path.split_once(':') else {
        return false;
    };
    scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        && !scheme.eq_ignore_ascii_case("file")
}

/// Arquivo que abre o jogo; no macOS o `.app` é uma pasta.
fn launchable_exists(path: &str) -> bool {
    let p = Path::new(path);
    p.is_file() || (p.is_dir() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")))
}

fn parent_dir(path: &str) -> Option<String> {
    if is_uri(path) {
        return None;
    }
    Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty())
}

/// Caminho local → `file:///…` (barras normais, espaço e afins codificados).
pub fn file_uri(path: &str) -> String {
    let slashed = path.replace('\\', "/");
    let mut out = String::from(if slashed.starts_with('/') {
        "file://"
    } else {
        "file:///"
    });
    for b in slashed.bytes() {
        if b.is_ascii_alphanumeric() || b"/:-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        "2026-10-14T20:00:00Z".parse().unwrap()
    }

    #[test]
    fn uris_stay_uris_and_paths_are_paths() {
        assert!(is_uri("steam://rungameid/570"));
        assert!(is_uri("com.epicgames.launcher://apps/x?action=launch"));
        assert!(!is_uri(r"C:\Games\a.exe"));
        assert!(!is_uri("/usr/bin/game"));
        assert!(!is_uri("file:///home/a/game.AppImage"));
    }

    #[test]
    fn manual_record_becomes_a_game() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Jogo Novo.AppImage");
        std::fs::write(&exe, b"x").unwrap();
        let cover = dir.path().join("capa.png");
        std::fs::write(&cover, b"x").unwrap();
        let mut record = ManualGameRecord::create("Jogo Novo", exe.to_str().unwrap(), now());
        record.cover_path = Some(cover.to_string_lossy().into_owned());
        record.hero_path = Some("/nao/existe.png".into());

        let game = record.to_game();
        assert!(game.id.starts_with("manual:"));
        assert!(game.is_installed);
        assert_eq!(game.store(), GameStore::Manual);
        assert_eq!(game.install_path, record.working_directory);
        assert!(game.cover_uri.as_deref().unwrap().starts_with("file:///"));
        assert_eq!(game.hero_uri, game.cover_uri); // fundo some: usa a capa
        assert!(game.rom_path.is_none());

        let missing = ManualGameRecord::create("Sumiu", "/nao/existe/jogo.sh", now()).to_game();
        assert!(!missing.is_installed);
        let uri = ManualGameRecord::create("Steam", "steam://rungameid/570", now()).to_game();
        assert!(uri.is_installed && uri.install_path.is_none());

        let rom = ManualGameRecord::create_rom("Mario", "/roms/mario.sfc", "snes", now()).to_game();
        assert_eq!(rom.rom_path.as_deref(), Some("/roms/mario.sfc"));
        assert!(!rom.can_play());
    }

    #[test]
    fn mac_app_bundle_is_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Jogo.app");
        std::fs::create_dir(&app).unwrap();
        assert!(
            ManualGameRecord::create("Jogo", app.to_str().unwrap(), now())
                .to_game()
                .is_installed
        );
        assert!(
            !ManualGameRecord::create("Pasta", dir.path().to_str().unwrap(), now())
                .to_game()
                .is_installed
        );
    }

    #[test]
    fn file_uri_encodes_windows_and_unix_paths() {
        assert_eq!(file_uri(r"C:\Capas\a b.png"), "file:///C:/Capas/a%20b.png");
        assert_eq!(file_uri("/home/u/ç.png"), "file:///home/u/%C3%A7.png");
    }
}
