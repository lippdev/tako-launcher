//! Port de `Library/Game.cs`, `Metadata/Art/ArtKind.cs` e `Session/GameTracking.cs` (só o enum).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// Loja ou origem de um jogo. A ordem é a da barra lateral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GameStore {
    Steam,
    Epic,
    Gog,
    Xbox,
    Ubisoft,
    Ea,
    BattleNet,
    Amazon,
    /// Heroic Games Launcher (Linux/macOS): Epic, GOG e Amazon pelo Heroic.
    Heroic,
    /// Lutris (Linux).
    Lutris,
    Manual,
}

impl GameStore {
    pub const ALL: [GameStore; 11] = [
        GameStore::Steam,
        GameStore::Epic,
        GameStore::Gog,
        GameStore::Xbox,
        GameStore::Ubisoft,
        GameStore::Ea,
        GameStore::BattleNet,
        GameStore::Amazon,
        GameStore::Heroic,
        GameStore::Lutris,
        GameStore::Manual,
    ];

    /// Nome usado no banco e nas configurações (`DisabledStores`), igual ao do C# (`GameStore.ToString()`).
    pub fn key(self) -> &'static str {
        match self {
            GameStore::Steam => "Steam",
            GameStore::Epic => "Epic",
            GameStore::Gog => "Gog",
            GameStore::Xbox => "Xbox",
            GameStore::Ubisoft => "Ubisoft",
            GameStore::Ea => "Ea",
            GameStore::BattleNet => "BattleNet",
            GameStore::Amazon => "Amazon",
            GameStore::Heroic => "Heroic",
            GameStore::Lutris => "Lutris",
            GameStore::Manual => "Manual",
        }
    }

    pub fn from_key(key: &str) -> Option<GameStore> {
        GameStore::ALL
            .into_iter()
            .find(|s| s.key().eq_ignore_ascii_case(key.trim()))
    }

    /// Prefixo do id do jogo (`steam:570`).
    pub fn id_prefix(self) -> &'static str {
        match self {
            GameStore::Steam => "steam",
            GameStore::Epic => "epic",
            GameStore::Gog => "gog",
            GameStore::Xbox => "xbox",
            GameStore::Ubisoft => "ubisoft",
            GameStore::Ea => "ea",
            GameStore::BattleNet => "battlenet",
            GameStore::Amazon => "amazon",
            GameStore::Heroic => "heroic",
            GameStore::Lutris => "lutris",
            GameStore::Manual => "manual",
        }
    }
}

/// Como reconhecer o fim do jogo (MVP-026). `None` no jogo = automático.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameTrackingStrategy {
    /// Não acompanhar: só abre.
    None,
    /// O processo iniciado e os filhos dele.
    Process,
    /// Qualquer processo cujo executável esteja na pasta do jogo.
    InstallFolder,
    /// Só o sinal da Steam (`RunningAppID`).
    SteamApp,
}

impl GameTrackingStrategy {
    /// Nome gravado em `manual_games.track_by`, igual ao C#.
    pub fn key(self) -> &'static str {
        match self {
            GameTrackingStrategy::None => "None",
            GameTrackingStrategy::Process => "Process",
            GameTrackingStrategy::InstallFolder => "InstallFolder",
            GameTrackingStrategy::SteamApp => "SteamApp",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        [
            Self::None,
            Self::Process,
            Self::InstallFolder,
            Self::SteamApp,
        ]
        .into_iter()
        .find(|s| s.key().eq_ignore_ascii_case(key.trim()))
    }
}

/// Tipo de arte que o usuário pode escolher (ADR-0023 do C#).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtKind {
    Cover,
    Hero,
    Logo,
    Icon,
}

impl ArtKind {
    pub const ALL: [ArtKind; 4] = [ArtKind::Cover, ArtKind::Hero, ArtKind::Logo, ArtKind::Icon];

    /// Chave gravada em `game_art.kind` (`ArtKinds.Key` no C#).
    pub fn key(self) -> &'static str {
        match self {
            ArtKind::Cover => "cover",
            ArtKind::Hero => "hero",
            ArtKind::Logo => "logo",
            ArtKind::Icon => "icon",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|k| k.key().eq_ignore_ascii_case(key.trim()))
    }
}

/// Chaves de `Game::media` (port de `Metadata/GameMedia`).
pub mod media {
    pub const LOGO: &str = "logo";
    pub const ICON: &str = "icon";
    pub const BOX_3D: &str = "box3d";
    pub const MARQUEE: &str = "marquee";
    pub const SCREENSHOT: &str = "screenshot";
    pub const FANART: &str = "fanart";
    pub const VIDEO: &str = "video";
}

/// Jogo da biblioteca. Dados de catálogo vêm da fonte; metadados e estado do usuário chegam depois.
///
/// Arte e mídia são URIs em texto: `file:///…`, `https://…`, ou esquemas internos (`steam-art://`, `ssmedia://`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Game {
    /// `steam:<appid>`, `epic:<AppName>`, `gog:<id>`, `xbox:<PackageFamily>`, `ubisoft:<installId>`,
    /// `ea:<contentId>`, `battlenet:<uid>`, `amazon:<productId>`, `heroic:<runner>:<appName>`,
    /// `lutris:<slug>`, `manual:<guid>`.
    pub id: String,
    pub title: String,
    pub store: Option<GameStore>,
    pub size_label: Option<String>,
    /// Cor de destaque (#RRGGBB).
    pub accent: String,

    pub cover_uri: Option<String>,
    pub hero_uri: Option<String>,
    /// A capa é só um plano B da loja: os metadados podem trocá-la.
    pub has_fallback_art: bool,
    /// Enquadramento da arte de fundo ("50% 40%").
    pub hero_alignment: Option<String>,

    pub genres: Vec<String>,
    pub themes: Vec<String>,
    pub game_modes: Vec<String>,
    pub franchise: Option<String>,
    pub description: String,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub metacritic_score: Option<i32>,
    pub critic_score: Option<i32>,
    /// Aviso nos detalhes (ex.: port não oficial).
    pub note: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub upcoming_release: Option<NaiveDate>,
    pub players: Option<String>,

    pub is_installed: bool,
    /// URI (`steam://rungameid/570`) ou caminho do executável/atalho/script. `None` = jogo de demonstração.
    pub launch_target: Option<String>,
    pub launch_arguments: Option<String>,
    pub working_directory: Option<String>,
    pub install_path: Option<String>,
    pub tracking: Option<GameTrackingStrategy>,
    /// Id do produto na loja quando difere do `id` (Epic: CatalogItemId; Xbox: StoreId).
    pub store_product_id: Option<String>,
    /// Sistema da ROM ("snes", "ps2"); `None` = PC.
    pub platform: Option<String>,
    pub rom_path: Option<String>,

    /// Outras mídias por tipo (chaves em [`media`]).
    pub media: BTreeMap<String, String>,
    /// Artes escolhidas pelo usuário: valem mais que as automáticas e os metadados nunca as trocam.
    pub pinned_art: BTreeMap<ArtKind, String>,
    pub steam_app_id: Option<u32>,
    /// Outras lojas em que o mesmo jogo está instalado.
    pub also_installed_in: Vec<GameStore>,

    pub added_at: Option<DateTime<Utc>>,
    pub is_favorite: bool,
    /// Jogo adicionado cujo arquivo ou pasta sumiu (LIB-004); a UI preenche depois de verificar o disco.
    #[serde(default)]
    pub is_broken: bool,
    pub last_played_at: Option<DateTime<Utc>>,
    #[serde(with = "duration_secs")]
    pub play_time: Duration,
    /// Ids das coleções que têm este jogo.
    pub collections: BTreeSet<String>,
}

impl Game {
    pub fn store(&self) -> GameStore {
        self.store.unwrap_or(GameStore::Manual)
    }

    pub fn is_rom(&self) -> bool {
        self.rom_path.is_some()
    }

    pub fn is_upcoming(&self) -> bool {
        self.upcoming_release.is_some()
    }

    pub fn can_play(&self) -> bool {
        self.is_installed && !self.is_upcoming() && !self.is_rom()
    }

    pub fn shown_cover(&self) -> Option<&str> {
        self.pinned_art
            .get(&ArtKind::Cover)
            .or(self.cover_uri.as_ref())
            .map(String::as_str)
    }

    pub fn shown_hero(&self) -> Option<&str> {
        self.pinned_art
            .get(&ArtKind::Hero)
            .or(self.hero_uri.as_ref())
            .map(String::as_str)
    }

    pub fn shown_logo(&self) -> Option<&str> {
        self.pinned_art
            .get(&ArtKind::Logo)
            .or(self.media.get(media::LOGO))
            .map(String::as_str)
    }

    pub fn shown_icon(&self) -> Option<&str> {
        self.pinned_art
            .get(&ArtKind::Icon)
            .or(self.media.get(media::ICON))
            .map(String::as_str)
    }
}

mod duration_secs {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_f64(d.as_secs_f64())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let secs = f64::deserialize(d)?;
        Ok(Duration::from_secs_f64(secs.max(0.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_art_wins_over_automatic() {
        let mut g = Game {
            cover_uri: Some("https://a/cover.jpg".into()),
            ..Default::default()
        };
        assert_eq!(g.shown_cover(), Some("https://a/cover.jpg"));
        g.pinned_art.insert(ArtKind::Cover, "file:///c.png".into());
        assert_eq!(g.shown_cover(), Some("file:///c.png"));
    }

    #[test]
    fn rom_and_upcoming_cannot_play() {
        let mut g = Game {
            is_installed: true,
            ..Default::default()
        };
        assert!(g.can_play());
        g.rom_path = Some("/roms/a.sfc".into());
        assert!(!g.can_play());
    }

    #[test]
    fn keys_round_trip() {
        for s in GameStore::ALL {
            assert_eq!(GameStore::from_key(s.key()), Some(s));
        }
        for k in ArtKind::ALL {
            assert_eq!(ArtKind::from_key(k.key()), Some(k));
        }
    }
}
