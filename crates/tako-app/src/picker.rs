//! Controle dos seletores de arquivos e pastas do sistema operacional (WEB-12e).
//!
//! A chamada `system.pick` responde na hora com um id e despacha um `HostRequest::Pick`.
//! Quando o hospedeiro encerra o diálogo, chama `AppService::deliver_picked` para emitir
//! `system.picked`. Pedidos desconhecidos ou já atendidos são ignorados para segurança.

use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct PickerState {
    next_id: u32,
    pending: HashSet<u32>,
}

impl PickerState {
    pub(crate) fn next_id(&mut self) -> u32 {
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1;
        }
        self.next_id
    }

    pub(crate) fn register(&mut self, id: u32) {
        self.pending.insert(id);
    }

    pub(crate) fn complete(&mut self, id: u32) -> bool {
        self.pending.remove(&id)
    }

    #[cfg(test)]
    pub(crate) fn is_pending(&self, id: u32) -> bool {
        self.pending.contains(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_state_registers_and_completes_ids() {
        let mut state = PickerState::default();
        let id1 = state.next_id();
        assert_eq!(id1, 1);
        state.register(id1);
        assert!(state.is_pending(id1));

        let id2 = state.next_id();
        assert_eq!(id2, 2);
        state.register(id2);

        assert!(state.complete(id1));
        assert!(!state.is_pending(id1));
        assert!(!state.complete(id1), "já foi atendido");
        assert!(state.is_pending(id2));
        assert!(state.complete(id2));
    }

    #[test]
    fn picker_state_skips_zero_on_overflow() {
        let mut state = PickerState {
            next_id: u32::MAX,
            pending: HashSet::new(),
        };
        let id = state.next_id();
        assert_eq!(id, 1);
    }
}
