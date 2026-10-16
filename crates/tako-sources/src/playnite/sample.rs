//! Biblioteca do Playnite fictícia (port de `tests/Shared/PlayniteSample.cs`): um jogo adicionado à
//! mão, um da Steam, um da Epic que o Tako não tem, um por script, um não instalado, uma ROM de SNES,
//! um link e um com descrição longa (página de extensão).

use super::litedb::{Bson, Document, writer};
use std::path::Path;
use uuid::{Uuid, uuid};

pub const STEAM_PLUGIN: Uuid = uuid!("cb91dfc9-b977-43bf-8e70-55f46e410fab");
pub const EPIC_PLUGIN: Uuid = uuid!("00000002-dbd1-46c6-b5d0-b1ba559d10e4");

pub const LANTERNA: Uuid = uuid!("11111111-1111-1111-1111-111111111111");
pub const FAROL: Uuid = uuid!("22222222-2222-2222-2222-222222222222");
pub const CORRIDA: Uuid = uuid!("33333333-3333-3333-3333-333333333333");
pub const ROTEIRO: Uuid = uuid!("44444444-4444-4444-4444-444444444444");
pub const AUSENTE: Uuid = uuid!("55555555-5555-5555-5555-555555555555");
pub const CARTUCHO: Uuid = uuid!("66666666-6666-6666-6666-666666666666");
pub const NAVEGADOR: Uuid = uuid!("77777777-7777-7777-7777-777777777777");
pub const LONGO: Uuid = uuid!("88888888-8888-8888-8888-888888888888");

pub const PC: Uuid = uuid!("a0000000-0000-0000-0000-000000000001");
pub const SNES: Uuid = uuid!("a0000000-0000-0000-0000-000000000002");
pub const SOFA: Uuid = uuid!("b0000000-0000-0000-0000-000000000001");
pub const ZERADOS: Uuid = uuid!("b0000000-0000-0000-0000-000000000002");

pub fn ms(y: i32, m: u32, d: u32, h: u32) -> i64 {
    chrono::NaiveDate::from_ymd_opt(y, m, d)
        .and_then(|d| d.and_hms_opt(h, 0, 0))
        .map(|t| t.and_utc().timestamp_millis())
        .unwrap_or_default()
}

fn s(v: &str) -> Bson {
    Bson::String(v.into())
}

fn opt(v: Option<&str>) -> Bson {
    v.map_or(Bson::Null, s)
}

pub fn doc(pairs: Vec<(&str, Bson)>) -> Document {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

fn action(
    kind: &str,
    path: Option<&str>,
    arguments: Option<&str>,
    working_dir: Option<&str>,
) -> Bson {
    Bson::Document(doc(vec![
        ("Type", s(kind)),
        ("Path", opt(path)),
        ("Arguments", opt(arguments)),
        ("WorkingDir", opt(working_dir)),
        ("IsPlayAction", Bson::Bool(true)),
        ("Name", s("Jogar")),
    ]))
}

#[derive(Default)]
struct G<'a> {
    plugin: Option<Uuid>,
    game_id: Option<&'a str>,
    install_dir: Option<&'a str>,
    not_installed: bool,
    playtime: Option<Bson>,
    favorite: bool,
    platform: Option<Uuid>,
    categories: Vec<Uuid>,
    action: Option<Bson>,
    roms: Vec<Bson>,
    added: Option<i64>,
    last_activity: Option<i64>,
    description: Option<String>,
}

fn game(id: Uuid, name: &str, g: G) -> Document {
    let list = |v: Vec<Bson>| {
        if v.is_empty() {
            Bson::Null
        } else {
            Bson::Array(v)
        }
    };
    doc(vec![
        ("_id", Bson::Guid(id)),
        ("Name", s(name)),
        ("PluginId", Bson::Guid(g.plugin.unwrap_or_default())),
        ("GameId", opt(g.game_id)),
        ("InstallDirectory", opt(g.install_dir)),
        ("IsInstalled", Bson::Bool(!g.not_installed)),
        ("Hidden", Bson::Bool(false)),
        ("Favorite", Bson::Bool(g.favorite)),
        ("Playtime", g.playtime.unwrap_or(Bson::Int64(0))),
        (
            "PlatformIds",
            list(g.platform.into_iter().map(Bson::Guid).collect()),
        ),
        (
            "CategoryIds",
            list(g.categories.into_iter().map(Bson::Guid).collect()),
        ),
        ("GameActions", list(g.action.into_iter().collect())),
        ("Roms", list(g.roms)),
        ("Added", g.added.map_or(Bson::Null, Bson::DateTime)),
        (
            "LastActivity",
            g.last_activity.map_or(Bson::Null, Bson::DateTime),
        ),
        (
            "Description",
            g.description.map_or(Bson::Null, Bson::String),
        ),
    ])
}

pub fn games() -> Vec<Document> {
    vec![
        game(
            LANTERNA,
            "Lanterna Distante",
            G {
                install_dir: Some(r"C:\Jogos\Lanterna"),
                playtime: Some(Bson::Int64(7200)),
                favorite: true,
                platform: Some(PC),
                categories: vec![SOFA, ZERADOS],
                action: Some(action(
                    "File",
                    Some(r"{InstallDir}\lanterna.exe"),
                    Some("-tela-cheia"),
                    Some("{InstallDir}"),
                )),
                added: Some(ms(2025, 3, 1, 12)),
                last_activity: Some(ms(2026, 9, 30, 20)),
                ..Default::default()
            },
        ),
        game(
            FAROL,
            "Farol",
            G {
                plugin: Some(STEAM_PLUGIN),
                game_id: Some("7"),
                install_dir: Some(r"C:\Steam\steamapps\common\Farol"),
                playtime: Some(Bson::Int32(600)),
                favorite: true,
                categories: vec![SOFA],
                ..Default::default()
            },
        ),
        game(
            CORRIDA,
            "Corrida Noturna",
            G {
                plugin: Some(EPIC_PLUGIN),
                game_id: Some("CorridaNoturna"),
                install_dir: Some(r"D:\Epic\Corrida"),
                ..Default::default()
            },
        ),
        game(
            ROTEIRO,
            "Roteiro",
            G {
                install_dir: Some(r"C:\Jogos\Roteiro"),
                action: Some(action("Script", None, None, None)),
                ..Default::default()
            },
        ),
        game(
            AUSENTE,
            "Ausente",
            G {
                not_installed: true,
                action: Some(action(
                    "File",
                    Some(r"C:\Jogos\Ausente\ausente.exe"),
                    None,
                    None,
                )),
                ..Default::default()
            },
        ),
        game(
            CARTUCHO,
            "Cartucho Azul",
            G {
                install_dir: Some(r"D:\ROMs\SNES"),
                platform: Some(SNES),
                action: Some(action("Emulator", None, None, None)),
                roms: vec![Bson::Document(doc(vec![
                    ("Name", s("Cartucho Azul")),
                    ("Path", s(r"{InstallDir}\azul.sfc")),
                ]))],
                ..Default::default()
            },
        ),
        game(
            NAVEGADOR,
            "Jogo no Navegador",
            G {
                action: Some(action("URL", Some("https://example.com/jogo"), None, None)),
                ..Default::default()
            },
        ),
        game(
            LONGO,
            "Descrição Longa",
            G {
                install_dir: Some(r"C:\Jogos\Longo"),
                action: Some(action(
                    "File",
                    Some(r"C:\Jogos\Longo\longo.exe"),
                    None,
                    None,
                )),
                description: Some("x".repeat(9000)),
                ..Default::default()
            },
        ),
    ]
}

pub fn platforms() -> Vec<Document> {
    vec![
        doc(vec![
            ("_id", Bson::Guid(PC)),
            ("Name", s("PC (Windows)")),
            ("SpecificationId", s("pc_windows")),
        ]),
        doc(vec![
            ("_id", Bson::Guid(SNES)),
            ("Name", s("Super Nintendo")),
            ("SpecificationId", s("nintendo_super_nes")),
        ]),
    ]
}

pub fn categories() -> Vec<Document> {
    vec![
        doc(vec![
            ("_id", Bson::Guid(SOFA)),
            ("Name", s("Jogar no sofá")),
        ]),
        doc(vec![("_id", Bson::Guid(ZERADOS)), ("Name", s("Zerados"))]),
    ]
}

/// Grava `games.db`, `platforms.db`, `categories.db` e `sources.db` na pasta.
pub fn write(folder: &Path) {
    let w = |name: &str, collection: &str, docs: Vec<Document>| {
        std::fs::write(folder.join(name), writer::write(&[(collection, docs)])).unwrap_or_default();
    };
    std::fs::create_dir_all(folder).unwrap_or_default();
    w("games.db", "Game", games());
    w("platforms.db", "Platform", platforms());
    w("categories.db", "Category", categories());
    w(
        "sources.db",
        "GameSource",
        vec![doc(vec![
            (
                "_id",
                Bson::Guid(uuid!("c0000000-0000-0000-0000-000000000001")),
            ),
            ("Name", s("Steam")),
        ])],
    );
}
