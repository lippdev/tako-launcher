//! Geometria da barra única. Coordenadas lógicas, iguais às usadas pelo Slint.
#[derive(Debug, PartialEq)]
pub(crate) enum Hit {
    Client,
    Caption,
    Minimize,
    Maximize,
    Close,
}

/// Altura da barra e largura dos botões (`topbar-height` e `window-control-width`). Os três terminam a 6 da
/// borda; o último estende o alvo até ela, como nos cantos do Windows.
const BAR: f64 = 52.;
const BUTTON: f64 = 40.;
const EDGE: f64 = 6.;

pub(crate) fn hit(x: f64, y: f64, width: f64, drag: (f64, f64)) -> Hit {
    if x < 0. || x >= width || !(0. ..BAR).contains(&y) {
        Hit::Client
    } else if x >= width - BUTTON - EDGE {
        Hit::Close
    } else if x >= width - 2. * BUTTON - EDGE {
        Hit::Maximize
    } else if x >= width - 3. * BUTTON - EDGE {
        Hit::Minimize
    } else if x >= drag.0 && x < drag.1 {
        Hit::Caption
    } else {
        Hit::Client
    }
}

#[cfg(windows)]
mod native {
    use super::{Hit, hit};
    use slint::winit_030::{
        WinitWindowAccessor,
        winit::raw_window_handle::{HasWindowHandle, RawWindowHandle},
    };
    use slint::{ComponentHandle, Weak};
    use slint_interpreter::{ComponentInstance, Value};
    use tako_platform::windows::window_chrome::{self, ChromeHit, ChromeHost};

    // O weak não prolonga a vida do app; os callbacks chegam na thread da janela.
    struct Host(Weak<ComponentInstance>);

    impl ChromeHost for Host {
        fn hit(&self, x: i32, y: i32, width: i32) -> ChromeHit {
            let Some(ui) = self.0.upgrade() else {
                return ChromeHit::Client;
            };
            let scale = f64::from(ui.window().scale_factor());
            let number = |name| match ui.get_property(name) {
                Ok(Value::Number(n)) => n,
                _ => 0.,
            };
            match hit(
                f64::from(x) / scale,
                f64::from(y) / scale,
                f64::from(width) / scale,
                (number("window-drag-start"), number("window-drag-end")),
            ) {
                Hit::Caption => ChromeHit::Caption,
                Hit::Minimize => ChromeHit::Minimize,
                Hit::Maximize => ChromeHit::Maximize,
                Hit::Close => ChromeHit::Close,
                Hit::Client => ChromeHit::Client,
            }
        }
        fn set_hover(&self, button: u8) {
            if let Some(ui) = self.0.upgrade() {
                let _ = ui.set_property("window-native-hover", Value::Number(f64::from(button)));
            }
        }
        fn set_pressed(&self, button: u8) {
            if let Some(ui) = self.0.upgrade() {
                let _ = ui.set_property("window-native-pressed", Value::Number(f64::from(button)));
            }
        }
        fn activate(&self, button: u8) {
            let action = match button {
                1 => "max",
                2 => "close",
                3 => "min",
                _ => return,
            };
            // Fora do procedimento da janela: a ação mexe na própria janela (maximizar, fechar).
            let ui = self.0.clone();
            slint::Timer::single_shot(std::time::Duration::ZERO, move || {
                if let Some(ui) = ui.upgrade() {
                    let _ = ui.invoke("window-action", &[Value::String(action.into())]);
                }
            });
        }
    }

    pub(crate) fn install(ui: &ComponentInstance) {
        ui.window().with_winit_window(|w| {
            let Ok(handle) = w.window_handle() else {
                return;
            };
            let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                return;
            };
            window_chrome::install(handle.hwnd.get(), Box::new(Host(ui.as_weak())));
        });
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
            assert_eq!(hit(width - 127., 20., width, drag), Hit::Client);
            assert_eq!(hit(width - 126., 20., width, drag), Hit::Minimize);
            assert_eq!(hit(width - 87., 20., width, drag), Hit::Minimize);
            assert_eq!(hit(width - 86., 0., width, drag), Hit::Maximize);
            assert_eq!(hit(width - 46., 51., width, drag), Hit::Close);
            assert_eq!(hit(width - 1., 0., width, drag), Hit::Close);
            assert_eq!(hit(width, 0., width, drag), Hit::Client);
            assert_eq!(hit(500., 52., width, drag), Hit::Client);
            assert_eq!(hit(-1., 20., width, drag), Hit::Client);
        }
    }
}
