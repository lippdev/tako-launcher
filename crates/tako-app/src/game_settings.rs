//! Chamadas `game.settings` e `game.settings.set` (UX-007): as configurações deste jogo, que valem no lugar das do Tako.
//!
//! As regras são do núcleo (`GameSettingsOverride`, `resolve`); aqui ficam a validação do que a interface manda e a
//! conta de cada linha para a tela: o que o jogo guardou, o padrão do Tako e o que vale de fato. Em demonstração
//! (`simulate`) as escolhas ficam só na memória, como no app Slint.

use crate::organize::{bad, internal};
use crate::protocol::*;
use crate::{Core, system};
use std::collections::HashMap;
use std::path::Path;
use tako_core::devices::DisplayMode;
use tako_core::persistence::{GameSettingsStore, parse_bool};
use tako_core::session::audio::AudioOutputChoice;
use tako_core::session::settings::{
    GameSettingsOverride, GlobalSessionSettings, MonitorPresets, normalize_arguments,
    parse_environment, resolve,
};

/// Argumentos de uma linha de comando comprida já é muito; o teto só impede texto sem fim.
const MAX_ARGUMENTS_CHARS: usize = 2048;
const MAX_FOLDER_CHARS: usize = 1024;
const MAX_ENVIRONMENT_ENTRIES: usize = 32;
const MAX_ENVIRONMENT_CHARS: usize = 4096;
/// Id de tela, de saída de áudio ou de jogo: bem mais do que qualquer sistema usa.
const MAX_ID_CHARS: usize = 512;

/// As escolhas de cada jogo: no banco; em demonstração, na memória.
pub(crate) struct Overrides {
    store: Option<GameSettingsStore>,
    memory: HashMap<String, GameSettingsOverride>,
}

impl Overrides {
    pub(crate) fn new(store: Option<GameSettingsStore>) -> Self {
        Self {
            store,
            memory: HashMap::new(),
        }
    }

    fn load(&self, game_id: &str) -> Result<GameSettingsOverride, BridgeError> {
        match &self.store {
            Some(store) => store.load(game_id).map_err(internal),
            None => Ok(self.memory.get(game_id).cloned().unwrap_or_default()),
        }
    }

    fn save(&mut self, game_id: &str, value: GameSettingsOverride) -> Result<(), BridgeError> {
        match &self.store {
            Some(store) => store.save(game_id, &value).map_err(internal),
            None => {
                self.memory.insert(game_id.to_owned(), value);
                Ok(())
            }
        }
    }
}

fn not_found(what: &str) -> BridgeError {
    BridgeError::new(ErrorCode::NotFound, format!("{what} não encontrado"))
}

fn check_game(core: &Core, game_id: &str) -> Result<(), BridgeError> {
    if game_id.trim().is_empty() || game_id.chars().count() > MAX_ID_CHARS || game_id.contains('\0')
    {
        return Err(bad("id de jogo inválido"));
    }
    if core.library.find(game_id).is_none() {
        return Err(BridgeError::new(
            ErrorCode::NotFound,
            format!("jogo {game_id}"),
        ));
    }
    Ok(())
}

fn flag(core: &Core, key: &str) -> bool {
    core.setting(key).and_then(parse_bool).unwrap_or(false)
}

fn text(core: &Core, key: &str) -> String {
    core.setting(key).unwrap_or_default().to_owned()
}

/// O que o Tako usa quando o jogo não escolhe (Configurações › Tela e áudio do jogo).
fn global_settings(core: &Core) -> GlobalSessionSettings {
    let audio_id = text(core, "GameAudioDeviceId");
    let audio_name = text(core, "GameAudioDeviceName");
    GlobalSessionSettings {
        game_screen_enabled: flag(core, "GameScreenEnabled"),
        game_screen_id: text(core, "GameScreenId"),
        game_screen_mode: DisplayMode::parse(&text(core, "GameScreenMode")),
        game_screen_hdr: flag(core, "GameScreenHdr"),
        game_screen_vrr: flag(core, "GameScreenVrr"),
        monitor_presets: MonitorPresets::parse(&text(core, "GameScreenPresets")),
        game_audio: (!audio_id.trim().is_empty()).then(|| {
            let name = if audio_name.is_empty() {
                &audio_id
            } else {
                &audio_name
            };
            AudioOutputChoice::new(&audio_id, name)
        }),
    }
}

/// Sem a leitura das telas a página ainda abre: as escolhas aparecem, só não há o que escolher.
fn read_displays(core: &Core) -> Option<DisplayList> {
    system::displays(&core.devices)
        .inspect_err(|e| tracing::warn!("telas do jogo: {e}"))
        .ok()
}

fn read_outputs(core: &Core) -> Option<AudioOutputList> {
    system::audio_outputs(&core.devices)
        .inspect_err(|e| tracing::warn!("saídas de áudio do jogo: {e}"))
        .ok()
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn choice(value: Option<&str>) -> GameChoice {
    match value {
        None => GameChoice::Default,
        Some("") => GameChoice::Keep,
        Some(v) => GameChoice::Pick { id: v.to_owned() },
    }
}

fn join_environment(entries: &[(String, String)]) -> Option<String> {
    (!entries.is_empty()).then(|| {
        entries
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(";")
    })
}

fn text_setting(saved: Option<&str>, effective: Option<String>) -> GameTextSetting {
    GameTextSetting {
        saved: normalize_arguments(saved),
        effective,
    }
}

/// As configurações do jogo para a tela: o guardado, o padrão e o que vale em cada linha.
pub(crate) fn view(core: &Core, game_id: &str) -> Result<GameSettingsView, BridgeError> {
    check_game(core, game_id)?;
    let saved = core.game_settings.load(game_id)?;
    let global = global_settings(core);
    let effective = resolve(&global, Some(&saved));
    let displays = read_displays(core);
    let screens = displays.as_ref().map_or(&[][..], |d| d.displays.as_slice());
    let outputs = read_outputs(core);

    // A tela do jogo: a dele (com a grafia que o sistema usa) ou a de jogo do Tako.
    let own_screen = normalize_arguments(saved.screen_id.as_deref()).map(|id| {
        screens
            .iter()
            .find(|d| same(&d.id, &id))
            .map_or(id, |d| d.id.clone())
    });
    let screen_id = own_screen
        .clone()
        .unwrap_or_else(|| global.game_screen_id.trim().to_owned());
    let shown_screen = screens.iter().find(|d| same(&d.id, &screen_id));
    // Em outra tela o padrão é o preset dela (a resolução do Tako é da tela de jogo dele).
    let defaults = global.screen_defaults(&screen_id);

    let use_screen_default = own_screen.is_some() || global.game_screen_enabled;
    let use_screen = GameFlagSetting {
        saved: saved.use_game_screen,
        default: use_screen_default,
        effective: saved.use_game_screen.unwrap_or(use_screen_default),
    };

    let mut screen_options: Vec<GameDeviceOption> = screens
        .iter()
        .map(|d| GameDeviceOption {
            id: d.id.clone(),
            name: d.name.clone(),
            connected: true,
        })
        .collect();
    if let Some(id) = &own_screen
        && !screen_options.iter().any(|o| same(&o.id, id))
    {
        screen_options.push(GameDeviceOption {
            id: id.clone(),
            name: saved.screen_name.clone().unwrap_or_else(|| id.clone()),
            connected: false,
        });
    }
    let tako_screen = global.game_screen_id.trim();
    let screen = GameScreenSetting {
        saved: own_screen,
        default: (!tako_screen.is_empty()).then(|| GameDevice {
            id: tako_screen.to_owned(),
            name: Some(text(core, "GameScreenName"))
                .filter(|n| !n.trim().is_empty())
                .or_else(|| {
                    screens
                        .iter()
                        .find(|d| same(&d.id, tako_screen))
                        .map(|d| d.name.clone())
                })
                .unwrap_or_else(|| tako_screen.to_owned()),
        }),
        effective: effective.game_screen.as_ref().map(|s| s.display_id.clone()),
        options: screen_options,
        no_screen_chosen: effective.game_screen_problem.is_some(),
    };

    let saved_mode = choice(saved.screen_mode.as_deref());
    let default_mode = defaults.mode.map(|m| m.key());
    let effective_mode = match &saved_mode {
        GameChoice::Default => default_mode.clone(),
        GameChoice::Keep => None,
        GameChoice::Pick { id } => DisplayMode::parse(id)
            .map(|m| m.key())
            .or_else(|| default_mode.clone()),
    };
    let mut mode_options = shown_screen.map_or_else(Vec::new, |d| d.modes.clone());
    if let GameChoice::Pick { id } = &saved_mode
        && !mode_options.contains(id)
    {
        mode_options.push(id.clone()); // resolução de uma tela desligada agora: continua escolhível
    }
    let mode = GameModeSetting {
        saved: saved_mode,
        default: default_mode,
        effective: effective_mode,
        options: mode_options,
    };

    let hdr = GameFlagSetting {
        saved: saved.hdr,
        default: defaults.hdr,
        effective: saved.hdr.unwrap_or(defaults.hdr),
    };
    let vrr = GameVrrSetting {
        saved: saved.vrr,
        default: defaults.vrr,
        effective: saved.vrr.unwrap_or(defaults.vrr),
        unsupported: shown_screen.is_some_and(|d| d.vrr == DisplayFeature::Unsupported),
        limitation: displays.as_ref().and_then(|d| d.vrr_limitation.clone()),
    };

    let can_switch = outputs.as_ref().is_some_and(|o| o.can_switch);
    let mut audio_options: Vec<GameDeviceOption> = outputs
        .iter()
        .flat_map(|o| &o.outputs)
        .map(|o| GameDeviceOption {
            id: o.id.clone(),
            name: o.name.clone(),
            connected: true,
        })
        .collect();
    let saved_audio = match saved.audio_device_id.as_deref() {
        Some(id) if !id.is_empty() => Some(
            audio_options
                .iter()
                .find(|o| same(&o.id, id))
                .map_or_else(|| id.to_owned(), |o| o.id.clone()),
        ),
        _ => None,
    };
    if let Some(id) = &saved_audio
        && !audio_options.iter().any(|o| o.id == *id)
    {
        let name = saved.audio_device_name.clone().filter(|n| !n.is_empty());
        audio_options.push(GameDeviceOption {
            id: id.clone(),
            name: name.unwrap_or_else(|| id.clone()),
            connected: false,
        });
    }
    let device = |c: AudioOutputChoice| GameDevice {
        id: c.device_id,
        name: c.name,
    };
    let audio = GameAudioSetting {
        saved: match (&saved_audio, saved.audio_device_id.as_deref()) {
            (Some(id), _) => GameChoice::Pick { id: id.clone() },
            (None, other) => choice(other),
        },
        default: global.game_audio.clone().map(device),
        effective: effective.game_audio.clone().map(device),
        options: audio_options,
        can_switch,
    };

    Ok(GameSettingsView {
        game_id: game_id.to_owned(),
        changes: saved.changed_count(),
        use_screen,
        screen,
        mode,
        hdr,
        vrr,
        audio,
        arguments: text_setting(saved.extra_arguments.as_deref(), effective.extra_arguments),
        working_directory: text_setting(
            saved.working_directory.as_deref(),
            effective.working_directory,
        ),
        environment: text_setting(
            saved.environment.as_deref(),
            join_environment(&effective.environment),
        ),
        before_command: text_setting(saved.before_command.as_deref(), effective.before_command),
        after_command: text_setting(saved.after_command.as_deref(), effective.after_command),
        commands_approved: effective.commands_approved,
    })
}

/// Uma linha só: sem controle (quebra de linha, tabulação) que bagunce a linha de comando.
fn one_line(value: &str, max: usize, what: &str) -> Result<Option<String>, BridgeError> {
    if value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(bad(&format!("{what} inválido")));
    }
    Ok(normalize_arguments(Some(value)))
}

/// `NOME=valor` separados por `;` ou por linha; cada item precisa ser uma variável válida.
fn environment(value: &str) -> Result<Option<String>, BridgeError> {
    if value.chars().count() > MAX_ENVIRONMENT_CHARS
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(bad("variáveis de ambiente inválidas"));
    }
    let items: Vec<&str> = value
        .split(['\n', ';'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect();
    if items.len() > MAX_ENVIRONMENT_ENTRIES {
        return Err(bad("variáveis de ambiente demais"));
    }
    let mut entries = Vec::new();
    for item in items {
        // Um item mal escrito seria ignorado em silêncio ao abrir o jogo: melhor recusar já.
        let [entry] = parse_environment(Some(item))
            .try_into()
            .map_err(|_| bad("variável de ambiente inválida: use NOME=valor"))?;
        entries.push(entry);
    }
    Ok(join_environment(&entries))
}

fn valid_text_id(value: &str) -> Result<(), BridgeError> {
    if value.trim().is_empty() || value.chars().count() > MAX_ID_CHARS || value.contains('\0') {
        return Err(bad("id inválido"));
    }
    Ok(())
}

/// Grava uma configuração do jogo e devolve as configurações já atualizadas.
pub(crate) fn set(
    core: &mut Core,
    request: GameSettingsSet,
) -> Result<GameSettingsView, BridgeError> {
    let GameSettingsSet { game_id, change } = request;
    check_game(core, &game_id)?;
    let mut value = core.game_settings.load(&game_id)?;
    apply(core, &mut value, change)?;
    core.game_settings.save(&game_id, value)?;
    view(core, &game_id)
}

fn apply(
    core: &Core,
    value: &mut GameSettingsOverride,
    change: GameSettingChange,
) -> Result<(), BridgeError> {
    match change {
        GameSettingChange::UseScreen(on) => value.use_game_screen = on,
        GameSettingChange::Hdr(on) => value.hdr = on,
        GameSettingChange::Vrr(on) => {
            if on.is_some() && vrr_unsupported(core, value) {
                return Err(BridgeError::new(
                    ErrorCode::Conflict,
                    "a tela não informa VRR",
                ));
            }
            value.vrr = on;
        }
        GameSettingChange::Screen(None) => {
            value.screen_id = None;
            value.screen_name = None;
            // As resoluções são de cada tela: a escolhida para a outra não vale aqui.
            value.screen_mode = None;
        }
        GameSettingChange::Screen(Some(id)) => {
            valid_text_id(&id)?;
            if normalize_arguments(value.screen_id.as_deref())
                .is_some_and(|saved| same(&saved, &id))
            {
                return Ok(());
            }
            let list = system::displays(&core.devices)?;
            let screen = list
                .displays
                .iter()
                .find(|d| same(&d.id, &id))
                .ok_or_else(|| not_found("tela"))?;
            value.screen_id = Some(screen.id.clone());
            value.screen_name = Some(screen.name.clone());
            value.screen_mode = None;
        }
        GameSettingChange::Mode(GameChoice::Default) => value.screen_mode = None,
        GameSettingChange::Mode(GameChoice::Keep) => {
            value.screen_mode = Some(GameSettingsOverride::KEEP_CURRENT_MODE.to_owned());
        }
        GameSettingChange::Mode(GameChoice::Pick { id }) => {
            let mode = DisplayMode::parse(&id).ok_or_else(|| bad("resolução inválida"))?;
            let key = mode.key();
            let already = value
                .screen_mode
                .as_deref()
                .and_then(DisplayMode::parse)
                .is_some_and(|saved| saved == mode);
            if !already {
                let screen_id = screen_id_of(core, value);
                if screen_id.is_empty() {
                    return Err(BridgeError::new(
                        ErrorCode::Conflict,
                        "nenhuma tela de jogo escolhida",
                    ));
                }
                let list = system::displays(&core.devices)?;
                let known = list
                    .displays
                    .iter()
                    .find(|d| same(&d.id, &screen_id))
                    .is_some_and(|d| d.modes.contains(&key));
                if !known {
                    return Err(not_found("resolução"));
                }
            }
            value.screen_mode = Some(key);
        }
        GameSettingChange::Audio(GameChoice::Default) => {
            value.audio_device_id = None;
            value.audio_device_name = None;
        }
        GameSettingChange::Audio(GameChoice::Keep) => {
            value.audio_device_id = Some(GameSettingsOverride::KEEP_AUDIO.to_owned());
            value.audio_device_name = None;
        }
        GameSettingChange::Audio(GameChoice::Pick { id }) => {
            valid_text_id(&id)?;
            let outputs = system::audio_outputs(&core.devices)?;
            if !outputs.can_switch {
                return Err(BridgeError::new(
                    ErrorCode::Conflict,
                    "o sistema não deixa trocar a saída de áudio",
                ));
            }
            let output = outputs
                .outputs
                .iter()
                .find(|o| same(&o.id, &id))
                .ok_or_else(|| not_found("saída de áudio"))?;
            value.audio_device_id = Some(output.id.clone());
            value.audio_device_name = Some(output.name.clone());
        }
        GameSettingChange::Arguments(text) => {
            value.extra_arguments = match text {
                Some(text) => one_line(&text, MAX_ARGUMENTS_CHARS, "argumentos")?,
                None => None,
            };
        }
        GameSettingChange::WorkingDirectory(text) => {
            value.working_directory = match text {
                Some(text) => {
                    let folder = one_line(&text, MAX_FOLDER_CHARS, "pasta de trabalho")?;
                    if folder.as_deref().is_some_and(|f| !Path::new(f).is_dir()) {
                        return Err(not_found("pasta de trabalho"));
                    }
                    folder
                }
                None => None,
            };
        }
        GameSettingChange::Environment(text) => {
            value.environment = match text {
                Some(text) => environment(&text)?,
                None => None,
            };
        }
        GameSettingChange::ResetAll => {
            // O FPS não aparece na interface, mas veio das alphas: não se perde (D-06).
            *value = GameSettingsOverride {
                fps_limit: value.fps_limit,
                show_fps: value.show_fps,
                ..Default::default()
            };
        }
    }
    Ok(())
}

/// Id da tela que o jogo usa: a dele ou a de jogo do Tako (vazio sem nenhuma).
fn screen_id_of(core: &Core, value: &GameSettingsOverride) -> String {
    normalize_arguments(value.screen_id.as_deref())
        .unwrap_or_else(|| text(core, "GameScreenId").trim().to_owned())
}

fn vrr_unsupported(core: &Core, value: &GameSettingsOverride) -> bool {
    let screen_id = screen_id_of(core, value);
    read_displays(core).is_some_and(|list| {
        list.displays
            .iter()
            .any(|d| same(&d.id, &screen_id) && d.vrr == DisplayFeature::Unsupported)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppService;
    use serde_json::{Value, json};
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    const DESK: &str = "DISPLAY1";
    const TV: &str = "DISPLAY2";

    /// Núcleo com banco de verdade num diretório temporário, a biblioteca de demonstração e a mesa simulada.
    fn app_at(path: &Path, simulate: bool) -> AppService {
        let core =
            Core::open_with_sources(DataPaths::at(path), Platform::simulated(), simulate, vec![])
                .unwrap();
        let mut app = AppService::with_core("test", core);
        // A leitura das lojas do boot troca a biblioteca quando termina: espera, e só então põe os jogos.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while matches!(app.handle(Call::LibrarySources), Ok(Reply::LibrarySources(s)) if s.scanning)
        {
            assert!(std::time::Instant::now() < deadline, "leitura não terminou");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        if !simulate {
            app.reload_library(crate::demo::games()).unwrap();
        }
        app
    }

    fn app() -> (AppService, tempfile::TempDir, String) {
        let dir = tempfile::tempdir().unwrap();
        let app = app_at(dir.path(), false);
        let id = crate::demo::games()[0].id.clone();
        (app, dir, id)
    }

    fn wire(app: &mut AppService, name: &str, payload: Value) -> Value {
        let value = json!({"v": 1, "id": 5, "name": name, "payload": payload});
        let response = match parse_request(&value.to_string()) {
            Ok(request) => match app.handle(request.call) {
                Ok(reply) => Response::ok(request.id, reply),
                Err(error) => Response::err(request.id, error),
            },
            Err((id, error)) => Response::err(id, error),
        };
        serde_json::to_value(response).unwrap()
    }

    fn get(app: &mut AppService, id: &str) -> GameSettingsView {
        match app
            .handle(Call::GameSettings(GameRequest { game_id: id.into() }))
            .unwrap()
        {
            Reply::GameSettings(view) => *view,
            other => panic!("{other:?}"),
        }
    }

    fn put(
        app: &mut AppService,
        id: &str,
        change: GameSettingChange,
    ) -> Result<GameSettingsView, BridgeError> {
        app.handle(Call::GameSettingsSet(GameSettingsSet {
            game_id: id.into(),
            change,
        }))
        .map(|reply| match reply {
            Reply::GameSettingsSet(view) => *view,
            other => panic!("{other:?}"),
        })
    }

    fn setting(app: &mut AppService, key: &str, value: &str) {
        app.handle(Call::SettingsSet(Setting {
            key: key.into(),
            value: value.into(),
        }))
        .unwrap();
    }

    fn code(result: Result<GameSettingsView, BridgeError>) -> ErrorCode {
        result.unwrap_err().code
    }

    #[test]
    fn a_game_without_choices_follows_the_tako_defaults() {
        let (mut app, _dir, id) = app();
        let view = get(&mut app, &id);
        assert_eq!(view.game_id, id);
        assert_eq!(view.changes, 0);
        assert_eq!(
            view.use_screen,
            GameFlagSetting {
                saved: None,
                default: false,
                effective: false
            }
        );
        assert_eq!(view.screen.saved, None);
        assert_eq!(view.screen.default, None);
        assert_eq!(view.screen.effective, None);
        assert!(!view.screen.no_screen_chosen);
        assert_eq!(view.screen.options.len(), 2);
        assert!(view.screen.options.iter().all(|o| o.connected));
        assert_eq!(view.mode.saved, GameChoice::Default);
        assert_eq!((view.mode.default, view.mode.effective), (None, None));
        assert!(
            view.mode.options.is_empty(),
            "sem tela de jogo não há resoluções"
        );
        assert_eq!(
            view.hdr,
            GameFlagSetting {
                saved: None,
                default: false,
                effective: false
            }
        );
        assert_eq!(view.audio.saved, GameChoice::Default);
        assert_eq!(view.audio.effective, None);
        assert!(view.audio.can_switch);
        assert_eq!(view.audio.options.len(), 2);
        assert_eq!(
            view.arguments,
            GameTextSetting {
                saved: None,
                effective: None
            }
        );
        assert!(view.commands_approved);
    }

    #[test]
    fn defaults_come_from_the_tako_settings_and_the_effective_value_follows_the_game() {
        let (mut app, _dir, id) = app();
        for (key, value) in [
            ("GameScreenEnabled", "true"),
            ("GameScreenId", TV),
            ("GameScreenName", "TV da sala"),
            ("GameScreenMode", "3840x2160@120"),
            ("GameScreenHdr", "true"),
            ("GameAudioDeviceId", "tv"),
            ("GameAudioDeviceName", "TV (HDMI)"),
        ] {
            setting(&mut app, key, value);
        }
        let view = get(&mut app, &id);
        assert_eq!(
            view.use_screen,
            GameFlagSetting {
                saved: None,
                default: true,
                effective: true
            }
        );
        assert_eq!(
            view.screen.default,
            Some(GameDevice {
                id: TV.into(),
                name: "TV da sala".into()
            })
        );
        assert_eq!(view.screen.effective.as_deref(), Some(TV));
        assert_eq!(view.mode.default.as_deref(), Some("3840x2160@120"));
        assert_eq!(view.mode.effective.as_deref(), Some("3840x2160@120"));
        assert_eq!(
            view.mode.options,
            ["3840x2160@120", "3840x2160@60", "1920x1080@120"]
        );
        assert!(view.hdr.effective && view.hdr.saved.is_none());
        assert!(!view.vrr.unsupported, "a TV simulada informa VRR");
        assert_eq!(
            view.audio.default,
            Some(GameDevice {
                id: "tv".into(),
                name: "TV (HDMI)".into()
            })
        );
        assert_eq!(view.audio.effective, view.audio.default);

        // O jogo muda o HDR e a resolução: o efetivo segue o jogo; o padrão não se mexe.
        put(&mut app, &id, GameSettingChange::Hdr(Some(false))).unwrap();
        let view = put(
            &mut app,
            &id,
            GameSettingChange::Mode(GameChoice::Pick {
                id: "1920x1080@120".into(),
            }),
        )
        .unwrap();
        assert_eq!(
            view.hdr,
            GameFlagSetting {
                saved: Some(false),
                default: true,
                effective: false
            }
        );
        assert_eq!(
            view.mode.saved,
            GameChoice::Pick {
                id: "1920x1080@120".into()
            }
        );
        assert_eq!(view.mode.default.as_deref(), Some("3840x2160@120"));
        assert_eq!(view.mode.effective.as_deref(), Some("1920x1080@120"));
        assert_eq!(view.changes, 2);

        let view = put(&mut app, &id, GameSettingChange::Mode(GameChoice::Keep)).unwrap();
        assert_eq!(view.mode.saved, GameChoice::Keep);
        assert_eq!(view.mode.effective, None);
        let view = put(&mut app, &id, GameSettingChange::Audio(GameChoice::Keep)).unwrap();
        assert_eq!(view.audio.saved, GameChoice::Keep);
        assert_eq!(view.audio.effective, None);
        assert!(view.audio.default.is_some());
    }

    #[test]
    fn what_the_view_calls_effective_is_what_the_session_resolves() {
        let (mut app, _dir, id) = app();
        for (key, value) in [
            ("GameScreenEnabled", "true"),
            ("GameScreenId", TV),
            ("GameScreenMode", "3840x2160@120"),
            ("GameAudioDeviceId", "tv"),
            ("GameAudioDeviceName", "TV (HDMI)"),
        ] {
            setting(&mut app, key, value);
        }
        put(&mut app, &id, GameSettingChange::Screen(Some(DESK.into()))).unwrap();
        put(&mut app, &id, GameSettingChange::Hdr(Some(true))).unwrap();
        put(
            &mut app,
            &id,
            GameSettingChange::Arguments(Some("-dx11".into())),
        )
        .unwrap();
        let view = put(
            &mut app,
            &id,
            GameSettingChange::Environment(Some("A=1; B=2".into())),
        )
        .unwrap();

        let core = app.core.as_ref().unwrap();
        let session = resolve(
            &global_settings(core),
            Some(&core.game_settings.load(&id).unwrap()),
        );
        let screen = session.game_screen.unwrap();
        assert_eq!(
            view.screen.effective.as_deref(),
            Some(screen.display_id.as_str())
        );
        // Em outra tela a resolução do Tako não vale: mantém a atual.
        assert_eq!((view.mode.effective, screen.mode), (None, None));
        assert_eq!((view.hdr.effective, screen.enable_hdr), (true, true));
        assert_eq!(view.vrr.effective, screen.enable_vrr);
        assert_eq!(view.arguments.effective, session.extra_arguments);
        assert_eq!(view.environment.effective.as_deref(), Some("A=1;B=2"));
        assert!(
            !view.commands_approved && !session.commands_approved,
            "argumentos e variáveis pedem permissão"
        );
    }

    #[test]
    fn choices_are_saved_in_the_database_and_survive_a_restart() {
        let (mut app, dir, id) = app();
        setting(&mut app, "GameScreenId", DESK);
        put(&mut app, &id, GameSettingChange::UseScreen(Some(true))).unwrap();
        put(&mut app, &id, GameSettingChange::Vrr(None)).unwrap();
        put(
            &mut app,
            &id,
            GameSettingChange::Audio(GameChoice::Pick { id: "TV".into() }),
        )
        .unwrap();
        let folder = dir.path().to_string_lossy().into_owned();
        put(
            &mut app,
            &id,
            GameSettingChange::WorkingDirectory(Some(format!("  {folder} "))),
        )
        .unwrap();
        drop(app);

        let mut app = app_at(dir.path(), false);
        let view = get(&mut app, &id);
        assert_eq!(view.use_screen.saved, Some(true));
        // O id vem na grafia do sistema, não na que a interface mandou.
        assert_eq!(view.audio.saved, GameChoice::Pick { id: "tv".into() });
        assert_eq!(
            view.working_directory.saved.as_deref(),
            Some(folder.as_str())
        );
        assert_eq!(view.changes, 3);
    }

    #[test]
    fn simulated_choices_stay_in_memory() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_at(dir.path(), true);
        let id = crate::demo::games()[0].id.clone();
        put(&mut app, &id, GameSettingChange::Hdr(Some(true))).unwrap();
        assert_eq!(get(&mut app, &id).hdr.saved, Some(true));
        drop(app);
        let mut app = app_at(dir.path(), true);
        assert_eq!(get(&mut app, &id).hdr.saved, None);
        assert!(!dir.path().join("tako.db").exists());
    }

    #[test]
    fn the_game_screen_must_exist_and_changing_it_forgets_the_resolution() {
        let (mut app, _dir, id) = app();
        assert_eq!(
            code(put(
                &mut app,
                &id,
                GameSettingChange::Screen(Some("DISPLAY9".into()))
            )),
            ErrorCode::NotFound
        );
        let view = put(
            &mut app,
            &id,
            GameSettingChange::Screen(Some("display2".into())),
        )
        .unwrap();
        assert_eq!(view.screen.saved.as_deref(), Some(TV));
        assert_eq!(view.screen.effective.as_deref(), Some(TV));
        // Escolher a tela já é pedir para usá-la, e a lista de resoluções passa a ser a dela.
        assert!(view.mode.options.contains(&"3840x2160@120".to_owned()));
        let view = put(
            &mut app,
            &id,
            GameSettingChange::Mode(GameChoice::Pick {
                id: "3840x2160@60".into(),
            }),
        )
        .unwrap();
        assert_eq!(view.mode.effective.as_deref(), Some("3840x2160@60"));
        // Escolher a mesma tela de novo não apaga a resolução.
        let view = put(&mut app, &id, GameSettingChange::Screen(Some(TV.into()))).unwrap();
        assert_eq!(
            view.mode.saved,
            GameChoice::Pick {
                id: "3840x2160@60".into()
            }
        );
        let view = put(&mut app, &id, GameSettingChange::Screen(Some(DESK.into()))).unwrap();
        assert_eq!(view.mode.saved, GameChoice::Default);
        assert_eq!(view.screen.saved.as_deref(), Some(DESK));
        let view = put(&mut app, &id, GameSettingChange::Screen(None)).unwrap();
        assert_eq!((view.screen.saved, view.screen.effective), (None, None));
        assert_eq!(view.changes, 0);
    }

    #[test]
    fn a_saved_screen_that_is_gone_stays_in_the_list_and_can_be_kept() {
        let (mut app, _dir, id) = app();
        put(&mut app, &id, GameSettingChange::Screen(Some(TV.into()))).unwrap();
        // Guarda uma tela que o sistema não tem mais, como se tivesse sido desligada.
        let core = app.core.as_mut().unwrap();
        let mut value = core.game_settings.load(&id).unwrap();
        value.screen_id = Some(r"\\.\DISPLAY7".into());
        value.screen_name = Some("Projetor".into());
        value.screen_mode = Some("1280x720@50".into());
        core.game_settings.save(&id, value).unwrap();

        let view = get(&mut app, &id);
        let gone = view.screen.options.last().unwrap();
        assert_eq!((gone.name.as_str(), gone.connected), ("Projetor", false));
        assert_eq!(view.mode.options, ["1280x720@50"]);
        // Reafirmar a escolha que já estava gravada é permitido; trocar para outra desconhecida, não.
        put(
            &mut app,
            &id,
            GameSettingChange::Screen(Some(r"\\.\display7".into())),
        )
        .unwrap();
        put(
            &mut app,
            &id,
            GameSettingChange::Mode(GameChoice::Pick {
                id: "1280x720@50".into(),
            }),
        )
        .unwrap();
        assert_eq!(
            code(put(
                &mut app,
                &id,
                GameSettingChange::Mode(GameChoice::Pick {
                    id: "800x600@60".into()
                })
            )),
            ErrorCode::NotFound
        );
    }

    #[test]
    fn the_resolution_needs_a_screen_and_one_of_its_modes() {
        let (mut app, _dir, id) = app();
        let pick = |key: &str| GameSettingChange::Mode(GameChoice::Pick { id: key.into() });
        assert_eq!(
            code(put(&mut app, &id, pick("1920x1080@60"))),
            ErrorCode::Conflict
        );
        // "Manter a atual" não precisa de tela.
        put(&mut app, &id, GameSettingChange::Mode(GameChoice::Keep)).unwrap();
        setting(&mut app, "GameScreenId", DESK);
        assert_eq!(
            code(put(&mut app, &id, pick("lixo"))),
            ErrorCode::BadRequest
        );
        assert_eq!(
            code(put(&mut app, &id, pick("3840x2160@120"))),
            ErrorCode::NotFound
        );
        let view = put(&mut app, &id, pick("1920x1080@144")).unwrap();
        assert_eq!(
            view.mode.saved,
            GameChoice::Pick {
                id: "1920x1080@144".into()
            }
        );
        let view = put(&mut app, &id, GameSettingChange::Mode(GameChoice::Default)).unwrap();
        assert_eq!(view.mode.saved, GameChoice::Default);
    }

    #[test]
    fn vrr_is_refused_on_a_screen_without_it() {
        let (mut app, _dir, id) = app();
        setting(&mut app, "GameScreenId", DESK);
        let view = get(&mut app, &id);
        assert!(view.vrr.unsupported);
        assert_eq!(
            code(put(&mut app, &id, GameSettingChange::Vrr(Some(true)))),
            ErrorCode::Conflict
        );
        // Voltar ao padrão é sempre possível.
        put(&mut app, &id, GameSettingChange::Vrr(None)).unwrap();
        put(&mut app, &id, GameSettingChange::Screen(Some(TV.into()))).unwrap();
        let view = put(&mut app, &id, GameSettingChange::Vrr(Some(true))).unwrap();
        assert!(!view.vrr.unsupported);
        assert_eq!((view.vrr.saved, view.vrr.effective), (Some(true), true));
    }

    #[test]
    fn the_audio_output_must_exist_and_the_system_must_allow_switching() {
        let (mut app, _dir, id) = app();
        assert_eq!(
            code(put(
                &mut app,
                &id,
                GameSettingChange::Audio(GameChoice::Pick { id: "fone".into() })
            )),
            ErrorCode::NotFound
        );
        let view = put(
            &mut app,
            &id,
            GameSettingChange::Audio(GameChoice::Pick { id: "tv".into() }),
        )
        .unwrap();
        let effective = view.audio.effective.unwrap();
        assert_eq!(
            (effective.id.as_str(), effective.name.as_str()),
            ("tv", "TV (HDMI)")
        );
        let view = put(&mut app, &id, GameSettingChange::Audio(GameChoice::Default)).unwrap();
        assert_eq!((view.audio.saved, view.changes), (GameChoice::Default, 0));

        // Saída guardada e desconectada continua na lista, com o nome que o jogo guardou.
        let core = app.core.as_mut().unwrap();
        let mut value = core.game_settings.load(&id).unwrap();
        value.audio_device_id = Some("{fone}".into());
        value.audio_device_name = Some("Fone".into());
        core.game_settings.save(&id, value).unwrap();
        let view = get(&mut app, &id);
        let gone = view.audio.options.last().unwrap();
        assert_eq!((gone.name.as_str(), gone.connected), ("Fone", false));
        assert_eq!(
            view.audio.effective,
            Some(GameDevice {
                id: "{fone}".into(),
                name: "Fone".into()
            })
        );
    }

    #[test]
    fn text_fields_are_trimmed_cleared_and_limited() {
        let (mut app, dir, id) = app();
        let view = put(
            &mut app,
            &id,
            GameSettingChange::Arguments(Some("  -dx11  -windowed ".into())),
        )
        .unwrap();
        assert_eq!(view.arguments.saved.as_deref(), Some("-dx11  -windowed"));
        assert_eq!(view.arguments.effective, view.arguments.saved);
        assert_eq!(view.changes, 1);
        // Em branco e `null` limpam.
        let view = put(
            &mut app,
            &id,
            GameSettingChange::Arguments(Some("   ".into())),
        )
        .unwrap();
        assert_eq!((view.arguments.saved, view.changes), (None, 0));
        put(
            &mut app,
            &id,
            GameSettingChange::Arguments(Some("-a".into())),
        )
        .unwrap();
        let view = put(&mut app, &id, GameSettingChange::Arguments(None)).unwrap();
        assert_eq!(view.arguments.saved, None);

        let at_limit = "a".repeat(MAX_ARGUMENTS_CHARS);
        put(&mut app, &id, GameSettingChange::Arguments(Some(at_limit))).unwrap();
        let too_long = "a".repeat(MAX_ARGUMENTS_CHARS + 1);
        assert_eq!(
            code(put(
                &mut app,
                &id,
                GameSettingChange::Arguments(Some(too_long))
            )),
            ErrorCode::BadRequest
        );
        for bad_text in ["-a\n-b", "-a\0", "-a\t-b"] {
            assert_eq!(
                code(put(
                    &mut app,
                    &id,
                    GameSettingChange::Arguments(Some(bad_text.into()))
                )),
                ErrorCode::BadRequest,
                "{bad_text:?}"
            );
        }

        let folder = dir.path().to_string_lossy().into_owned();
        let view = put(
            &mut app,
            &id,
            GameSettingChange::WorkingDirectory(Some(folder.clone())),
        )
        .unwrap();
        assert_eq!(
            view.working_directory.saved.as_deref(),
            Some(folder.as_str())
        );
        let missing = dir.path().join("nao-existe").to_string_lossy().into_owned();
        assert_eq!(
            code(put(
                &mut app,
                &id,
                GameSettingChange::WorkingDirectory(Some(missing))
            )),
            ErrorCode::NotFound
        );
        let long = format!("{folder}{}", "a".repeat(MAX_FOLDER_CHARS));
        assert_eq!(
            code(put(
                &mut app,
                &id,
                GameSettingChange::WorkingDirectory(Some(long))
            )),
            ErrorCode::BadRequest
        );
        // A pasta que falhou não apagou a que estava gravada.
        assert_eq!(
            get(&mut app, &id).working_directory.saved.as_deref(),
            Some(folder.as_str())
        );
        let view = put(
            &mut app,
            &id,
            GameSettingChange::WorkingDirectory(Some(String::new())),
        )
        .unwrap();
        assert_eq!(view.working_directory.saved, None);
    }

    #[test]
    fn environment_variables_are_validated_and_limited_in_number() {
        let (mut app, _dir, id) = app();
        let set_env = |app: &mut AppService, text: &str| {
            put(app, &id, GameSettingChange::Environment(Some(text.into())))
        };
        let view = set_env(&mut app, " DXVK_HUD=fps ;\nMANGOHUD=1;; ").unwrap();
        assert_eq!(
            view.environment.saved.as_deref(),
            Some("DXVK_HUD=fps;MANGOHUD=1")
        );
        assert_eq!(view.environment.effective, view.environment.saved);
        for bad_text in [
            "SEM_IGUAL",
            "com espaço=1",
            "=vazio",
            "9X=1",
            "A=1;B",
            "A=\u{7}",
        ] {
            assert_eq!(
                code(set_env(&mut app, bad_text)),
                ErrorCode::BadRequest,
                "{bad_text:?}"
            );
        }
        let many = (0..=MAX_ENVIRONMENT_ENTRIES)
            .map(|i| format!("V{i}=1"))
            .collect::<Vec<_>>();
        assert_eq!(
            code(set_env(&mut app, &many.join(";"))),
            ErrorCode::BadRequest
        );
        set_env(&mut app, &many[..MAX_ENVIRONMENT_ENTRIES].join(";")).unwrap();
        let huge = format!("A={}", "x".repeat(MAX_ENVIRONMENT_CHARS));
        assert_eq!(code(set_env(&mut app, &huge)), ErrorCode::BadRequest);
        let view = put(&mut app, &id, GameSettingChange::Environment(None)).unwrap();
        assert_eq!(
            view.environment,
            GameTextSetting {
                saved: None,
                effective: None
            }
        );
    }

    #[test]
    fn use_everything_from_tako_clears_the_choices_but_keeps_the_hidden_fps() {
        let (mut app, _dir, id) = app();
        put(&mut app, &id, GameSettingChange::Hdr(Some(true))).unwrap();
        put(
            &mut app,
            &id,
            GameSettingChange::Arguments(Some("-a".into())),
        )
        .unwrap();
        let core = app.core.as_mut().unwrap();
        let mut value = core.game_settings.load(&id).unwrap();
        value.show_fps = Some(true);
        value.before_command = Some("mount.bat".into());
        core.game_settings.save(&id, value).unwrap();
        assert_eq!(get(&mut app, &id).changes, 3);

        let view = put(&mut app, &id, GameSettingChange::ResetAll).unwrap();
        assert_eq!(view.changes, 0);
        assert_eq!(
            (
                view.hdr.saved,
                view.arguments.saved,
                view.before_command.saved
            ),
            (None, None, None)
        );
        let core = app.core.as_ref().unwrap();
        assert_eq!(core.game_settings.load(&id).unwrap().show_fps, Some(true));
    }

    #[test]
    fn commands_are_shown_but_cannot_be_written_yet() {
        let (mut app, _dir, id) = app();
        let core = app.core.as_mut().unwrap();
        let mut value = core.game_settings.load(&id).unwrap();
        value.before_command = Some(" mount.bat ".into());
        value.after_command = Some("umount.bat".into());
        core.game_settings.save(&id, value).unwrap();
        let view = get(&mut app, &id);
        assert_eq!(view.before_command.saved.as_deref(), Some("mount.bat"));
        assert_eq!(view.after_command.effective.as_deref(), Some("umount.bat"));
        assert!(!view.commands_approved);
        assert_eq!(view.changes, 2);
        // Não há como mandar um comando pela ponte: o campo nem existe no contrato.
        let reply = wire(
            &mut app,
            "game.settings.set",
            json!({"gameId": id, "change": {"field": "before_command", "value": "calc.exe"}}),
        );
        assert_eq!(reply["ok"], false, "{reply}");
        assert_eq!(reply["error"]["code"], "bad_request");
    }

    #[test]
    fn unknown_or_invalid_games_are_refused() {
        let (mut app, _dir, _) = app();
        let get_code = |app: &mut AppService, id: &str| {
            app.handle(Call::GameSettings(GameRequest { game_id: id.into() }))
                .unwrap_err()
                .code
        };
        assert_eq!(get_code(&mut app, "steam:nao-existe"), ErrorCode::NotFound);
        assert_eq!(get_code(&mut app, "  "), ErrorCode::BadRequest);
        assert_eq!(
            get_code(&mut app, &"x".repeat(MAX_ID_CHARS + 1)),
            ErrorCode::BadRequest
        );
        assert_eq!(
            code(put(
                &mut app,
                "steam:nao-existe",
                GameSettingChange::Hdr(Some(true))
            )),
            ErrorCode::NotFound
        );
        let mut bare = AppService::new("1");
        let request = GameRequest {
            game_id: "x".into(),
        };
        assert_eq!(
            bare.handle(Call::GameSettings(request)).unwrap_err().code,
            ErrorCode::Internal
        );
    }

    #[test]
    fn the_wire_format_is_the_documented_one() {
        let (mut app, _dir, id) = app();
        let reply = wire(&mut app, "game.settings", json!({"gameId": id}));
        assert_eq!(reply["ok"], true, "{reply}");
        let data = &reply["reply"]["data"];
        assert_eq!(reply["reply"]["name"], "game.settings");
        assert_eq!(data["gameId"], id);
        assert_eq!(
            data["useScreen"],
            json!({"saved": null, "default": false, "effective": false})
        );
        assert_eq!(data["mode"]["saved"], json!({"kind": "default"}));
        assert_eq!(data["audio"]["canSwitch"], true);

        let set = |app: &mut AppService, change: Value| {
            wire(
                app,
                "game.settings.set",
                json!({"gameId": id, "change": change}),
            )
        };
        let reply = set(&mut app, json!({"field": "hdr", "value": true}));
        assert_eq!(reply["reply"]["name"], "game.settings.set");
        assert_eq!(reply["reply"]["data"]["hdr"]["saved"], true);
        let reply = set(
            &mut app,
            json!({"field": "mode", "value": {"kind": "keep"}}),
        );
        assert_eq!(
            reply["reply"]["data"]["mode"]["saved"],
            json!({"kind": "keep"})
        );
        let reply = set(
            &mut app,
            json!({"field": "audio", "value": {"kind": "pick", "id": "tv"}}),
        );
        assert_eq!(
            reply["reply"]["data"]["audio"]["saved"],
            json!({"kind": "pick", "id": "tv"})
        );
        let reply = set(&mut app, json!({"field": "hdr", "value": null}));
        assert_eq!(reply["reply"]["data"]["hdr"]["saved"], Value::Null);
        let reply = set(&mut app, json!({"field": "reset_all"}));
        assert_eq!(reply["reply"]["data"]["changes"], 0, "{reply}");

        for bad_request in [
            json!({"gameId": id}),
            json!({"gameId": id, "change": {"field": "hdr", "value": "sim"}}),
            json!({"gameId": id, "change": {"field": "nada", "value": true}}),
            json!({"gameId": id, "change": {"field": "hdr", "value": true}, "extra": 1}),
            json!([id]),
        ] {
            let reply = wire(&mut app, "game.settings.set", bad_request.clone());
            assert_eq!(reply["error"]["code"], "bad_request", "{bad_request}");
        }
        let reply = wire(&mut app, "game.settings", json!({"gameId": id, "x": 1}));
        assert_eq!(reply["error"]["code"], "bad_request");
    }

    #[test]
    fn game_settings_do_not_announce_a_library_change() {
        let (mut app, _dir, id) = app();
        let (tx, rx) = std::sync::mpsc::channel();
        app.set_event_sink(tx);
        put(&mut app, &id, GameSettingChange::Hdr(Some(true))).unwrap();
        assert!(rx.try_recv().is_err());
    }
}
