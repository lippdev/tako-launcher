//! Metadados e artes dos jogos (port de `Launcher.Core/Library/Metadata` e da lógica de
//! `MetadataService`, `ArtService`, `SteamArtResolver` e `ArtCatalog`). Ver `docs/metadados.md`.
//!
//! Os provedores são síncronos e bloqueantes: rodam na thread do [`worker::MetadataWorker`], nunca na
//! da UI. Rede e JSON externos nunca entram em pânico: falha vira `None`/vazio.

pub mod agent;
pub mod art;
pub mod art_cache;
pub mod art_catalog;
pub mod cache;
pub mod http;
pub mod igdb;
pub mod igdb_parse;
pub mod merge;
pub mod review;
pub mod rom_hash;
pub mod screenscraper;
pub mod screenscraper_parse;
pub mod steam_art;
pub mod steam_store;
pub mod steamgriddb;
pub mod title_matcher;
pub mod worker;

pub use agent::{MetadataAgent, MetadataUpdate};
pub use merge::{MetadataField, MetadataKind, combine};

use crate::library::{Game, GameStore};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Nomes das fontes: chave nos ids, na prioridade e na tela.
pub mod sources {
    pub const STEAM: &str = "Steam";
    pub const IGDB: &str = "IGDB";
    /// Provedor de ROMs (ADR-0022).
    pub const SCREENSCRAPER: &str = "ScreenScraper";
    /// Artes da comunidade (ADR-0023): só completa a capa e o fundo de quem ficou sem.
    pub const STEAMGRIDDB: &str = "SteamGridDB";
}

/// Esquema das artes da Steam por appid (`steam-art://570/cover`).
pub const STEAM_ART_SCHEME: &str = "steam-art";

/// Ids do jogo em lojas, usados pelos provedores para casar sem depender do nome.
pub mod external_ids {
    use super::*;

    pub const STEAM: &str = "Steam";
    pub const GOG: &str = "GOG";
    pub const EPIC: &str = "Epic";
    pub const MICROSOFT: &str = "Microsoft";

    pub fn for_game(game: &Game) -> BTreeMap<String, String> {
        let mut ids = BTreeMap::new();
        // Só o appid da própria Steam: o de outras lojas pode ter vindo de uma correspondência pelo
        // nome, que a atualização precisa poder refazer.
        if game.store == Some(GameStore::Steam)
            && let Some(app) = game.steam_app_id
        {
            ids.insert(STEAM.into(), app.to_string());
        }
        let product = game
            .store_product_id
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty());
        match game.store {
            Some(GameStore::Gog) => {
                if let Some(id) = game.id.strip_prefix("gog:") {
                    ids.insert(GOG.into(), id.into());
                }
            }
            Some(GameStore::Epic) => {
                if let Some(p) = product {
                    ids.insert(EPIC.into(), p.into());
                }
            }
            Some(GameStore::Xbox) => {
                if let Some(p) = product {
                    ids.insert(MICROSOFT.into(), p.into());
                }
            }
            _ => {}
        }
        ids
    }
}

/// Dados de um jogo vindos de um provedor ou da combinação de vários. Também é o formato do cache
/// em disco, por jogo. Nomes em PascalCase para ler o cache do Tako em C# (MIG-030).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct GameMetadata {
    /// Provedor ("Steam") ou, na combinação, os que contribuíram ("Steam + IGDB").
    pub source: String,
    pub fetched_at: Option<DateTime<Utc>>,
    /// Correspondência escolhida pelo usuário no "Corrigir correspondência" antigo.
    pub is_pinned: bool,
    pub steam_app_id: Option<u32>,
    /// Id do jogo em cada fonte e, quando o provedor informa, em outras lojas.
    pub source_ids: BTreeMap<String, String>,
    /// Provedores consultados para esta combinação (mesmo os que não acharam nada).
    pub consulted: Vec<String>,
    /// Resultado de cada provedor, para recombinar sem consultar de novo.
    pub parts: Vec<GameMetadata>,
    pub title: Option<String>,
    pub description: Option<String>,
    /// "pt-BR", "en".
    pub description_language: Option<String>,
    pub genres: Vec<String>,
    pub themes: Vec<String>,
    pub game_modes: Vec<String>,
    pub franchise: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub metacritic_score: Option<i32>,
    /// Nota da crítica (0–100) de outras fontes (IGDB, ScreenScraper).
    pub critic_score: Option<i32>,
    /// `steam-art://`, `https://` ou `ssmedia://`.
    pub cover_url: Option<String>,
    pub background_url: Option<String>,
    pub players: Option<String>,
    /// Outras mídias por tipo (chaves em [`crate::library::media`]), só endereços.
    pub media: BTreeMap<String, String>,
    /// Numa parte: a fonte achou o jogo só pelo nome (sem id da loja nem escolha do usuário). A revisão de
    /// capas (ART-001) mostra estas quando o nome não bate exatamente.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub matched_by_title: bool,
}

impl GameMetadata {
    pub fn new(source: &str) -> Self {
        GameMetadata {
            source: source.into(),
            ..Default::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.source_ids.is_empty() && self.steam_app_id.is_none()
    }

    /// Id deste jogo na fonte, se conhecido.
    pub fn id_in(&self, source: &str) -> Option<String> {
        self.source_ids.get(source).cloned().or_else(|| {
            (source == sources::STEAM)
                .then_some(self.steam_app_id)
                .flatten()
                .map(|a| a.to_string())
        })
    }

    /// Preenche o jogo sem apagar o que a loja já trouxe; capa só para jogo sem capa própria.
    pub fn apply_to(&self, game: &mut Game) {
        if let Some(d) = self.description.as_ref().filter(|d| !d.trim().is_empty()) {
            game.description = d.clone();
        }
        for (src, dst) in [
            (&self.genres, &mut game.genres),
            (&self.themes, &mut game.themes),
            (&self.game_modes, &mut game.game_modes),
        ] {
            if !src.is_empty() {
                *dst = src.clone();
            }
        }
        fill(&mut game.franchise, &self.franchise);
        if self.release_date.is_some() {
            game.release_date = self.release_date;
        }
        fill(&mut game.developer, &self.developer);
        fill(&mut game.publisher, &self.publisher);
        if self.metacritic_score.is_some() {
            game.metacritic_score = self.metacritic_score;
        }
        if self.critic_score.is_some() {
            game.critic_score = self.critic_score;
        }
        fill(&mut game.players, &self.players);
        if !self.media.is_empty() {
            game.media = self.media.clone();
        }
        // Logo e ícone da Steam pelo appid quando nenhuma fonte trouxe (ART-001): a tela mostra o nome se a
        // Steam não tiver a imagem.
        if let Some(app) = self.steam_app_id {
            for kind in [crate::library::media::LOGO, crate::library::media::ICON] {
                game.media
                    .entry(kind.to_owned())
                    .or_insert_with(|| steam_art(app, kind));
            }
        }
        if self.steam_app_id.is_some() {
            game.steam_app_id = self.steam_app_id;
        }
        // Cache antigo (só Steam) não tem endereço de arte: vale a arte da Steam pelo appid.
        let cover = self
            .cover_url
            .clone()
            .or_else(|| self.steam_app_id.map(|a| steam_art(a, "cover")));
        let background = self
            .background_url
            .clone()
            .or_else(|| self.steam_app_id.map(|a| steam_art(a, "hero")));
        if let Some(cover) = cover {
            let replaceable = match &game.cover_uri {
                None => true,
                Some(current) => game.has_fallback_art || is_provider_art(current),
            };
            if replaceable {
                game.hero_uri = Some(background.unwrap_or_else(|| cover.clone()));
                game.cover_uri = Some(cover);
            }
        }
    }
}

fn fill(target: &mut Option<String>, value: &Option<String>) {
    if value.is_some() {
        target.clone_from(value);
    }
}

pub fn steam_art(app_id: u32, kind: &str) -> String {
    format!("{STEAM_ART_SCHEME}://{app_id}/{kind}")
}

/// Esquema de um endereço (`https`, `steam-art`), em minúsculas.
pub fn scheme_of(uri: &str) -> Option<String> {
    uri.split_once("://").map(|(s, _)| s.to_ascii_lowercase())
}

// Arte que veio de um provedor (não do usuário nem do disco) pode ser trocada por outra de provedor.
fn is_provider_art(uri: &str) -> bool {
    matches!(
        scheme_of(uri).as_deref(),
        Some(STEAM_ART_SCHEME | "https" | screenscraper::MEDIA_SCHEME)
    )
}

/// Resultado de busca de um provedor ("Corrigir correspondência" mostra a fonte).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MetadataMatch {
    pub source: String,
    pub id: String,
    pub title: String,
    pub year: Option<i32>,
    /// "PC", "SNES".
    pub platforms: Option<String>,
    /// Casado por identificador, sem ambiguidade.
    pub is_exact: bool,
    pub cover_url: Option<String>,
}

impl MetadataMatch {
    pub fn new(source: &str, id: impl Into<String>, title: impl Into<String>) -> Self {
        MetadataMatch {
            source: source.into(),
            id: id.into(),
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn steam(title: &str, app_id: u32) -> Self {
        Self::new(sources::STEAM, app_id.to_string(), title)
    }

    pub fn steam_app_id(&self) -> Option<u32> {
        (self.source == sources::STEAM)
            .then(|| self.id.parse().ok())
            .flatten()
    }

    /// "✓ Hades (2020) · PC · IGDB" para a lista de resultados.
    pub fn label(&self) -> String {
        let mut s = String::new();
        if self.is_exact {
            s.push_str("✓ ");
        }
        s.push_str(&self.title);
        if let Some(y) = self.year {
            s.push_str(&format!(" ({y})"));
        }
        if let Some(p) = &self.platforms {
            s.push_str(" · ");
            s.push_str(p);
        }
        s.push_str(" · ");
        s.push_str(&self.source);
        s
    }
}

/// O que se sabe do jogo para procurá-lo: título, plataforma (`None` = PC) e ids em lojas.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MetadataLookup {
    pub title: String,
    pub external_ids: BTreeMap<String, String>,
    pub platform: Option<String>,
    /// Arquivo da ROM, para casar pelo hash (ScreenScraper).
    pub rom_path: Option<String>,
}

impl MetadataLookup {
    pub fn new(title: &str) -> Self {
        MetadataLookup {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn for_game(game: &Game) -> Self {
        MetadataLookup {
            title: game.title.clone(),
            external_ids: external_ids::for_game(game),
            platform: game.platform.clone(),
            rom_path: game.rom_path.clone(),
        }
    }
}

/// Fonte de dados de jogos. Síncrona e bloqueante: chamada só da thread do worker.
pub trait MetadataProvider: Send + Sync {
    /// Nome curto ([`sources`]); chave nos ids e na prioridade.
    fn name(&self) -> &str;
    /// Falso quando falta configuração: o provedor é ignorado, sem erro.
    fn is_enabled(&self) -> bool;
    /// Pausado por um tempo (cota do dia): não é consultado, mas os dados que trouxe ficam.
    fn is_paused(&self) -> bool {
        false
    }
    /// `Some(auto_fill)` quando a fonte só completa artes (SteamGridDB): fora da busca automática.
    fn art_fallback(&self) -> Option<bool> {
        None
    }
    fn search(&self, lookup: &MetadataLookup) -> Vec<MetadataMatch>;
    /// Correspondência sem ambiguidade pelos ids em lojas.
    fn find_exact(&self, lookup: &MetadataLookup) -> Option<String>;
    fn fetch(&self, id: &str) -> Option<GameMetadata>;
    /// Nome na URL (`igdb.com/games/hades`) → id.
    fn id_from_slug(&self, _slug: &str) -> Option<String> {
        None
    }
    /// Lote antes da rodada (IGDB resolve a biblioteca em poucas consultas).
    fn prefetch(&self, _lookups: &[MetadataLookup]) {}
}

/// Cofre de segredos (implementado por `tako_platform::secrets::Secrets`).
pub trait SecretStore {
    fn get(&self, key: &str) -> Option<String>;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
}

/// Texto colado costuma trazer espaços e quebras de linha: saem todos.
pub fn clean_key(value: &str) -> String {
    value.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Chave de API com cara de chave: 20 a 64 letras ASCII e números.
fn looks_like_key(value: &str) -> bool {
    (20..=64).contains(&value.len()) && value.chars().all(|c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::http::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::time::Duration;

    /// Relógio parado; `sleep` só anota e avança.
    pub struct TestClock {
        pub now: Mutex<chrono::DateTime<chrono::Utc>>,
        pub sleeps: Mutex<Vec<Duration>>,
    }

    impl TestClock {
        pub fn at(now: chrono::DateTime<chrono::Utc>) -> Self {
            TestClock {
                now: Mutex::new(now),
                sleeps: Mutex::new(Vec::new()),
            }
        }
        pub fn advance(&self, d: chrono::Duration) {
            *self.now.lock().unwrap() += d;
        }
    }

    impl Clock for TestClock {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            *self.now.lock().unwrap()
        }
        fn sleep(&self, d: Duration) {
            self.sleeps.lock().unwrap().push(d);
        }
    }

    pub fn now() -> chrono::DateTime<chrono::Utc> {
        use chrono::TimeZone;
        chrono::Utc
            .with_ymd_and_hms(2026, 10, 15, 12, 0, 0)
            .single()
            .unwrap()
    }

    #[derive(Debug, Clone)]
    pub struct Sent {
        pub url: String,
        pub headers: Vec<(String, String)>,
        pub body: Option<String>,
    }

    impl Sent {
        pub fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str())
        }
    }

    type Responder = Box<dyn Fn(&HttpRequest) -> Option<HttpResponse> + Send + Sync>;

    /// Rede de mentira: respostas em fila (ou por regra) e os pedidos anotados.
    pub struct FakeHttp {
        pub queue: Mutex<VecDeque<(u16, String)>>,
        pub sent: Mutex<Vec<Sent>>,
        pub fallback: (u16, String),
        pub rule: Option<Responder>,
    }

    impl FakeHttp {
        pub fn new() -> Self {
            Self::with_fallback(200, "[]")
        }
        pub fn with_fallback(status: u16, body: &str) -> Self {
            FakeHttp {
                queue: Mutex::new(VecDeque::new()),
                sent: Mutex::new(Vec::new()),
                fallback: (status, body.into()),
                rule: None,
            }
        }
        pub fn push(&self, status: u16, body: &str) {
            self.queue.lock().unwrap().push_back((status, body.into()));
        }
        pub fn sent(&self) -> Vec<Sent> {
            self.sent.lock().unwrap().clone()
        }
    }

    impl Http for FakeHttp {
        fn send(&self, req: &HttpRequest) -> Result<HttpResponse, String> {
            self.sent.lock().unwrap().push(Sent {
                url: req.url.clone(),
                headers: req
                    .headers
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.clone()))
                    .collect(),
                body: req.body.clone(),
            });
            if let Some(r) = self.rule.as_ref().and_then(|f| f(req)) {
                return Ok(r);
            }
            let (status, body) = self
                .queue
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| self.fallback.clone());
            Ok(HttpResponse {
                status,
                body: body.into_bytes(),
                content_type: Some("application/json".into()),
                retry_after: None,
            })
        }
    }

    pub fn fixture(path: &str) -> String {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/metadata/fixtures")
            .join(path);
        std::fs::read_to_string(p).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(store: GameStore) -> Game {
        Game {
            id: format!("{}:1", store.id_prefix()),
            title: "Lanterna Distante".into(),
            store: Some(store),
            ..Default::default()
        }
    }

    #[test]
    fn metadata_fills_game_and_gives_manual_games_steam_art() {
        let mut g = game(GameStore::Manual);
        let m = GameMetadata {
            steam_app_id: Some(1030300),
            description: Some("Descubra um reino vasto".into()),
            genres: vec!["Ação".into(), "Aventura".into()],
            release_date: NaiveDate::from_ymd_opt(2025, 9, 4),
            developer: Some("Team Cherry".into()),
            metacritic_score: Some(91),
            ..GameMetadata::new("Steam")
        };
        m.apply_to(&mut g);
        assert_eq!(g.description, "Descubra um reino vasto");
        assert_eq!(g.genres, ["Ação", "Aventura"]);
        assert_eq!(g.metacritic_score, Some(91));
        assert_eq!(g.cover_uri.as_deref(), Some("steam-art://1030300/cover"));
    }

    #[test]
    fn metadata_keeps_a_cover_chosen_by_the_user() {
        let mut g = game(GameStore::Manual);
        g.cover_uri = Some("file:///C:/capas/silksong.png".into());
        GameMetadata {
            steam_app_id: Some(1030300),
            ..GameMetadata::new("Steam")
        }
        .apply_to(&mut g);
        assert_eq!(
            g.cover_uri.as_deref(),
            Some("file:///C:/capas/silksong.png")
        );
    }

    #[test]
    fn store_ids_come_from_each_store() {
        let mut steam = game(GameStore::Steam);
        steam.steam_app_id = Some(570);
        assert_eq!(external_ids::for_game(&steam)["Steam"], "570");
        let mut epic = game(GameStore::Epic);
        epic.steam_app_id = Some(570);
        assert!(external_ids::for_game(&epic).is_empty()); // appid de correspondência pelo nome não vale
        let mut gog = game(GameStore::Gog);
        gog.id = "gog:1207658924".into();
        assert_eq!(external_ids::for_game(&gog)["GOG"], "1207658924");
        epic.store_product_id = Some("cat123".into());
        assert_eq!(external_ids::for_game(&epic)["Epic"], "cat123");
        let mut xbox = game(GameStore::Xbox);
        xbox.store_product_id = Some("9NBLGGH".into());
        assert_eq!(external_ids::for_game(&xbox)["Microsoft"], "9NBLGGH");
    }

    #[test]
    fn match_labels() {
        let mut m = MetadataMatch::new("IGDB", "1", "Lanterna Distante");
        m.year = Some(2024);
        m.platforms = Some("PC, Nintendo Switch".into());
        m.is_exact = true;
        assert_eq!(
            m.label(),
            "✓ Lanterna Distante (2024) · PC, Nintendo Switch · IGDB"
        );
        assert_eq!(
            MetadataMatch::steam("Hades", 1145360).label(),
            "Hades · Steam"
        );
        assert_eq!(
            MetadataMatch::steam("Hades", 1145360).steam_app_id(),
            Some(1145360)
        );
    }

    #[test]
    fn cache_json_round_trips_and_reads_csharp_names() {
        let m = GameMetadata {
            steam_app_id: Some(570),
            cover_url: Some("steam-art://570/cover".into()),
            ..GameMetadata::new("Steam")
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"SteamAppId\":570"));
        assert_eq!(serde_json::from_str::<GameMetadata>(&json).unwrap(), m);
        let legacy: GameMetadata = serde_json::from_str(
            r#"{"Source":"Steam","FetchedAt":"2026-10-15T12:00:00+00:00","SteamAppId":570,"ReleaseDate":"2025-09-04","IsEmpty":false}"#,
        )
        .unwrap();
        assert_eq!(legacy.steam_app_id, Some(570));
        assert!(legacy.fetched_at.is_some());
    }
}
