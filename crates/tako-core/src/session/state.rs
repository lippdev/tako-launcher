//! Estados da sessão (PRD §5.2), port de `Session/SessionStateMachine.cs` com os estados do Rust:
//! `Browsing → Launching → Playing → Returning → Restoring → Browsing`, com `Error → Recovery → Desktop`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SessionState {
    #[default]
    Browsing,
    Launching,
    Playing,
    Returning,
    Restoring,
    Error,
    Recovery,
    /// O Tako desistiu da tela cheia e devolveu o desktop.
    Desktop,
}

impl SessionState {
    pub const ALL: [SessionState; 8] = [
        SessionState::Browsing,
        SessionState::Launching,
        SessionState::Playing,
        SessionState::Returning,
        SessionState::Restoring,
        SessionState::Error,
        SessionState::Recovery,
        SessionState::Desktop,
    ];

    /// Estados alcançáveis a partir deste (sem contar o próprio). `Launching → Browsing` cobre
    /// cancelamento e falha ao abrir (nada foi alterado ainda); `Desktop → Browsing` é o usuário voltando.
    pub fn next_states(self) -> &'static [SessionState] {
        use SessionState::*;
        match self {
            Browsing => &[Launching],
            Launching => &[Playing, Browsing, Error],
            Playing => &[Returning, Error],
            Returning => &[Restoring, Error],
            Restoring => &[Browsing, Error],
            Error => &[Recovery],
            Recovery => &[Browsing, Desktop],
            Desktop => &[Browsing],
        }
    }

    /// Pedido para o estado atual é aceito sem mudar nada (transições idempotentes).
    pub fn can_transition(self, to: SessionState) -> bool {
        self == to || self.next_states().contains(&to)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionTransition {
    pub from: SessionState,
    pub to: SessionState,
    pub accepted: bool,
}

impl SessionTransition {
    /// Aceito sem mudar nada: quem chama não repete efeitos (restaurar duas vezes não restaura de novo).
    pub fn is_no_op(&self) -> bool {
        self.accepted && self.from == self.to
    }

    pub fn changed(&self) -> bool {
        self.accepted && self.from != self.to
    }
}

/// Só as transições da tabela são aceitas; uma inválida não muda o estado (e nada é alterado no SO).
#[derive(Debug, Default)]
pub struct SessionStateMachine {
    state: SessionState,
}

impl SessionStateMachine {
    pub fn new(initial: SessionState) -> Self {
        SessionStateMachine { state: initial }
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn try_transition(&mut self, to: SessionState) -> SessionTransition {
        let from = self.state;
        let accepted = from.can_transition(to);
        if accepted {
            self.state = to;
        } else {
            tracing::warn!("transição de sessão recusada: {from:?} → {to:?}");
        }
        SessionTransition { from, to, accepted }
    }
}

#[cfg(test)]
mod tests {
    use super::SessionState::*;
    use super::*;

    // Tabela esperada (PRD §5.2): mudar a máquina exige mudar este teste.
    const VALID: [(SessionState, SessionState); 14] = [
        (Browsing, Launching),
        (Launching, Playing),
        (Launching, Browsing),
        (Launching, Error),
        (Playing, Returning),
        (Playing, Error),
        (Returning, Restoring),
        (Returning, Error),
        (Restoring, Browsing),
        (Restoring, Error),
        (Error, Recovery),
        (Recovery, Browsing),
        (Recovery, Desktop),
        (Desktop, Browsing),
    ];

    #[test]
    fn every_pair_of_states_follows_the_table() {
        for from in SessionState::ALL {
            for to in SessionState::ALL {
                let mut m = SessionStateMachine::new(from);
                let r = m.try_transition(to);
                let valid = VALID.contains(&(from, to));
                if from == to {
                    assert!(r.is_no_op() && !r.changed());
                } else if valid {
                    assert!(r.accepted && r.changed());
                    assert_eq!(m.state(), to);
                } else {
                    assert!(!r.accepted, "{from:?} → {to:?}");
                    assert_eq!(m.state(), from);
                }
                assert_eq!(from.can_transition(to), from == to || valid);
            }
        }
    }

    #[test]
    fn next_states_list_exactly_the_valid_transitions() {
        for from in SessionState::ALL {
            let expected: Vec<_> = VALID.iter().filter(|v| v.0 == from).map(|v| v.1).collect();
            assert_eq!(from.next_states(), expected.as_slice());
        }
    }

    #[test]
    fn a_full_game_session_goes_around_and_back_to_browsing() {
        let mut m = SessionStateMachine::default();
        for s in [Launching, Playing, Returning, Restoring, Browsing] {
            assert!(m.try_transition(s).changed());
        }
        assert_eq!(m.state(), Browsing);
    }

    #[test]
    fn cancelling_or_failing_to_open_returns_to_browsing() {
        let mut m = SessionStateMachine::new(Browsing);
        m.try_transition(Launching);
        assert!(m.try_transition(Browsing).accepted);
    }

    #[test]
    fn ending_a_game_twice_does_not_restore_twice() {
        let mut m = SessionStateMachine::new(Playing);
        assert!(m.try_transition(Returning).changed());
        assert!(m.try_transition(Returning).is_no_op());
        assert!(!m.try_transition(Playing).accepted);
        assert_eq!(m.state(), Returning);
    }

    #[test]
    fn errors_recover_to_browsing_or_the_desktop() {
        let mut m = SessionStateMachine::new(Playing);
        for s in [Error, Recovery, Desktop, Browsing] {
            assert!(m.try_transition(s).changed());
        }
    }

    #[test]
    fn starts_browsing_by_default() {
        assert_eq!(SessionStateMachine::default().state(), Browsing);
    }
}
