//! Como reconhecer que o jogo está rodando e quando terminou (port de `Session/{GameTracking,
//! GameRunTracker}.cs`). Puro: a cada leitura de processos ([`ProcessLauncher::snapshot`]) e do
//! `RunningAppID` da Steam, o acompanhamento diz o estado do jogo.
//!
//! [`ProcessLauncher::snapshot`]: crate::devices::ProcessLauncher::snapshot

use crate::devices::ProcessEntry;
use crate::library::{Game, GameStore, GameTrackingStrategy};
use chrono::{DateTime, TimeDelta, Utc};
use std::collections::HashSet;

/// Estratégia escolhida e a pasta observada. `steam_app_id` liga o sinal da Steam junto com qualquer
/// estratégia: confirma o início e marca o fim com precisão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackingPlan {
    pub strategy: GameTrackingStrategy,
    pub folder: Option<String>,
    pub steam_app_id: Option<u32>,
}

impl TrackingPlan {
    pub const NONE: TrackingPlan = TrackingPlan {
        strategy: GameTrackingStrategy::None,
        folder: None,
        steam_app_id: None,
    };

    pub fn new(strategy: GameTrackingStrategy) -> Self {
        TrackingPlan {
            strategy,
            ..Self::NONE
        }
    }

    fn folder(folder: &str) -> Self {
        TrackingPlan {
            folder: Some(folder.to_owned()),
            ..Self::new(GameTrackingStrategy::InstallFolder)
        }
    }

    fn is_none(&self) -> bool {
        self.strategy == GameTrackingStrategy::None
    }
}

/// A escolha do usuário quando possível; senão pasta para jogos de loja e processo para os adicionados
/// à mão. `has_process`: abrir o jogo devolveu um processo (URIs e alguns atalhos não devolvem).
pub fn plan(game: &Game, has_process: bool) -> TrackingPlan {
    use GameTrackingStrategy as S;
    let folder = game
        .install_path
        .as_deref()
        .filter(|p| !p.trim().is_empty());
    let steam_app_id = game
        .steam_app_id
        .filter(|id| game.store() == GameStore::Steam && *id > 0);
    let by_folder = folder.map_or(TrackingPlan::NONE, TrackingPlan::folder);
    let by_process = if has_process {
        TrackingPlan::new(S::Process)
    } else {
        TrackingPlan::NONE
    };
    let by_steam = if steam_app_id.is_some() {
        TrackingPlan::new(S::SteamApp)
    } else {
        TrackingPlan::NONE
    };

    let plan = match game.tracking {
        Some(S::None) => TrackingPlan::NONE,
        Some(S::InstallFolder) if !by_folder.is_none() => by_folder,
        Some(S::Process) if has_process => by_process,
        Some(S::SteamApp) if steam_app_id.is_some() => by_steam,
        _ if game.store() != GameStore::Manual => [by_folder, by_process, by_steam]
            .into_iter()
            .find(|p| !p.is_none())
            .unwrap_or(TrackingPlan::NONE),
        // Manual: o processo iniciado; sem ele, a pasta só serve para executáveis (a de um atalho é a do atalho).
        _ if has_process => by_process,
        _ if is_executable(game.launch_target.as_deref()) => by_folder,
        _ => TrackingPlan::NONE,
    };
    if plan.is_none() {
        plan
    } else {
        TrackingPlan {
            steam_app_id,
            ..plan
        }
    }
}

/// O caminho está dentro da pasta (aceita `\` e `/`). Sem diferenciar maiúsculas só no Windows.
pub fn is_inside(path: Option<&str>, folder: &str) -> bool {
    is_inside_with(path, folder, cfg!(target_os = "windows"))
}

pub fn is_inside_with(path: Option<&str>, folder: &str, ignore_case: bool) -> bool {
    let root = folder.trim_end_matches(['\\', '/']);
    let Some(path) = path else { return false };
    if root.is_empty() || path.len() <= root.len() + 1 {
        return false;
    }
    let (Some(prefix), Some(rest)) = (path.get(..root.len()), path.get(root.len()..)) else {
        return false;
    };
    let same = if ignore_case {
        prefix.to_lowercase() == root.to_lowercase()
    } else {
        prefix == root
    };
    same && rest.starts_with(['\\', '/'])
}

/// Processos de um destes programas (EXT-001): o executável é o caminho, ou está dentro dele (`.app` do macOS).
/// Pelo caminho e não pelo PID: o PID pode já ser de outro processo no fim do jogo, e o `open` não dá o do app.
pub fn processes_of(snapshot: &[ProcessEntry], programs: &[String]) -> Vec<u32> {
    processes_of_with(snapshot, programs, cfg!(target_os = "windows"))
}

pub fn processes_of_with(
    snapshot: &[ProcessEntry],
    programs: &[String],
    ignore_case: bool,
) -> Vec<u32> {
    snapshot
        .iter()
        .filter(|p| {
            let exe = p.exe_path.as_deref();
            programs.iter().any(|t| {
                let same = exe.is_some_and(|e| {
                    if ignore_case {
                        e.eq_ignore_ascii_case(t)
                    } else {
                        e == t
                    }
                });
                same || is_inside_with(exe, t, ignore_case)
            })
        })
        .map(|p| p.pid)
        .collect()
}

/// Executáveis e scripts (a pasta deles é a do jogo); atalhos e URIs não.
fn is_executable(target: Option<&str>) -> bool {
    let Some(t) = target.filter(|t| !t.contains("://")) else {
        return false;
    };
    let name = t.rsplit(['\\', '/']).next().unwrap_or(t);
    match name.rsplit_once('.') {
        // Sem extensão: binário do Linux ou do macOS.
        None => !name.is_empty(),
        Some((_, ext)) => ["exe", "bat", "cmd", "sh", "appimage", "app", "x86_64"]
            .iter()
            .any(|e| ext.eq_ignore_ascii_case(e)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameRunStatus {
    /// Aguardando o jogo aparecer (ex.: a Steam baixando uma atualização).
    Waiting,
    Running,
    Exited,
    /// O jogo não apareceu dentro do prazo.
    TimedOut,
}

/// Decisão pura do acompanhamento: a cada leitura da lista de processos diz se o jogo está rodando.
#[derive(Debug)]
pub struct GameRunTracker {
    pub plan: TrackingPlan,
    launched_at: DateTime<Utc>,
    /// Prazo para o jogo aparecer depois de aberto (a Steam pode atualizar antes).
    pub start_timeout: TimeDelta,
    /// Tempo sem nenhum processo do jogo antes de considerar encerrado (troca de launcher para jogo).
    pub exit_grace: TimeDelta,
    tree: HashSet<u32>,
    gone: HashSet<u32>,
    current: Vec<u32>,
    current_paths: Vec<String>,
    pub status: GameRunStatus,
    pub first_seen_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    /// A Steam já confirmou este jogo rodando: daí em diante ela decide o fim.
    pub steam_confirmed: bool,
    /// O Tako pediu para o jogo fechar: o fim vale assim que um dos sinais (processos ou Steam) disser que
    /// acabou. A Steam leva alguns segundos para zerar o `RunningAppID` e "Fechando o jogo…" ficava esperando.
    pub close_requested: bool,
    /// A janela do jogo foi confirmada; o processo raiz sozinho pode ser apenas um launcher.
    shown: bool,
    /// Houve processo numa leitura real (não apenas o PID fornecido ao abrir).
    processes_seen: bool,
}

// Tolerância de relógio ao comparar o início de um filho com a hora em que o jogo foi aberto.
const CLOCK_SKEW: TimeDelta = TimeDelta::seconds(5);

impl GameRunTracker {
    pub const DEFAULT_START_TIMEOUT: TimeDelta = TimeDelta::minutes(2);
    pub const DEFAULT_EXIT_GRACE: TimeDelta = TimeDelta::seconds(4);
    /// Margem para a variação do horário dos retratos: uma leitura aos 499 ms não deve custar mais 250 ms.
    pub const RUNNING_EXIT_GRACE: TimeDelta = TimeDelta::milliseconds(450);

    /// `root_pid`: processo iniciado (obrigatório na estratégia por processo).
    pub fn new(
        plan: TrackingPlan,
        launched_at: DateTime<Utc>,
        root_pid: Option<u32>,
    ) -> Result<Self, String> {
        use GameTrackingStrategy as S;
        match plan.strategy {
            S::None => return Err("sem estratégia não há o que acompanhar".into()),
            S::InstallFolder if plan.folder.as_deref().is_none_or(|f| f.trim().is_empty()) => {
                return Err("a estratégia por pasta precisa da pasta do jogo".into());
            }
            S::Process if root_pid.is_none() => {
                return Err("a estratégia por processo precisa do processo iniciado".into());
            }
            S::SteamApp if plan.steam_app_id.is_none() => {
                return Err("a estratégia pela Steam precisa do id do jogo na Steam".into());
            }
            _ => {}
        }
        let mut t = GameRunTracker {
            plan,
            launched_at,
            start_timeout: Self::DEFAULT_START_TIMEOUT,
            exit_grace: Self::DEFAULT_EXIT_GRACE,
            tree: HashSet::new(),
            gone: HashSet::new(),
            current: Vec::new(),
            current_paths: Vec::new(),
            status: GameRunStatus::Waiting,
            first_seen_at: None,
            last_seen_at: None,
            steam_confirmed: false,
            close_requested: false,
            shown: false,
            processes_seen: false,
        };
        if let Some(root) = root_pid {
            // O processo foi iniciado agora: o jogo já conta como rodando.
            t.tree.insert(root);
            t.current = vec![root];
            t.first_seen_at = Some(launched_at);
            t.last_seen_at = Some(launched_at);
            t.status = GameRunStatus::Running;
        }
        Ok(t)
    }

    /// Processos do jogo na última leitura (para trazer a janela do jogo para frente).
    pub fn game_process_ids(&self) -> &[u32] {
        &self.current
    }

    /// Executáveis dos processos do jogo na última leitura, sem repetidos.
    pub fn game_executable_paths(&self) -> &[String] {
        &self.current_paths
    }

    /// Tempo jogado: da primeira à última vez em que o jogo foi visto.
    pub fn play_time(&self) -> TimeDelta {
        match (self.first_seen_at, self.last_seen_at) {
            (Some(f), Some(l)) => l - f,
            _ => TimeDelta::zero(),
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.status, GameRunStatus::Exited | GameRunStatus::TimedOut)
    }

    /// Confirma a fase de jogo sem confundir o PID inicial do launcher com a janela do jogo.
    pub fn mark_shown(&mut self) {
        if !self.is_finished() {
            self.shown = true;
        }
    }

    pub fn was_shown(&self) -> bool {
        self.shown
    }

    /// Aplica uma leitura feita em `now`. `steam_running_app_id`: `RunningAppID` lido junto
    /// (`None` = não lido ou ilegível; `Some(0)` = nenhum jogo).
    pub fn update(
        &mut self,
        snapshot: &[ProcessEntry],
        now: DateTime<Utc>,
        steam_running_app_id: Option<u32>,
    ) -> GameRunStatus {
        if self.is_finished() {
            return self.status;
        }
        self.current = match self.plan.strategy {
            GameTrackingStrategy::Process => self.match_tree(snapshot),
            GameTrackingStrategy::InstallFolder => {
                let folder = self.plan.folder.as_deref().unwrap_or_default();
                snapshot
                    .iter()
                    .filter(|p| is_inside(p.exe_path.as_deref(), folder))
                    .map(|p| p.pid)
                    .collect()
            }
            _ => Vec::new(),
        };
        let mut paths: Vec<String> = Vec::new();
        for p in snapshot.iter().filter(|p| self.current.contains(&p.pid)) {
            if let Some(path) = &p.exe_path
                && !paths.iter().any(|x| x.eq_ignore_ascii_case(path))
            {
                paths.push(path.clone());
            }
        }
        self.current_paths = paths;
        self.processes_seen |= !self.current.is_empty();

        let exit_grace = if self.shown {
            self.exit_grace.min(Self::RUNNING_EXIT_GRACE)
        } else {
            self.exit_grace
        };

        if self.is_present(steam_running_app_id) {
            self.first_seen_at.get_or_insert(now);
            self.last_seen_at = Some(now);
            self.status = GameRunStatus::Running;
        } else if self.status == GameRunStatus::Waiting
            && now - self.launched_at >= self.start_timeout
        {
            self.status = GameRunStatus::TimedOut;
        } else if self.status == GameRunStatus::Running
            && self.last_seen_at.is_some_and(|l| now - l >= exit_grace)
        {
            self.status = GameRunStatus::Exited;
        }
        self.status
    }

    /// Com o sinal da Steam: o id dela confirma o início mesmo sem processo visto; depois de confirmado,
    /// a Steam decide o fim (processos que ficam para trás na pasta não seguram a sessão).
    fn is_present(&mut self, steam_running: Option<u32>) -> bool {
        let (Some(app), Some(running)) = (self.plan.steam_app_id, steam_running) else {
            return !self.current.is_empty();
        };
        let steam_says = running == app;
        self.steam_confirmed |= steam_says;
        let watches_processes = matches!(
            self.plan.strategy,
            GameTrackingStrategy::InstallFolder | GameTrackingStrategy::Process
        );
        // Depois da janela confirmada, o registro da Steam pode demorar a zerar. Só usar a ausência
        // dos processos se já vimos algum: há jogos que só podem ser acompanhados pelo registro.
        if watches_processes
            && (self.close_requested || (self.shown && self.processes_seen && self.steam_confirmed))
        {
            return steam_says && !self.current.is_empty();
        }
        if self.steam_confirmed {
            steam_says
        } else {
            steam_says || !self.current.is_empty()
        }
    }

    /// Processo iniciado e descendentes. Filhos de um processo que fechou continuam contando; um id que
    /// sumiu não volta a contar (outro processo pode reaproveitar o número).
    fn match_tree(&mut self, snapshot: &[ProcessEntry]) -> Vec<u32> {
        let min_start = self.launched_at - CLOCK_SKEW;
        loop {
            let mut grew = false;
            for p in snapshot {
                let child = p
                    .parent_pid
                    .is_some_and(|pp| pp != p.pid && self.tree.contains(&pp));
                if !self.tree.contains(&p.pid)
                    && child
                    && p.started_at.is_none_or(|s| s >= min_start)
                {
                    self.tree.insert(p.pid);
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        let alive: Vec<u32> = snapshot
            .iter()
            .map(|p| p.pid)
            .filter(|pid| self.tree.contains(pid) && !self.gone.contains(pid))
            .collect();
        let gone: Vec<u32> = self
            .tree
            .iter()
            .copied()
            .filter(|id| !alive.contains(id))
            .collect();
        self.gone.extend(gone);
        alive
    }
}

/// Como o jogo terminou, para avisar quando algo deu errado (SESSION-003). O código de saída só existe quando o
/// Tako abriu o executável direto e ficou com o processo; as lojas (URI) abrem os jogos por fora, e aí o que se
/// sabe é o tempo e se o Tako pediu para fechar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameEnd {
    #[default]
    Normal,
    /// Fechou sozinho em menos de [`GameEnd::QUICK`]: caiu ou desistiu ao abrir. Com o código, se houver.
    Quick(Option<u32>),
    /// Não apareceu dentro do prazo.
    NeverStarted,
    /// Fechou sozinho depois de [`GameEnd::QUICK`] com um código de erro (diferente de 0).
    Failed(u32),
}

impl GameEnd {
    pub const QUICK: std::time::Duration = std::time::Duration::from_secs(10);

    pub fn of(
        status: GameRunStatus,
        close_requested: bool,
        played: std::time::Duration,
        exit_code: Option<u32>,
    ) -> GameEnd {
        match status {
            // Fechar à força dá código de erro: o Tako pediu, então é o esperado.
            _ if close_requested => GameEnd::Normal,
            GameRunStatus::TimedOut => GameEnd::NeverStarted,
            GameRunStatus::Exited if played < Self::QUICK => GameEnd::Quick(exit_code),
            GameRunStatus::Exited => match exit_code {
                Some(code) if code != 0 => GameEnd::Failed(code),
                _ => GameEnd::Normal,
            },
            _ => GameEnd::Normal,
        }
    }

    /// Para passar entre o supervisor e a interface (argumento e mensagem), junto com [`GameEnd::exit_code`].
    pub fn key(self) -> &'static str {
        match self {
            GameEnd::Normal => "normal",
            GameEnd::Quick(_) => "quick",
            GameEnd::NeverStarted => "never_started",
            GameEnd::Failed(_) => "failed",
        }
    }

    pub fn exit_code(self) -> Option<u32> {
        match self {
            GameEnd::Quick(code) => code,
            GameEnd::Failed(code) => Some(code),
            _ => None,
        }
    }

    /// Valor desconhecido é `Normal`: na dúvida, não assusta ninguém. Sem código (supervisor antigo), "failed"
    /// também é `Normal`.
    pub fn from_key(key: &str, exit_code: Option<u32>) -> GameEnd {
        match (key, exit_code) {
            ("quick", code) => GameEnd::Quick(code),
            ("never_started", _) => GameEnd::NeverStarted,
            ("failed", Some(code)) if code != 0 => GameEnd::Failed(code),
            _ => GameEnd::Normal,
        }
    }
}

/// O código de saída em hex e decimal, com o que ele costuma querer dizer (os códigos de exceção do Windows):
/// "0xC0000135 (3221225781): uma DLL que o jogo precisa não foi encontrada…".
pub fn exit_code_text(code: u32) -> String {
    exit_code_text_in(crate::i18n::lang(), code)
}

pub fn exit_code_text_in(lang: crate::i18n::Lang, code: u32) -> String {
    use crate::i18n::t_in;
    let explain = match code {
        0xC000_0135 => Some("dll_not_found"),
        0xC000_0139 => Some("entry_point"),
        0xC000_0142 => Some("dll_init"),
        0xC000_0005 => Some("access_violation"),
        0xC000_007B => Some("bad_image"),
        0xC000_001D => Some("illegal_instruction"),
        0xC000_00FD => Some("stack_overflow"),
        0xC000_0374 => Some("heap_corruption"),
        0xC000_0409 => Some("fail_fast"),
        0xC000_0017 => Some("no_memory"),
        _ => None,
    };
    let code_text =
        t_in(lang, "session.exit.code").replace("{code}", &format!("{code:#X} ({code})"));
    match explain {
        Some(k) => format!("{code_text}: {}", t_in(lang, &format!("session.exit.{k}"))),
        None => code_text,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    #[test]
    fn tool_processes_are_found_by_path_or_inside_an_app_bundle() {
        let entry = |pid, path: &str| ProcessEntry {
            pid,
            parent_pid: None,
            exe_path: Some(path.into()),
            started_at: None,
        };
        let snapshot = [
            entry(1, "C:\\FPS\\Fps.exe"),
            entry(2, "/Applications/Mods.app/Contents/MacOS/Mods"),
            entry(3, "C:\\FPS\\Fps.exe.bak"),
            ProcessEntry {
                exe_path: None,
                ..entry(4, "")
            },
        ];
        let tools = [
            "C:\\FPS\\fps.exe".to_owned(),
            "/Applications/Mods.app".to_owned(),
        ];
        assert_eq!(processes_of_with(&snapshot, &tools, true), [1, 2]);
        assert_eq!(processes_of_with(&snapshot, &tools, false), [2]);
        assert!(processes_of_with(&snapshot, &[], true).is_empty());
    }

    use super::*;
    use GameTrackingStrategy as S;

    const FOLDER: &str = r"C:\Steam\steamapps\common\Hades";
    const APP_ID: u32 = 1145360;

    fn launched() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-15T20:00:00Z")
            .unwrap()
            .to_utc()
    }
    fn secs(s: i64) -> DateTime<Utc> {
        launched() + TimeDelta::seconds(s)
    }

    fn steam_game(folder: Option<&str>, app_id: Option<u32>, tracking: Option<S>) -> Game {
        Game {
            id: "steam:1145360".into(),
            title: "Hades".into(),
            store: Some(GameStore::Steam),
            launch_target: Some("steam://rungameid/1145360".into()),
            install_path: folder.map(Into::into),
            steam_app_id: app_id,
            tracking,
            ..Default::default()
        }
    }

    fn manual_game(target: &str, tracking: Option<S>) -> Game {
        Game {
            id: "manual:1".into(),
            title: "Indie".into(),
            store: Some(GameStore::Manual),
            launch_target: Some(target.into()),
            install_path: (!target.contains("://"))
                .then(|| target[..target.rfind('\\').unwrap()].to_owned()),
            tracking,
            ..Default::default()
        }
    }

    fn folder_plan(app: Option<u32>) -> TrackingPlan {
        TrackingPlan {
            steam_app_id: app,
            ..TrackingPlan::folder(FOLDER)
        }
    }

    fn proc(pid: u32, path: Option<&str>) -> ProcessEntry {
        ProcessEntry {
            pid,
            parent_pid: Some(4),
            exe_path: path.map(Into::into),
            started_at: None,
        }
    }

    fn child(pid: u32, parent: u32, started: Option<DateTime<Utc>>) -> ProcessEntry {
        ProcessEntry {
            pid,
            parent_pid: Some(parent),
            exe_path: None,
            started_at: started,
        }
    }

    // Escolha da estratégia (GameTrackingTests e SessionDevicesTests)

    #[test]
    fn steam_games_are_tracked_by_install_folder_with_the_steam_signal() {
        let game = steam_game(Some(FOLDER), Some(APP_ID), None);
        assert_eq!(plan(&game, false), folder_plan(Some(APP_ID)));
        let no_id = steam_game(Some(FOLDER), None, None);
        assert_eq!(plan(&no_id, false), folder_plan(None));
    }

    #[test]
    fn steam_games_without_a_known_folder_are_tracked_by_steam_alone() {
        let expected = TrackingPlan {
            steam_app_id: Some(APP_ID),
            ..TrackingPlan::new(S::SteamApp)
        };
        assert_eq!(plan(&steam_game(None, Some(APP_ID), None), false), expected);
        assert_eq!(
            plan(&steam_game(None, None, None), false),
            TrackingPlan::NONE
        );
        let off = steam_game(Some(FOLDER), Some(APP_ID), Some(S::None));
        assert_eq!(plan(&off, false), TrackingPlan::NONE);
    }

    #[test]
    fn non_steam_games_never_get_the_steam_signal() {
        let mut manual = manual_game(r"C:\Jogos\Hades\Hades.exe", None);
        manual.steam_app_id = Some(APP_ID);
        assert_eq!(plan(&manual, true).steam_app_id, None);
    }

    #[test]
    fn manual_executables_are_tracked_by_the_started_process() {
        let game = manual_game(r"C:\Jogos\Indie\jogo.exe", None);
        assert_eq!(plan(&game, true).strategy, S::Process);
    }

    #[test]
    fn manual_executable_without_process_falls_back_to_its_folder() {
        let game = manual_game(r"C:\Jogos\Indie\jogo.exe", None);
        assert_eq!(plan(&game, false), TrackingPlan::folder(r"C:\Jogos\Indie"));
        let linux = Game {
            launch_target: Some("/opt/indie/run".into()),
            install_path: Some("/opt/indie".into()),
            ..game
        };
        assert_eq!(plan(&linux, false), TrackingPlan::folder("/opt/indie"));
    }

    #[test]
    fn shortcuts_and_uris_without_process_are_not_tracked() {
        let url = manual_game(r"C:\Users\eu\Desktop\Jogo.url", None);
        assert_eq!(plan(&url, false), TrackingPlan::NONE);
        assert_eq!(
            plan(&manual_game("steam://rungameid/570", None), false),
            TrackingPlan::NONE
        );
    }

    #[test]
    fn the_user_choice_wins_when_it_is_possible() {
        let manual = manual_game(r"C:\Jogos\Indie\launcher.exe", Some(S::InstallFolder));
        assert_eq!(plan(&manual, true).strategy, S::InstallFolder);
        let by_process = steam_game(Some(FOLDER), None, Some(S::Process));
        assert_eq!(plan(&by_process, true).strategy, S::Process);
        // Processo escolhido, mas a Steam não devolveu processo: volta ao automático (pasta).
        assert_eq!(plan(&by_process, false).strategy, S::InstallFolder);
    }

    #[test]
    fn folder_match_requires_a_path_inside_the_folder() {
        let folder = format!(r"{FOLDER}\");
        for (path, expected) in [
            (Some(r"C:\Steam\steamapps\common\Hades\x64\Hades.exe"), true),
            (Some(r"c:\steam\STEAMAPPS\common\hades\Hades.exe"), true),
            (Some("C:/Steam/steamapps/common/Hades/Hades.exe"), false),
            (
                Some(r"C:\Steam\steamapps\common\Hades II\Hades2.exe"),
                false,
            ),
            (Some(r"C:\Steam\steamapps\common\Hades"), false),
            (None, false),
        ] {
            assert_eq!(is_inside_with(path, &folder, true), expected, "{path:?}");
        }
        assert!(!is_inside_with(
            Some("/Games/hades/x"),
            "/Games/Hades",
            false
        ));
        assert!(is_inside_with(
            Some("/Games/Hades/x"),
            "/Games/Hades/",
            false
        ));
        assert!(!is_inside_with(Some("/Games/Hadés/x"), "/Games/Had", false));
    }

    // Acompanhamento por pasta

    #[test]
    fn folder_tracking_waits_then_runs_then_exits_after_the_grace_period() {
        let mut t = GameRunTracker::new(folder_plan(None), launched(), None).unwrap();
        let game_exe = format!(r"{FOLDER}\Hades.exe");
        let steam_only = vec![proc(10, Some(r"C:\Steam\steam.exe")), proc(11, None)];
        let mut playing = steam_only.clone();
        playing.push(proc(20, Some(&game_exe)));

        assert_eq!(
            t.update(&steam_only, secs(30), None),
            GameRunStatus::Waiting
        );
        assert_eq!(t.update(&playing, secs(40), None), GameRunStatus::Running);
        assert_eq!(t.game_process_ids(), [20]);
        assert_eq!(t.game_executable_paths(), std::slice::from_ref(&game_exe));
        assert_eq!(t.update(&playing, secs(1800), None), GameRunStatus::Running);
        assert_eq!(
            t.update(&steam_only, secs(1802), None),
            GameRunStatus::Running
        );
        assert_eq!(
            t.update(&steam_only, secs(1804), None),
            GameRunStatus::Exited
        );
        assert_eq!(t.play_time(), TimeDelta::seconds(1800 - 40));
    }

    #[test]
    fn folder_tracking_times_out_when_the_game_never_shows_up() {
        let mut t = GameRunTracker::new(folder_plan(None), launched(), None).unwrap();
        assert_eq!(t.update(&[], secs(119), None), GameRunStatus::Waiting);
        assert_eq!(t.update(&[], secs(120), None), GameRunStatus::TimedOut);
        assert_eq!(t.play_time(), TimeDelta::zero());
        let late = [proc(20, Some(&format!(r"{FOLDER}\Hades.exe")))];
        assert_eq!(t.update(&late, secs(180), None), GameRunStatus::TimedOut);
    }

    #[test]
    fn a_short_gap_while_the_game_restarts_does_not_end_the_session() {
        let mut t = GameRunTracker::new(folder_plan(None), launched(), None).unwrap();
        t.update(
            &[proc(20, Some(&format!(r"{FOLDER}\launcher.exe")))],
            secs(2),
            None,
        );
        t.update(&[], secs(4), None);
        let game = [proc(21, Some(&format!(r"{FOLDER}\bin\game.exe")))];
        assert_eq!(t.update(&game, secs(6), None), GameRunStatus::Running);
    }

    // Acompanhamento por processo

    #[test]
    fn opening_keeps_the_launcher_handoff_grace() {
        let mut t =
            GameRunTracker::new(TrackingPlan::new(S::Process), launched(), Some(100)).unwrap();
        t.update(&[child(100, 1, None)], secs(1), None);
        assert!(!t.was_shown(), "o PID raiz não confirma a janela");
        assert_eq!(t.update(&[], secs(4), None), GameRunStatus::Running);
        assert_eq!(
            t.update(&[child(200, 100, Some(secs(4)))], secs(4), None),
            GameRunStatus::Running
        );
        assert_eq!(t.game_process_ids(), [200]);
    }

    #[test]
    fn after_shown_process_exit_is_detected_within_half_a_second() {
        for plan in [TrackingPlan::new(S::Process), folder_plan(None)] {
            let mut t = GameRunTracker::new(plan, launched(), Some(100)).unwrap();
            let game = [proc(100, Some(&format!(r"{FOLDER}\game.exe")))];
            t.update(&game, secs(10), None);
            t.mark_shown();
            assert_eq!(
                t.update(&[], secs(10) + TimeDelta::milliseconds(250), None),
                GameRunStatus::Running
            );
            assert_eq!(
                t.update(&[], secs(10) + TimeDelta::milliseconds(449), None),
                GameRunStatus::Running
            );
            // O retrato pode levar tempos diferentes; a segunda leitura não precisa cair em 500 ms exatos.
            assert_eq!(
                t.update(&[], secs(10) + TimeDelta::milliseconds(499), None),
                GameRunStatus::Exited
            );
        }
    }

    #[test]
    fn a_child_handoff_during_play_survives_a_missing_sample() {
        let mut t =
            GameRunTracker::new(TrackingPlan::new(S::Process), launched(), Some(100)).unwrap();
        t.update(&[child(200, 100, None)], secs(10), None);
        t.mark_shown();
        assert_eq!(
            t.update(&[], secs(10) + TimeDelta::milliseconds(250), None),
            GameRunStatus::Running
        );
        assert_eq!(
            t.update(
                &[child(300, 200, None)],
                secs(10) + TimeDelta::milliseconds(500),
                None
            ),
            GameRunStatus::Running
        );
        assert_eq!(t.game_process_ids(), [300]);
    }

    #[test]
    fn after_shown_process_exit_does_not_wait_for_a_stale_steam_signal() {
        let mut t = GameRunTracker::new(folder_plan(Some(APP_ID)), launched(), None).unwrap();
        let game = [proc(20, Some(&format!(r"{FOLDER}\game.exe")))];
        t.update(&game, secs(10), Some(APP_ID));
        t.mark_shown();
        assert_eq!(
            t.update(&[], secs(10) + TimeDelta::milliseconds(500), Some(APP_ID)),
            GameRunStatus::Exited
        );
    }

    #[test]
    fn a_shown_game_does_not_require_steam_to_have_confirmed_it() {
        let mut t = GameRunTracker::new(folder_plan(Some(APP_ID)), launched(), None).unwrap();
        let game = [proc(20, Some(&format!(r"{FOLDER}\game.exe")))];
        t.update(&game, secs(10), Some(0));
        t.mark_shown();
        assert!(!t.steam_confirmed);
        assert_eq!(t.update(&game, secs(20), Some(0)), GameRunStatus::Running);
        assert_eq!(
            t.update(&[], secs(20) + TimeDelta::milliseconds(500), Some(0)),
            GameRunStatus::Exited
        );
    }

    #[test]
    fn cold_steam_and_games_without_visible_processes_keep_their_start_rules() {
        for strategy in [S::SteamApp, S::InstallFolder] {
            let mut plan = folder_plan(Some(APP_ID));
            plan.strategy = strategy;
            let mut t = GameRunTracker::new(plan, launched(), None).unwrap();
            assert_eq!(t.update(&[], secs(90), Some(0)), GameRunStatus::Waiting);
            assert_eq!(
                t.update(&[], secs(100), Some(APP_ID)),
                GameRunStatus::Running
            );
            t.mark_shown();
            assert_eq!(
                t.update(&[], secs(101), Some(APP_ID)),
                GameRunStatus::Running
            );
            assert_eq!(
                t.update(&[], secs(101) + TimeDelta::milliseconds(500), Some(0)),
                GameRunStatus::Exited
            );
        }
    }

    #[test]
    fn process_tracking_counts_as_running_from_launch_and_follows_children() {
        let mut t =
            GameRunTracker::new(TrackingPlan::new(S::Process), launched(), Some(100)).unwrap();
        assert_eq!(t.status, GameRunStatus::Running);
        t.update(
            &[child(100, 1, None), child(200, 100, Some(secs(1)))],
            secs(2),
            None,
        );
        let s = t.update(
            &[child(200, 100, None), child(300, 200, None)],
            secs(4),
            None,
        );
        assert_eq!(s, GameRunStatus::Running);
        assert_eq!(t.game_process_ids(), [200, 300]);
        assert_eq!(
            t.update(&[child(300, 200, None)], secs(600), None),
            GameRunStatus::Running
        );
        t.update(&[child(999, 1, None)], secs(602), None);
        assert_eq!(
            t.update(&[child(999, 1, None)], secs(604), None),
            GameRunStatus::Exited
        );
        assert_eq!(t.play_time(), TimeDelta::minutes(10));
    }

    #[test]
    fn process_tracking_ignores_reused_ids_and_older_processes() {
        let mut t =
            GameRunTracker::new(TrackingPlan::new(S::Process), launched(), Some(100)).unwrap();
        t.update(&[child(1, 0, None)], secs(2), None);
        t.update(
            &[child(100, 1, None), child(50, 100, Some(secs(-3600)))],
            secs(4),
            None,
        );
        assert_eq!(t.status, GameRunStatus::Exited);
        assert!(t.game_process_ids().is_empty());
    }

    #[test]
    fn constructor_rejects_plans_without_what_they_need() {
        assert!(GameRunTracker::new(TrackingPlan::NONE, launched(), None).is_err());
        assert!(
            GameRunTracker::new(TrackingPlan::new(S::InstallFolder), launched(), None).is_err()
        );
        assert!(GameRunTracker::new(TrackingPlan::new(S::Process), launched(), None).is_err());
        assert!(GameRunTracker::new(TrackingPlan::new(S::SteamApp), launched(), None).is_err());
    }

    // Com o RunningAppID da Steam (SessionDevicesTests)

    #[test]
    fn steam_confirms_the_start_even_when_no_game_process_is_visible() {
        let mut t = GameRunTracker::new(folder_plan(Some(APP_ID)), launched(), None).unwrap();
        assert_eq!(t.update(&[], secs(5), Some(0)), GameRunStatus::Waiting);
        assert_eq!(
            t.update(&[], secs(10), Some(APP_ID)),
            GameRunStatus::Running
        );
        assert!(t.steam_confirmed);
    }

    #[test]
    fn once_confirmed_steam_decides_the_end() {
        let mut t = GameRunTracker::new(folder_plan(Some(APP_ID)), launched(), None).unwrap();
        let crash_handler = vec![proc(
            30,
            Some(&format!(r"{FOLDER}\UnityCrashHandler64.exe")),
        )];
        let mut playing = crash_handler.clone();
        playing.push(proc(20, Some(&format!(r"{FOLDER}\Hades.exe"))));
        t.update(&playing, secs(10), Some(APP_ID));
        t.update(&crash_handler, secs(1800), Some(APP_ID));
        assert_eq!(
            t.update(&crash_handler, secs(1802), Some(0)),
            GameRunStatus::Running
        );
        assert_eq!(
            t.update(&crash_handler, secs(1804), Some(0)),
            GameRunStatus::Exited
        );
        assert_eq!(t.play_time(), TimeDelta::seconds(1790));
    }

    #[test]
    fn after_a_close_request_the_end_does_not_wait_for_steam() {
        let mut t = GameRunTracker::new(folder_plan(Some(APP_ID)), launched(), None).unwrap();
        t.exit_grace = TimeDelta::seconds(1);
        let playing = vec![proc(20, Some(&format!(r"{FOLDER}\Hades.exe")))];
        t.update(&playing, secs(10), Some(APP_ID));
        t.update(&playing, secs(20), Some(APP_ID));
        t.close_requested = true;
        // O processo saiu, mas a Steam ainda diz que o jogo roda.
        assert_eq!(
            t.update(&[], secs(20), Some(APP_ID)),
            GameRunStatus::Running
        );
        assert_eq!(t.update(&[], secs(21), Some(APP_ID)), GameRunStatus::Exited);
        assert_eq!(t.play_time(), TimeDelta::seconds(10));
    }

    #[test]
    fn unreadable_steam_state_falls_back_to_the_processes() {
        let mut t = GameRunTracker::new(folder_plan(Some(APP_ID)), launched(), None).unwrap();
        let game = [proc(20, Some(&format!(r"{FOLDER}\Hades.exe")))];
        t.update(&game, secs(10), Some(APP_ID));
        assert_eq!(t.update(&game, secs(300), None), GameRunStatus::Running);
        assert_eq!(t.update(&[], secs(304), None), GameRunStatus::Exited);
    }

    #[test]
    fn steam_only_tracking_waits_for_steam_and_times_out_without_it() {
        let p = TrackingPlan {
            steam_app_id: Some(APP_ID),
            ..TrackingPlan::new(S::SteamApp)
        };
        let mut t = GameRunTracker::new(p.clone(), launched(), None).unwrap();
        assert_eq!(
            t.update(&[], secs(20), Some(APP_ID)),
            GameRunStatus::Running
        );
        assert_eq!(t.update(&[], secs(30), Some(570)), GameRunStatus::Exited);
        let mut never = GameRunTracker::new(p, launched(), None).unwrap();
        assert_eq!(
            never.update(&[], secs(120), Some(0)),
            GameRunStatus::TimedOut
        );
    }

    #[test]
    fn game_end_flags_quick_exits_errors_and_games_that_never_started() {
        use std::time::Duration;
        let s = Duration::from_secs;
        let of = GameEnd::of;
        assert_eq!(
            of(GameRunStatus::Exited, false, s(3), None),
            GameEnd::Quick(None)
        );
        assert_eq!(
            of(GameRunStatus::Exited, false, s(3), Some(0xC000_0135)),
            GameEnd::Quick(Some(0xC000_0135))
        );
        assert_eq!(
            of(GameRunStatus::Exited, false, s(600), None),
            GameEnd::Normal
        );
        assert_eq!(
            of(GameRunStatus::Exited, false, s(600), Some(0)),
            GameEnd::Normal
        );
        // Código de erro avisa mesmo depois dos 10 s.
        assert_eq!(
            of(GameRunStatus::Exited, false, s(600), Some(0xC000_0005)),
            GameEnd::Failed(0xC000_0005)
        );
        // O Tako pediu para fechar (Central › Encerrar): fechar rápido ou com erro é o esperado.
        assert_eq!(of(GameRunStatus::Exited, true, s(3), None), GameEnd::Normal);
        assert_eq!(
            of(GameRunStatus::Exited, true, s(600), Some(1)),
            GameEnd::Normal
        );
        assert_eq!(
            of(GameRunStatus::TimedOut, false, s(0), None),
            GameEnd::NeverStarted
        );
        assert_eq!(
            of(GameRunStatus::TimedOut, true, s(0), None),
            GameEnd::Normal
        );
        for e in [
            GameEnd::Normal,
            GameEnd::Quick(None),
            GameEnd::Quick(Some(1)),
            GameEnd::NeverStarted,
            GameEnd::Failed(0xC000_007B),
        ] {
            assert_eq!(GameEnd::from_key(e.key(), e.exit_code()), e);
        }
        assert_eq!(GameEnd::from_key("lixo", Some(1)), GameEnd::Normal);
        // Supervisor antigo: sem código, "failed" não assusta.
        assert_eq!(GameEnd::from_key("failed", None), GameEnd::Normal);
        assert_eq!(GameEnd::from_key("failed", Some(0)), GameEnd::Normal);
    }

    #[test]
    fn exit_code_text_shows_hex_decimal_and_what_it_means() {
        use crate::i18n::Lang;
        let pt = |code| exit_code_text_in(Lang::PtBr, code);
        let dll = pt(0xC000_0135);
        assert!(
            dll.starts_with("Código de saída 0xC0000135 (3221225781): "),
            "{dll}"
        );
        assert!(dll.contains("DLL"), "{dll}");
        assert!(pt(0xC000_0005).contains("memória"));
        assert!(pt(0xC000_007B).contains("32"));
        // Código sem explicação: só o número.
        assert_eq!(pt(1), "Código de saída 0x1 (1)");
        for code in [
            0xC000_0135u32,
            0xC000_0139,
            0xC000_0142,
            0xC000_0005,
            0xC000_007B,
            0xC000_001D,
            0xC000_00FD,
            0xC000_0374,
            0xC000_0409,
            0xC000_0017,
        ] {
            for lang in [Lang::PtBr, Lang::EnUs] {
                assert!(
                    !exit_code_text_in(lang, code).contains("session."),
                    "{code:#X}"
                );
            }
        }
    }
}
