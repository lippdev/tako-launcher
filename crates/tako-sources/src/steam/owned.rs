//! Jogos da conta Steam que não estão instalados (SRC-005). Os arquivos locais da Steam não dizem com segurança o
//! que a conta possui, então a lista vem da Web API da Steam com a chave do próprio usuário. Quem busca é a
//! interface; aqui só se lê o cache (a resposta da API gravada em disco), e a biblioteca continua offline.

use super::{ACCENTS, ART_SCHEME, TOOL_APP_IDS};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;
use tako_core::library::{Game, GameStore};

pub const OWNED_URL: &str = "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/";
/// Página onde o usuário cria a própria chave da Web API.
pub const KEY_PAGE: &str = "https://steamcommunity.com/dev/apikey";
const MAX_CACHE: u64 = 16 << 20;
/// SteamID64 = esta base + o id da conta (o nome da pasta em `userdata`).
const STEAM_ID64_BASE: u64 = 76_561_197_960_265_728;

/// Jogo da conta, como a API informa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedApp {
    pub app_id: u32,
    pub name: String,
    pub play_time: Duration,
    pub last_played: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct Answer {
    response: Response,
}

#[derive(Deserialize, Default)]
struct Response {
    #[serde(default)]
    games: Vec<RawGame>,
}

#[derive(Deserialize)]
struct RawGame {
    appid: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    playtime_forever: u64,
    #[serde(default)]
    rtime_last_played: i64,
}

pub fn steam_id(account_id: u32) -> u64 {
    STEAM_ID64_BASE + u64::from(account_id)
}

/// Conta usada por último neste PC.
pub fn account_id(steam_root: &Path) -> Option<u32> {
    super::last_user_config(steam_root)?
        .parent()?
        .file_name()?
        .to_str()?
        .parse()
        .ok()
}

/// Endereço da consulta. Contém a chave: nunca vai para o log.
pub fn url(key: &str, steam_id: u64) -> String {
    format!(
        "{OWNED_URL}?key={key}&steamid={steam_id}&include_appinfo=1&include_played_free_games=1&format=json"
    )
}

/// Jogos de uma resposta da API; `None` quando o texto não é a resposta esperada. Perfil privado responde sem
/// jogos (lista vazia).
pub fn parse(json: &str) -> Option<Vec<OwnedApp>> {
    let answer: Answer = serde_json::from_str(json).ok()?;
    Some(
        answer
            .response
            .games
            .into_iter()
            .filter(|g| g.appid > 0 && !g.name.trim().is_empty())
            .map(|g| OwnedApp {
                app_id: g.appid,
                name: g.name.trim().to_owned(),
                play_time: Duration::from_secs(g.playtime_forever.saturating_mul(60)),
                last_played: (g.rtime_last_played > 0)
                    .then(|| DateTime::from_timestamp(g.rtime_last_played, 0))
                    .flatten(),
            })
            .collect(),
    )
}

/// O cache gravado pela interface; ausente, grande demais ou quebrado = nenhum.
pub fn load(cache: &Path) -> Vec<OwnedApp> {
    let too_big = std::fs::metadata(cache).map_or(true, |m| m.len() > MAX_CACHE);
    if too_big {
        return Vec::new();
    }
    std::fs::read_to_string(cache)
        .ok()
        .and_then(|t| parse(&t))
        .unwrap_or_default()
}

/// Jogo não instalado: o mesmo id do instalado (`steam:<appid>`), para virar instalado no lugar quando a Steam
/// terminar de baixar.
pub fn to_game(app: &OwnedApp) -> Game {
    let id = app.app_id;
    Game {
        id: format!("steam:{id}"),
        title: app.name.clone(),
        store: Some(GameStore::Steam),
        accent: ACCENTS[id as usize % ACCENTS.len()].to_owned(),
        cover_uri: Some(format!("{ART_SCHEME}://{id}/cover")),
        hero_uri: Some(format!("{ART_SCHEME}://{id}/hero")),
        is_installed: false,
        last_played_at: app.last_played,
        play_time: app.play_time,
        steam_app_id: Some(id),
        ..Default::default()
    }
}

/// Os não instalados da conta: fora os que já estão em `installed` e as ferramentas da Steam.
pub fn not_installed(owned: &[OwnedApp], installed: &[Game]) -> Vec<Game> {
    owned
        .iter()
        .filter(|a| !TOOL_APP_IDS.contains(&a.app_id))
        .filter(|a| !installed.iter().any(|g| g.steam_app_id == Some(a.app_id)))
        .map(to_game)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibrarySource;

    const ANSWER: &str = r#"{"response":{"game_count":4,"games":[
        {"appid":570,"name":"Dota 2","playtime_forever":600,"rtime_last_played":1700000000,"img_icon_url":"x"},
        {"appid":1145360,"name":" Hades ","playtime_forever":0,"rtime_last_played":0},
        {"appid":228980,"name":"Steamworks Common Redistributables"},
        {"appid":7,"name":""}
    ]}}"#;

    #[test]
    fn reads_the_api_answer_and_ignores_anything_else() {
        let apps = parse(ANSWER).unwrap();
        assert_eq!(apps.len(), 3);
        assert_eq!(apps[0].play_time, Duration::from_secs(36_000));
        assert_eq!(apps[0].last_played.unwrap().timestamp(), 1_700_000_000);
        assert_eq!(apps[1].name, "Hades");
        assert_eq!(apps[1].last_played, None);
        // Perfil privado: resposta sem jogos.
        assert_eq!(parse(r#"{"response":{}}"#), Some(Vec::new()));
        assert_eq!(parse("<html>"), None);
        assert_eq!(parse(r#"{"response":{"games":[{"appid":"x"}]}}"#), None);
        assert!(load(Path::new("/nao/existe.json")).is_empty());
    }

    #[test]
    fn account_ids_become_steam_ids_and_urls() {
        assert_eq!(steam_id(124_233_287), 76_561_198_084_499_015);
        let u = url("CHAVE", 1);
        assert!(
            u.starts_with(OWNED_URL) && u.contains("key=CHAVE") && u.contains("include_appinfo=1")
        );
    }

    #[test]
    fn steam_lists_owned_games_that_are_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Steam");
        let apps = root.join("steamapps");
        std::fs::create_dir_all(apps.join("common").join("dota 2 beta")).unwrap();
        std::fs::write(
            apps.join("appmanifest_570.acf"),
            "\"AppState\" { \"appid\" \"570\" \"name\" \"Dota 2\" \"StateFlags\" \"4\" \"installdir\" \"dota 2 beta\" }",
        )
        .unwrap();
        let cache = dir.path().join("steam-owned.json");
        std::fs::write(&cache, ANSWER).unwrap();

        let scan = super::super::SteamLibrarySource::new(Some(root))
            .with_owned_cache(Some(cache))
            .scan();
        let mut games: Vec<_> = scan
            .games
            .iter()
            .map(|g| (g.title.as_str(), g.is_installed))
            .collect();
        games.sort();
        assert_eq!(games, [("Dota 2", true), ("Hades", false)]);
        let hades = scan.games.iter().find(|g| !g.is_installed).unwrap();
        assert_eq!(hades.id, "steam:1145360");
        assert_eq!(hades.launch_target, None);
    }
}
