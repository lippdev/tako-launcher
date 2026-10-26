//! Port de `ScreenScraperParser.cs`, `ScreenScraperSystems.cs` e `RomTitle.cs` (ADR-0022): respostas
//! JSON da API v2 → metadados, tabela de sistemas e título a partir do nome da ROM (puros, testados).

use super::art::ArtCandidate;
use super::igdb_parse::translate;
use super::screenscraper::media_from_response;
use super::{GameMetadata, MetadataMatch, sources};
use crate::i18n::{self, Lang, t};
use crate::library::{ArtKind, media};
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Caixa 2D (vira a capa; não fica em `media`).
pub const BOX_2D: &str = "box2d";

/// Regiões do nome, na ordem de preferência: Brasil, Portugal, EUA, mundo, Europa, ScreenScraper, Japão.
pub const NAME_REGIONS: [&str; 7] = ["br", "pt", "us", "wor", "eu", "ss", "jp"];
/// Regiões das mídias e da data.
pub const MEDIA_REGIONS: [&str; 6] = ["br", "us", "wor", "eu", "ss", "jp"];

// Tipo de mídia do ScreenScraper → chave em `media` (os primeiros têm preferência).
const MEDIA_TYPES: [(&str, &[&str]); 7] = [
    (BOX_2D, &["box-2D"]),
    (media::BOX_3D, &["box-3D"]),
    (media::LOGO, &["wheel-hd", "wheel"]),
    (
        media::MARQUEE,
        &["screenmarquee", "marquee", "screenmarqueesmall"],
    ),
    (media::SCREENSHOT, &["ss", "sstitle"]),
    (media::FANART, &["fanart"]),
    (media::VIDEO, &["video-normalized", "video"]),
];

/// Idiomas da sinopse e dos gêneros: o da interface primeiro.
fn languages() -> [&'static str; 2] {
    match i18n::lang() {
        Lang::PtBr => ["pt", "en"],
        Lang::EnUs => ["en", "pt"],
    }
}

// O ScreenScraper manda números como texto ("id": "3"); aceita os dois.
fn text(v: &Value, k: &str) -> Option<String> {
    match v.get(k)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn int(v: &Value, k: &str) -> Option<i64> {
    text(v, k)?.trim().parse().ok()
}

fn array<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// `response` de uma resposta completa; `None` se não for JSON ou o cabeçalho disser que falhou.
pub fn envelope(json: &str) -> Option<Value> {
    let root: Value = serde_json::from_str(json).ok()?;
    if let Some(s) = root.get("header").and_then(|h| text(h, "success"))
        && !s.eq_ignore_ascii_case("true")
    {
        return None;
    }
    root.get("response").filter(|r| r.is_object()).cloned()
}

/// Conta do usuário como o ScreenScraper informa (`ssuser`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScreenScraperAccount {
    pub user: String,
    pub level: i64,
    pub max_threads: i64,
    pub requests_today: i64,
    pub max_requests_per_day: i64,
    pub ko_today: i64,
    pub max_ko_per_day: i64,
}

impl ScreenScraperAccount {
    pub fn remaining_today(&self) -> i64 {
        (self.max_requests_per_day - self.requests_today).max(0)
    }

    /// Cota do dia esgotada (consultas ou ROMs não reconhecidas).
    pub fn is_exhausted(&self) -> bool {
        (self.max_requests_per_day > 0 && self.requests_today >= self.max_requests_per_day)
            || (self.max_ko_per_day > 0 && self.ko_today >= self.max_ko_per_day)
    }
}

pub fn account(response: &Value) -> Option<ScreenScraperAccount> {
    let u = response.get("ssuser").filter(|u| u.is_object())?;
    let user = text(u, "id").filter(|s| !s.is_empty())?;
    Some(ScreenScraperAccount {
        user,
        level: int(u, "niveau").unwrap_or(0),
        max_threads: int(u, "maxthreads").unwrap_or(1),
        requests_today: int(u, "requeststoday").unwrap_or(0),
        max_requests_per_day: int(u, "maxrequestsperday").unwrap_or(0),
        ko_today: int(u, "requestskotoday").unwrap_or(0),
        max_ko_per_day: int(u, "maxrequestskoperday").unwrap_or(0),
    })
}

/// O jogo de um `jeuInfos.php`.
pub fn game_info(response: &Value, now: DateTime<Utc>) -> Option<GameMetadata> {
    game(response.get("jeu")?, now)
}

/// Resultados de um `jeuRecherche.php`. Sem resultados, a API devolve `[{}]`.
pub fn search(response: &Value) -> Vec<MetadataMatch> {
    array(response, "jeux")
        .iter()
        .filter_map(match_of)
        .collect()
}

pub fn game(jeu: &Value, now: DateTime<Utc>) -> Option<GameMetadata> {
    let id = text(jeu, "id").filter(|s| !s.is_empty())?;
    let title = title(jeu)?;
    let synopsis = by_language(array(jeu, "synopsis"));
    let mut media = media_of(jeu);
    let cover = media.remove(BOX_2D);
    let field = |k: &str| jeu.get(k).and_then(|v| text(v, "text"));
    Some(GameMetadata {
        fetched_at: Some(now),
        source_ids: [(sources::SCREENSCRAPER.to_owned(), id)].into(),
        title: Some(title),
        description: synopsis.as_ref().map(|s| s.1.clone()),
        description_language: synopsis
            .map(|(l, _)| if l == "pt" { "pt-BR".into() } else { l.into() }),
        genres: genres(jeu).into_iter().take(3).collect(),
        release_date: release_date(jeu),
        developer: field("developpeur"),
        publisher: field("editeur"),
        // Nota de 0 a 20 no ScreenScraper.
        critic_score: jeu
            .get("note")
            .and_then(|n| int(n, "text"))
            .filter(|s| (0..=20).contains(s))
            .map(|s| s as i32 * 5),
        players: field("joueurs"),
        cover_url: cover,
        // Fundo: fanart; sem ela, a captura de tela.
        background_url: media
            .get(media::FANART)
            .or_else(|| media.get(media::SCREENSHOT))
            .cloned(),
        media,
        ..GameMetadata::new(sources::SCREENSCRAPER)
    })
}

/// Resultado para "Corrigir correspondência": nome, ano e sistema.
pub fn match_of(jeu: &Value) -> Option<MetadataMatch> {
    let id = text(jeu, "id").filter(|s| !s.is_empty())?;
    let mut m = MetadataMatch::new(sources::SCREENSCRAPER, id, title(jeu)?);
    m.platforms = jeu.get("systeme").and_then(|s| {
        int(s, "id")
            .and_then(system_by_screenscraper_id)
            .map(|k| k.name.to_owned())
            .or_else(|| text(s, "text"))
    });
    m.year = release_date(jeu)
        .map(|d| chrono::Datelike::year(&d))
        .or_else(|| {
            array(jeu, "dates")
                .iter()
                .filter_map(|d| text(d, "text"))
                .find_map(|t| t.get(..4).and_then(|y| y.parse().ok()))
        });
    m.cover_url = media_of(jeu).remove(BOX_2D);
    Some(m)
}

/// Nome pela região preferida; senão, o primeiro.
pub fn title(jeu: &Value) -> Option<String> {
    let names: Vec<(Option<String>, String)> = array(jeu, "noms")
        .iter()
        .filter_map(|n| {
            Some((
                text(n, "region"),
                text(n, "text").filter(|t| !t.is_empty())?,
            ))
        })
        .collect();
    if names.is_empty() {
        return text(jeu, "nom");
    }
    NAME_REGIONS
        .iter()
        .find_map(|r| {
            names
                .iter()
                .find(|(reg, _)| reg.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(r)))
        })
        .or(names.first())
        .map(|(_, t)| t.clone())
}

/// (tipo, região, url sem credenciais) das mídias.
fn medias(jeu: &Value) -> Vec<(String, Option<String>, String)> {
    array(jeu, "medias")
        .iter()
        .filter_map(|m| {
            Some((
                text(m, "type")?,
                text(m, "region"),
                media_from_response(&text(m, "url")?)?,
            ))
        })
        .collect()
}

fn region_rank(region: Option<&str>) -> usize {
    MEDIA_REGIONS
        .iter()
        .position(|r| region.is_some_and(|x| x.eq_ignore_ascii_case(r)))
        .unwrap_or(MEDIA_REGIONS.len())
}

/// Mídias por tipo, cada uma na região preferida, sem credenciais na URL.
pub fn media_of(jeu: &Value) -> BTreeMap<String, String> {
    let items = medias(jeu);
    let mut out = BTreeMap::new();
    for (kind, types) in MEDIA_TYPES {
        for ty in types {
            let of_type: Vec<_> = items
                .iter()
                .filter(|m| m.0.eq_ignore_ascii_case(ty))
                .collect();
            if let Some(best) = of_type.iter().min_by_key(|m| region_rank(m.1.as_deref())) {
                out.insert(kind.to_owned(), best.2.clone());
                break;
            }
        }
    }
    out
}

// Tipo de mídia → tipo de imagem do seletor e o rótulo.
const PICKER_TYPES: [(&str, ArtKind, &str); 7] = [
    ("box-2D", ArtKind::Cover, "box_2d"),
    ("box-3D", ArtKind::Cover, "box_3d"),
    ("fanart", ArtKind::Hero, "fanart"),
    ("ss", ArtKind::Hero, "screenshot"),
    ("sstitle", ArtKind::Hero, "title_screen"),
    ("wheel-hd", ArtKind::Logo, "logo_hd"),
    ("wheel", ArtKind::Logo, "logo"),
];

/// Todas as mídias do jogo para o seletor ("Caixa 2D · EUA"), sem credenciais na URL.
pub fn media_candidates(jeu: &Value) -> Vec<ArtCandidate> {
    let items = medias(jeu);
    let mut out: Vec<ArtCandidate> = Vec::new();
    for (ty, kind, label) in PICKER_TYPES {
        let mut of_type: Vec<_> = items
            .iter()
            .filter(|m| m.0.eq_ignore_ascii_case(ty))
            .collect();
        of_type.sort_by_key(|m| region_rank(m.1.as_deref()));
        for (_, region, url) in of_type {
            if out.iter().any(|c| &c.url == url) {
                continue;
            }
            let label = t(&format!("metadata.art.ss.{label}"));
            out.push(ArtCandidate {
                label: Some(match region {
                    Some(r) => format!("{label} · {}", region_name(r)),
                    None => label,
                }),
                ..ArtCandidate::new(sources::SCREENSCRAPER, kind, url.clone())
            });
        }
    }
    out
}

/// Nome da região para a tela.
pub fn region_name(region: &str) -> String {
    let r = region.to_lowercase();
    match r.as_str() {
        "br" | "pt" | "us" | "wor" | "eu" | "jp" | "fr" | "de" | "uk" | "kr" | "cn" | "asi"
        | "au" => t(&format!("metadata.region.{r}")),
        "ss" => "ScreenScraper".into(),
        _ => region.to_uppercase(),
    }
}

fn release_date(jeu: &Value) -> Option<NaiveDate> {
    let dates: Vec<(Option<String>, NaiveDate)> = array(jeu, "dates")
        .iter()
        .filter_map(|d| Some((text(d, "region"), parse_date(&text(d, "text")?)?)))
        .collect();
    MEDIA_REGIONS
        .iter()
        .find_map(|r| {
            dates
                .iter()
                .find(|(reg, _)| reg.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(r)))
                .map(|(_, d)| *d)
        })
        .or_else(|| dates.iter().map(|(_, d)| *d).min())
}

/// "1991-11-21", "1991-11" ou "1991".
pub fn parse_date(text: &str) -> Option<NaiveDate> {
    let parts: Vec<&str> = text.trim().split('-').collect();
    let num = |s: &str, len: usize| {
        (s.len() == len && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse::<u32>().ok())
            .flatten()
    };
    match parts.as_slice() {
        [y] => NaiveDate::from_ymd_opt(num(y, 4)? as i32, 1, 1),
        [y, m] => NaiveDate::from_ymd_opt(num(y, 4)? as i32, num(m, 2)?, 1),
        [y, m, d] => NaiveDate::from_ymd_opt(num(y, 4)? as i32, num(m, 2)?, num(d, 2)?),
        _ => None,
    }
}

fn genres(jeu: &Value) -> Vec<String> {
    array(jeu, "genres")
        .iter()
        .filter_map(|g| by_language(array(g, "noms")))
        .map(|(lang, name)| if lang == "en" { translate(&name) } else { name })
        .collect()
}

/// Texto no idioma da interface; senão, no outro.
fn by_language(items: &[Value]) -> Option<(&'static str, String)> {
    let texts: Vec<(Option<String>, String)> = items
        .iter()
        .filter_map(|i| {
            let t = text(i, "text")?.trim().to_owned();
            (!t.is_empty()).then(|| (text(i, "langue"), t))
        })
        .collect();
    languages().into_iter().find_map(|lang| {
        texts
            .iter()
            .find(|(l, _)| l.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(lang)))
            .map(|(_, t)| (lang, t.clone()))
    })
}

// ---------------------------------------------------------------- Sistemas

/// Um sistema que o Tako conhece: chave em `Game::platform`, nome e id no ScreenScraper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmulatedSystem {
    /// A mesma de `igdb_parse::PLATFORMS` ("snes", "ps2").
    pub key: &'static str,
    pub name: &'static str,
    pub screenscraper_id: i64,
    /// Extensões típicas (minúsculas, com ponto).
    pub extensions: &'static [&'static str],
    /// O .zip é a própria ROM (MAME/Neo Geo): o hash é do .zip.
    pub archive_is_rom: bool,
}

const fn sys(
    key: &'static str,
    name: &'static str,
    id: i64,
    extensions: &'static [&'static str],
) -> EmulatedSystem {
    EmulatedSystem {
        key,
        name,
        screenscraper_id: id,
        extensions,
        archive_is_rom: false,
    }
}

pub const ARCHIVE_EXTENSIONS: [&str; 2] = [".zip", ".7z"];
/// Imagens de disco compactadas: o hash do contêiner não identifica o jogo.
pub const COMPRESSED_DISC_EXTENSIONS: [&str; 9] = [
    ".chd", ".rvz", ".wia", ".gcz", ".cso", ".wbfs", ".pbp", ".nsz", ".xcz",
];
const NOT_ROMS: [&str; 5] = [".exe", ".lnk", ".url", ".bat", ".cmd"];
const CARTRIDGE: [&str; 13] = [
    "nes",
    "snes",
    "n64",
    "gb",
    "gbc",
    "gba",
    "nds",
    "mastersystem",
    "megadrive",
    "32x",
    "gamegear",
    "pcengine",
    "atari2600",
];

/// Em ordem de exibição; ids conferidos com `systemesListe.php`.
pub const SYSTEMS: [EmulatedSystem; 30] = [
    sys("nes", "NES", 3, &[".nes", ".fds", ".unf", ".unif"]),
    sys("snes", "SNES", 4, &[".sfc", ".smc", ".fig", ".swc", ".bs"]),
    sys("n64", "Nintendo 64", 14, &[".n64", ".z64", ".v64"]),
    sys(
        "gamecube",
        "GameCube",
        13,
        &[".gcm", ".iso", ".rvz", ".gcz", ".ciso"],
    ),
    sys("wii", "Wii", 16, &[".wbfs", ".iso", ".rvz", ".wia", ".wad"]),
    sys("wiiu", "Wii U", 18, &[".wud", ".wux", ".rpx", ".wua"]),
    sys(
        "switch",
        "Nintendo Switch",
        225,
        &[".nsp", ".xci", ".nsz", ".xcz"],
    ),
    sys("gb", "Game Boy", 9, &[".gb"]),
    sys("gbc", "Game Boy Color", 10, &[".gbc"]),
    sys("gba", "Game Boy Advance", 12, &[".gba"]),
    sys("nds", "Nintendo DS", 15, &[".nds", ".dsi"]),
    sys(
        "3ds",
        "Nintendo 3DS",
        17,
        &[".3ds", ".cia", ".cci", ".cxi", ".3dsx"],
    ),
    sys("mastersystem", "Master System", 2, &[".sms"]),
    sys(
        "megadrive",
        "Mega Drive",
        1,
        &[".md", ".gen", ".smd", ".bin"],
    ),
    sys("segacd", "Sega CD", 20, &[".cue", ".chd", ".iso"]),
    sys("32x", "Sega 32X", 19, &[".32x"]),
    sys("gamegear", "Game Gear", 21, &[".gg"]),
    sys("saturn", "Saturn", 22, &[".cue", ".chd", ".iso", ".mdf"]),
    sys("dreamcast", "Dreamcast", 23, &[".gdi", ".cdi", ".chd"]),
    sys(
        "ps1",
        "PlayStation",
        57,
        &[".cue", ".chd", ".pbp", ".iso", ".m3u", ".ecm"],
    ),
    sys("ps2", "PlayStation 2", 58, &[".iso", ".chd", ".cso"]),
    sys("ps3", "PlayStation 3", 59, &[".pkg", ".iso"]),
    sys("psp", "PSP", 61, &[".iso", ".cso", ".pbp", ".chd"]),
    sys("psvita", "PS Vita", 62, &[".vpk"]),
    sys("xbox", "Xbox", 32, &[".iso", ".xiso"]),
    sys("xbox360", "Xbox 360", 33, &[".iso", ".xex"]),
    EmulatedSystem {
        archive_is_rom: true,
        ..sys("arcade", "Arcade (MAME)", 75, &[".zip", ".7z"])
    },
    EmulatedSystem {
        archive_is_rom: true,
        ..sys("neogeo", "Neo Geo", 142, &[".zip", ".7z"])
    },
    sys("pcengine", "PC Engine", 31, &[".pce", ".sgx"]),
    sys("atari2600", "Atari 2600", 26, &[".a26"]),
];

/// Sistema pela chave do Tako; `None` para PC ou chave desconhecida.
pub fn system_by_key(key: Option<&str>) -> Option<&'static EmulatedSystem> {
    let key = key?.trim();
    SYSTEMS.iter().find(|s| s.key.eq_ignore_ascii_case(key))
}

pub fn system_by_screenscraper_id(id: i64) -> Option<&'static EmulatedSystem> {
    SYSTEMS.iter().find(|s| s.screenscraper_id == id)
}

/// Extensão em minúsculas, com ponto ("" sem extensão).
pub fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default()
}

/// Sistemas prováveis pela extensão. Um .zip volta o arcade, o Neo Geo e os de cartucho.
pub fn candidates_for(path: &str) -> Vec<&'static EmulatedSystem> {
    let ext = extension_of(path);
    if ext.is_empty() {
        return Vec::new();
    }
    let mut direct: Vec<&EmulatedSystem> = SYSTEMS
        .iter()
        .filter(|s| s.extensions.contains(&ext.as_str()))
        .collect();
    if ARCHIVE_EXTENSIONS.contains(&ext.as_str()) {
        for s in SYSTEMS.iter().filter(|s| CARTRIDGE.contains(&s.key)) {
            if !direct.contains(&s) {
                direct.push(s);
            }
        }
    }
    direct
}

/// Arquivo com cara de ROM ou imagem de disco.
pub fn looks_like_rom(path: &str) -> bool {
    let ext = extension_of(path);
    let e = ext.as_str();
    !e.is_empty()
        && !NOT_ROMS.contains(&e)
        && (SYSTEMS.iter().any(|s| s.extensions.contains(&e))
            || COMPRESSED_DISC_EXTENSIONS.contains(&e))
}

/// Filtro do diálogo de arquivo ("*.sfc;*.smc;…").
pub fn file_dialog_pattern() -> String {
    let mut all: Vec<&str> = Vec::new();
    let every = SYSTEMS
        .iter()
        .flat_map(|s| s.extensions.iter().copied())
        .chain(COMPRESSED_DISC_EXTENSIONS)
        .chain(ARCHIVE_EXTENSIONS);
    for e in every {
        if !all.contains(&e) {
            all.push(e);
        }
    }
    all.iter()
        .map(|e| format!("*{e}"))
        .collect::<Vec<_>>()
        .join(";")
}

/// Título legível do nome da ROM: "Super Mario World (USA) [!].sfc" → "Super Mario World".
pub fn rom_title(path: &str) -> String {
    let name = Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().replace('_', " "))
        .unwrap_or_default();
    // Tira "(…)" e "[…]" com o espaço antes, como a regex `\s*[\(\[][^\)\]]*[\)\]]` do C#.
    let mut clean = String::with_capacity(name.len());
    let mut chars = name.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '(' || c == '[' {
            let rest = &name[i + 1..];
            if let Some(end) = rest.find([')', ']']) {
                let trimmed = clean.trim_end().len();
                clean.truncate(trimmed);
                clean.push(' ');
                let skip_to = i + 1 + end + 1;
                while chars.peek().is_some_and(|(j, _)| *j < skip_to) {
                    chars.next();
                }
                continue;
            }
        }
        clean.push(c);
    }
    let clean = clean
        .split(' ')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let clean = clean.trim_matches([' ', '-', '.']);
    if clean.is_empty() {
        name.trim().to_owned()
    } else {
        clean.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::igdb_parse::platform_ids;
    use crate::metadata::test_support::{fixture, now};

    fn response(name: &str) -> Value {
        envelope(&fixture(name)).unwrap()
    }

    #[test]
    fn systems_map_platform_keys_to_screenscraper_ids() {
        for (key, id) in [
            ("nes", 3),
            ("snes", 4),
            ("n64", 14),
            ("gamecube", 13),
            ("wii", 16),
            ("gb", 9),
            ("gbc", 10),
            ("gba", 12),
            ("nds", 15),
            ("3ds", 17),
            ("switch", 225),
            ("ps1", 57),
            ("ps2", 58),
            ("psp", 61),
            ("ps3", 59),
            ("xbox", 32),
            ("arcade", 75),
            ("neogeo", 142),
            ("pcengine", 31),
            ("atari2600", 26),
        ] {
            assert_eq!(system_by_key(Some(key)).unwrap().screenscraper_id, id);
            assert_eq!(
                system_by_key(Some(&key.to_uppercase()))
                    .unwrap()
                    .screenscraper_id,
                id
            );
            assert_eq!(system_by_screenscraper_id(id).unwrap().key, key);
        }
    }

    #[test]
    fn systems_share_keys_with_igdb_and_pc_is_not_a_system() {
        assert!(system_by_key(None).is_none());
        assert!(system_by_key(Some("pc")).is_none());
        assert!(system_by_screenscraper_id(-1).is_none());
        let mut keys: Vec<_> = SYSTEMS.iter().map(|s| s.key).collect();
        keys.dedup();
        assert_eq!(keys.len(), SYSTEMS.len());
        for s in SYSTEMS {
            assert!(!platform_ids(Some(s.key)).is_empty(), "{}", s.key);
            assert!(s.extensions.iter().all(|e| e.starts_with('.')));
        }
    }

    #[test]
    fn candidates_come_from_the_extension() {
        let keys = |p| candidates_for(p).iter().map(|s| s.key).collect::<Vec<_>>();
        assert_eq!(keys(r"D:\roms\Lanterna (USA).SFC"), ["snes"]);
        assert_eq!(keys("lanterna.gba"), ["gba"]);
        let zip = keys("lanterna.zip");
        assert_eq!(&zip[..2], ["arcade", "neogeo"]);
        assert!(zip.contains(&"snes"));
        assert!(!zip.contains(&"ps2"));
        assert!(keys("lanterna.cue").contains(&"ps1"));
        assert!(keys("lanterna.exe").is_empty());
        assert!(keys("lanterna").is_empty());
        assert!(looks_like_rom("lanterna.sfc"));
        assert!(looks_like_rom("lanterna.chd"));
        assert!(!looks_like_rom("lanterna.exe"));
        assert!(!looks_like_rom("lanterna.lnk"));
        assert!(!looks_like_rom("leia-me.txt"));
        assert!(file_dialog_pattern().contains("*.sfc"));
    }

    #[test]
    fn rom_titles_drop_dat_tags() {
        for (file, title) in [
            ("Super Lanterna World (USA).sfc", "Super Lanterna World"),
            ("Lanterna (Europe) (Rev 1) [!].zip", "Lanterna"),
            ("lanterna_distante_v1.gba", "lanterna distante v1"),
            ("(Beta).nes", "(Beta)"),
            ("Ação (Brasil) - Edição.sfc", "Ação - Edição"),
            ("sem fim (aberto.sfc", "sem fim (aberto"),
        ] {
            assert_eq!(rom_title(file), title, "{file}");
        }
    }

    #[test]
    fn game_info_prefers_brazil_ui_language_and_regional_media() {
        let g = game_info(&response("jeuInfos-lanterna.json"), now()).unwrap();
        assert_eq!(g.source, sources::SCREENSCRAPER);
        assert_eq!(g.id_in(sources::SCREENSCRAPER).as_deref(), Some("900077"));
        assert_eq!(g.title.as_deref(), Some("Lanterna Distante"));
        if i18n::lang() == Lang::PtBr {
            assert!(
                g.description
                    .as_deref()
                    .unwrap()
                    .starts_with("Um faroleiro")
            );
            assert_eq!(g.description_language.as_deref(), Some("pt-BR"));
            assert_eq!(
                g.genres,
                [translate("Platform"), "Quebra-cabeça".to_owned()]
            );
        }
        assert_eq!(g.release_date, NaiveDate::from_ymd_opt(1994, 8, 1));
        assert_eq!(g.developer.as_deref(), Some("Maré Baixa Games"));
        assert_eq!(g.publisher.as_deref(), Some("Estúdio Farol"));
        assert_eq!(g.critic_score, Some(80));
        assert_eq!(g.players.as_deref(), Some("1-2"));
        assert_eq!(
            g.cover_url.as_deref(),
            Some(
                "ssmedia://neoclone.screenscraper.fr/api2/mediaJeu.php?systemeid=4&jeuid=900077&media=box-2D(us)"
            )
        );
        assert_eq!(
            g.background_url.as_deref(),
            Some(
                "ssmedia://neoclone.screenscraper.fr/api2/mediaJeu.php?systemeid=4&jeuid=900077&media=fanart"
            )
        );
        assert!(g.media[media::BOX_3D].ends_with("media=box-3D(us)"));
        assert!(g.media[media::LOGO].ends_with("media=wheel-hd(wor)"));
        assert!(g.media[media::MARQUEE].ends_with("media=screenmarquee(wor)"));
        assert!(g.media[media::SCREENSHOT].ends_with("media=ss(wor)"));
        assert!(g.media[media::VIDEO].ends_with("media=video-normalized"));
        assert!(!g.media.contains_key(BOX_2D));
        let json = serde_json::to_string(&g).unwrap();
        for secret in ["SEGREDOFALSO", "senhafalsa", "DEVFALSO"] {
            assert!(!json.contains(secret));
        }
    }

    #[test]
    fn search_results_show_title_year_and_system() {
        let r = search(&response("jeuRecherche-lanterna.json"));
        assert_eq!(r.len(), 2);
        let mut first = MetadataMatch::new(sources::SCREENSCRAPER, "900077", "Lanterna Distante");
        first.year = Some(1994);
        first.platforms = Some("SNES".into());
        assert_eq!(r[0], first);
        assert_eq!(r[1].platforms.as_deref(), Some("Sistema Desconhecido"));
        assert_eq!(r[1].year, Some(1996));
        assert_eq!(
            r[0].label(),
            "Lanterna Distante (1994) · SNES · ScreenScraper"
        );
        assert!(search(&response("jeuRecherche-vazia.json")).is_empty());
    }

    #[test]
    fn account_and_envelope_are_read_defensively() {
        let a = account(&response("ssuserInfos.json")).unwrap();
        assert_eq!(
            a,
            ScreenScraperAccount {
                user: "jogador".into(),
                level: 1,
                max_threads: 1,
                requests_today: 20,
                max_requests_per_day: 20000,
                ko_today: 0,
                max_ko_per_day: 2000,
            }
        );
        assert_eq!(a.remaining_today(), 19980);
        assert!(!a.is_exhausted());
        assert!(
            ScreenScraperAccount {
                requests_today: 20000,
                ..a.clone()
            }
            .is_exhausted()
        );
        assert!(
            ScreenScraperAccount {
                ko_today: 2000,
                ..a
            }
            .is_exhausted()
        );
        assert!(envelope("Erreur : Rom/Iso/Dossier non trouvée !").is_none());
        assert!(envelope(r#"{"header":{"success":"false","error":"x"},"response":{}}"#).is_none());
        assert!(account(&envelope(r#"{"response":{}}"#).unwrap()).is_none());
        assert!(
            game_info(
                &envelope(r#"{"response":{"jeu":{"id":"1"}}}"#).unwrap(),
                now()
            )
            .is_none()
        );
        assert!(game_info(&envelope(r#"{"response":{"jeu":[1,2]}}"#).unwrap(), now()).is_none());
        assert_eq!(parse_date("1996"), NaiveDate::from_ymd_opt(1996, 1, 1));
        assert_eq!(parse_date("cedo"), None);
        assert_eq!(parse_date("1996-13"), None);
    }

    #[test]
    fn media_candidates_for_the_picker() {
        let jeu = serde_json::json!({"medias":[
          {"type":"box-2D","region":"jp","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?devid=x&devpassword=y&ssid=u&sspassword=p&media=box-2D(jp)"},
          {"type":"box-2D","region":"us","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?devid=x&media=box-2D(us)"},
          {"type":"wheel-hd","region":"wor","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?media=wheel-hd(wor)"},
          {"type":"fanart","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?media=fanart"},
          {"type":"video","url":"https://neoclone.screenscraper.fr/api2/mediaJeu.php?media=video"}
        ]});
        let m = media_candidates(&jeu);
        let labels: Vec<_> = m.iter().map(|c| c.label.clone().unwrap()).collect();
        let l = |k: &str| t(&format!("metadata.art.ss.{k}"));
        assert_eq!(
            labels,
            [
                format!("{} · {}", l("box_2d"), region_name("us")),
                format!("{} · {}", l("box_2d"), region_name("jp")),
                l("fanart"),
                format!("{} · {}", l("logo_hd"), region_name("wor")),
            ]
        );
        assert!(m.iter().all(|c| !c.url.contains("password")));
    }
}
