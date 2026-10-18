//! Geometria da barra única. Coordenadas lógicas, iguais às usadas pelo Slint.
#[derive(Debug, PartialEq)]
pub(crate) enum Hit {
    Client,
    Caption,
    Maximize,
    Close,
}

pub(crate) fn hit(x: f64, y: f64, width: f64, drag: (f64, f64)) -> Hit {
    if x < 0. || x >= width || !(0. ..64.).contains(&y) {
        Hit::Client
    } else if x >= width - 46. {
        Hit::Close
    } else if x >= width - 92. {
        Hit::Maximize
    } else if x >= drag.0 && x < drag.1 {
        Hit::Caption
    } else {
        Hit::Client
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod native {
    use super::{Hit, hit};
    use slint::winit_030::{
        WinitWindowAccessor,
        winit::raw_window_handle::{HasWindowHandle, RawWindowHandle},
    };
    use slint::{ComponentHandle, Weak};
    use slint_interpreter::{ComponentInstance, Value};
    use windows::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
        Graphics::{
            Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute},
            Gdi::ScreenToClient,
        },
        UI::{
            Input::KeyboardAndMouse::{TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TrackMouseEvent},
            Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::*,
        },
    };
    const ID: usize = 0x54414b4f;

    // A instalação e todos os callbacks acontecem na thread da janela. O weak não prolonga a vida do app.
    pub(crate) fn install(ui: &ComponentInstance) {
        ui.window().with_winit_window(|w| {
            let Ok(handle) = w.window_handle() else {
                return;
            };
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return;
            };
            let hwnd = HWND(handle.hwnd.get() as *mut _);
            unsafe {
                let mut existing = 0;
                if GetWindowSubclass(hwnd, Some(proc), ID, Some(&mut existing)).as_bool() {
                    return;
                }
                // O DWM aplica os cantos de 8 e os remove ao maximizar; Windows antigos ignoram o atributo.
                let corner = DWMWCP_ROUND;
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    std::ptr::from_ref(&corner).cast(),
                    std::mem::size_of_val(&corner) as u32,
                );
                let state = Box::into_raw(Box::new(ui.as_weak()));
                if !SetWindowSubclass(hwnd, Some(proc), ID, state as usize).as_bool() {
                    drop(Box::from_raw(state));
                    tracing::warn!("não foi possível instalar o hit-test da barra desktop");
                }
            }
        });
    }

    unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
        _: usize,
        data: usize,
    ) -> LRESULT {
        // SAFETY: `data` pertence a esta subclasse, instalada na thread da janela; liberado só em NCDESTROY.
        unsafe {
            if msg == WM_NCDESTROY {
                let _ = RemoveWindowSubclass(hwnd, Some(proc), ID);
                drop(Box::from_raw(data as *mut Weak<ComponentInstance>));
                return DefSubclassProc(hwnd, msg, wp, lp);
            }
            let weak = &*(data as *const Weak<ComponentInstance>);
            if let Some(ui) = weak.upgrade() {
                if msg == WM_NCHITTEST {
                    let mut p = POINT {
                        x: lp.0 as i16 as i32,
                        y: (lp.0 >> 16) as i16 as i32,
                    };
                    let mut rect = RECT::default();
                    if ScreenToClient(hwnd, &mut p).as_bool()
                        && GetClientRect(hwnd, &mut rect).is_ok()
                    {
                        let scale = f64::from(ui.window().scale_factor());
                        let number = |name| match ui.get_property(name) {
                            Ok(Value::Number(n)) => n,
                            _ => 0.,
                        };
                        let target = hit(
                            f64::from(p.x) / scale,
                            f64::from(p.y) / scale,
                            f64::from(rect.right) / scale,
                            (number("window-drag-start"), number("window-drag-end")),
                        );
                        let code = match target {
                            Hit::Caption => HTCAPTION,
                            Hit::Maximize => HTMAXBUTTON,
                            Hit::Close => HTCLOSE,
                            Hit::Client => return DefSubclassProc(hwnd, msg, wp, lp),
                        };
                        return LRESULT(code as isize);
                    }
                }
                if msg == WM_NCLBUTTONDOWN && matches!(wp.0 as u32, HTMAXBUTTON | HTCLOSE) {
                    let pressed = if wp.0 as u32 == HTMAXBUTTON { 1. } else { 2. };
                    let _ = ui.set_property("window-native-pressed", Value::Number(pressed));
                    // DefWindowProc acompanha o clique até soltar/cancelar, preservando o comando nativo.
                    let result = DefSubclassProc(hwnd, msg, wp, lp);
                    let _ = ui.set_property("window-native-pressed", Value::Number(0.));
                    return result;
                }
                if msg == WM_NCMOUSEMOVE {
                    let hover = match wp.0 as u32 {
                        HTMAXBUTTON => 1,
                        HTCLOSE => 2,
                        _ => 0,
                    };
                    let _ = ui.set_property("window-native-hover", Value::Number(f64::from(hover)));
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE | TME_NONCLIENT,
                        hwndTrack: hwnd,
                        ..Default::default()
                    };
                    let _ = TrackMouseEvent(&mut track);
                }
                if msg == WM_NCMOUSELEAVE || msg == WM_MOUSEMOVE {
                    let _ = ui.set_property("window-native-hover", Value::Number(0.));
                }
            }
            // O Windows cuida de duplo clique, menu de sistema, snap e comandos dos botões não cliente.
            DefSubclassProc(hwnd, msg, wp, lp)
        }
    }
}

#[cfg(windows)]
pub(crate) use native::install;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hit_test_keeps_controls_and_tabs_out_of_the_caption() {
        for width in [960., 1600.] {
            let drag = (440., width - 300.);
            assert_eq!(hit(36., 20., width, drag), Hit::Client);
            assert_eq!(hit(439., 20., width, drag), Hit::Client);
            assert_eq!(hit(440., 20., width, drag), Hit::Caption);
            assert_eq!(hit(drag.1, 20., width, drag), Hit::Client);
            assert_eq!(hit(width - 100., 20., width, drag), Hit::Client);
            assert_eq!(hit(width - 92., 0., width, drag), Hit::Maximize);
            assert_eq!(hit(width - 46., 63., width, drag), Hit::Close);
            assert_eq!(hit(width - 1., 0., width, drag), Hit::Close);
            assert_eq!(hit(width, 0., width, drag), Hit::Client);
            assert_eq!(hit(500., 64., width, drag), Hit::Client);
            assert_eq!(hit(-1., 20., width, drag), Hit::Client);
        }
    }
}
