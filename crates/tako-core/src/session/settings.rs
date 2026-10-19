//! Configurações por jogo (port de `Session/GameSettings.cs`). O resolvedor (global + jogo) é do MIG-024.

use serde::{Deserialize, Serialize};

/// Ajustes de um jogo. `None` = "Padrão" (usa o do Tako). Gravado em `game_settings`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GameSettingsOverride {
    pub use_game_screen: Option<bool>,
    /// Tela deste jogo (PERF-002), no lugar da tela de jogo do Tako. A resolução e o HDR do jogo valem para ela.
    #[serde(default)]
    pub screen_id: Option<String>,
    /// Nome da tela escolhida, para mostrar quando ela estiver desconectada.
    #[serde(default)]
    pub screen_name: Option<String>,
    /// Chave do modo (`1920x1080@60`) ou [`GameSettingsOverride::KEEP_CURRENT_MODE`].
    pub screen_mode: Option<String>,
    pub hdr: Option<bool>,
    /// Id da saída de áudio ou [`GameSettingsOverride::KEEP_AUDIO`].
    pub audio_device_id: Option<String>,
    pub audio_device_name: Option<String>,
    /// Fora do escopo do Rust (FPS é do Console Mode), mas preservado para não perder dados das alphas.
    pub fps_limit: Option<i32>,
    pub show_fps: Option<bool>,
    pub extra_arguments: Option<String>,
    /// Pasta de trabalho do jogo (PRD §6); vazio = a do executável.
    pub working_directory: Option<String>,
    /// Variáveis de ambiente (SESSION-004): `NOME=valor`, separadas por `;` ou por linha.
    #[serde(default)]
    pub environment: Option<String>,
    /// Comando rodado antes de abrir o jogo (espera até [`BEFORE_COMMAND_WAIT`]).
    #[serde(default)]
    pub before_command: Option<String>,
    /// Comando rodado quando o jogo termina.
    #[serde(default)]
    pub after_command: Option<String>,
    /// Os comandos que o usuário permitiu, exatamente como estavam (ver [`commands_text`]). Mudou o texto, pergunta
    /// de novo: comandos vindos de fora (importação, extensão) nunca rodam sem o usuário ver.
    #[serde(default)]
    pub approved_commands: Option<String>,
    /// Programas abertos junto do jogo (EXT-001), separados por `;` ou por linha. Entram na permissão dos comandos.
    #[serde(default)]
    pub tools: Option<String>,
    /// Fechar esses programas quando o jogo terminar.
    #[serde(default)]
    pub close_tools: Option<bool>,
}

/// Quanto o comando de antes pode demorar antes de o jogo abrir mesmo assim.
pub const BEFORE_COMMAND_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

/// Os comandos e as ferramentas num texto só, para comparar com o que foi permitido; `None` sem nenhum.
/// Sem ferramentas o texto é o do SESSION-004, então o que já foi permitido continua valendo.
pub fn commands_text(
    before: Option<&str>,
    after: Option<&str>,
    tools: Option<&str>,
) -> Option<String> {
    let before = normalize_arguments(before).unwrap_or_default();
    let after = normalize_arguments(after).unwrap_or_default();
    let mut text = format!("{before}\n{after}");
    for t in parse_tools(tools) {
        text = format!("{text}\n{t}");
    }
    (text != "\n").then_some(text)
}

/// Caminhos das ferramentas, separados por `;` ou por linha, sem os vazios.
// shortcut: caminho com `;` não cabe; trocar por uma lista própria se alguém precisar.
pub fn parse_tools(text: Option<&str>) -> Vec<String> {
    text.unwrap_or_default()
        .split(['\n', ';'])
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(Into::into)
        .collect()
}

/// `NOME=valor` separados por `;` ou por linha. Nome inválido (vazio, com espaço ou `=`) é ignorado.
pub fn parse_environment(text: Option<&str>) -> Vec<(String, String)> {
    text.unwrap_or_default()
        .split(['\n', ';'])
        .filter_map(|item| {
            let (name, value) = item.split_once('=')?;
            let name = name.trim();
            let valid = name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            valid.then(|| (name.to_owned(), value.trim().to_owned()))
        })
        .collect()
}

impl GameSettingsOverride {
    /// "Manter a resolução atual".
    pub const KEEP_CURRENT_MODE: &'static str = "";
    /// "Não alterar o áudio".
    pub const KEEP_AUDIO: &'static str = "";

    pub fn changed_count(&self) -> usize {
        [
            self.use_game_screen.is_some(),
            normalize_arguments(self.screen_id.as_deref()).is_some(),
            self.screen_mode.is_some(),
            self.hdr.is_some(),
            self.audio_device_id.is_some(),
            self.extra_arguments
                .as_deref()
                .is_some_and(|a| !a.trim().is_empty()),
            self.working_directory
                .as_deref()
                .is_some_and(|a| !a.trim().is_empty()),
            !parse_environment(self.environment.as_deref()).is_empty(),
            normalize_arguments(self.before_command.as_deref()).is_some(),
            normalize_arguments(self.after_command.as_deref()).is_some(),
            !parse_tools(self.tools.as_deref()).is_empty(),
            self.close_tools == Some(true),
        ]
        .into_iter()
        .filter(|b| *b)
        .count()
    }

    /// FPS não conta: não aparece mais na interface.
    pub fn is_default(&self) -> bool {
        self.changed_count() == 0
    }

    /// O usuário viu e permitiu os comandos como estão agora.
    pub fn approve_commands(&mut self) {
        self.approved_commands = commands_text(
            self.before_command.as_deref(),
            self.after_command.as_deref(),
            self.tools.as_deref(),
        );
    }
}

use super::audio::AudioOutputChoice;
use super::screen::GameScreenSettings;
use crate::devices::DisplayMode;

/// Configurações do Tako que a sessão de jogo usa.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GlobalSessionSettings {
    pub game_screen_enabled: bool,
    /// Tela escolhida, mesmo com a tela de jogo desligada (um jogo pode pedir para usá-la).
    pub game_screen_id: String,
    pub game_screen_mode: Option<DisplayMode>,
    pub game_screen_hdr: bool,
    pub game_audio: Option<AudioOutputChoice>,
    /// O que o Tako lembra de cada monitor (PERF-002).
    pub monitor_presets: MonitorPresets,
}

impl GlobalSessionSettings {
    /// Resolução e HDR que valem numa tela quando o jogo não escolhe: na tela de jogo do Tako, os de
    /// Configurações; em outra, o preset dela (sem preset, mantém a resolução atual).
    pub fn screen_defaults(&self, display_id: &str) -> (Option<DisplayMode>, bool) {
        let display_id = display_id.trim();
        if self.game_screen_id.trim().eq_ignore_ascii_case(display_id) {
            return (self.game_screen_mode, self.game_screen_hdr);
        }
        match self.monitor_presets.get(display_id) {
            Some(p) => (p.mode, p.hdr),
            None => (None, self.game_screen_hdr),
        }
    }
}

/// Resolução (com a taxa) e HDR escolhidos para um monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MonitorPreset {
    /// `None` = manter a resolução atual.
    pub mode: Option<DisplayMode>,
    pub hdr: bool,
}

/// Presets por monitor, gravados como JSON numa configuração:
/// `{"<id da tela>": {"mode": "1920x1080@60", "hdr": true}}`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MonitorPresets(Vec<(String, MonitorPreset)>);

#[derive(Deserialize)]
struct StoredPreset {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    hdr: bool,
}

impl MonitorPresets {
    /// Bem mais telas do que alguém conecta: o resto de um valor estranho é ignorado.
    const MAX: usize = 32;

    /// Valor quebrado volta ao padrão (sem presets); uma tela ilegível é ignorada e uma resolução ilegível
    /// vira "manter a atual".
    pub fn parse(text: &str) -> MonitorPresets {
        let map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(text).unwrap_or_default();
        let mut presets = MonitorPresets::default();
        for (id, value) in map.into_iter().take(Self::MAX) {
            if let Ok(p) = serde_json::from_value::<StoredPreset>(value) {
                let mode = DisplayMode::parse(&p.mode);
                presets.set(&id, MonitorPreset { mode, hdr: p.hdr });
            }
        }
        presets
    }

    pub fn to_text(&self) -> String {
        let map: serde_json::Map<String, serde_json::Value> = self
            .0
            .iter()
            .map(|(id, p)| {
                let mode = p.mode.map(|m| m.key()).unwrap_or_default();
                (
                    id.clone(),
                    serde_json::json!({ "mode": mode, "hdr": p.hdr }),
                )
            })
            .collect();
        serde_json::Value::Object(map).to_string()
    }

    /// O id da tela não diferencia maiúsculas (o Windows não diferencia).
    pub fn get(&self, display_id: &str) -> Option<MonitorPreset> {
        let id = display_id.trim();
        self.0
            .iter()
            .find(|(i, _)| i.eq_ignore_ascii_case(id))
            .map(|(_, p)| *p)
    }

    pub fn set(&mut self, display_id: &str, preset: MonitorPreset) {
        let id = display_id.trim();
        if id.is_empty() {
            return;
        }
        self.0.retain(|(i, _)| !i.eq_ignore_ascii_case(id));
        self.0.push((id.to_owned(), preset));
    }
}

/// Por que o jogo pediu a tela de jogo e ela não pode ser usada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameScreenProblem {
    NoScreenChosen,
}

impl GameScreenProblem {
    pub fn message(self) -> String {
        match self {
            GameScreenProblem::NoScreenChosen => crate::i18n::t("session.screen.none_chosen"),
        }
    }
}

/// O que vale na sessão deste jogo: o global com as escolhas do jogo por cima. FPS não entra (é do
/// Console Mode).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EffectiveSessionSettings {
    /// `None` = não usar a tela de jogo.
    pub game_screen: Option<GameScreenSettings>,
    pub game_screen_problem: Option<GameScreenProblem>,
    pub game_audio: Option<AudioOutputChoice>,
    pub extra_arguments: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Vec<(String, String)>,
    pub before_command: Option<String>,
    pub after_command: Option<String>,
    /// Sem comandos, ou com os comandos já permitidos como estão.
    pub commands_approved: bool,
    /// Programas abertos junto do jogo (EXT-001); só com `commands_approved`.
    pub tools: Vec<String>,
    pub close_tools: bool,
}

impl EffectiveSessionSettings {
    /// Argumentos do jogo seguidos dos extras deste jogo.
    pub fn arguments(&self, game_arguments: Option<&str>) -> String {
        join_arguments(game_arguments, self.extra_arguments.as_deref())
    }
}

/// Ponto único onde a sessão decide o que vale: Configurações do Tako + deste jogo → efetivo.
pub fn resolve(
    global: &GlobalSessionSettings,
    game: Option<&GameSettingsOverride>,
) -> EffectiveSessionSettings {
    let none = GameSettingsOverride::default();
    let game = game.unwrap_or(&none);
    let mut screen = None;
    let mut problem = None;
    let own_screen = game
        .screen_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let display_id = own_screen.unwrap_or(global.game_screen_id.trim());
    // Escolher uma tela para o jogo já é pedir para usá-la.
    let asked = game.use_game_screen.or(own_screen.map(|_| true));
    if asked.unwrap_or(global.game_screen_enabled) {
        if display_id.is_empty() {
            // Só é problema quando o próprio jogo pediu; desligada no global e sem tela é o normal.
            problem = (asked == Some(true)).then_some(GameScreenProblem::NoScreenChosen);
        } else {
            // A resolução do Tako é da tela de jogo dele: em outra tela, sem escolha do jogo, vale o preset dela.
            let (default_mode, default_hdr) = global.screen_defaults(display_id);
            let mode = match game.screen_mode.as_deref() {
                None => default_mode,
                Some(GameSettingsOverride::KEEP_CURRENT_MODE) => None,
                // Valor ilegível volta ao padrão.
                Some(key) => DisplayMode::parse(key).or(default_mode),
            };
            screen = Some(GameScreenSettings {
                display_id: display_id.to_owned(),
                mode,
                enable_hdr: game.hdr.unwrap_or(default_hdr),
            });
        }
    }
    let game_audio = match game.audio_device_id.as_deref() {
        None => global.game_audio.clone(),
        Some(GameSettingsOverride::KEEP_AUDIO) => None,
        Some(id) => {
            let name = game
                .audio_device_name
                .as_deref()
                .filter(|n| !n.trim().is_empty());
            Some(AudioOutputChoice::new(id, name.unwrap_or(id)))
        }
    };
    EffectiveSessionSettings {
        game_screen: screen,
        game_screen_problem: problem,
        game_audio,
        extra_arguments: normalize_arguments(game.extra_arguments.as_deref()),
        working_directory: normalize_arguments(game.working_directory.as_deref()),
        environment: parse_environment(game.environment.as_deref()),
        before_command: normalize_arguments(game.before_command.as_deref()),
        after_command: normalize_arguments(game.after_command.as_deref()),
        commands_approved: match commands_text(
            game.before_command.as_deref(),
            game.after_command.as_deref(),
            game.tools.as_deref(),
        ) {
            None => true,
            current => game.approved_commands == current,
        },
        tools: parse_tools(game.tools.as_deref()),
        close_tools: game.close_tools == Some(true),
    }
}

/// Junta argumentos com um espaço, ignorando os vazios.
pub fn join_arguments(first: Option<&str>, second: Option<&str>) -> String {
    [first, second]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Texto digitado → sem espaços nas pontas; vazio vira `None`.
pub fn normalize_arguments(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|t| !t.is_empty())
        .map(Into::into)
}

/// Combinações de tela + modo já confirmadas ("Você consegue ver esta tela?"), uma por linha, a mais
/// recente primeiro (valor `GameScreenConfirmations` das configurações).
pub mod game_screen_confirmations {
    pub const MAX_REMEMBERED: usize = 8;

    fn split(saved: Option<&str>) -> impl Iterator<Item = &str> {
        saved
            .unwrap_or_default()
            .split('\n')
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    /// `setup_key` quando já confirmada.
    pub fn matching(saved: Option<&str>, setup_key: &str) -> Option<String> {
        split(saved)
            .any(|k| k.eq_ignore_ascii_case(setup_key))
            .then(|| setup_key.to_owned())
    }

    /// Lembra a combinação confirmada, mantendo as [`MAX_REMEMBERED`] mais recentes.
    pub fn remember(saved: Option<&str>, setup_key: &str) -> String {
        let key = setup_key.trim();
        if key.is_empty() {
            return saved.unwrap_or_default().to_owned();
        }
        std::iter::once(key)
            .chain(split(saved).filter(|k| !k.eq_ignore_ascii_case(key)))
            .take(MAX_REMEMBERED)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const TV_4K: DisplayMode = DisplayMode::new(3840, 2160, 120);

    fn global() -> GlobalSessionSettings {
        GlobalSessionSettings {
            game_screen_enabled: true,
            game_screen_id: r"\\.\DISPLAY2".into(),
            game_screen_mode: Some(TV_4K),
            game_screen_hdr: true,
            game_audio: Some(AudioOutputChoice::new("{tv}", "TV (HDMI)")),
            monitor_presets: MonitorPresets::default(),
        }
    }

    fn with(f: impl FnOnce(&mut GameSettingsOverride)) -> GameSettingsOverride {
        let mut o = GameSettingsOverride::default();
        f(&mut o);
        o
    }

    #[test]
    fn environment_and_commands_need_the_user_to_allow_them() {
        let env = parse_environment(Some(
            "DXVK_HUD=fps; bad name=1\n_X=a=b\n=vazio;9X=1;SEM_IGUAL",
        ));
        assert_eq!(
            env,
            [
                ("DXVK_HUD".into(), "fps".into()),
                ("_X".into(), "a=b".into())
            ]
        );

        let mut o = with(|o| {
            o.before_command = Some(" mount.bat ".into());
            o.environment = Some("A=1".into());
        });
        assert_eq!(o.changed_count(), 2);
        let e = resolve(&global(), Some(&o));
        assert_eq!(e.before_command.as_deref(), Some("mount.bat"));
        assert!(!e.commands_approved);
        o.approve_commands();
        assert!(resolve(&global(), Some(&o)).commands_approved);
        // Mudou o comando: pergunta de novo.
        o.after_command = Some("umount.bat".into());
        assert!(!resolve(&global(), Some(&o)).commands_approved);
        // Sem comandos não há o que permitir.
        assert!(resolve(&global(), None).commands_approved);
    }

    #[test]
    fn tools_need_the_same_permission_and_old_approvals_still_hold() {
        let mut o = with(|o| o.before_command = Some("mount.bat".into()));
        o.approve_commands();
        // O texto sem ferramentas é o do SESSION-004: quem já permitiu não é perguntado de novo.
        assert_eq!(o.approved_commands.as_deref(), Some("mount.bat\n"));
        o.tools = Some(" C:\\FPS\\fps.exe ; ;\nC:\\Mods\\mods.exe".into());
        let e = resolve(&global(), Some(&o));
        assert!(!e.commands_approved);
        assert_eq!(e.tools, ["C:\\FPS\\fps.exe", "C:\\Mods\\mods.exe"]);
        assert!(!e.close_tools);
        o.approve_commands();
        o.close_tools = Some(true);
        let e = resolve(&global(), Some(&o));
        assert!(e.commands_approved && e.close_tools);
        assert_eq!(o.changed_count(), 3);
        // Só ferramentas também pede permissão.
        let tools_only = with(|o| o.tools = Some("fps.exe".into()));
        assert!(!resolve(&global(), Some(&tools_only)).commands_approved);
    }

    #[test]
    fn no_game_settings_means_the_global_ones() {
        let e = resolve(&global(), None);
        let screen = GameScreenSettings {
            display_id: r"\\.\DISPLAY2".into(),
            mode: Some(TV_4K),
            enable_hdr: true,
        };
        assert_eq!(e.game_screen, Some(screen));
        assert_eq!(e.game_screen_problem, None);
        assert_eq!(e.game_audio, global().game_audio);
        assert_eq!(e.extra_arguments, None);
        assert_eq!(
            e,
            resolve(&global(), Some(&GameSettingsOverride::default()))
        );
    }

    #[test]
    fn game_can_skip_the_game_screen() {
        let e = resolve(&global(), Some(&with(|o| o.use_game_screen = Some(false))));
        assert_eq!((e.game_screen, e.game_screen_problem), (None, None));
    }

    #[test]
    fn game_can_use_the_game_screen_even_when_it_is_off_globally() {
        let off = GlobalSessionSettings {
            game_screen_enabled: false,
            ..global()
        };
        let e = resolve(&off, Some(&with(|o| o.use_game_screen = Some(true))));
        assert_eq!(e.game_screen.unwrap().display_id, r"\\.\DISPLAY2");
    }

    #[test]
    fn game_asking_for_the_game_screen_without_one_chosen_explains_why() {
        let no_screen = GlobalSessionSettings {
            game_screen_enabled: false,
            game_screen_id: String::new(),
            ..global()
        };
        let asked = resolve(&no_screen, Some(&with(|o| o.use_game_screen = Some(true))));
        let enabled = GlobalSessionSettings {
            game_screen_enabled: true,
            ..no_screen
        };
        let not_asked = resolve(&enabled, None);
        assert_eq!(asked.game_screen, None);
        assert_eq!(
            asked.game_screen_problem,
            Some(GameScreenProblem::NoScreenChosen)
        );
        assert!(!GameScreenProblem::NoScreenChosen.message().is_empty());
        assert_eq!(
            (not_asked.game_screen, not_asked.game_screen_problem),
            (None, None)
        );
    }

    #[test]
    fn game_can_pick_its_own_screen_with_its_own_mode() {
        const DESK: &str = r"\\.\DISPLAY1";
        let desk = with(|o| o.screen_id = Some(DESK.into()));
        assert_eq!(desk.changed_count(), 1);
        // Vale mesmo com a tela de jogo desligada e sem tela escolhida no Tako.
        let none = GlobalSessionSettings {
            game_screen_enabled: false,
            game_screen_id: String::new(),
            ..global()
        };
        for g in [global(), none] {
            let s = resolve(&g, Some(&desk)).game_screen.unwrap();
            assert_eq!(s.display_id, DESK);
            // A resolução do Tako é da TV: na outra tela, mantém a atual.
            assert_eq!(s.mode, None);
            assert!(s.enable_hdr);
        }
        let with_mode = with(|o| {
            o.screen_id = Some(DESK.into());
            o.screen_mode = Some("2560x1440@144".into());
        });
        let s = resolve(&global(), Some(&with_mode)).game_screen.unwrap();
        assert_eq!(s.mode, Some(DisplayMode::new(2560, 1440, 144)));
        // A tela do Tako escolhida para o jogo leva a resolução dela.
        let same = with(|o| o.screen_id = Some(r"\\.\display2".into()));
        assert_eq!(
            resolve(&global(), Some(&same)).game_screen.unwrap().mode,
            Some(TV_4K)
        );
        // Escolheu a tela, mas desligou o uso: não usa.
        let off = with(|o| {
            o.screen_id = Some(DESK.into());
            o.use_game_screen = Some(false);
        });
        assert_eq!(resolve(&global(), Some(&off)).game_screen, None);
        assert!(with(|o| o.screen_id = Some(" ".into())).is_default());
    }

    #[test]
    fn own_screen_without_a_mode_uses_the_monitor_preset() {
        const DESK: &str = r"\\.\DISPLAY1";
        let qhd = DisplayMode::new(2560, 1440, 144);
        let mut g = global();
        g.monitor_presets.set(
            DESK,
            MonitorPreset {
                mode: Some(qhd),
                hdr: false,
            },
        );
        let desk = with(|o| o.screen_id = Some(r"\\.\display1".into()));
        let s = resolve(&g, Some(&desk)).game_screen.unwrap();
        assert_eq!((s.mode, s.enable_hdr), (Some(qhd), false));
        // A escolha do jogo vence o preset; "manter a atual" continua valendo.
        let chosen = with(|o| {
            o.screen_id = Some(DESK.into());
            o.screen_mode = Some("1920x1080@60".into());
            o.hdr = Some(true);
        });
        let s = resolve(&g, Some(&chosen)).game_screen.unwrap();
        assert_eq!(
            (s.mode, s.enable_hdr),
            (Some(DisplayMode::new(1920, 1080, 60)), true)
        );
        let keep = with(|o| {
            o.screen_id = Some(DESK.into());
            o.screen_mode = Some(GameSettingsOverride::KEEP_CURRENT_MODE.into());
        });
        assert_eq!(resolve(&g, Some(&keep)).game_screen.unwrap().mode, None);
        // Resolução ilegível volta ao preset.
        let bad = with(|o| {
            o.screen_id = Some(DESK.into());
            o.screen_mode = Some("lixo".into());
        });
        assert_eq!(resolve(&g, Some(&bad)).game_screen.unwrap().mode, Some(qhd));
        // Na tela de jogo do Tako vale o que está em Configurações, mesmo com um preset antigo dela.
        g.monitor_presets
            .set(r"\\.\DISPLAY2", MonitorPreset::default());
        let s = resolve(&g, None).game_screen.unwrap();
        assert_eq!((s.mode, s.enable_hdr), (Some(TV_4K), true));
    }

    #[test]
    fn monitor_presets_survive_a_round_trip_and_broken_values() {
        let mut p = MonitorPresets::default();
        let tv = MonitorPreset {
            mode: Some(TV_4K),
            hdr: true,
        };
        p.set(r"\\.\DISPLAY2", tv);
        p.set("HDMI-1", MonitorPreset::default());
        // Mesmo monitor com outra caixa: substitui.
        p.set(r"\\.\display2", tv);
        let back = MonitorPresets::parse(&p.to_text());
        assert_eq!(back, MonitorPresets::parse(&p.to_text()));
        assert_eq!(back.get(r"\\.\DISPLAY2"), Some(tv));
        assert_eq!(back.get("hdmi-1"), Some(MonitorPreset::default()));
        assert_eq!(back.get("DP-3"), None);
        for broken in ["", "lixo", "[1,2]", "{\"x\": 3}", "{\"\": {\"hdr\": true}}"] {
            let p = MonitorPresets::parse(broken);
            assert_eq!(p, MonitorPresets::default(), "{broken}");
        }
        // Uma tela quebrada não leva as outras; resolução ilegível vira "manter a atual".
        let p = MonitorPresets::parse(
            r#"{"A": {"mode": "lixo", "hdr": true}, "B": {"hdr": "sim"}, "C": {}}"#,
        );
        assert_eq!(
            p.get("a"),
            Some(MonitorPreset {
                mode: None,
                hdr: true
            })
        );
        assert_eq!(p.get("B"), None);
        assert_eq!(p.get("C"), Some(MonitorPreset::default()));
    }

    #[test]
    fn game_resolution_overrides_the_global_mode() {
        for (mode, expected) in [
            ("1920x1080@60", Some(DisplayMode::new(1920, 1080, 60))),
            ("", None),
            ("lixo", Some(TV_4K)),
        ] {
            let o = with(|o| {
                o.screen_mode = Some(mode.into());
                o.hdr = Some(false);
            });
            let screen = resolve(&global(), Some(&o)).game_screen.unwrap();
            assert_eq!(screen.mode, expected, "{mode}");
            assert!(!screen.enable_hdr);
        }
    }

    #[test]
    fn game_audio_can_keep_the_current_output_or_pick_another() {
        let keep = with(|o| o.audio_device_id = Some(String::new()));
        let headset = with(|o| {
            o.audio_device_id = Some("{fone}".into());
            o.audio_device_name = Some("Fone".into());
        });
        let unnamed = with(|o| o.audio_device_id = Some("{fone}".into()));
        assert_eq!(resolve(&global(), Some(&keep)).game_audio, None);
        assert_eq!(
            resolve(&global(), Some(&headset)).game_audio,
            Some(AudioOutputChoice::new("{fone}", "Fone"))
        );
        assert_eq!(
            resolve(&global(), Some(&unnamed)).game_audio.unwrap().name,
            "{fone}"
        );
    }

    #[test]
    fn extra_arguments_go_after_the_game_arguments() {
        for (game, extra, expected) in [
            (None, None, ""),
            (Some("-a"), None, "-a"),
            (None, Some(" -dx11 "), "-dx11"),
            (Some("-a -b"), Some("-windowed"), "-a -b -windowed"),
            (Some("  "), Some("   "), ""),
        ] {
            let o = with(|o| o.extra_arguments = extra.map(Into::into));
            assert_eq!(resolve(&global(), Some(&o)).arguments(game), expected);
        }
    }

    #[test]
    fn working_directory_is_normalized() {
        let o = with(|o| o.working_directory = Some("  /jogos/x ".into()));
        assert_eq!(
            resolve(&global(), Some(&o)).working_directory.as_deref(),
            Some("/jogos/x")
        );
        let blank = with(|o| o.working_directory = Some(" ".into()));
        assert_eq!(resolve(&global(), Some(&blank)).working_directory, None);
    }

    #[test]
    fn changed_count_ignores_blank_arguments() {
        assert!(with(|o| o.extra_arguments = Some("  ".into())).is_default());
        let two = with(|o| {
            o.hdr = Some(false);
            o.audio_device_id = Some(String::new());
        });
        assert_eq!(two.changed_count(), 2);
    }

    #[test]
    fn confirmed_game_screen_setups_remember_each_mode() {
        use game_screen_confirmations::*;
        let mut saved = remember(None, "tv|current");
        saved = remember(Some(&saved), "tv|1920x1080@60");
        assert_eq!(
            matching(Some(&saved), "tv|current").as_deref(),
            Some("tv|current")
        );
        let upper = matching(Some(&saved), "TV|1920x1080@60");
        assert_eq!(upper.as_deref(), Some("TV|1920x1080@60"));
        assert_eq!(matching(Some(&saved), "tv|3840x2160@120"), None);
        let old = matching(Some("tv|current"), "tv|current");
        assert_eq!(old.as_deref(), Some("tv|current")); // valor antigo (uma só)
        for i in 0..20 {
            saved = remember(Some(&saved), &format!("tv|{i}"));
        }
        assert_eq!(saved.split('\n').count(), MAX_REMEMBERED);
        assert_eq!(saved.split('\n').next(), Some("tv|19"));
    }
}
