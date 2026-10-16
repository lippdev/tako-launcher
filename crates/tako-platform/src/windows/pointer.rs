//! Cursor e mouse pelo controle: port de `SessionTools/WindowsPointer.cs`.
//!
//! O Windows não tem "esconder o ponteiro em todos os apps": os cursores do sistema são trocados por um vazio
//! (`SetSystemCursor`) e recarregados do esquema do usuário depois (`SPI_SETCURSORS`). O marcador
//! `<data_root>/cursor-hidden` cobre uma queda no meio.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use tako_core::devices::{CursorControl, DeviceError, DeviceResult, MouseButton, PointerInjector};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::PCWSTR;

// OCR_NORMAL, IBEAM, WAIT, CROSS, UP, SIZENWSE, SIZENESW, SIZEWE, SIZENS, SIZEALL, NO, HAND, APPSTARTING, HELP, PIN, PERSON.
const SYSTEM_CURSORS: [u32; 16] = [
    32512, 32513, 32514, 32515, 32516, 32642, 32643, 32644, 32645, 32646, 32648, 32649, 32650,
    32651, 32671, 32672,
];

pub struct WindowsPointer {
    marker: PathBuf,
    hidden: Mutex<bool>,
}

impl WindowsPointer {
    pub fn new(data_root: &std::path::Path) -> Self {
        WindowsPointer {
            marker: data_root.join("cursor-hidden"),
            hidden: Mutex::new(false),
        }
    }

    fn write_marker(&self, hidden: bool) {
        let r = if hidden {
            self.marker
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(&self.marker, b""))
        } else {
            match std::fs::remove_file(&self.marker) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                r => r,
            }
        };
        if let Err(e) = r {
            tracing::warn!("marcador do ponteiro escondido não pôde ser gravado: {e}");
        }
    }
}

fn hide_cursors() -> DeviceResult<()> {
    // SAFETY: só lê métricas do sistema.
    let (w, h) = unsafe {
        (
            GetSystemMetrics(SM_CXCURSOR).max(32),
            GetSystemMetrics(SM_CYCURSOR).max(32),
        )
    };
    // Máscara AND toda 1 mantém a tela; XOR toda 0 não acrescenta nada. Linhas alinhadas em WORD.
    let size = ((w + 15) / 16 * 2 * h) as usize;
    let and = vec![0xFFu8; size];
    let xor = vec![0u8; size];
    // SAFETY: as máscaras têm o tamanho que `CreateCursor` lê para `w`×`h` monocromático.
    let blank = unsafe {
        let module = GetModuleHandleW(PCWSTR::null()).ok();
        CreateCursor(
            module.map(Into::into),
            0,
            0,
            w,
            h,
            and.as_ptr().cast(),
            xor.as_ptr().cast(),
        )
    }
    .map_err(|e| DeviceError::Failed(format!("CreateCursor falhou ({e})")))?;
    for id in SYSTEM_CURSORS {
        // SAFETY: `SetSystemCursor` fica dono de cada cópia; o original é destruído no fim.
        unsafe {
            if let Ok(copy) = CopyIcon(HICON(blank.0)) {
                let _ = SetSystemCursor(HCURSOR(copy.0), SYSTEM_CURSOR_ID(id));
            }
        }
    }
    // SAFETY: o cursor original não foi entregue a ninguém.
    let _ = unsafe { DestroyCursor(blank) };
    Ok(())
}

fn restore_cursors() {
    // Recarrega o esquema do próprio usuário (Configurações › Ponteiro do mouse), seja qual for.
    // SAFETY: SPI_SETCURSORS não usa parâmetros.
    if let Err(e) = unsafe {
        SystemParametersInfoW(
            SPI_SETCURSORS,
            0,
            None,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    } {
        tracing::warn!("cursores do sistema: {e}");
    }
}

impl CursorControl for WindowsPointer {
    fn set_hidden(&self, hidden: bool) -> DeviceResult<()> {
        let mut state = self.hidden.lock().unwrap_or_else(PoisonError::into_inner);
        if *state == hidden {
            return Ok(());
        }
        if hidden {
            self.write_marker(true);
            if let Err(e) = hide_cursors() {
                self.write_marker(false);
                return Err(e);
            }
        } else {
            restore_cursors();
            self.write_marker(false);
        }
        *state = hidden;
        tracing::info!("ponteiro escondido: {hidden}");
        Ok(())
    }

    fn is_hidden(&self) -> bool {
        *self.hidden.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn recover_after_crash(&self) {
        if self.marker.exists() {
            restore_cursors();
            self.write_marker(false);
            tracing::info!("ponteiro restaurado depois de um encerramento inesperado");
        }
    }
}

fn send(flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32, data: i32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data as u32,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    // Falha em silêncio contra janela de administrador (UIPI): nada a fazer daqui, e a 125 Hz o log encheria.
    // SAFETY: um INPUT de mouse completo, com o tamanho da estrutura.
    let _ = unsafe { SendInput(&[input], size_of::<INPUT>() as i32) };
}

impl PointerInjector for WindowsPointer {
    fn move_by(&self, dx: i32, dy: i32) -> DeviceResult<()> {
        send(MOUSEEVENTF_MOVE, dx, dy, 0);
        Ok(())
    }

    fn button(&self, button: MouseButton, down: bool) -> DeviceResult<()> {
        let flags = match (button, down) {
            (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
            (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
            (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
            (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
            (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
            (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
        };
        send(flags, 0, 0, 0);
        Ok(())
    }

    fn wheel(&self, vertical: i32, horizontal: i32) -> DeviceResult<()> {
        let delta = WHEEL_DELTA as i32;
        if vertical != 0 {
            send(MOUSEEVENTF_WHEEL, 0, 0, vertical.saturating_mul(delta));
        }
        if horizontal != 0 {
            send(MOUSEEVENTF_HWHEEL, 0, 0, horizontal.saturating_mul(delta));
        }
        Ok(())
    }
}
