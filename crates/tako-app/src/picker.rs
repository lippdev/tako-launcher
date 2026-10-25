//! Controle dos seletores de arquivos e pastas do sistema operacional (WEB-12e).
//!
//! A chamada `system.pick` responde na hora com um id e despacha um `HostRequest::Pick`.
//! Quando o hospedeiro encerra o diálogo, chama `AppService::deliver_picked` para emitir
//! `system.picked`. Pedidos desconhecidos ou já atendidos são ignorados para segurança.

#[derive(Default)]
pub(crate) struct PickerState {
    next_id: u32,
    pending: Option<u32>,
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
        self.pending = Some(id);
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    pub(crate) fn complete(&mut self, id: u32) -> bool {
        if self.pending == Some(id) {
            self.pending = None;
            true
        } else {
            false
        }
    }

    #[cfg(test)]
    pub(crate) fn is_pending(&self, id: u32) -> bool {
        self.pending == Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_rejects_overlap_and_releases_only_the_matching_request() {
        use crate::{AppService, HostRequest, protocol::*};
        let mut app = AppService::new("test");
        let (tx, rx) = std::sync::mpsc::channel::<HostRequest>();
        app.set_host_sink(tx);
        let request = || {
            Call::SystemPick(SystemPickRequest {
                kind: PickKind::File,
                purpose: "test".into(),
                filters: None,
            })
        };
        let Reply::SystemPick(first) = app.handle(request()).unwrap() else {
            panic!("seletor")
        };
        rx.try_recv().unwrap();
        assert_eq!(app.handle(request()).unwrap_err().code, ErrorCode::Conflict);
        assert!(rx.try_recv().is_err());
        assert!(!app.deliver_picked(first.id + 1, None));
        assert_eq!(app.handle(request()).unwrap_err().code, ErrorCode::Conflict);
        assert!(app.deliver_picked(first.id, None));
        assert!(app.handle(request()).is_ok());
    }

    #[test]
    fn failed_host_dispatch_does_not_reserve_picker() {
        use crate::{AppService, protocol::*};
        let mut app = AppService::new("test");
        let request = || {
            Call::SystemPick(SystemPickRequest {
                kind: PickKind::Folder,
                purpose: "test".into(),
                filters: None,
            })
        };
        assert_eq!(app.handle(request()).unwrap_err().code, ErrorCode::Internal);
        assert!(!app.deliver_picked(1, None));
        let (tx, _rx) = std::sync::mpsc::channel();
        app.set_host_sink(tx);
        assert!(app.handle(request()).is_ok());
    }

    #[test]
    fn picker_state_registers_and_completes_ids() {
        let mut state = PickerState::default();
        let id1 = state.next_id();
        assert_eq!(id1, 1);
        state.register(id1);
        assert!(state.is_pending(id1));

        let id2 = state.next_id();
        assert_eq!(id2, 2);

        assert!(state.complete(id1));
        assert!(!state.is_pending(id1));
        state.register(id2);
        assert!(!state.complete(id1), "já foi atendido");
        assert!(state.is_pending(id2));
        assert!(state.complete(id2));
    }

    #[test]
    fn picker_state_skips_zero_on_overflow() {
        let mut state = PickerState {
            next_id: u32::MAX,
            pending: None,
        };
        let id = state.next_id();
        assert_eq!(id, 1);
    }
}
