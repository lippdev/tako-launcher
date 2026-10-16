//! Cursor no macOS: esconder com `CGDisplayHideCursor` e mover pelo controle com `CGEventPost`
//! (exige a permissão de Acessibilidade).

use crate::unix::run;
use objc2_core_graphics::{
    CGDisplayHideCursor, CGDisplayShowCursor, CGEvent, CGEventTapLocation, CGEventType,
    CGMainDisplayID, CGMouseButton, CGPreflightPostEventAccess, CGScrollEventUnit,
};
use std::path::PathBuf;
use std::sync::Mutex;
use tako_core::devices::{CursorControl, DeviceError, DeviceResult, MouseButton, PointerInjector};
use tako_core::i18n::t;

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct MacCursor {
    marker: PathBuf,
    hidden: Mutex<bool>,
}

impl MacCursor {
    pub fn new(marker: PathBuf) -> Self {
        MacCursor {
            marker,
            hidden: Mutex::new(false),
        }
    }
}

impl CursorControl for MacCursor {
    fn set_hidden(&self, hidden: bool) -> DeviceResult<()> {
        let mut h = lock(&self.hidden);
        if *h == hidden {
            return Ok(());
        }
        // Esconder é contado por processo: um show para cada hide.
        let d = CGMainDisplayID();
        let e = if hidden {
            CGDisplayHideCursor(d)
        } else {
            CGDisplayShowCursor(d)
        };
        if e.0 != 0 {
            return Err(DeviceError::Failed(format!("cursor: CGError {}", e.0)));
        }
        run::set_marker(&self.marker, hidden);
        *h = hidden;
        Ok(())
    }

    fn is_hidden(&self) -> bool {
        *lock(&self.hidden)
    }

    fn recover_after_crash(&self) {
        // O WindowServer mostra o cursor de novo quando o processo que o escondeu morre.
        if self.marker.exists() {
            run::set_marker(&self.marker, false);
        }
    }

    fn limitation(&self) -> Option<String> {
        Some(t("platform.unix.cursor_macos"))
    }
}

/// Lembra os botões apertados: com um botão segurado, mover vira "arrastar".
#[derive(Default)]
pub struct MacPointer(Mutex<[bool; 3]>);

fn post(event: Option<objc2_core_foundation::CFRetained<CGEvent>>) -> DeviceResult<()> {
    let e = event.ok_or_else(|| DeviceError::Failed("CGEvent".into()))?;
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&e));
    Ok(())
}

fn location() -> objc2_core_foundation::CGPoint {
    CGEvent::location(CGEvent::new(None).as_deref())
}

impl PointerInjector for MacPointer {
    fn move_by(&self, dx: i32, dy: i32) -> DeviceResult<()> {
        let mut p = location();
        p.x += dx as f64;
        p.y += dy as f64;
        let held = *lock(&self.0);
        let (kind, button) = match held {
            [true, ..] => (CGEventType::LeftMouseDragged, CGMouseButton::Left),
            [_, true, _] => (CGEventType::RightMouseDragged, CGMouseButton::Right),
            [.., true] => (CGEventType::OtherMouseDragged, CGMouseButton::Center),
            _ => (CGEventType::MouseMoved, CGMouseButton::Left),
        };
        post(CGEvent::new_mouse_event(None, kind, p, button))
    }

    fn button(&self, button: MouseButton, down: bool) -> DeviceResult<()> {
        let (i, b, kind) = match (button, down) {
            (MouseButton::Left, true) => (0, CGMouseButton::Left, CGEventType::LeftMouseDown),
            (MouseButton::Left, false) => (0, CGMouseButton::Left, CGEventType::LeftMouseUp),
            (MouseButton::Right, true) => (1, CGMouseButton::Right, CGEventType::RightMouseDown),
            (MouseButton::Right, false) => (1, CGMouseButton::Right, CGEventType::RightMouseUp),
            (MouseButton::Middle, true) => (2, CGMouseButton::Center, CGEventType::OtherMouseDown),
            (MouseButton::Middle, false) => (2, CGMouseButton::Center, CGEventType::OtherMouseUp),
        };
        lock(&self.0)[i] = down;
        post(CGEvent::new_mouse_event(None, kind, location(), b))
    }

    fn wheel(&self, vertical: i32, horizontal: i32) -> DeviceResult<()> {
        // No CoreGraphics o eixo 2 positivo rola para a esquerda (a validar em hardware).
        post(CGEvent::new_scroll_wheel_event2(
            None,
            CGScrollEventUnit::Line,
            2,
            vertical,
            -horizontal,
            0,
        ))
    }

    fn check_access(&self) -> DeviceResult<()> {
        if CGPreflightPostEventAccess() {
            Ok(())
        } else {
            Err(DeviceError::PermissionDenied(t(
                "platform.unix.accessibility_denied",
            )))
        }
    }
}
