//! Cursor: esconder durante a sessão e mover pelo controle (PRD D-03).
//! A matemática do analógico (aceleração, cliques, rolagem) fica no core (`input::controller_mouse`);
//! aqui só ficam as operações do SO.

use super::DeviceResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub trait CursorControl: Send + Sync {
    /// Esconde ou mostra o cursor do sistema. Deve sobreviver a uma queda: a implementação grava um
    /// marcador e [`CursorControl::recover_after_crash`] desfaz na próxima execução.
    fn set_hidden(&self, hidden: bool) -> DeviceResult<()>;
    fn is_hidden(&self) -> bool;
    /// Mostra o cursor se uma execução anterior caiu com ele escondido.
    fn recover_after_crash(&self);
    /// Limitação conhecida, para a UI explicar (ex.: "só sobre a janela do Tako no Wayland").
    fn limitation(&self) -> Option<String> {
        None
    }
}

pub trait PointerInjector: Send + Sync {
    fn move_by(&self, dx: i32, dy: i32) -> DeviceResult<()>;
    fn button(&self, button: MouseButton, down: bool) -> DeviceResult<()>;
    /// Rolagem em "cliques" (positivo = para cima / direita).
    fn wheel(&self, vertical: i32, horizontal: i32) -> DeviceResult<()>;
    /// Falta de permissão é o caso comum (uinput, Acessibilidade): a UI orienta.
    fn check_access(&self) -> DeviceResult<()> {
        Ok(())
    }
}
