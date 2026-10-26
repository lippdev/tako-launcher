//! A biblioteca do Playnite (port de `PlayniteLibrary.cs`): jogos, plataformas, categorias e fontes,
//! cada um no próprio arquivo do LiteDB 4, lidos sem alterar nada.

use super::litedb::{Bson, Document, InvalidData, LiteDbV7Reader};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

/// Bancos do Playnite maiores que isso não são lidos (arquivo não confiável).
const MAX_DB_FILE: u64 = 1 << 30;

#[derive(Debug, thiserror::Error)]
pub enum PlayniteError {
    #[error("Biblioteca do Playnite não encontrada: {0}")]
    NotFound(PathBuf),
    #[error("Biblioteca do Playnite ilegível: {0}")]
    InvalidData(#[from] InvalidData),
    #[error("Biblioteca do Playnite inacessível: {0}")]
    Io(#[from] std::io::Error),
}

/// Tipo da ação de jogar (`GameActionType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayniteActionType {
    File,
    Url,
    Emulator,
    Script,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayniteAction {
    pub kind: PlayniteActionType,
    pub path: Option<String>,
    pub arguments: Option<String>,
    pub working_dir: Option<String>,
    pub is_play_action: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayniteRom {
    pub name: Option<String>,
    pub path: Option<String>,
}

/// Um jogo do Playnite, só com o que a importação usa.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayniteGame {
    pub id: Uuid,
    pub name: String,
    /// Plugin da biblioteca (Steam, Epic…); nulo = jogo adicionado à mão no Playnite.
    pub plugin_id: Uuid,
    /// Id do jogo na loja do plugin (appid da Steam, AppName da Epic…).
    pub game_id: Option<String>,
    pub install_directory: Option<String>,
    pub is_installed: bool,
    pub hidden: bool,
    pub favorite: bool,
    pub playtime: Duration,
    pub last_activity: Option<DateTime<Utc>>,
    pub added: Option<DateTime<Utc>>,
    pub platform_ids: Vec<Uuid>,
    pub category_ids: Vec<Uuid>,
    pub source_id: Uuid,
    pub actions: Vec<PlayniteAction>,
    pub roms: Vec<PlayniteRom>,
}

/// Plataforma: nome e id de especificação ("pc_windows", "nintendo_super_nes").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaynitePlatform {
    pub name: String,
    pub specification_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayniteLibrary {
    pub games: Vec<PlayniteGame>,
    pub platforms: HashMap<Uuid, PlaynitePlatform>,
    pub categories: HashMap<Uuid, String>,
    pub sources: HashMap<Uuid, String>,
}

impl PlayniteLibrary {
    /// Lê a pasta `library`. Sem `games.db` é [`PlayniteError::NotFound`].
    pub fn read(folder: &Path) -> Result<Self, PlayniteError> {
        let games = folder.join("games.db");
        if !games.is_file() {
            return Err(PlayniteError::NotFound(games));
        }
        Ok(Self::from_documents(
            documents(&games)?,
            documents(&folder.join("platforms.db"))?,
            documents(&folder.join("categories.db"))?,
            documents(&folder.join("sources.db"))?,
        ))
    }

    /// Monta a biblioteca a partir dos documentos BSON já lidos.
    pub fn from_documents(
        games: Vec<Document>,
        platforms: Vec<Document>,
        categories: Vec<Document>,
        sources: Vec<Document>,
    ) -> Self {
        let named = |docs: Vec<Document>| -> Vec<(Uuid, Document)> {
            docs.into_iter()
                .filter_map(|d| Some((id(&d)?, d)))
                .filter(|(_, d)| text(d, "Name").is_some())
                .collect()
        };
        PlayniteLibrary {
            games: games.iter().filter_map(to_game).collect(),
            platforms: named(platforms)
                .into_iter()
                .map(|(id, d)| {
                    let p = PlaynitePlatform {
                        name: text(&d, "Name").unwrap_or_default(),
                        specification_id: text(&d, "SpecificationId"),
                    };
                    (id, p)
                })
                .collect(),
            categories: named(categories)
                .into_iter()
                .map(|(id, d)| (id, text(&d, "Name").unwrap_or_default()))
                .collect(),
            sources: named(sources)
                .into_iter()
                .map(|(id, d)| (id, text(&d, "Name").unwrap_or_default()))
                .collect(),
        }
    }

    /// A plataforma do jogo ("PC (Windows)").
    pub fn platform_of(&self, game: &PlayniteGame) -> Option<&PlaynitePlatform> {
        game.platform_ids
            .iter()
            .find_map(|id| self.platforms.get(id))
    }
}

fn documents(file: &Path) -> Result<Vec<Document>, PlayniteError> {
    if !file.is_file() {
        return Ok(Vec::new());
    }
    // O Playnite pode estar aberto: só leitura, sem travar o arquivo.
    let reader = LiteDbV7Reader::new(crate::read_limited(file, MAX_DB_FILE)?)?;
    // Uma coleção por arquivo (o nome é o do tipo); se houver outras, a maior é a dos dados.
    let mut best = Vec::new();
    for name in reader.collections() {
        let docs = reader.documents(name)?;
        if docs.len() > best.len() {
            best = docs;
        }
    }
    Ok(best)
}

fn to_game(d: &Document) -> Option<PlayniteGame> {
    let id = id(d)?;
    let name = text(d, "Name")?;
    let docs = |key| {
        list(d, key).iter().filter_map(|v| match v {
            Bson::Document(doc) => Some(doc),
            _ => None,
        })
    };
    Some(PlayniteGame {
        id,
        name: name.trim().to_owned(),
        plugin_id: guid(d.get("PluginId")).unwrap_or_default(),
        game_id: text(d, "GameId"),
        install_directory: text(d, "InstallDirectory"),
        is_installed: d.get("IsInstalled") == Some(&Bson::Bool(true)),
        hidden: d.get("Hidden") == Some(&Bson::Bool(true)),
        favorite: d.get("Favorite") == Some(&Bson::Bool(true)),
        playtime: Duration::try_from_secs_f64(number(d.get("Playtime")).unwrap_or(0.0).max(0.0))
            .unwrap_or_default(),
        last_activity: date(d.get("LastActivity")),
        added: date(d.get("Added")),
        platform_ids: guids(d, "PlatformIds"),
        category_ids: guids(d, "CategoryIds"),
        source_id: guid(d.get("SourceId")).unwrap_or_default(),
        actions: docs("GameActions")
            .map(|a| PlayniteAction {
                kind: action_type(a.get("Type")),
                path: text(a, "Path"),
                arguments: text(a, "Arguments"),
                working_dir: text(a, "WorkingDir"),
                is_play_action: a.get("IsPlayAction") == Some(&Bson::Bool(true)),
            })
            .collect(),
        roms: docs("Roms")
            .map(|r| PlayniteRom {
                name: text(r, "Name"),
                path: text(r, "Path"),
            })
            .collect(),
    })
}

/// O LiteDB grava enums como texto ("File") por padrão; aceita também o número. Desconhecido = script (não importado).
fn action_type(v: Option<&Bson>) -> PlayniteActionType {
    use PlayniteActionType::*;
    let n = match v {
        Some(Bson::String(s)) => {
            return match s.to_ascii_lowercase().as_str() {
                "file" => File,
                "url" => Url,
                "emulator" => Emulator,
                _ => Script,
            };
        }
        Some(Bson::Int32(n)) => i64::from(*n),
        Some(Bson::Int64(n)) => *n,
        _ => -1,
    };
    match n {
        0 => File,
        1 => Url,
        2 => Emulator,
        _ => Script,
    }
}

fn id(d: &Document) -> Option<Uuid> {
    guid(d.get("_id")).or_else(|| guid(d.get("Id")))
}

fn guid(v: Option<&Bson>) -> Option<Uuid> {
    match v? {
        Bson::Guid(g) => Some(*g),
        Bson::String(s) => Uuid::parse_str(s).ok(),
        _ => None,
    }
    .filter(|g| !g.is_nil())
}

fn guids(d: &Document, key: &str) -> Vec<Uuid> {
    list(d, key).iter().filter_map(|v| guid(Some(v))).collect()
}

fn list<'a>(d: &'a Document, key: &str) -> &'a [Bson] {
    match d.get(key) {
        Some(Bson::Array(v)) => v,
        _ => &[],
    }
}

fn text(d: &Document, key: &str) -> Option<String> {
    match d.get(key) {
        Some(Bson::String(s)) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn number(v: Option<&Bson>) -> Option<f64> {
    match v? {
        Bson::Int32(i) => Some(f64::from(*i)),
        Bson::Int64(l) => Some(*l as f64),
        Bson::Double(x) => Some(*x),
        Bson::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|x: &f64| x.is_finite())
}

/// Datas fora do intervalo do .NET (`DateTime.MinValue`/`MaxValue`) são "sem data".
fn date(v: Option<&Bson>) -> Option<DateTime<Utc>> {
    const MAX: i64 = 253_402_300_800_000;
    const MIN: i64 = -62_135_596_800_000;
    match v? {
        Bson::DateTime(ms) if *ms > MIN && *ms < MAX => DateTime::from_timestamp_millis(*ms),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playnite::sample;

    #[test]
    fn library_folder_is_read_with_platforms_categories_and_actions() {
        let dir = tempfile::tempdir().unwrap();
        sample::write(dir.path());
        let games_db = dir.path().join("games.db");
        let before = std::fs::metadata(&games_db).unwrap();

        let lib = PlayniteLibrary::read(dir.path()).unwrap();

        assert_eq!(lib.games.len(), 8);
        let lanterna = lib.games.iter().find(|g| g.id == sample::LANTERNA).unwrap();
        assert!(lanterna.favorite);
        assert_eq!(lanterna.playtime, Duration::from_secs(7200));
        assert_eq!(lanterna.actions[0].kind, PlayniteActionType::File);
        assert_eq!(lib.platform_of(lanterna).unwrap().name, "PC (Windows)");
        assert_eq!(lib.categories[&sample::SOFA], "Jogar no sofá");
        let nav = lib
            .games
            .iter()
            .find(|g| g.id == sample::NAVEGADOR)
            .unwrap();
        assert_eq!(nav.actions[0].kind, PlayniteActionType::Url);
        assert_eq!(lib.sources.values().collect::<Vec<_>>(), ["Steam"]);
        // Só leitura: o arquivo do Playnite não muda.
        let after = std::fs::metadata(&games_db).unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    }

    #[test]
    fn missing_library_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            PlayniteLibrary::read(&dir.path().join("x")),
            Err(PlayniteError::NotFound(_))
        ));
        std::fs::write(dir.path().join("games.db"), b"lixo").unwrap();
        assert!(matches!(
            PlayniteLibrary::read(dir.path()),
            Err(PlayniteError::InvalidData(_))
        ));
    }
}
