//! Política de reinício e detecção de travamento (port de `Supervision/SupervisorPolicy.cs`) e o
//! acompanhamento da UI que o `UiSupervisor.cs` fazia junto com a E/S ([`UiWatch`]). Relógio monotônico.

use super::{SupervisorStatus, UiCloseReason};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UiLaunchMode {
    #[default]
    Normal,
    /// `--safe-mode`: tema padrão embutido, sem atualizações automáticas.
    SafeMode,
}

/// Como a UI terminou, do ponto de vista do supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiExitKind {
    /// De propósito (código 0 ou saída anunciada): o supervisor sai junto.
    Clean,
    /// Código diferente de 0.
    Crashed,
    /// Parou de mandar heartbeat e foi encerrada pelo supervisor.
    Hung,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorAction {
    /// Sai junto com a UI.
    Exit,
    /// Abre a UI de novo, no modo da decisão.
    Restart,
    /// Desiste: sai e deixa o usuário no desktop.
    GiveUp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorDecision {
    pub action: SupervisorAction,
    pub mode: UiLaunchMode,
    pub failures_in_window: usize,
    /// Texto para o log.
    pub reason: String,
}

/// Limites do supervisor (padrões de produção; testes encurtam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorOptions {
    /// Reinícios no modo normal dentro de `restart_window` antes de tentar o modo seguro.
    pub max_restarts: usize,
    /// Falhas mais antigas não contam (uma queda por dia não leva ao modo seguro).
    pub restart_window: Duration,
    pub heartbeat_interval: Duration,
    /// Sem heartbeat por este tempo depois de conectar: a UI travou.
    pub heartbeat_timeout: Duration,
    /// Prazo para a UI abrir e mandar o primeiro heartbeat.
    pub startup_timeout: Duration,
    /// De quanto em quanto tempo o supervisor confere a UI.
    pub check_interval: Duration,
    /// Atualização anunciada que não termina neste prazo: o supervisor encerra a UI e sai.
    pub closing_grace: Duration,
    pub restart_delay: Duration,
}

impl Default for SupervisorOptions {
    fn default() -> Self {
        SupervisorOptions {
            max_restarts: 3,
            restart_window: Duration::from_secs(5 * 60),
            heartbeat_interval: Duration::from_secs(2),
            heartbeat_timeout: Duration::from_secs(20),
            startup_timeout: Duration::from_secs(60),
            check_interval: Duration::from_secs(1),
            closing_grace: Duration::from_secs(30),
            restart_delay: Duration::from_millis(500),
        }
    }
}

/// Escada de reinício: a UI que cai ou trava é reaberta até `max_restarts` vezes na janela; depois, uma
/// vez em modo seguro; se ainda falhar, o supervisor desiste. Saída limpa encerra o supervisor.
#[derive(Debug)]
pub struct SupervisorPolicy {
    options: SupervisorOptions,
    failures: VecDeque<Duration>,
    pub mode: UiLaunchMode,
    /// Reinícios feitos nesta execução do supervisor.
    pub restarts: u32,
}

impl SupervisorPolicy {
    pub fn new(options: SupervisorOptions) -> Self {
        SupervisorPolicy {
            options,
            failures: VecDeque::new(),
            mode: UiLaunchMode::Normal,
            restarts: 0,
        }
    }

    /// Modo da primeira abertura: `--safe-mode` já começa no último degrau.
    pub fn begin(&mut self, safe_mode_requested: bool) -> UiLaunchMode {
        self.mode = if safe_mode_requested {
            UiLaunchMode::SafeMode
        } else {
            UiLaunchMode::Normal
        };
        self.mode
    }

    /// Saída anunciada é limpa mesmo com código diferente de 0 (o instalador pode forçar o fechamento).
    pub fn classify(exit_code: i32, hung: bool, announced: Option<UiCloseReason>) -> UiExitKind {
        match (announced, hung, exit_code) {
            (Some(_), _, _) => UiExitKind::Clean,
            (_, true, _) => UiExitKind::Hung,
            (_, _, 0) => UiExitKind::Clean,
            _ => UiExitKind::Crashed,
        }
    }

    pub fn on_ui_exited(&mut self, kind: UiExitKind, now: Duration) -> SupervisorDecision {
        let decision = |action, mode, n, reason: String| SupervisorDecision {
            action,
            mode,
            failures_in_window: n,
            reason,
        };
        if kind == UiExitKind::Clean {
            let n = self.failures.len();
            let reason = "a UI fechou normalmente".into();
            return decision(SupervisorAction::Exit, self.mode, n, reason);
        }
        while self
            .failures
            .front()
            .is_some_and(|f| now.saturating_sub(*f) >= self.options.restart_window)
        {
            self.failures.pop_front();
        }
        self.failures.push_back(now);
        let n = self.failures.len();
        let what = if kind == UiExitKind::Hung {
            "travou"
        } else {
            "caiu"
        };
        if self.mode == UiLaunchMode::SafeMode {
            let reason = format!("a UI {what} também no modo seguro: voltando ao desktop");
            return decision(SupervisorAction::GiveUp, self.mode, n, reason);
        }
        self.restarts += 1;
        let max = self.options.max_restarts;
        if n <= max {
            let reason = format!("a UI {what} ({n}/{max} na janela)");
            return decision(SupervisorAction::Restart, self.mode, n, reason);
        }
        self.mode = UiLaunchMode::SafeMode;
        let reason = format!("a UI {what} {n} vezes seguidas: reabrindo em modo seguro");
        decision(SupervisorAction::Restart, self.mode, n, reason)
    }
}

/// Detecção de UI travada pelos heartbeats. Antes do primeiro vale o prazo de abertura; depois, o de
/// heartbeat. Se o próprio supervisor ficou parado (PC suspenso), o prazo recomeça ao acordar.
#[derive(Debug)]
pub struct HeartbeatMonitor {
    options: SupervisorOptions,
    last_beat: Duration,
    last_check: Duration,
    connected: bool,
    pub beats: u64,
}

impl HeartbeatMonitor {
    pub fn new(options: SupervisorOptions) -> Self {
        HeartbeatMonitor {
            options,
            last_beat: Duration::ZERO,
            last_check: Duration::ZERO,
            connected: false,
            beats: 0,
        }
    }

    /// Nova UI iniciada em `now`.
    pub fn reset(&mut self, now: Duration) {
        self.last_beat = now;
        self.last_check = now;
        self.connected = false;
        self.beats = 0;
    }

    pub fn beat(&mut self, now: Duration) {
        self.last_beat = now;
        self.connected = true;
        self.beats += 1;
    }

    /// Conferência periódica: verdadeiro quando a UI passou do prazo sem heartbeat.
    pub fn is_hung(&mut self, now: Duration) -> bool {
        // Conferências deveriam vir a cada check_interval; um buraco muito maior é o supervisor que não rodou.
        let suspend_gap = self.options.check_interval * 5 + Duration::from_secs(5);
        if now.saturating_sub(self.last_check) > suspend_gap {
            self.last_beat = now;
        }
        self.last_check = now;
        let limit = if self.connected {
            self.options.heartbeat_timeout
        } else {
            self.options.startup_timeout
        };
        now.saturating_sub(self.last_beat) > limit
    }
}

/// Por que o supervisor deve encerrar a UI agora.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillReason {
    /// Sem heartbeat no prazo.
    Hung,
    /// Atualização anunciada que não terminou no prazo.
    OverdueClose,
}

/// O laço do supervisor sem E/S: quem chama inicia o processo, repassa heartbeats e anúncios do IPC,
/// confere a cada `check_interval` e encerra a UI quando [`UiWatch::check`] pede.
#[derive(Debug)]
pub struct UiWatch {
    options: SupervisorOptions,
    pub policy: SupervisorPolicy,
    pub heartbeats: HeartbeatMonitor,
    current_pid: Option<u32>,
    announced: Option<(UiCloseReason, Duration)>,
}

impl UiWatch {
    pub fn new(options: SupervisorOptions) -> Self {
        UiWatch {
            policy: SupervisorPolicy::new(options.clone()),
            heartbeats: HeartbeatMonitor::new(options.clone()),
            options,
            current_pid: None,
            announced: None,
        }
    }

    pub fn begin(&mut self, safe_mode_requested: bool) -> UiLaunchMode {
        self.policy.begin(safe_mode_requested)
    }

    /// A UI foi aberta com o PID `pid`.
    pub fn started(&mut self, pid: u32, now: Duration) {
        self.current_pid = Some(pid);
        self.announced = None;
        self.heartbeats.reset(now);
    }

    /// Só o PID atual conta: uma UI antiga não mantém a nova viva.
    pub fn heartbeat(&mut self, beat: &super::UiHeartbeat, now: Duration) {
        if self.current_pid == Some(beat.process_id) {
            self.heartbeats.beat(now);
        }
    }

    pub fn closing(&mut self, closing: &super::UiClosing, now: Duration) {
        if self.current_pid == Some(closing.process_id) {
            self.announced = Some((closing.reason, now));
        }
    }

    /// Conferência periódica. Fim de sessão do SO pode ser cancelado, então a UI continua vigiada;
    /// só a atualização anunciada tem prazo para sair.
    pub fn check(&mut self, now: Duration) -> Option<KillReason> {
        match self.announced {
            Some((UiCloseReason::Update, at)) => (now.saturating_sub(at)
                > self.options.closing_grace)
                .then_some(KillReason::OverdueClose),
            _ => self.heartbeats.is_hung(now).then_some(KillReason::Hung),
        }
    }

    /// A UI terminou com `exit_code` (`killed` = encerrada pelo supervisor por este motivo).
    pub fn exited(
        &mut self,
        exit_code: i32,
        killed: Option<KillReason>,
        now: Duration,
    ) -> SupervisorDecision {
        let hung = killed == Some(KillReason::Hung);
        let kind = SupervisorPolicy::classify(exit_code, hung, self.announced.map(|a| a.0));
        self.current_pid = None;
        self.policy.on_ui_exited(kind, now)
    }

    pub fn status(&self, own_pid: u32) -> SupervisorStatus {
        SupervisorStatus {
            process_id: own_pid,
            restarts: self.policy.restarts,
            mode: self.policy.mode,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::{UiClosing, UiHeartbeat};
    use super::*;

    fn at(s: u64) -> Duration {
        Duration::from_secs(s)
    }
    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    #[test]
    fn clean_exit_ends_the_supervisor() {
        let mut p = SupervisorPolicy::new(SupervisorOptions::default());
        p.begin(false);
        assert_eq!(
            p.on_ui_exited(UiExitKind::Clean, at(1)).action,
            SupervisorAction::Exit
        );
        assert_eq!(p.restarts, 0);
    }

    #[test]
    fn failures_restart_then_escalate_to_safe_mode_then_give_up() {
        let mut p = SupervisorPolicy::new(SupervisorOptions::default());
        assert_eq!(p.begin(false), UiLaunchMode::Normal);
        let d: Vec<_> = (1..=5)
            .map(|i| {
                let kind = if i % 2 == 0 {
                    UiExitKind::Hung
                } else {
                    UiExitKind::Crashed
                };
                p.on_ui_exited(kind, at(i))
            })
            .collect();
        use SupervisorAction::*;
        use UiLaunchMode::*;
        let got: Vec<_> = d.iter().map(|d| (d.action, d.mode)).collect();
        assert_eq!(
            got,
            [
                (Restart, Normal),
                (Restart, Normal),
                (Restart, Normal),
                (Restart, SafeMode),
                (GiveUp, SafeMode)
            ]
        );
        assert!(d[1].reason.contains("travou"));
        assert!(d[3].reason.contains("modo seguro"));
        assert_eq!(p.restarts, 4);
    }

    #[test]
    fn failures_outside_the_window_do_not_add_up() {
        let mut p = SupervisorPolicy::new(SupervisorOptions::default());
        p.begin(false);
        for i in 0..10 {
            let d = p.on_ui_exited(UiExitKind::Crashed, at(i * 600));
            assert_eq!(
                (d.action, d.mode, d.failures_in_window),
                (SupervisorAction::Restart, UiLaunchMode::Normal, 1)
            );
        }
    }

    #[test]
    fn requested_safe_mode_gives_up_on_the_first_failure() {
        let mut p = SupervisorPolicy::new(SupervisorOptions::default());
        assert_eq!(p.begin(true), UiLaunchMode::SafeMode);
        assert_eq!(
            p.on_ui_exited(UiExitKind::Crashed, at(1)).action,
            SupervisorAction::GiveUp
        );
    }

    #[test]
    fn exit_classification() {
        use UiCloseReason::*;
        for (code, hung, announced, expected) in [
            (0, false, None, UiExitKind::Clean),
            (1, false, None, UiExitKind::Crashed),
            (-1, true, None, UiExitKind::Hung),
            (-1, false, Some(Update), UiExitKind::Clean),
            (-1, true, Some(SessionEnding), UiExitKind::Clean),
        ] {
            assert_eq!(SupervisorPolicy::classify(code, hung, announced), expected);
        }
    }

    fn opts(startup: Duration, heartbeat: Duration) -> SupervisorOptions {
        SupervisorOptions {
            startup_timeout: startup,
            heartbeat_timeout: heartbeat,
            check_interval: at(1),
            ..SupervisorOptions::default()
        }
    }

    #[test]
    fn heartbeat_monitor_uses_startup_timeout_then_heartbeat_timeout() {
        let mut m = HeartbeatMonitor::new(opts(at(60), at(20)));
        m.reset(at(0));
        for s in 1..=59 {
            assert!(!m.is_hung(at(s)));
        }
        m.beat(at(59));
        for s in 60..=79 {
            assert!(!m.is_hung(at(s)));
        }
        assert!(m.is_hung(at(80)));
        assert_eq!(m.beats, 1);
    }

    #[test]
    fn heartbeat_monitor_never_connected_hangs_after_startup_timeout() {
        let mut m = HeartbeatMonitor::new(opts(at(3), at(20)));
        m.reset(at(0));
        assert!(!m.is_hung(at(1)));
        assert!(!m.is_hung(at(2)));
        assert!(!m.is_hung(at(3)));
        assert!(m.is_hung(at(4)));
    }

    #[test]
    fn heartbeat_monitor_forgives_a_suspended_pc() {
        let mut m = HeartbeatMonitor::new(opts(at(60), at(20)));
        m.reset(at(0));
        m.beat(at(1));
        assert!(!m.is_hung(at(2)));
        assert!(!m.is_hung(at(3600)));
        for s in 3601..=3620 {
            assert!(!m.is_hung(at(s)));
        }
        assert!(m.is_hung(at(3621)));
    }

    // Cenários do UiSupervisorTests, sem IPC nem processos.

    fn fast() -> SupervisorOptions {
        SupervisorOptions {
            max_restarts: 2,
            heartbeat_interval: ms(30),
            heartbeat_timeout: ms(400),
            startup_timeout: ms(3000),
            check_interval: ms(25),
            closing_grace: ms(300),
            restart_delay: ms(1),
            ..SupervisorOptions::default()
        }
    }

    fn beat(pid: u32) -> UiHeartbeat {
        UiHeartbeat {
            process_id: pid,
            sequence: 0,
        }
    }

    /// Confere a cada 25 ms até `until`; devolve o motivo e quando, se pediu para encerrar.
    fn watch_until(w: &mut UiWatch, from: u64, until: u64) -> Option<(KillReason, u64)> {
        (from..=until)
            .step_by(25)
            .find_map(|t| w.check(ms(t)).map(|k| (k, t)))
    }

    #[test]
    fn clean_ui_exit_ends_the_supervisor() {
        let mut w = UiWatch::new(fast());
        w.begin(false);
        w.started(1000, ms(0));
        w.heartbeat(&beat(1000), ms(30));
        assert_eq!(
            w.status(42),
            SupervisorStatus {
                process_id: 42,
                restarts: 0,
                mode: UiLaunchMode::Normal
            }
        );
        assert_eq!(w.exited(0, None, ms(100)).action, SupervisorAction::Exit);
    }

    #[test]
    fn crashing_ui_is_restarted_then_opened_in_safe_mode_then_abandoned() {
        let mut w = UiWatch::new(fast());
        let mut mode = w.begin(false);
        let mut modes = vec![];
        let mut t = 0;
        loop {
            modes.push(mode);
            w.started(1000 + modes.len() as u32, ms(t));
            let d = w.exited(1, None, ms(t + 50));
            t += 100;
            match d.action {
                SupervisorAction::Restart => mode = d.mode,
                other => {
                    assert_eq!(other, SupervisorAction::GiveUp);
                    break;
                }
            }
        }
        use UiLaunchMode::*;
        assert_eq!(modes, [Normal, Normal, Normal, SafeMode]); // 1 + 2 reinícios + 1 em modo seguro
    }

    #[test]
    fn ui_that_stops_sending_heartbeats_is_killed_and_restarted() {
        let mut w = UiWatch::new(fast());
        w.begin(false);
        w.started(1000, ms(0));
        w.heartbeat(&beat(1000), ms(30));
        w.heartbeat(&beat(1000), ms(60));
        let (kill, when) = watch_until(&mut w, 25, 2000).unwrap();
        assert_eq!(kill, KillReason::Hung);
        assert!(when > 460, "{when}");
        let d = w.exited(-1, Some(kill), ms(when));
        assert_eq!(d.action, SupervisorAction::Restart);
        w.started(1001, ms(when + 1));
        assert_eq!(
            w.exited(0, None, ms(when + 100)).action,
            SupervisorAction::Exit
        );
    }

    #[test]
    fn ui_that_never_connects_counts_as_hung() {
        let mut w = UiWatch::new(SupervisorOptions {
            startup_timeout: ms(300),
            ..fast()
        });
        w.begin(false);
        w.started(1000, ms(0));
        let (kill, _) = watch_until(&mut w, 25, 1000).unwrap();
        assert_eq!(kill, KillReason::Hung);
    }

    #[test]
    fn announced_update_exit_is_not_restarted_even_with_an_error_code() {
        let mut w = UiWatch::new(fast());
        w.begin(false);
        w.started(1000, ms(0));
        w.heartbeat(&beat(1000), ms(30));
        let closing = UiClosing {
            process_id: 1000,
            reason: UiCloseReason::Update,
        };
        w.closing(&closing, ms(60));
        assert!(watch_until(&mut w, 75, 150).is_none());
        assert_eq!(w.exited(-1, None, ms(160)).action, SupervisorAction::Exit);
    }

    #[test]
    fn announced_exit_that_never_finishes_is_killed_after_the_grace_period() {
        let mut w = UiWatch::new(fast());
        w.begin(false);
        w.started(1000, ms(0));
        let closing = UiClosing {
            process_id: 1000,
            reason: UiCloseReason::Update,
        };
        w.closing(&closing, ms(10));
        let (kill, when) = watch_until(&mut w, 25, 2000).unwrap();
        assert_eq!(kill, KillReason::OverdueClose);
        assert_eq!(
            w.exited(-1, Some(kill), ms(when)).action,
            SupervisorAction::Exit
        );
    }

    #[test]
    fn heartbeats_from_an_old_ui_process_do_not_keep_the_new_one_alive() {
        let mut w = UiWatch::new(SupervisorOptions {
            startup_timeout: ms(400),
            ..fast()
        });
        w.begin(false);
        w.started(1000, ms(0));
        w.exited(1, None, ms(50));
        w.started(1001, ms(60));
        for t in (80..=600).step_by(20) {
            w.heartbeat(&beat(1000), ms(t));
            w.closing(
                &UiClosing {
                    process_id: 1000,
                    reason: UiCloseReason::Update,
                },
                ms(t),
            );
        }
        assert_eq!(
            watch_until(&mut w, 85, 1000).map(|k| k.0),
            Some(KillReason::Hung)
        );
    }

    #[test]
    fn session_ending_keeps_watching_but_does_not_restart() {
        let mut w = UiWatch::new(fast());
        w.begin(false);
        w.started(1000, ms(0));
        w.heartbeat(&beat(1000), ms(30));
        let closing = UiClosing {
            process_id: 1000,
            reason: UiCloseReason::SessionEnding,
        };
        w.closing(&closing, ms(40));
        for t in (50..=500).step_by(30) {
            w.heartbeat(&beat(1000), ms(t));
            assert_eq!(w.check(ms(t)), None); // viva além do prazo de saída
        }
        assert_eq!(w.exited(1, None, ms(520)).action, SupervisorAction::Exit);
    }
}
