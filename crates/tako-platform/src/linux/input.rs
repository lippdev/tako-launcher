//! Cursor no Linux: esconder (XFixes no X11; no Wayland só sobre a janela do Tako) e mover pelo
//! controle com um mouse virtual no `/dev/uinput` (funciona em X11 e Wayland).

use crate::unix::run;
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, EventType, InputEvent, KeyCode, RelativeAxisCode};
use std::path::PathBuf;
use std::sync::Mutex;
use tako_core::devices::{CursorControl, DeviceError, DeviceResult, MouseButton, PointerInjector};
use tako_core::i18n::t;
use x11rb::connection::Connection;
use x11rb::protocol::xfixes::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub struct LinuxCursor {
    marker: PathBuf,
    x11: bool,
    /// No X11 o cursor fica escondido enquanto esta conexão vive; se o Tako cair, o servidor X
    /// fecha a conexão e o cursor volta sozinho.
    hidden_on: Mutex<Option<(RustConnection, u32)>>,
    hidden: Mutex<bool>,
}

impl LinuxCursor {
    pub fn new(marker: PathBuf, x11: bool) -> Self {
        LinuxCursor {
            marker,
            x11,
            hidden_on: Mutex::new(None),
            hidden: Mutex::new(false),
        }
    }

    fn hide_x11() -> Result<(RustConnection, u32), String> {
        let (conn, screen) = x11rb::connect(None).map_err(|e| e.to_string())?;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .ok_or("tela X inexistente")?
            .root;
        // O XFixes exige negociar a versão antes do primeiro pedido.
        conn.xfixes_query_version(4, 0)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?;
        conn.xfixes_hide_cursor(root)
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;
        Ok((conn, root))
    }
}

impl CursorControl for LinuxCursor {
    fn set_hidden(&self, hidden: bool) -> DeviceResult<()> {
        if *lock(&self.hidden) == hidden {
            return Ok(());
        }
        if self.x11 {
            let mut conn = lock(&self.hidden_on);
            if hidden {
                *conn = Some(
                    Self::hide_x11().map_err(|e| DeviceError::Failed(format!("XFixes: {e}")))?,
                );
            } else if let Some((c, root)) = conn.take() {
                let _ = c.xfixes_show_cursor(root);
                let _ = c.flush();
            }
        }
        run::set_marker(&self.marker, hidden);
        *lock(&self.hidden) = hidden;
        Ok(())
    }

    fn is_hidden(&self) -> bool {
        *lock(&self.hidden)
    }

    fn recover_after_crash(&self) {
        // XFixes desfaz o esconder quando a conexão cai; basta limpar o marcador.
        if self.marker.exists() {
            tracing::info!("cursor estava escondido numa execução anterior");
            run::set_marker(&self.marker, false);
        }
    }

    fn limitation(&self) -> Option<String> {
        (!self.x11).then(|| t("platform.unix.cursor_wayland"))
    }
}

/// Mouse virtual criado no primeiro uso.
#[derive(Default)]
pub struct UinputPointer(Mutex<Option<VirtualDevice>>);

const UINPUT: &str = "/dev/uinput";

fn denied() -> DeviceError {
    DeviceError::PermissionDenied(t("platform.unix.uinput_denied"))
}

impl UinputPointer {
    fn emit(&self, events: &[InputEvent]) -> DeviceResult<()> {
        let mut dev = lock(&self.0);
        if dev.is_none() {
            *dev = Some(create().map_err(|e| match e.kind() {
                std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotFound => denied(),
                _ => DeviceError::Failed(format!("uinput: {e}")),
            })?);
        }
        let Some(d) = dev.as_mut() else {
            return Err(denied());
        };
        // `emit` acrescenta o SYN_REPORT.
        d.emit(events).map_err(|e| {
            *dev = None;
            DeviceError::Failed(format!("uinput: {e}"))
        })
    }
}

fn create() -> std::io::Result<VirtualDevice> {
    let mut axes = AttributeSet::<RelativeAxisCode>::new();
    for a in [
        RelativeAxisCode::REL_X,
        RelativeAxisCode::REL_Y,
        RelativeAxisCode::REL_WHEEL,
        RelativeAxisCode::REL_HWHEEL,
    ] {
        axes.insert(a);
    }
    let mut keys = AttributeSet::<KeyCode>::new();
    for k in [KeyCode::BTN_LEFT, KeyCode::BTN_RIGHT, KeyCode::BTN_MIDDLE] {
        keys.insert(k);
    }
    VirtualDevice::builder()?
        .name("Tako virtual mouse")
        .with_relative_axes(&axes)?
        .with_keys(&keys)?
        .build()
}

fn rel(axis: RelativeAxisCode, v: i32) -> InputEvent {
    InputEvent::new(EventType::RELATIVE.0, axis.0, v)
}

impl PointerInjector for UinputPointer {
    fn move_by(&self, dx: i32, dy: i32) -> DeviceResult<()> {
        self.emit(&[
            rel(RelativeAxisCode::REL_X, dx),
            rel(RelativeAxisCode::REL_Y, dy),
        ])
    }

    fn button(&self, button: MouseButton, down: bool) -> DeviceResult<()> {
        let key = match button {
            MouseButton::Left => KeyCode::BTN_LEFT,
            MouseButton::Right => KeyCode::BTN_RIGHT,
            MouseButton::Middle => KeyCode::BTN_MIDDLE,
        };
        self.emit(&[InputEvent::new(EventType::KEY.0, key.0, down.into())])
    }

    fn wheel(&self, vertical: i32, horizontal: i32) -> DeviceResult<()> {
        // REL_WHEEL positivo = para cima; REL_HWHEEL positivo = para a direita (igual ao contrato).
        self.emit(&[
            rel(RelativeAxisCode::REL_WHEEL, vertical),
            rel(RelativeAxisCode::REL_HWHEEL, horizontal),
        ])
    }

    fn check_access(&self) -> DeviceResult<()> {
        if lock(&self.0).is_some() {
            return Ok(());
        }
        std::fs::OpenOptions::new()
            .write(true)
            .open(UINPUT)
            .map(drop)
            .map_err(|_| denied())
    }
}
