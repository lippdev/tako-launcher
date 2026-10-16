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
}

// Tolerância de relógio ao comparar o início de um filho com a hora em que o jogo foi aberto.
const CLOCK_SKEW: TimeDelta = TimeDelta::seconds(5);

impl GameRunTracker {
    pub const DEFAULT_START_TIMEOUT: TimeDelta = TimeDelta::minutes(2);
    pub const DEFAULT_EXIT_GRACE: TimeDelta = TimeDelta::seconds(4);

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

        if self.is_present(steam_running_app_id) {
            self.first_seen_at.get_or_insert(now);
            self.last_seen_at = Some(now);
            self.status = GameRunStatus::Running;
        } else if self.status == GameRunStatus::Waiting
            && now - self.launched_at >= self.start_timeout
        {
            self.status = GameRunStatus::TimedOut;
        } else if self.status == GameRunStatus::Running
            && self
                .last_seen_at
                .is_some_and(|l| now - l >= self.exit_grace)
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
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
}
