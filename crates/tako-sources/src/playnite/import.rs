//! Importação única da biblioteca do Playnite (port de `PlayniteImport.cs`), com prévia e confirmação.
//! Decide jogo a jogo sem tocar em arquivo: os de lojas que o Tako lê casam pelo id da loja ou pela
//! pasta (nunca duplicam); os demais instalados com ação de arquivo ou link entram como
//! "Adicionados"; ROMs entram com o sistema.

use super::library::{PlayniteActionType, PlayniteGame, PlayniteLibrary, PlaynitePlatform};
use crate::text::{fold, title_key};
use crate::windows_path;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use tako_core::i18n::{t, tf, tn};
use tako_core::library::{Game, GameStore};
use uuid::{Uuid, uuid};

/// Tamanho máximo do nome de coleção (`CollectionNames.MaxLength`).
const COLLECTION_MAX_LEN: usize = 40;

/// Plugins oficiais de biblioteca do Playnite ↔ lojas que o Tako lê.
pub const STORE_PLUGINS: [(Uuid, GameStore); 8] = [
    (
        uuid!("cb91dfc9-b977-43bf-8e70-55f46e410fab"),
        GameStore::Steam,
    ),
    (
        uuid!("00000002-dbd1-46c6-b5d0-b1ba559d10e4"),
        GameStore::Epic,
    ),
    (
        uuid!("aebe8b7c-6dc3-4a66-af31-e7375c6b5e9e"),
        GameStore::Gog,
    ),
    (
        uuid!("7e4fbb5e-2ae3-48d4-8ba0-6b30e7a4e287"),
        GameStore::Xbox,
    ),
    (
        uuid!("c2f038e5-8b92-4877-91f1-da9094155fc5"),
        GameStore::Ubisoft,
    ),
    (uuid!("85dd7072-2f20-4e76-a007-41035e390724"), GameStore::Ea),
    (
        uuid!("e3c26a3d-d695-4cb7-a769-5ff7612c7edd"),
        GameStore::BattleNet,
    ),
    (
        uuid!("402674cd-4af6-4886-b6ec-0e695bfa0688"),
        GameStore::Amazon,
    ),
];

/// Id de especificação de plataforma do Playnite → sistema do Tako (`Game::platform`).
pub const PLATFORM_KEYS: [(&str, &str); 28] = [
    ("nintendo_nes", "nes"),
    ("nintendo_super_nes", "snes"),
    ("nintendo_64", "n64"),
    ("nintendo_gamecube", "gamecube"),
    ("nintendo_wii", "wii"),
    ("nintendo_wiiu", "wiiu"),
    ("nintendo_switch", "switch"),
    ("nintendo_gameboy", "gb"),
    ("nintendo_gameboycolor", "gbc"),
    ("nintendo_gameboyadvance", "gba"),
    ("nintendo_ds", "nds"),
    ("nintendo_3ds", "3ds"),
    ("sega_mastersystem", "mastersystem"),
    ("sega_genesis", "megadrive"),
    ("sega_cd", "segacd"),
    ("sega_32x", "32x"),
    ("sega_gamegear", "gamegear"),
    ("sega_saturn", "saturn"),
    ("sega_dreamcast", "dreamcast"),
    ("sony_playstation", "ps1"),
    ("sony_playstation2", "ps2"),
    ("sony_playstation3", "ps3"),
    ("sony_psp", "psp"),
    ("sony_vita", "psvita"),
    ("xbox", "xbox"),
    ("xbox360", "xbox360"),
    ("nec_turbografx_16", "pcengine"),
    ("atari_2600", "atari2600"),
];

pub fn store_of_plugin(plugin: Uuid) -> Option<GameStore> {
    STORE_PLUGINS
        .iter()
        .find(|(p, _)| *p == plugin)
        .map(|(_, s)| *s)
}

/// Jogo a gravar como "Adicionado" (os campos de `ManualGameRecord` que a importação preenche).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayniteRecord {
    pub id: String,
    pub title: String,
    /// Executável, atalho ou URI; para ROM, o arquivo da ROM.
    pub path: String,
    pub arguments: Option<String>,
    pub working_directory: Option<String>,
    pub added_at: DateTime<Utc>,
    /// Sistema da ROM ("snes"); `None` = PC.
    pub platform: Option<String>,
}

/// Estado do usuário de um jogo (`GameStateRecord`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayniteState {
    pub is_favorite: bool,
    pub last_played_at: Option<DateTime<Utc>>,
    pub play_minutes: f64,
}

/// Por que um jogo fica de fora.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayniteSkipReason {
    /// Jogo de uma loja que o Tako lê sozinho.
    StoreGame,
    /// Não instalado no Playnite ou sem ação de jogar.
    NotPlayable,
    /// Abre por script do Playnite ou usa variáveis que só o Playnite resolve.
    PlayniteOnly,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayniteNewGame {
    pub source: PlayniteGame,
    pub record: PlayniteRecord,
    pub platform_name: Option<String>,
    pub collections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayniteMatchedGame {
    pub source: PlayniteGame,
    pub tako_id: String,
    pub collections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayniteSkippedGame {
    pub source: PlayniteGame,
    pub reason: PlayniteSkipReason,
}

/// Prévia da importação, antes da confirmação.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayniteImportPlan {
    pub new: Vec<PlayniteNewGame>,
    pub matched: Vec<PlayniteMatchedGame>,
    pub skipped: Vec<PlayniteSkippedGame>,
    /// Coleções que serão criadas (as que o Tako ainda não tem, pelo nome).
    pub collection_names: Vec<String>,
}

impl PlayniteImportPlan {
    pub fn is_empty(&self) -> bool {
        self.new.is_empty() && self.matched.is_empty()
    }

    /// "12 jogos novos, 30 que o Tako já tem (…) e 3 coleções".
    pub fn summary(&self) -> String {
        let n = |v: usize| v as i64;
        if self.is_empty() {
            return if self.skipped.is_empty() {
                t("sources.playnite.empty")
            } else {
                tn("sources.playnite.nothing", n(self.skipped.len()), &[])
            };
        }
        let mut parts = Vec::new();
        for (key, count) in [
            ("sources.playnite.new", self.new.len()),
            ("sources.playnite.matched", self.matched.len()),
            ("sources.playnite.collections", self.collection_names.len()),
        ] {
            if count > 0 {
                parts.push(tn(key, n(count), &[]));
            }
        }
        let list = match parts.split_last() {
            Some((last, rest)) if !rest.is_empty() => tf(
                "sources.playnite.and",
                &[("list", &rest.join(", ")), ("last", last)],
            ),
            _ => parts.concat(),
        };
        if self.skipped.is_empty() {
            list
        } else {
            tn(
                "sources.playnite.skipped",
                n(self.skipped.len()),
                &[("list", &list)],
            )
        }
    }
}

/// Id fixo do jogo importado: importar de novo não duplica.
pub fn imported_id(playnite_id: Uuid) -> String {
    format!("manual:playnite-{}", playnite_id.simple())
}

fn clean_collection(name: &str) -> String {
    let clean = name.split_whitespace().collect::<Vec<_>>().join(" ");
    clean
        .chars()
        .take(COLLECTION_MAX_LEN)
        .collect::<String>()
        .trim_end()
        .to_owned()
}

fn by_id(existing: &[Game]) -> HashMap<String, &Game> {
    let mut map = HashMap::new();
    for g in existing {
        map.entry(g.id.to_lowercase()).or_insert(g);
    }
    map
}

pub fn plan(
    library: &PlayniteLibrary,
    existing: &[Game],
    existing_collections: &[String],
    now: DateTime<Utc>,
) -> PlayniteImportPlan {
    let ids = by_id(existing);
    let mut plan = PlayniteImportPlan::default();
    for source in &library.games {
        let mut collections: Vec<String> = Vec::new();
        for name in source
            .category_ids
            .iter()
            .filter_map(|id| library.categories.get(id))
        {
            let c = clean_collection(name);
            if !c.is_empty() && !collections.iter().any(|x| fold(x) == fold(&c)) {
                collections.push(c);
            }
        }
        if let Some(m) = find_match(source, existing, &ids) {
            plan.matched.push(PlayniteMatchedGame {
                source: source.clone(),
                tako_id: m.id.clone(),
                collections,
            });
            continue;
        }
        if store_of_plugin(source.plugin_id).is_some() {
            plan.skipped.push(PlayniteSkippedGame {
                source: source.clone(),
                reason: PlayniteSkipReason::StoreGame,
            });
            continue;
        }
        let platform = library.platform_of(source);
        match to_record(source, platform, now) {
            Ok(record) => plan.new.push(PlayniteNewGame {
                source: source.clone(),
                record,
                platform_name: platform.map(|p| p.name.clone()),
                collections,
            }),
            Err(reason) => plan.skipped.push(PlayniteSkippedGame {
                source: source.clone(),
                reason,
            }),
        }
    }
    let known: HashSet<String> = existing_collections.iter().map(|c| fold(c)).collect();
    let mut created: Vec<String> = Vec::new();
    for c in plan
        .new
        .iter()
        .flat_map(|g| &g.collections)
        .chain(plan.matched.iter().flat_map(|g| &g.collections))
    {
        if !known.contains(&fold(c)) && !created.iter().any(|x| fold(x) == fold(c)) {
            created.push(c.clone());
        }
    }
    created.sort_by_cached_key(|c| fold(c));
    plan.collection_names = created;
    plan
}

/// Estado a gravar para um jogo que o Tako já tem: favorito vale se estiver em qualquer um; tempo e
/// "jogado por último" ficam com o maior (a Steam já soma o próprio tempo, então o dela não muda).
pub fn merge_state(
    current: Option<&PlayniteState>,
    source: &PlayniteGame,
    store: GameStore,
) -> PlayniteState {
    let current = current.cloned().unwrap_or_default();
    let last = match (source.last_activity, current.last_played_at) {
        (Some(a), Some(c)) => Some(a.max(c)),
        (a, c) => a.or(c),
    };
    let minutes = if store == GameStore::Steam {
        current.play_minutes
    } else {
        current
            .play_minutes
            .max(source.playtime.as_secs_f64() / 60.0)
    };
    PlayniteState {
        is_favorite: current.is_favorite || source.favorite,
        last_played_at: last,
        play_minutes: minutes,
    }
}

/// O jogo do Tako que é este jogo do Playnite: já importado, mesmo id de loja ou mesma pasta.
pub fn find_match<'a>(
    source: &PlayniteGame,
    existing: &'a [Game],
    ids: &HashMap<String, &'a Game>,
) -> Option<&'a Game> {
    if let Some(g) = ids.get(&imported_id(source.id)) {
        return Some(g);
    }
    let plugin_store = store_of_plugin(source.plugin_id);
    if let (Some(store), Some(game_id)) = (plugin_store, source.game_id.as_deref().map(str::trim)) {
        let prefix = match store {
            GameStore::Steam
            | GameStore::Epic
            | GameStore::Gog
            | GameStore::Ubisoft
            | GameStore::Amazon
            | GameStore::Ea => Some(store.id_prefix()),
            _ => None,
        };
        if let Some(g) = prefix.and_then(|p| ids.get(&format!("{p}:{game_id}").to_lowercase())) {
            return Some(g);
        }
        // Xbox (StoreId ou família), Battle.net (código "Pro"), EA (oferta): pelo id do produto na loja.
        if let Some(g) = existing.iter().find(|g| {
            g.store == Some(store)
                && g.store_product_id
                    .as_deref()
                    .is_some_and(|p| p.eq_ignore_ascii_case(game_id))
        }) {
            return Some(g);
        }
    }
    // Mesma pasta de instalação (qualquer loja do Tako, inclusive um "Adicionado" igual).
    if let Some(dir) = source.install_directory.as_deref() {
        let folder = windows_path::normalize(dir);
        if let Some(g) = existing.iter().find(|g| {
            g.install_path
                .as_deref()
                .is_some_and(|p| windows_path::normalize(p).eq_ignore_ascii_case(&folder))
                && plugin_store.is_none_or(|s| s == g.store() || g.store() == GameStore::Manual)
        }) {
            return Some(g);
        }
    }
    // Jogo adicionado à mão nos dois: mesmo nome entre os "Adicionados".
    let key = title_key(&source.name);
    if key.is_empty() {
        return None;
    }
    existing
        .iter()
        .find(|g| g.store() == GameStore::Manual && title_key(&g.title) == key)
}

/// Registro de "Adicionado": a ação de jogar de arquivo ou link (com `{InstallDir}` resolvido) ou,
/// para jogo de emulador, a primeira ROM com o sistema reconhecido.
pub fn to_record(
    source: &PlayniteGame,
    platform: Option<&PlaynitePlatform>,
    now: DateTime<Utc>,
) -> Result<PlayniteRecord, PlayniteSkipReason> {
    use PlayniteSkipReason::*;
    let action = source
        .actions
        .iter()
        .find(|a| a.is_play_action)
        .or(source.actions.first());
    let id = imported_id(source.id);
    let added_at = source.added.unwrap_or(now);
    if action.is_some_and(|a| a.kind == PlayniteActionType::Emulator) {
        let system = platform
            .and_then(|p| p.specification_id.as_deref())
            .and_then(|spec| {
                PLATFORM_KEYS
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(spec))
            })
            .map(|(_, v)| v.to_string());
        let rom = source
            .roms
            .iter()
            .filter_map(|r| expand(r.path.as_deref(), source))
            .find(|p| windows_path::is_rooted(p));
        let (Some(system), Some(rom)) = (system, rom) else {
            return Err(PlayniteOnly);
        };
        return Ok(PlayniteRecord {
            id,
            title: source.name.clone(),
            working_directory: windows_path::directory_of(&rom),
            path: rom,
            arguments: None,
            added_at,
            platform: Some(system),
        });
    }
    let action = match action {
        Some(a) if source.is_installed => a,
        _ => return Err(NotPlayable),
    };
    if !matches!(
        action.kind,
        PlayniteActionType::File | PlayniteActionType::Url
    ) {
        return Err(PlayniteOnly);
    }
    let path = expand(action.path.as_deref(), source).ok_or(PlayniteOnly)?;
    let expand_opt = |v: &Option<String>| match v {
        None => Ok(None),
        Some(v) => expand(Some(v), source).map(Some).ok_or(PlayniteOnly),
    };
    let arguments = expand_opt(&action.arguments)?;
    let working_dir = expand_opt(&action.working_dir)?;
    let is_uri = action.kind == PlayniteActionType::Url || is_non_file_uri(&path);
    let mut path = path;
    if !is_uri
        && !windows_path::is_rooted(&path)
        && let Some(dir) = source.install_directory.as_deref()
    {
        path = windows_path::combine(dir, &[&path]); // relativo à pasta do jogo, como o Playnite resolve
    }
    if !is_uri && !windows_path::is_rooted(&path) {
        return Err(PlayniteOnly);
    }
    let working_directory = working_dir.or_else(|| {
        if is_uri {
            None
        } else {
            source
                .install_directory
                .clone()
                .or_else(|| windows_path::directory_of(&path))
        }
    });
    Ok(PlayniteRecord {
        id,
        title: source.name.clone(),
        path,
        arguments: arguments.filter(|a| !a.trim().is_empty()),
        working_directory,
        added_at,
        platform: None,
    })
}

/// URI absoluta que não é arquivo (`https://`, `steam://`); "C:\…" e `file:` não contam.
fn is_non_file_uri(s: &str) -> bool {
    let Some((scheme, _)) = s.split_once(':') else {
        return false;
    };
    scheme.len() >= 2
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
        && !scheme.eq_ignore_ascii_case("file")
}

/// Resolve `{InstallDir}` e `{InstallDirName}`; outra variável do Playnite = `None` (não importável).
pub fn expand(value: Option<&str>, source: &PlayniteGame) -> Option<String> {
    let mut text = value.map(str::trim).filter(|v| !v.is_empty())?.to_owned();
    if let Some(dir) = source.install_directory.as_deref() {
        let folder = windows_path::normalize(dir);
        text = replace_ignore_case(&text, "{InstallDirName}", &windows_path::last_name(&folder));
        text = replace_ignore_case(&text, "{InstallDir}", &folder);
    }
    (!(text.contains('{') && text.contains('}'))).then_some(text)
}

fn replace_ignore_case(text: &str, from: &str, to: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let needle = from.to_ascii_lowercase();
    let mut out = String::new();
    let mut last = 0;
    for (i, _) in lower.match_indices(&needle) {
        out.push_str(&text[last..i]);
        out.push_str(to);
        last = i + from.len();
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::playnite::library::PlayniteAction;
    use crate::playnite::litedb::{LiteDbV7Reader, writer};
    use crate::playnite::sample;
    use std::time::Duration;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp_millis(sample::ms(2026, 10, 15, 12)).unwrap()
    }

    fn library() -> PlayniteLibrary {
        let reader = LiteDbV7Reader::new(writer::write(&[("Game", sample::games())])).unwrap();
        PlayniteLibrary::from_documents(
            reader.documents("Game").unwrap(),
            sample::platforms(),
            sample::categories(),
            vec![],
        )
    }

    fn game(id: &str, title: &str, store: GameStore, install_path: Option<&str>) -> Game {
        Game {
            id: id.into(),
            title: title.into(),
            store: Some(store),
            is_installed: true,
            install_path: install_path.map(Into::into),
            store_product_id: (store == GameStore::BattleNet).then(|| "Pro".into()),
            ..Default::default()
        }
    }

    fn match_id(source: &PlayniteGame, existing: &[Game]) -> Option<String> {
        find_match(source, existing, &by_id(existing)).map(|g| g.id.clone())
    }

    #[test]
    fn plan_imports_new_games_matches_store_games_and_skips_the_rest() {
        let steam = game("steam:7", "Farol", GameStore::Steam, None);
        let plan = plan(&library(), &[steam], &["Zerados".into()], now());

        let titles: Vec<_> = plan.new.iter().map(|g| g.record.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Lanterna Distante",
                "Cartucho Azul",
                "Jogo no Navegador",
                "Descrição Longa"
            ]
        );
        assert_eq!(plan.matched.len(), 1);
        assert_eq!(plan.matched[0].tako_id, "steam:7");
        assert_eq!(plan.matched[0].collections, ["Jogar no sofá"]);
        let skipped: Vec<_> = plan
            .skipped
            .iter()
            .map(|s| (s.source.id, s.reason))
            .collect();
        assert_eq!(
            skipped,
            [
                (sample::CORRIDA, PlayniteSkipReason::StoreGame),
                (sample::ROTEIRO, PlayniteSkipReason::PlayniteOnly),
                (sample::AUSENTE, PlayniteSkipReason::NotPlayable),
            ]
        );
        assert_eq!(plan.collection_names, ["Jogar no sofá"]); // "Zerados" já existe no Tako
        assert_eq!(
            plan.summary(),
            "4 jogos novos, 1 jogo que o Tako já tem (favoritos e tempo completados) e 1 coleção. Ficam de fora 3 jogos (lojas que o Tako já lê, não instalados ou só pelo Playnite)"
        );
    }

    #[test]
    fn new_game_records_resolve_install_dir_and_keep_rom_platform() {
        let plan = plan(&library(), &[], &[], now());
        let find = |id| plan.new.iter().find(|g| g.source.id == id).unwrap();

        let lanterna = find(sample::LANTERNA);
        assert_eq!(
            lanterna.record.id,
            "manual:playnite-11111111111111111111111111111111"
        );
        assert_eq!(lanterna.record.path, r"C:\Jogos\Lanterna\lanterna.exe");
        assert_eq!(lanterna.record.arguments.as_deref(), Some("-tela-cheia"));
        assert_eq!(
            lanterna.record.working_directory.as_deref(),
            Some(r"C:\Jogos\Lanterna")
        );
        assert_eq!(
            lanterna.record.added_at.timestamp_millis(),
            sample::ms(2025, 3, 1, 12)
        );
        assert_eq!(lanterna.platform_name.as_deref(), Some("PC (Windows)"));
        assert_eq!(lanterna.collections, ["Jogar no sofá", "Zerados"]);

        let rom = &find(sample::CARTUCHO).record;
        assert_eq!(rom.path, r"D:\ROMs\SNES\azul.sfc");
        assert_eq!(rom.platform.as_deref(), Some("snes"));

        let url = &find(sample::NAVEGADOR).record;
        assert_eq!(url.path, "https://example.com/jogo");
        assert_eq!(url.working_directory, None);
    }

    #[test]
    fn matching_never_duplicates_store_games_or_a_previous_import() {
        let lib = library();
        let corrida = lib.games.iter().find(|g| g.id == sample::CORRIDA).unwrap();
        let lanterna = lib.games.iter().find(|g| g.id == sample::LANTERNA).unwrap();
        let imported = "manual:playnite-11111111111111111111111111111111";

        assert_eq!(
            match_id(
                corrida,
                &[game(
                    "epic:CorridaNoturna",
                    "Corrida",
                    GameStore::Epic,
                    None
                )]
            )
            .as_deref(),
            Some("epic:CorridaNoturna")
        );
        assert_eq!(
            match_id(
                lanterna,
                &[game(
                    "gog:1",
                    "Outro nome",
                    GameStore::Gog,
                    Some(r"C:\Jogos\Lanterna\")
                )]
            )
            .as_deref(),
            Some("gog:1")
        );
        assert_eq!(
            match_id(
                lanterna,
                &[game(imported, "Lanterna", GameStore::Manual, None)]
            )
            .as_deref(),
            Some(imported)
        );
        assert_eq!(
            match_id(
                lanterna,
                &[game(
                    "manual:abc",
                    "Lanterna Distante™",
                    GameStore::Manual,
                    None
                )]
            )
            .as_deref(),
            Some("manual:abc")
        );
        assert_eq!(
            match_id(
                corrida,
                &[game("epic:Outro", "Corrida Noturna", GameStore::Epic, None)]
            ),
            None
        );

        let battle = PlayniteGame {
            id: Uuid::new_v4(),
            name: "Overwatch 2".into(),
            plugin_id: uuid!("e3c26a3d-d695-4cb7-a769-5ff7612c7edd"),
            game_id: Some("Pro".into()),
            ..Default::default()
        };
        let bnet = game(
            "battlenet:prometheus",
            "Overwatch 2",
            GameStore::BattleNet,
            None,
        );
        assert_eq!(
            match_id(&battle, &[bnet]).as_deref(),
            Some("battlenet:prometheus")
        );
    }

    #[test]
    fn unknown_playnite_variables_are_not_imported() {
        let action = |path: &str| PlayniteAction {
            kind: PlayniteActionType::File,
            path: Some(path.into()),
            arguments: None,
            working_dir: None,
            is_play_action: true,
        };
        let mut source = PlayniteGame {
            id: Uuid::new_v4(),
            name: "Variável".into(),
            is_installed: true,
            install_directory: Some(r"C:\Jogo".into()),
            actions: vec![action(r"{PlayniteDir}\x.exe")],
            ..Default::default()
        };
        assert_eq!(
            to_record(&source, None, now()),
            Err(PlayniteSkipReason::PlayniteOnly)
        );
        source.actions = vec![action(r"bin\jogo.exe")];
        assert_eq!(
            to_record(&source, None, now()).unwrap().path,
            r"C:\Jogo\bin\jogo.exe"
        );
    }

    #[test]
    fn merge_keeps_favorite_and_the_larger_time_but_not_for_steam() {
        let source = PlayniteGame {
            id: Uuid::new_v4(),
            name: "X".into(),
            favorite: true,
            playtime: Duration::from_secs(90 * 60),
            last_activity: Some(now()),
            ..Default::default()
        };
        let current = PlayniteState {
            is_favorite: false,
            play_minutes: 30.0,
            last_played_at: Some(now() - chrono::Duration::days(3)),
        };
        assert_eq!(
            merge_state(Some(&current), &source, GameStore::Gog),
            PlayniteState {
                is_favorite: true,
                play_minutes: 90.0,
                last_played_at: Some(now())
            }
        );
        assert_eq!(
            merge_state(Some(&current), &source, GameStore::Steam).play_minutes,
            30.0
        );
        let more = PlayniteState {
            play_minutes: 120.0,
            ..current
        };
        assert_eq!(
            merge_state(Some(&more), &source, GameStore::Gog).play_minutes,
            120.0
        );
    }

    #[test]
    fn empty_plan_explains_why() {
        let empty = plan(&PlayniteLibrary::default(), &[], &[], now());
        assert!(empty.is_empty());
        assert_eq!(empty.summary(), "A biblioteca do Playnite está vazia");
    }

    #[test]
    fn non_file_uris_are_told_apart_from_paths() {
        assert!(is_non_file_uri("steam://rungameid/1") && is_non_file_uri("https://x"));
        assert!(
            !is_non_file_uri(r"C:\x.exe")
                && !is_non_file_uri("file:///x")
                && !is_non_file_uri(r"bin\x")
        );
        assert_eq!(
            replace_ignore_case("{installdir}\\a", "{InstallDir}", "C:"),
            "C:\\a"
        );
    }
}
