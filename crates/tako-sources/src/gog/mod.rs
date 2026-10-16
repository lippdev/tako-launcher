//! GOG (port de `Library/Gog`): jogos do GOG Galaxy ou dos instaladores offline, lidos do registro
//! e do `goggame-<id>.info` na pasta. Abre pelo Galaxy quando instalado; senão, pelo executável
//! (jogos do GOG não têm DRM).

use crate::fs::StoreFs;
use crate::registry::RegistryReader;
use crate::{LibraryScan, LibrarySource};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::Arc;
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore};

pub const GAMES_KEY: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\Games";
pub const GALAXY_KEY: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\GalaxyClient";
const ACCENT: &str = "#7A3FD1";

/// Jogo gravado no registro pelo instalador (`HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\<id>`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GogRegistryEntry {
    pub game_id: String,
    pub name: String,
    pub path: String,
    pub exe: Option<String>,
    pub launch_param: Option<String>,
    pub working_dir: Option<String>,
    pub launch_command: Option<String>,
    pub depends_on: Option<String>,
}

/// Tarefa principal de `goggame-<id>.info`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GogInfo {
    pub game_id: String,
    pub root_game_id: Option<String>,
    pub name: Option<String>,
    pub play_path: Option<String>,
    pub play_arguments: Option<String>,
    pub play_working_dir: Option<String>,
}

/// Como abrir: executável e argumentos já resolvidos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GogLaunch {
    pub target: String,
    pub arguments: Option<String>,
    pub working_directory: Option<String>,
    pub via_galaxy: bool,
}

/// DLC (depende de outro jogo) ou entrada sem pasta não vira item da biblioteca.
pub fn is_playable(entry: &GogRegistryEntry, info: Option<&GogInfo>) -> bool {
    let is_dlc = blank(entry.depends_on.as_deref()).is_some()
        || info.is_some_and(|i| {
            i.root_game_id
                .as_deref()
                .is_some_and(|r| !r.is_empty() && r != i.game_id)
        });
    !is_dlc && !entry.path.trim().is_empty() && !entry.name.trim().is_empty()
}

/// `goggame-<id>.info`: a tarefa de jogo principal (`isPrimary`, tipo FileTask). JSON inválido é `Err`.
pub fn parse_info(json: &str) -> Result<Option<GogInfo>, serde_json::Error> {
    let root: Value = serde_json::from_str(json)?;
    let Some(game_id) = text(&root, "gameId") else {
        return Ok(None);
    };
    let files: Vec<&Value> = match root.get("playTasks") {
        Some(Value::Array(tasks)) => tasks
            .iter()
            .filter(|t| {
                text(t, "type").is_some_and(|k| k.eq_ignore_ascii_case("FileTask"))
                    && text(t, "path").is_some()
            })
            .collect(),
        _ => Vec::new(),
    };
    let task = files
        .iter()
        .find(|t| t.get("isPrimary") == Some(&Value::Bool(true)))
        .or(files.first());
    Ok(Some(GogInfo {
        game_id,
        root_game_id: text(&root, "rootGameId"),
        name: text(&root, "name"),
        play_path: task.and_then(|t| text(t, "path")),
        play_arguments: task.and_then(|t| text(t, "arguments")),
        play_working_dir: task.and_then(|t| text(t, "workingDir")),
    }))
}

/// Pelo GOG Galaxy quando instalado (mantém nuvem e conquistas); sem ele, o executável do jogo.
/// `None` quando não há executável conhecido nem Galaxy.
pub fn launch(
    entry: &GogRegistryEntry,
    info: Option<&GogInfo>,
    galaxy_exe: Option<&str>,
) -> Option<GogLaunch> {
    if let Some(galaxy) = blank(galaxy_exe) {
        return Some(GogLaunch {
            target: galaxy.to_owned(),
            arguments: Some(format!(
                "/command=runGame /gameId={} /path=\"{}\"",
                entry.game_id,
                entry.path.trim_end_matches(['\\', '/'])
            )),
            working_directory: crate::windows_path::directory_of(galaxy),
            via_galaxy: true,
        });
    }
    let fallback_dir = || Some(entry.path.clone());
    if let Some(exe) = blank(entry.exe.as_deref()) {
        return Some(GogLaunch {
            target: exe.to_owned(),
            arguments: blank(entry.launch_param.as_deref()).map(str::to_owned),
            working_directory: blank(entry.working_dir.as_deref())
                .map(str::to_owned)
                .or_else(fallback_dir),
            via_galaxy: false,
        });
    }
    if let Some(relative) = info.and_then(|i| i.play_path.as_deref()) {
        let info = info?;
        return Some(GogLaunch {
            target: combine(&entry.path, relative),
            arguments: blank(info.play_arguments.as_deref()).map(str::to_owned),
            working_directory: info
                .play_working_dir
                .as_deref()
                .map(|wd| combine(&entry.path, wd))
                .or_else(fallback_dir),
            via_galaxy: false,
        });
    }
    if let (Some(exe), arguments) = split_command(entry.launch_command.as_deref()) {
        return Some(GogLaunch {
            target: exe,
            arguments,
            working_directory: blank(entry.working_dir.as_deref())
                .map(str::to_owned)
                .or_else(fallback_dir),
            via_galaxy: false,
        });
    }
    None
}

/// Linha de comando (`"C:\Jogo\jogo.exe" -arg`) em executável e argumentos.
pub fn split_command(command: Option<&str>) -> (Option<String>, Option<String>) {
    let text = command.unwrap_or("").trim();
    if text.is_empty() {
        return (None, None);
    }
    let rest_args = |rest: &str| blank(Some(rest.trim())).map(str::to_owned);
    if let Some(quoted) = text.strip_prefix('"') {
        return match quoted.find('"') {
            None => (Some(text.trim_matches('"').to_owned()), None),
            Some(end) => (
                Some(quoted[..end].to_owned()),
                rest_args(&quoted[end + 1..]),
            ),
        };
    }
    match text.to_ascii_lowercase().find(".exe") {
        None => (Some(text.to_owned()), None),
        Some(i) => (Some(text[..i + 4].to_owned()), rest_args(&text[i + 4..])),
    }
}

fn combine(folder: &str, relative: &str) -> String {
    if crate::windows_path::is_rooted(relative) || relative.starts_with(['/', '\\']) {
        relative.to_owned()
    } else {
        format!(
            "{}\\{}",
            folder.trim_end_matches(['\\', '/']),
            relative.trim_start_matches(['\\', '/'])
        )
    }
}

fn blank(s: Option<&str>) -> Option<&str> {
    s.filter(|s| !s.trim().is_empty())
}

/// Texto ou número (ids às vezes vêm como número).
fn text(v: &Value, key: &str) -> Option<String> {
    match v.get(key)? {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        _ => None,
    }
}

pub fn id_for(game_id: &str) -> String {
    format!("gog:{game_id}")
}

pub fn to_game(entry: &GogRegistryEntry, l: &GogLaunch, added_at: Option<DateTime<Utc>>) -> Game {
    Game {
        id: id_for(&entry.game_id),
        title: entry.name.trim().to_owned(),
        store: Some(GameStore::Gog),
        accent: ACCENT.to_owned(),
        is_installed: true,
        added_at,
        launch_target: Some(l.target.clone()),
        launch_arguments: l.arguments.clone(),
        working_directory: l.working_directory.clone(),
        install_path: Some(entry.path.clone()),
        ..Default::default()
    }
}

pub struct GogLibrarySource {
    registry: Arc<dyn RegistryReader>,
    pub fs: StoreFs,
}

impl GogLibrarySource {
    pub fn new(registry: Arc<dyn RegistryReader>) -> Self {
        GogLibrarySource {
            registry,
            fs: StoreFs::real(),
        }
    }

    /// `GalaxyClient.exe` quando o GOG Galaxy está instalado.
    pub fn galaxy_executable(&self) -> Option<String> {
        let folder = self
            .registry
            .get_string(&format!(r"{GALAXY_KEY}\paths"), "client")?;
        let exe = self
            .registry
            .get_string(GALAXY_KEY, "clientExecutable")
            .unwrap_or_else(|| "GalaxyClient.exe".into());
        if folder.trim().is_empty() {
            return None;
        }
        let path = format!("{}\\{exe}", folder.trim_end_matches(['\\', '/']));
        (self.fs.exists)(&path).then_some(path)
    }

    fn read_info(&self, entry: &GogRegistryEntry) -> Option<GogInfo> {
        let file = format!(
            "{}\\goggame-{}.info",
            entry.path.trim_end_matches(['\\', '/']),
            entry.game_id
        );
        parse_info(&self.fs.read_text(&file)?).ok().flatten()
    }
}

impl LibrarySource for GogLibrarySource {
    fn store(&self) -> GameStore {
        GameStore::Gog
    }

    fn scan(&self) -> LibraryScan {
        let galaxy = self.galaxy_executable();
        let mut games: Vec<Game> = Vec::new();
        for id in self.registry.sub_keys(GAMES_KEY) {
            let key = format!(r"{GAMES_KEY}\{id}");
            let get = |v: &str| self.registry.get_string(&key, v);
            let entry = GogRegistryEntry {
                game_id: get("gameID").unwrap_or_else(|| id.clone()),
                name: get("gameName").unwrap_or_default(),
                path: get("path").unwrap_or_default(),
                exe: get("exe"),
                launch_param: get("launchParam"),
                working_dir: get("workingDir"),
                launch_command: get("launchCommand"),
                depends_on: get("dependsOn"),
            };
            if entry.path.trim().is_empty() || !(self.fs.exists)(&entry.path) {
                continue; // desinstalado sem limpar o registro
            }
            let info = self.read_info(&entry);
            if is_playable(&entry, info.as_ref())
                && let Some(l) = launch(&entry, info.as_ref(), galaxy.as_deref())
                && !games.iter().any(|g| g.id == id_for(&entry.game_id))
            {
                games.push(to_game(&entry, &l, crate::install_dates::of(&entry.path)));
            }
        }
        crate::text::sort_by_title(&mut games);
        // Sem Galaxy os jogos abrem direto; o aviso só explica por que nuvem e conquistas não sincronizam.
        let notice = (galaxy.is_none() && !games.is_empty()).then(|| t("sources.gog.no_galaxy"));
        LibraryScan {
            games,
            client_installed: galaxy.is_some(),
            notice,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::MemoryRegistry;
    use std::path::Path;

    const CIDADE: &str = r"D:\GOG Games\Cidade Submersa";
    const GALAXY: &str = r"C:\Program Files (x86)\GOG Galaxy";

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/Stores/Gog")
                .join(name),
        )
        .unwrap()
    }

    fn entry(id: &str, name: &str, path: &str) -> GogRegistryEntry {
        GogRegistryEntry {
            game_id: id.into(),
            name: name.into(),
            path: path.into(),
            ..Default::default()
        }
    }

    fn launch_of(target: &str, args: Option<&str>, wd: &str) -> Option<GogLaunch> {
        Some(GogLaunch {
            target: target.into(),
            arguments: args.map(Into::into),
            working_directory: Some(wd.into()),
            via_galaxy: false,
        })
    }

    fn registry() -> MemoryRegistry {
        MemoryRegistry::default()
            .with(&format!(r"{GAMES_KEY}\1400000001"), "gameID", "1400000001")
            .with(
                &format!(r"{GAMES_KEY}\1400000001"),
                "gameName",
                "Cidade Submersa",
            )
            .with(&format!(r"{GAMES_KEY}\1400000001"), "path", CIDADE)
            .with(
                &format!(r"{GAMES_KEY}\1400000002"),
                "gameName",
                "Cidade Submersa - Expansão",
            )
            .with(&format!(r"{GAMES_KEY}\1400000002"), "path", CIDADE)
            .with(
                &format!(r"{GAMES_KEY}\1207000000"),
                "gameName",
                "Desinstalado",
            )
            .with(&format!(r"{GAMES_KEY}\1207000000"), "path", r"F:\Sumiu")
    }

    /// Os `goggame-*.info` de exemplo, como se estivessem na pasta do jogo.
    fn source(reg: MemoryRegistry, galaxy: bool) -> GogLibrarySource {
        let mut s = GogLibrarySource::new(Arc::new(reg));
        s.fs = StoreFs {
            exists: Arc::new(move |p| {
                p.starts_with(CIDADE) || (galaxy && p == format!(r"{GALAXY}\GalaxyClient.exe"))
            }),
            read: Arc::new(|p| {
                p.strip_prefix(&format!(r"{CIDADE}\"))
                    .filter(|n| n.starts_with("goggame-"))
                    .map(|n| fixture(n).into_bytes())
            }),
        };
        s
    }

    #[test]
    fn info_picks_the_primary_game_task() {
        let info = parse_info(&fixture("goggame-1400000001.info"))
            .unwrap()
            .unwrap();
        assert_eq!(info.game_id, "1400000001");
        assert_eq!(info.play_path.as_deref(), Some(r"bin\x64\cidade.exe"));
        assert_eq!(info.play_arguments.as_deref(), Some("-skipintro"));
        assert!(parse_info("{ quebrado").is_err());
    }

    #[test]
    fn dlc_is_not_a_library_item() {
        let dlc = parse_info(&fixture("goggame-1400000002.info"))
            .unwrap()
            .unwrap();
        let e = entry("1400000002", "Expansão", CIDADE);
        assert_eq!(dlc.game_id, "1400000002"); // id numérico também vale
        assert!(!is_playable(&e, Some(&dlc)));
        let with_dep = GogRegistryEntry {
            depends_on: Some("1400000001".into()),
            ..e.clone()
        };
        assert!(!is_playable(&with_dep, None));
        assert!(is_playable(&e, None));
    }

    #[test]
    fn opens_through_galaxy_when_installed() {
        let e = GogRegistryEntry {
            exe: Some(format!(r"{CIDADE}\cidade.exe")),
            ..entry("1400000001", "Cidade Submersa", &format!("{CIDADE}\\"))
        };
        let l = launch(&e, None, Some(&format!(r"{GALAXY}\GalaxyClient.exe"))).unwrap();
        assert!(l.via_galaxy);
        assert_eq!(l.target, format!(r"{GALAXY}\GalaxyClient.exe"));
        assert_eq!(
            l.arguments.unwrap(),
            format!("/command=runGame /gameId=1400000001 /path=\"{CIDADE}\"")
        );
    }

    #[test]
    fn without_galaxy_opens_the_game_itself() {
        let e = entry("1", "Cidade", CIDADE);
        let from_registry = GogRegistryEntry {
            exe: Some(format!(r"{CIDADE}\cidade.exe")),
            launch_param: Some("-x".into()),
            working_dir: Some(format!(r"{CIDADE}\bin")),
            ..e.clone()
        };
        assert_eq!(
            launch(&from_registry, None, None),
            launch_of(
                &format!(r"{CIDADE}\cidade.exe"),
                Some("-x"),
                &format!(r"{CIDADE}\bin")
            )
        );
        let info = GogInfo {
            game_id: "1".into(),
            root_game_id: Some("1".into()),
            name: Some("Cidade".into()),
            play_path: Some(r"bin\x64\cidade.exe".into()),
            play_arguments: Some("-skipintro".into()),
            play_working_dir: None,
        };
        assert_eq!(
            launch(&e, Some(&info), None),
            launch_of(
                &format!(r"{CIDADE}\bin\x64\cidade.exe"),
                Some("-skipintro"),
                CIDADE
            )
        );
        let from_command = GogRegistryEntry {
            launch_command: Some(format!("\"{CIDADE}\\cidade.exe\" -windowed")),
            ..e.clone()
        };
        assert_eq!(
            launch(&from_command, None, None),
            launch_of(&format!(r"{CIDADE}\cidade.exe"), Some("-windowed"), CIDADE)
        );
        assert_eq!(launch(&e, None, None), None);
    }

    #[test]
    fn launch_command_splits_into_exe_and_arguments() {
        let s = |c: &str| split_command(Some(c));
        let o = |a: &str| Some(a.to_owned());
        assert_eq!(
            s(r#""C:\Jogo\jogo.exe" -a -b"#),
            (o(r"C:\Jogo\jogo.exe"), o("-a -b"))
        );
        assert_eq!(
            s(r"C:\Jogo Sem Aspas\jogo.exe -a"),
            (o(r"C:\Jogo Sem Aspas\jogo.exe"), o("-a"))
        );
        assert_eq!(s(r#""C:\Jogo\jogo.exe""#), (o(r"C:\Jogo\jogo.exe"), None));
        assert_eq!(s("  "), (None, None));
        assert_eq!(s("Ação.EXE -x"), (o("Ação.EXE"), o("-x")));
    }

    #[test]
    fn source_reads_registry_and_info_and_skips_dlc_and_uninstalled() {
        let scan = source(registry(), false).scan();
        assert_eq!(scan.games.len(), 1);
        let g = &scan.games[0];
        assert_eq!(g.id, "gog:1400000001");
        assert_eq!(g.title, "Cidade Submersa");
        assert_eq!(g.store, Some(GameStore::Gog));
        assert_eq!(
            g.launch_target.as_deref(),
            Some(format!(r"{CIDADE}\bin\x64\cidade.exe").as_str())
        );
        assert_eq!(g.launch_arguments.as_deref(), Some("-skipintro"));
        assert_eq!(g.install_path.as_deref(), Some(CIDADE));
        assert!(!scan.client_installed);
        assert_eq!(
            scan.notice.as_deref(),
            Some("Sem GOG Galaxy: os jogos abrem direto pelo executável")
        );
    }

    #[test]
    fn source_prefers_galaxy_when_its_executable_exists() {
        let reg = registry()
            .with(&format!(r"{GALAXY_KEY}\paths"), "client", GALAXY)
            .with(GALAXY_KEY, "clientExecutable", "GalaxyClient.exe");
        let scan = source(reg, true).scan();
        assert!(scan.client_installed);
        assert_eq!(scan.notice, None);
        assert_eq!(scan.games.len(), 1);
        assert_eq!(
            scan.games[0].launch_target.as_deref(),
            Some(format!(r"{GALAXY}\GalaxyClient.exe").as_str())
        );
    }
}
