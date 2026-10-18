//! "Tela de jogo" (port de `ConsoleMode.Runtime/Display/GameScreen.cs` e de
//! `Session/GameScreen/GameScreenSession.cs`, só o modo que desconecta as outras telas): ao abrir o jogo,
//! liga a tela escolhida, torna-a principal, desliga as outras, aplica resolução/taxa e liga o HDR e o VRR; ao
//! terminar, devolve exatamente a topologia lida antes. Falhas nunca derrubam a sessão.

use crate::devices::{
    ConnectedDisplay, DeviceError, DeviceResult, DisplayMode, DisplayTopology, Displays, HdrStatus,
    ScreenBounds,
};
use crate::i18n::{t, tf};
use std::time::Duration;

/// Preferências da tela de jogo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameScreenSettings {
    pub display_id: String,
    /// `None` = manter o modo atual.
    pub mode: Option<DisplayMode>,
    pub enable_hdr: bool,
    pub enable_vrr: bool,
}

impl GameScreenSettings {
    /// Tela + modo que o usuário confirmou ver (`id|LxA@Hz` ou `id|current`). Outra combinação pergunta
    /// "Você consegue ver esta tela?" antes de abrir o jogo.
    pub fn setup_key(&self) -> String {
        let mode = self.mode.map_or("current".to_owned(), |m| m.key());
        format!("{}|{mode}", self.display_id)
    }

    pub fn needs_confirmation(&self, confirmed: Option<&str>) -> bool {
        !confirmed.is_some_and(|c| c.eq_ignore_ascii_case(&self.setup_key()))
    }
}

/// Passos para focar a tela de jogo. `problem` preenchido: nada a fazer (o jogo abre como está).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameScreenPlan {
    pub target: Option<ConnectedDisplay>,
    pub enable_target: bool,
    pub hide_ids: Vec<String>,
    pub mode: Option<DisplayMode>,
    pub turn_on_hdr: bool,
    pub turn_on_vrr: bool,
    pub notes: Vec<String>,
    pub problem: Option<String>,
}

impl GameScreenPlan {
    pub fn can_apply(&self) -> bool {
        self.problem.is_none() && self.target.is_some()
    }
}

pub fn plan(settings: &GameScreenSettings, displays: &[ConnectedDisplay]) -> GameScreenPlan {
    let Some(target) = displays
        .iter()
        .find(|d| d.id.eq_ignore_ascii_case(&settings.display_id))
    else {
        return GameScreenPlan {
            target: None,
            enable_target: false,
            hide_ids: vec![],
            mode: None,
            turn_on_hdr: false,
            turn_on_vrr: false,
            notes: vec![],
            problem: Some(t("session.screen.not_connected")),
        };
    };
    let hide_ids = displays
        .iter()
        .filter(|d| d.is_active && d.id != target.id)
        .map(|d| d.id.clone())
        .collect();
    // Modo pedido só se diferente do atual (com a tela desligada não dá para saber, então aplica).
    let mode = settings
        .mode
        .filter(|m| m.is_valid() && (!target.is_active || target.current_mode != Some(*m)));
    let mut notes = vec![];
    let mut wants = |enable: bool, status: HdrStatus, note: &str| {
        enable
            && match status {
                HdrStatus::Unsupported => {
                    notes.push(tf(note, &[("screen", &target.name)]));
                    false
                }
                HdrStatus::On => false,
                // Desligado, ou desconhecido porque a tela ainda está desligada: tenta depois de ligá-la.
                HdrStatus::Off | HdrStatus::Unknown => true,
            }
    };
    let turn_on_hdr = wants(settings.enable_hdr, target.hdr, "session.screen.no_hdr");
    let turn_on_vrr = wants(settings.enable_vrr, target.vrr, "vrr.session.unavailable");
    GameScreenPlan {
        enable_target: !target.is_active,
        target: Some(target.clone()),
        hide_ids,
        mode,
        turn_on_hdr,
        turn_on_vrr,
        notes,
        problem: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameScreenOutcome {
    /// Nada foi alterado; o jogo abre como está.
    Skipped,
    /// A tela de jogo está em foco; [`GameScreenSession::end`] desfaz.
    Applied,
    /// Algo deu errado no meio; as telas já voltaram como estavam.
    Reverted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameScreenResult {
    pub outcome: GameScreenOutcome,
    /// Motivo para o usuário (Skipped e Reverted).
    pub reason: Option<String>,
    pub target: Option<ConnectedDisplay>,
    pub target_bounds: Option<ScreenBounds>,
    pub needs_confirmation: bool,
}

impl GameScreenResult {
    fn skip(reason: String) -> Self {
        GameScreenResult {
            outcome: GameScreenOutcome::Skipped,
            reason: Some(reason),
            target: None,
            target_bounds: None,
            needs_confirmation: false,
        }
    }
}

#[derive(Debug)]
struct Active {
    before: DisplayTopology,
    hdr_turned_on_for: Option<String>,
    vrr_turned_on_for: Option<String>,
}

/// Entre [`GameScreenSession::begin`] e [`GameScreenSession::end`]. Lento (espera a TV ligar): fora da
/// thread da UI.
#[derive(Debug)]
pub struct GameScreenSession {
    active: Option<Active>,
    sleep: fn(Duration),
}

impl Default for GameScreenSession {
    fn default() -> Self {
        Self::new(std::thread::sleep)
    }
}

impl GameScreenSession {
    /// Quanto esperar a tela ligar (uma TV saindo da espera leva alguns segundos).
    pub const WAKE_TIMEOUT: Duration = Duration::from_secs(8);
    const POLL: Duration = Duration::from_millis(400);

    /// `sleep`: espera entre conferências (os testes passam uma que não espera).
    pub fn new(sleep: fn(Duration)) -> Self {
        GameScreenSession {
            active: None,
            sleep,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    /// Tela em que a sessão ligou o HDR (para desligar numa recuperação depois de uma queda).
    pub fn hdr_display_id(&self) -> Option<&str> {
        self.active.as_ref()?.hdr_turned_on_for.as_deref()
    }

    /// `confirmed_setup`: a combinação já confirmada; diferente dela, o resultado pede confirmação.
    pub fn begin(
        &mut self,
        displays: &dyn Displays,
        settings: &GameScreenSettings,
        confirmed_setup: Option<&str>,
    ) -> GameScreenResult {
        if self.active.is_some() {
            return GameScreenResult::skip(t("session.screen.already_active"));
        }
        if displays.is_managed_by_session() {
            return GameScreenResult::skip(t("session.screen.managed"));
        }
        let read = displays
            .list()
            .and_then(|list| Ok((plan(settings, &list), displays.capture()?)));
        let (plan, before) = match read {
            Ok((plan, _)) if !plan.can_apply() => {
                let problem = plan.problem.unwrap_or_default();
                tracing::info!("tela de jogo não aplicada: {problem}");
                return GameScreenResult::skip(problem);
            }
            Ok(v) => v,
            Err(e) => {
                // Sem a foto de antes não há como voltar: melhor não mexer.
                tracing::warn!("tela de jogo: falha ao ler as telas: {e}");
                return GameScreenResult::skip(t("session.screen.read_failed"));
            }
        };
        let Some(target) = plan.target.clone() else {
            return GameScreenResult::skip(t("session.screen.not_connected"));
        };
        tracing::info!(
            "tela de jogo: {}, desligar {} tela(s), modo {:?}, ligar HDR {}, ligar VRR {}",
            target.name,
            plan.hide_ids.len(),
            plan.mode,
            plan.turn_on_hdr,
            plan.turn_on_vrr
        );
        for note in &plan.notes {
            tracing::info!("tela de jogo: {note}");
        }
        self.active = Some(Active {
            before,
            hdr_turned_on_for: None,
            vrr_turned_on_for: None,
        });
        match self.apply(displays, settings, &plan, &target, confirmed_setup) {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!(
                    "tela de jogo: falha ao focar {}: {e}; voltando as telas",
                    target.name
                );
                self.end(displays);
                GameScreenResult {
                    outcome: GameScreenOutcome::Reverted,
                    reason: Some(e.to_string()),
                    target: Some(target),
                    target_bounds: None,
                    needs_confirmation: false,
                }
            }
        }
    }

    fn apply(
        &mut self,
        displays: &dyn Displays,
        settings: &GameScreenSettings,
        plan: &GameScreenPlan,
        target: &ConnectedDisplay,
        confirmed_setup: Option<&str>,
    ) -> Result<GameScreenResult, DeviceError> {
        let id = target.id.clone();
        if plan.enable_target {
            // Liga junto com as que já estão ligadas: nunca desligar as outras com a de jogo apagada.
            let mut on = plan.hide_ids.clone();
            on.push(id.clone());
            displays.set_active(&on, &id)?;
            if !self.wait_active(displays, &id) {
                displays.set_active(&on, &id)?;
                if !self.wait_active(displays, &id) {
                    tracing::warn!("tela de jogo: {} não ligou a tempo", target.name);
                    self.end(displays);
                    return Ok(GameScreenResult {
                        outcome: GameScreenOutcome::Reverted,
                        reason: Some(tf(
                            "session.screen.did_not_wake",
                            &[("screen", &target.name)],
                        )),
                        target: Some(target.clone()),
                        target_bounds: None,
                        needs_confirmation: false,
                    });
                }
            }
        }
        // Desliga as outras (ou só torna principal a que já era a única ligada).
        if !plan.hide_ids.is_empty() || (!target.is_primary && !plan.enable_target) {
            displays.set_active(std::slice::from_ref(&id), &id)?;
        }
        if let Some(mode) = plan.mode
            && let Err(e) = displays.set_mode(&id, mode)
        {
            // Resolução recusada não impede o jogo: segue no modo atual da tela.
            tracing::warn!("tela de jogo: {mode} recusado em {}: {e}", target.name);
        }
        if plan.turn_on_hdr {
            let status = find(displays, &id).map(|d| d.hdr);
            let on = switch_on("HDR", status, &target.name, || displays.set_hdr(&id, true));
            if let Some(a) = self.active.as_mut().filter(|_| on) {
                a.hdr_turned_on_for = Some(id.clone());
            }
        }
        if plan.turn_on_vrr {
            let status = find(displays, &id).map(|d| d.vrr);
            let on = switch_on("VRR", status, &target.name, || displays.set_vrr(&id, true));
            if let Some(a) = self.active.as_mut().filter(|_| on) {
                a.vrr_turned_on_for = Some(id.clone());
            }
        }
        let bounds = find(displays, &id).and_then(|d| d.bounds);
        let confirm = settings.needs_confirmation(confirmed_setup);
        tracing::info!(
            "tela de jogo em foco: {} {bounds:?}; pedir confirmação: {confirm}",
            target.name
        );
        Ok(GameScreenResult {
            outcome: GameScreenOutcome::Applied,
            reason: None,
            target: Some(target.clone()),
            target_bounds: bounds,
            needs_confirmation: confirm,
        })
    }

    /// Desfaz: desliga o HDR e o VRR que foram ligados e devolve a topologia de antes. Sem sessão ativa, nada
    /// muda (terminar duas vezes é seguro). Falso se a conferência ainda encontrou diferença.
    pub fn end(&mut self, displays: &dyn Displays) -> bool {
        let Some(active) = self.active.take() else {
            return true;
        };
        if let Some(id) = &active.hdr_turned_on_for
            && let Err(e) = displays.set_hdr(id, false)
        {
            tracing::warn!("tela de jogo: falha ao desligar o HDR de {id}: {e}");
        }
        if let Some(id) = &active.vrr_turned_on_for
            && let Err(e) = displays.set_vrr(id, false)
        {
            tracing::warn!("tela de jogo: falha ao desligar o VRR de {id}: {e}");
        }
        // Duas tentativas: depois de religar telas o SO às vezes reposiciona tudo de novo.
        for attempt in 1..=2 {
            match displays.capture() {
                Ok(now) if now.is_equivalent_to(&active.before) => {
                    tracing::info!("tela de jogo desfeita ({} restauração(ões))", attempt - 1);
                    return true;
                }
                Ok(_) => {
                    if let Err(e) = displays.apply(&active.before) {
                        tracing::warn!(
                            "tela de jogo: falha ao restaurar (tentativa {attempt}): {e}"
                        );
                    }
                    (self.sleep)(Self::POLL);
                }
                Err(e) => {
                    tracing::warn!("tela de jogo: falha ao ler as telas (tentativa {attempt}): {e}")
                }
            }
        }
        let ok = displays
            .capture()
            .is_ok_and(|now| now.is_equivalent_to(&active.before));
        if !ok {
            tracing::warn!("tela de jogo: as telas não voltaram exatamente como antes");
        }
        ok
    }

    fn wait_active(&self, displays: &dyn Displays, id: &str) -> bool {
        let mut waited = Duration::ZERO;
        loop {
            if find(displays, id).is_some_and(|d| d.is_active) {
                return true;
            }
            if waited >= Self::WAKE_TIMEOUT {
                return false;
            }
            (self.sleep)(Self::POLL);
            waited += Self::POLL;
        }
    }
}

/// Liga o HDR ou o VRR se estiver desligado; `true` se foi o Tako que ligou (e então desliga no fim).
fn switch_on(
    what: &str,
    status: Option<HdrStatus>,
    name: &str,
    set: impl FnOnce() -> DeviceResult<()>,
) -> bool {
    match status {
        Some(HdrStatus::Off) => match set() {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("tela de jogo: falha ao ligar o {what} de {name}: {e}");
                false
            }
        },
        Some(HdrStatus::On) => false,
        _ => {
            tracing::info!("tela de jogo: {what} não disponível em {name}");
            false
        }
    }
}

fn find(displays: &dyn Displays, id: &str) -> Option<ConnectedDisplay> {
    displays
        .list()
        .ok()?
        .into_iter()
        .find(|d| d.id.eq_ignore_ascii_case(id))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
pub(crate) mod tests {
    use super::*;
    use crate::devices::fake::FakeDisplays;
    use crate::devices::{DeviceResult, MonitorState};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    const MONITOR: &str = "DISPLAY1";
    const TV: &str = "DISPLAY2";

    fn display(
        id: &str,
        active: bool,
        primary: bool,
        bounds: Option<(i32, u32, u32)>,
        mode: Option<DisplayMode>,
        hdr: HdrStatus,
    ) -> ConnectedDisplay {
        ConnectedDisplay {
            id: id.into(),
            name: id.into(),
            is_active: active,
            is_primary: primary,
            bounds: bounds.map(|(x, w, h)| ScreenBounds {
                x,
                y: 0,
                width: w as i32,
                height: h as i32,
            }),
            current_mode: mode,
            hdr,
            vrr: hdr,
        }
    }

    fn planner_desk() -> [ConnectedDisplay; 3] {
        [
            display(
                "D",
                true,
                true,
                Some((0, 2560, 1440)),
                Some(DisplayMode::new(2560, 1440, 144)),
                HdrStatus::Unsupported,
            ),
            display(
                "S",
                true,
                false,
                Some((2560, 1920, 1080)),
                Some(DisplayMode::new(1920, 1080, 60)),
                HdrStatus::Off,
            ),
            display("T", false, false, None, None, HdrStatus::Unknown),
        ]
    }

    fn settings(id: &str, mode: Option<DisplayMode>, hdr: bool) -> GameScreenSettings {
        GameScreenSettings {
            display_id: id.into(),
            mode,
            enable_hdr: hdr,
            enable_vrr: false,
        }
    }

    // GameScreenPlannerTests

    #[test]
    fn setup_key_changes_with_screen_or_mode() {
        let current = settings("T", None, false);
        let mode = settings("T", Some(DisplayMode::new(3840, 2160, 120)), false);
        assert_eq!(current.setup_key(), "T|current");
        assert_eq!(mode.setup_key(), "T|3840x2160@120");
        assert!(!current.needs_confirmation(Some("t|CURRENT")));
        assert!(mode.needs_confirmation(Some("T|current")));
        assert!(current.needs_confirmation(None));
        // HDR não muda o que aparece na tela de jogo.
        assert!(!settings("T", None, true).needs_confirmation(Some(&current.setup_key())));
    }

    #[test]
    fn inactive_tv_is_turned_on_and_every_other_active_screen_is_hidden() {
        let p = plan(&settings("T", None, false), &planner_desk());
        assert!(p.can_apply() && p.enable_target);
        assert_eq!(p.hide_ids, ["D", "S"]);
        assert_eq!(p.mode, None);
        assert!(!p.turn_on_hdr);
    }

    #[test]
    fn screen_that_is_not_connected_is_a_problem_and_changes_nothing() {
        let desk = planner_desk();
        let p = plan(&settings("gone", None, true), &desk[..2]);
        assert!(!p.can_apply());
        assert!(p.problem.is_some());
        assert!(p.hide_ids.is_empty());
    }

    #[test]
    fn mode_is_applied_only_when_different() {
        let desk = planner_desk();
        let same = plan(
            &settings("S", Some(DisplayMode::new(1920, 1080, 60)), false),
            &desk[..2],
        );
        let other = plan(
            &settings("S", Some(DisplayMode::new(1280, 720, 60)), false),
            &desk[..2],
        );
        let asleep = plan(
            &settings("T", Some(DisplayMode::new(3840, 2160, 120)), false),
            &[desk[0].clone(), desk[2].clone()],
        );
        assert_eq!(same.mode, None);
        assert_eq!(other.mode, Some(DisplayMode::new(1280, 720, 60)));
        assert_eq!(asleep.mode, Some(DisplayMode::new(3840, 2160, 120)));
        assert_eq!(same.hide_ids, ["D"]);
    }

    #[test]
    fn hdr_is_turned_on_only_where_it_can_be() {
        let desk = planner_desk();
        let off = plan(&settings("S", None, true), &desk[..2]);
        let unsupported = plan(&settings("D", None, true), &desk[..2]);
        let mut on = desk.clone();
        on[1].hdr = HdrStatus::On;
        let already_on = plan(&settings("S", None, true), &on[..2]);
        let asleep = plan(
            &settings("T", None, true),
            &[desk[0].clone(), desk[2].clone()],
        );
        assert!(off.turn_on_hdr);
        assert!(!unsupported.turn_on_hdr);
        assert_eq!(unsupported.notes.len(), 1);
        assert!(!already_on.turn_on_hdr);
        assert!(asleep.turn_on_hdr); // decide depois de ligar a TV
    }

    #[test]
    fn vrr_follows_the_same_rules_as_hdr() {
        let desk = planner_desk();
        let vrr = |id: &str, d: &[ConnectedDisplay]| {
            let mut st = settings(id, None, false);
            st.enable_vrr = true;
            plan(&st, d)
        };
        let off = vrr("S", &desk[..2]);
        let unsupported = vrr("D", &desk[..2]);
        assert!(off.turn_on_vrr && !off.turn_on_hdr);
        assert!(!unsupported.turn_on_vrr);
        assert_eq!(unsupported.notes, ["VRR não disponível em D"]);
    }

    // GameScreenSessionTests, contra o FakeDisplays com falhas injetadas.

    /// FakeDisplays que registra as chamadas e pode recusar ligar uma tela, falhar a leitura ou
    /// recusar modos que a tela não lista.
    pub(crate) struct Desk {
        pub inner: FakeDisplays,
        pub calls: Mutex<Vec<String>>,
        pub never_wake: Vec<String>,
        pub fail_next: AtomicBool,
    }

    impl Desk {
        pub fn new() -> Self {
            Desk {
                inner: FakeDisplays::desk(),
                calls: Mutex::new(vec![]),
                never_wake: vec![],
                fail_next: AtomicBool::new(false),
            }
        }
        fn log(&self, s: String) {
            self.calls.lock().unwrap().push(s);
        }
        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
        fn fail(&self) -> DeviceResult<()> {
            if self.fail_next.swap(false, Ordering::SeqCst) {
                return Err(DeviceError::Failed("leitura falhou (simulado)".into()));
            }
            Ok(())
        }
    }

    impl Displays for Desk {
        fn list(&self) -> DeviceResult<Vec<ConnectedDisplay>> {
            self.fail()?;
            self.inner.list()
        }
        fn modes(&self, id: &str) -> DeviceResult<Vec<DisplayMode>> {
            self.inner.modes(id)
        }
        fn set_mode(&self, id: &str, mode: DisplayMode) -> DeviceResult<()> {
            self.log(format!("mode {id} {}", mode.key()));
            if !self.inner.modes(id)?.contains(&mode) {
                return Err(DeviceError::Unsupported(mode.key()));
            }
            self.inner.set_mode(id, mode)
        }
        fn set_active(&self, ids: &[String], primary: &str) -> DeviceResult<()> {
            self.log(format!("active {} primary {primary}", ids.join(",")));
            let awake: Vec<String> = ids
                .iter()
                .filter(|i| !self.never_wake.contains(i))
                .cloned()
                .collect();
            self.inner.set_active(&awake, primary)
        }
        fn set_hdr(&self, id: &str, on: bool) -> DeviceResult<()> {
            self.log(format!("hdr {id} {}", if on { "on" } else { "off" }));
            self.inner.set_hdr(id, on)
        }
        fn set_vrr(&self, id: &str, on: bool) -> DeviceResult<()> {
            self.log(format!("vrr {id} {}", if on { "on" } else { "off" }));
            self.inner.set_vrr(id, on)
        }
        fn capture(&self) -> DeviceResult<DisplayTopology> {
            self.fail()?;
            self.inner.capture()
        }
        fn apply(&self, t: &DisplayTopology) -> DeviceResult<()> {
            self.log("apply".into());
            self.inner.apply(t)
        }
    }

    fn session() -> GameScreenSession {
        GameScreenSession::new(|_| {})
    }

    #[test]
    fn tv_is_turned_on_made_primary_and_the_desk_is_disconnected_then_everything_comes_back() {
        let desk = Desk::new();
        let before = desk.capture().unwrap();
        let mut s = session();
        let r = s.begin(
            &desk,
            &settings(TV, Some(DisplayMode::new(3840, 2160, 120)), true),
            None,
        );
        assert_eq!(r.outcome, GameScreenOutcome::Applied);
        assert!(r.needs_confirmation);
        assert_eq!(
            r.target_bounds,
            Some(ScreenBounds {
                x: 0,
                y: 0,
                width: 3840,
                height: 2160
            })
        );
        let during = desk.list().unwrap();
        let tv = during.iter().find(|d| d.id == TV).unwrap();
        assert!(tv.is_active && tv.is_primary && tv.hdr == HdrStatus::On);
        assert_eq!(tv.current_mode, Some(DisplayMode::new(3840, 2160, 120)));
        assert!(!during.iter().find(|d| d.id == MONITOR).unwrap().is_active);
        assert!(s.is_active() && s.hdr_display_id() == Some(TV));

        assert!(s.end(&desk));
        assert!(desk.capture().unwrap().is_equivalent_to(&before));
        assert!(desk.calls().contains(&format!("hdr {TV} off")));
        assert!(!s.is_active());
        let calls = desk.calls().len();
        assert!(s.end(&desk)); // terminar de novo não mexe em nada
        assert_eq!(desk.calls().len(), calls);
    }

    #[test]
    fn vrr_turned_on_for_the_game_is_turned_off_at_the_end() {
        let desk = Desk::new();
        let mut st = settings(TV, None, false);
        st.enable_vrr = true;
        let mut s = session();
        assert_eq!(
            s.begin(&desk, &st, None).outcome,
            GameScreenOutcome::Applied
        );
        let tv = find(&desk, TV).unwrap();
        assert_eq!(tv.vrr, crate::devices::VrrStatus::On);
        assert!(s.end(&desk));
        assert!(desk.calls().contains(&format!("vrr {TV} off")));
        assert_eq!(find(&desk, TV).unwrap().vrr, crate::devices::VrrStatus::Off);

        // Já ligado antes do jogo: o Tako não liga e, no fim, não desliga.
        let mut s = session();
        desk.inner.set_vrr(TV, true).unwrap();
        s.begin(&desk, &st, None);
        s.end(&desk);
        let vrr_calls = desk.calls().iter().filter(|c| c.starts_with("vrr")).count();
        assert_eq!(vrr_calls, 2); // só o liga/desliga da primeira sessão
        assert_eq!(find(&desk, TV).unwrap().vrr, crate::devices::VrrStatus::On);
    }

    #[test]
    fn calls_follow_the_console_mode_order() {
        let desk = Desk::new();
        session().begin(&desk, &settings(TV, None, false), None);
        assert_eq!(
            desk.calls(),
            [
                format!("active {MONITOR},{TV} primary {TV}"),
                format!("active {TV} primary {TV}")
            ]
        );
    }

    #[test]
    fn confirmed_setup_is_not_asked_again() {
        let desk = Desk::new();
        let st = settings(TV, None, false);
        assert!(
            !session()
                .begin(&desk, &st, Some(&st.setup_key()))
                .needs_confirmation
        );
    }

    #[test]
    fn tv_that_does_not_wake_puts_everything_back_and_never_hides_the_desk() {
        let mut desk = Desk::new();
        desk.never_wake.push(TV.into());
        let before = desk.capture().unwrap();
        let mut s = session();
        let r = s.begin(&desk, &settings(TV, None, false), None);
        assert_eq!(r.outcome, GameScreenOutcome::Reverted);
        assert_eq!(r.reason.as_deref(), Some("TV não ligou"));
        let calls = desk.calls();
        assert!(!calls.contains(&format!("active {TV} primary {TV}")));
        assert_eq!(
            calls
                .iter()
                .filter(|c| c.starts_with("active") && c.contains(MONITOR))
                .count(),
            2
        );
        assert!(desk.capture().unwrap().is_equivalent_to(&before));
        assert!(!s.is_active());
    }

    #[test]
    fn missing_screen_changes_nothing() {
        let desk = Desk::new();
        let r = session().begin(&desk, &settings("GONE", None, true), None);
        assert_eq!(r.outcome, GameScreenOutcome::Skipped);
        assert!(desk.calls().is_empty());
    }

    #[test]
    fn platform_failure_while_reading_changes_nothing() {
        let desk = Desk::new();
        desk.fail_next.store(true, Ordering::SeqCst);
        let mut s = session();
        assert_eq!(
            s.begin(&desk, &settings(TV, None, false), None).outcome,
            GameScreenOutcome::Skipped
        );
        assert!(desk.calls().is_empty());
        assert!(s.end(&desk));
    }

    #[test]
    fn refused_mode_keeps_the_screen_focused_in_its_current_mode() {
        let desk = Desk::new();
        let r = session().begin(
            &desk,
            &settings(TV, Some(DisplayMode::new(1280, 720, 30)), false),
            None,
        );
        assert_eq!(r.outcome, GameScreenOutcome::Applied);
        let tv = desk
            .list()
            .unwrap()
            .into_iter()
            .find(|d| d.id == TV)
            .unwrap();
        assert_eq!(tv.current_mode, Some(DisplayMode::new(1920, 1080, 60)));
    }

    #[test]
    fn already_focused_screen_with_nothing_else_on_is_left_alone() {
        let desk = Desk::new();
        let mut s = session();
        let r = s.begin(&desk, &settings(MONITOR, None, true), None);
        assert_eq!(r.outcome, GameScreenOutcome::Applied);
        assert!(desk.calls().is_empty()); // HDR sem suporte: nem tenta
        assert!(s.end(&desk));
        assert!(!desk.calls().contains(&"apply".to_owned())); // nada mudou: nenhuma restauração
    }

    #[test]
    fn a_session_managed_screen_is_only_reported() {
        struct Gamescope(Desk);
        impl Displays for Gamescope {
            fn list(&self) -> DeviceResult<Vec<ConnectedDisplay>> {
                self.0.list()
            }
            fn modes(&self, id: &str) -> DeviceResult<Vec<DisplayMode>> {
                self.0.modes(id)
            }
            fn set_mode(&self, id: &str, m: DisplayMode) -> DeviceResult<()> {
                self.0.set_mode(id, m)
            }
            fn set_active(&self, ids: &[String], p: &str) -> DeviceResult<()> {
                self.0.set_active(ids, p)
            }
            fn set_hdr(&self, id: &str, on: bool) -> DeviceResult<()> {
                self.0.set_hdr(id, on)
            }
            fn capture(&self) -> DeviceResult<DisplayTopology> {
                self.0.capture()
            }
            fn apply(&self, t: &DisplayTopology) -> DeviceResult<()> {
                self.0.apply(t)
            }
            fn is_managed_by_session(&self) -> bool {
                true
            }
        }
        let g = Gamescope(Desk::new());
        assert_eq!(
            session()
                .begin(&g, &settings(TV, None, false), None)
                .outcome,
            GameScreenOutcome::Skipped
        );
        assert!(g.0.calls().is_empty());
    }

    #[test]
    fn topology_equivalence_ignores_order_names_and_platform_data() {
        // EnvironmentContractTests: contratos de devices usados pela restauração.
        let a = MonitorState {
            id: "A".into(),
            name: "Monitor A".into(),
            is_primary: true,
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            refresh_hz: 60.0,
            rotation: 0,
        };
        let b = MonitorState {
            id: "B".into(),
            name: "Monitor B".into(),
            is_primary: false,
            x: 1920,
            ..a.clone()
        };
        let ab = DisplayTopology {
            monitors: vec![a.clone(), b.clone()],
            platform_data: Some("x".into()),
        };
        let renamed = MonitorState {
            name: "Outro".into(),
            ..b
        };
        assert!(ab.is_equivalent_to(&DisplayTopology {
            monitors: vec![renamed, a.clone()],
            platform_data: None
        }));
        assert!(!ab.is_equivalent_to(&DisplayTopology {
            monitors: vec![a.clone()],
            platform_data: None
        }));
        let rotated = MonitorState {
            rotation: 180,
            ..a.clone()
        };
        let one = DisplayTopology {
            monitors: vec![a],
            platform_data: None,
        };
        assert!(!one.is_equivalent_to(&DisplayTopology {
            monitors: vec![rotated],
            platform_data: None
        }));
    }
}
