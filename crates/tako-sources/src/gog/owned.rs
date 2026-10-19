//! Jogos da conta GOG que não estão instalados (SRC-005), lidos offline do banco do GOG Galaxy 2
//! (`%ProgramData%\GOG.com\Galaxy\storage\galaxy-2.0.db`, SQLite), aberto só para leitura. Mesmas tabelas que o
//! gog-galaxy-export-script (`galaxy_library_export.py`) usa: `ProductPurchaseDates` são os jogos da conta
//! (`gameReleaseKey`), e `GamePieces` + `GamePieceTypes` dão o título (`{"title":"…"}`) e as DLCs de cada jogo
//! (`{"dlcs":["gog_…"]}`); `originalImages` (`{"squareIcon":"https://…"}`) dá o ícone quadrado. O Galaxy junta outras lojas no mesmo banco; aqui só entram as chaves `gog_<id>`.

use super::ACCENT;
use rusqlite::{Connection, OpenFlags};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tako_core::library::{Game, GameStore, media};

/// O banco cresce com a biblioteca de todas as lojas ligadas ao Galaxy; acima disso não é um banco do Galaxy.
const MAX_DB: u64 = 1 << 30;

/// Jogo da conta, como o Galaxy registra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GogOwned {
    pub game_id: String,
    pub title: String,
    /// `squareIcon` do Galaxy, para o ícone do jogo.
    pub icon: Option<String>,
}

/// Endereço de imagem aceito: `https`, de tamanho razoável (vira nome de arquivo no cache de artes).
fn https_url(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|u| u.starts_with("https://") && u.len() <= 2048)
        .map(str::to_owned)
}

/// Jogos GOG da conta, sem DLCs. Banco inválido ou sem as tabelas é `Err`.
pub fn read(database: &Path) -> Result<Vec<GogOwned>, rusqlite::Error> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(database, flags)?;
    let mut stmt = conn.prepare(
        "SELECT DISTINCT p.gameReleaseKey, t.type, g.value FROM ProductPurchaseDates p \
         JOIN GamePieces g ON g.releaseKey = p.gameReleaseKey \
         JOIN GamePieceTypes t ON t.id = g.gamePieceTypeId \
         WHERE t.type IN ('title', 'dlcs', 'originalImages')",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut titles: HashMap<String, String> = HashMap::new();
    let mut icons: HashMap<String, String> = HashMap::new();
    let mut dlcs: HashSet<String> = HashSet::new();
    for row in rows {
        let (Some(key), Some(kind), Some(value)) = row? else {
            continue;
        };
        // Valor fora do formato: o jogo fica sem título (e fora da lista), os outros seguem.
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&value) else {
            continue;
        };
        match kind.as_str() {
            "title" => {
                if let Some(t) = value["title"]
                    .as_str()
                    .map(str::trim)
                    .filter(|t| !t.is_empty())
                {
                    titles.entry(key).or_insert_with(|| t.to_owned());
                }
            }
            "originalImages" => {
                if let Some(icon) = https_url(&value["squareIcon"]) {
                    icons.entry(key).or_insert(icon);
                }
            }
            _ => dlcs.extend(
                value["dlcs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|d| d.as_str().map(str::to_owned)),
            ),
        }
    }
    let mut out: Vec<GogOwned> = titles
        .into_iter()
        .filter(|(key, _)| !dlcs.contains(key))
        .filter_map(|(key, title)| {
            let id = key.strip_prefix("gog_")?;
            (!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).then(|| GogOwned {
                game_id: id.to_owned(),
                icon: icons.get(&key).cloned(),
                title,
            })
        })
        .collect();
    out.sort_by(|a, b| a.game_id.cmp(&b.game_id));
    Ok(out)
}

/// O banco no disco; ausente, grande demais ou ilegível = nenhum.
pub fn load(database: &Path) -> Vec<GogOwned> {
    match std::fs::metadata(database) {
        Ok(m) if m.len() <= MAX_DB => {}
        Ok(_) => {
            tracing::warn!("banco do GOG Galaxy grande demais: {}", database.display());
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    }
    read(database).unwrap_or_else(|e| {
        tracing::warn!("banco do GOG Galaxy ilegível ({}): {e}", database.display());
        Vec::new()
    })
}

/// Abre o jogo no GOG Galaxy, onde fica o botão de instalar (o mesmo endereço do plugin GOG do Playnite).
pub fn install_uri(game_id: &str) -> String {
    format!("goggalaxy://openGameView/{game_id}")
}

/// Jogo não instalado com o id do instalado (`gog:<id>`), para virar instalado no lugar.
pub fn to_game(o: &GogOwned) -> Game {
    Game {
        id: super::id_for(&o.game_id),
        title: o.title.clone(),
        store: Some(GameStore::Gog),
        accent: ACCENT.to_owned(),
        is_installed: false,
        install_uri: Some(install_uri(&o.game_id)),
        media: o
            .icon
            .iter()
            .map(|i| (media::ICON.to_owned(), i.clone()))
            .collect(),
        icon_origin: o
            .icon
            .as_ref()
            .map(|_| tako_core::library::IconOrigin::Store),
        ..Default::default()
    }
}

/// Ícone do Galaxy também nos jogos instalados (o registro do GOG não traz imagens).
pub fn add_icons(games: &mut [Game], owned: &[GogOwned]) {
    for game in games {
        if let Some(icon) = owned
            .iter()
            .find(|o| game.id == super::id_for(&o.game_id))
            .and_then(|o| o.icon.as_ref())
            && !game.media.contains_key(media::ICON)
        {
            game.media.insert(media::ICON.to_owned(), icon.clone());
            game.icon_origin = Some(tako_core::library::IconOrigin::Store);
        }
    }
}

/// Os da conta que não estão em `installed`.
pub fn not_installed(owned: &[GogOwned], installed: &[Game]) -> Vec<Game> {
    owned
        .iter()
        .map(to_game)
        .filter(|g| !installed.iter().any(|i| i.id == g.id))
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Um `galaxy-2.0.db` com as tabelas e colunas que a leitura usa, como o Galaxy 2 grava.
    pub(crate) fn galaxy_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE GamePieceTypes (id INTEGER PRIMARY KEY AUTOINCREMENT, type TEXT UNIQUE NOT NULL);
            CREATE TABLE GamePieces (releaseKey TEXT NOT NULL, gamePieceTypeId INTEGER NOT NULL,
                userId INTEGER NOT NULL, value TEXT NOT NULL);
            CREATE TABLE ProductPurchaseDates (gameReleaseKey TEXT NOT NULL, userId INTEGER NOT NULL,
                purchaseDate TEXT NOT NULL);
            INSERT INTO GamePieceTypes (id, type) VALUES (1, 'originalTitle'), (2, 'title'), (3, 'dlcs'),
                (4, 'allGameReleases'), (5, 'originalImages');
            INSERT INTO ProductPurchaseDates VALUES
                ('gog_1400000001', 1, '2020-01-01 10:00:00'),
                ('gog_1400000002', 1, '2020-01-01 10:00:00'),
                ('gog_1207658924', 1, '2019-06-01 10:00:00'),
                ('steam_570', 1, '2018-01-01 10:00:00'),
                ('gog_1500000000', 1, '2021-01-01 10:00:00'),
                ('gog_1600000000', 1, '2021-01-01 10:00:00');
            INSERT INTO GamePieces VALUES
                ('gog_1400000001', 2, 1, '{"title":"Cidade Submersa"}'),
                ('gog_1400000001', 1, 1, '{"title":"Cidade Submersa (original)"}'),
                ('gog_1400000001', 3, 1, '{"dlcs":["gog_1400000002"]}'),
                ('gog_1400000002', 2, 1, '{"title":"Cidade Submersa - Expansão"}'),
                ('gog_1400000002', 3, 1, '{"dlcs":null}'),
                ('gog_1400000001', 5, 1, '{"background":"https://images.gog.com/b.jpg","squareIcon":"https://images.gog.com/cidade_square.png"}'),
                ('gog_1207658924', 5, 1, '{"squareIcon":"http://inseguro/x.png"}'),
                ('steam_570', 5, 1, 'quebrado'),
                ('gog_1207658924', 2, 1, '{"title":" Tabuleiro Antigo "}'),
                ('gog_1207658924', 3, 1, '{"dlcs":[]}'),
                ('steam_570', 2, 1, '{"title":"Dota 2"}'),
                ('gog_1500000000', 2, 1, 'quebrado'),
                ('gog_1600000000', 2, 1, '{"title":null}');
            "#,
        )
        .unwrap();
    }

    #[test]
    fn reads_gog_games_of_the_account_without_dlc_or_other_stores() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("galaxy-2.0.db");
        galaxy_db(&db);
        assert_eq!(
            read(&db).unwrap(),
            [
                GogOwned {
                    game_id: "1207658924".into(),
                    title: "Tabuleiro Antigo".into(),
                    // `http` não vale.
                    icon: None,
                },
                GogOwned {
                    game_id: "1400000001".into(),
                    title: "Cidade Submersa".into(),
                    icon: Some("https://images.gog.com/cidade_square.png".into()),
                },
            ]
        );
    }

    #[test]
    fn installed_games_get_the_galaxy_icon_too() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("galaxy-2.0.db");
        galaxy_db(&db);
        let mut games = [Game {
            id: "gog:1400000001".into(),
            ..Default::default()
        }];
        add_icons(&mut games, &load(&db));
        assert_eq!(
            games[0].media["icon"],
            "https://images.gog.com/cidade_square.png"
        );
    }

    #[test]
    fn broken_database_is_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("galaxy-2.0.db");
        std::fs::write(&bad, "isto não é um banco SQLite").unwrap();
        assert!(read(&bad).is_err());
        assert!(load(&bad).is_empty());
        let empty = dir.path().join("vazio.db");
        Connection::open(&empty).unwrap();
        assert!(load(&empty).is_empty());
        assert!(load(&dir.path().join("nao-existe.db")).is_empty());
    }

    #[test]
    fn install_opens_the_game_in_galaxy_with_the_same_id_as_the_installed_game() {
        let g = to_game(&GogOwned {
            game_id: "1207658924".into(),
            title: "Tabuleiro".into(),
            icon: Some("https://images.gog.com/t.png".into()),
        });
        assert_eq!(g.media["icon"], "https://images.gog.com/t.png");
        assert_eq!(g.id, "gog:1207658924");
        assert_eq!(
            g.install_uri.as_deref(),
            Some("goggalaxy://openGameView/1207658924")
        );
        assert!(!g.can_play());
    }
}
