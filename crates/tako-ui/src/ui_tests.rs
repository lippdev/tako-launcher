//! Regressões das telas principais (PRD §8.4), sem janela nativa, loja, rede ou controle físico.

use crate::shell::{Cmd, Out, Shell};
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, PlatformError, WindowAdapter};
use slint::{ComponentHandle, Rgb8Pixel};
use slint_interpreter::Value;
use std::rc::Rc;
use std::time::Instant;
use tako_core::i18n::{self, Lang};
use tako_core::paths::DataPaths;

struct Headless(Rc<MinimalSoftwareWindow>);

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
    }
}

fn property<'a>(out: &'a Out, name: &str) -> &'a Value {
    &out.props
        .iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("sem {name}"))
        .1
}

fn number(out: &Out, name: &str) -> i32 {
    let Value::Number(n) = property(out, name) else {
        panic!("{name} não é número")
    };
    *n as i32
}

#[test]
fn controller_journey_restores_focus_after_details_search_and_menus() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for lang in [Lang::PtBr, Lang::EnUs] {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        i18n::set_lang(lang);
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.render(Instant::now());
        shell.handle(Cmd::Down);
        let id = shell.focused_game().unwrap();
        shell.handle(Cmd::Accept);
        assert_eq!(shell.page().kind(), "details");
        shell.handle(Cmd::Back);
        assert_eq!(shell.page().kind(), "home");
        assert_eq!(shell.focused_game().as_deref(), Some(id.as_str()));

        shell.open_route("library:all");
        shell.handle(Cmd::Right);
        let before = shell.focused_game().unwrap();
        let index = number(&shell.render(Instant::now()), "library-focus");
        shell.handle(Cmd::Menu);
        assert!(shell.has_layers());
        shell.handle(Cmd::Back);
        assert!(!shell.has_layers());
        assert_eq!(
            number(&shell.render(Instant::now()), "library-focus"),
            index
        );
        shell.handle(Cmd::Accept);
        assert_eq!(shell.page().kind(), "details");
        shell.handle(Cmd::Back);
        assert_eq!(shell.focused_game().as_deref(), Some(before.as_str()));
        shell.handle(Cmd::Search);
        assert!(shell.typing());
        shell.handle(Cmd::Back);
        assert!(!shell.typing());
        assert_eq!(shell.focused_game().as_deref(), Some(before.as_str()));

        shell.open_route("settings");
        for _ in 0..8 {
            shell.handle(Cmd::NextSection);
            shell.handle(Cmd::Accept);
            let out = shell.render(Instant::now());
            assert!(number(&out, "settings-row-focus") >= 0);
            shell.handle(Cmd::Back);
        }
        assert!(shell.ctx.effects.is_empty());
    }
}

#[test]
fn principal_screens_bind_and_render_in_the_slint_software_backend() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let theme = crate::theme::install_builtin(dir.path()).unwrap();
        let def = crate::theme::compile(&theme.join("main.slint"), &theme).unwrap();
        let ui = def.create().unwrap();
        ui.show().unwrap();
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        let game = shell.ctx.library.games()[0].id.clone();
        for lang in [Lang::PtBr, Lang::EnUs] {
            i18n::set_lang(lang);
            for (width, height) in [(1280, 720), (1920, 1080), (3840, 2160)] {
                window.set_size(slint::PhysicalSize::new(width, height));
                for route in ["home", "library:all", "settings", "details"] {
                    shell.open_route(if route == "details" {
                        "library:all"
                    } else {
                        route
                    });
                    if route == "details" {
                        shell.ctx.open_details = Some(game.clone());
                        shell.after_action();
                    }
                    for (name, value) in shell.render(Instant::now()).props {
                        ui.set_property(name, value)
                            .unwrap_or_else(|e| panic!("{route}/{name}: {e}"));
                    }
                    let mut pixels = vec![Rgb8Pixel::default(); (width * height) as usize];
                    ui.window().request_redraw();
                    assert!(window.draw_if_needed(|renderer| {
                        renderer.render(&mut pixels, width as usize);
                    }));
                    assert!(
                        pixels.iter().any(|p| p.r != 0 || p.g != 0 || p.b != 0),
                        "tela vazia: {route}/{width}/{lang:?}"
                    );
                }
            }
        }
        ui.hide().unwrap();
    })
    .join()
    .unwrap();
}

#[test]
fn solid_panel_text_meets_wcag_aa_contrast() {
    fn luminance(rgb: [u8; 3]) -> f64 {
        let c = rgb.map(|v| {
            let v = f64::from(v) / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        });
        c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
    }
    let tokens = include_str!("../theme/tokens.slint");
    for key in ["on-surface-solid", "on-surface-solid-muted"] {
        let value = tokens
            .lines()
            .find(|line| line.contains(&format!("{key}:")))
            .unwrap();
        let hex = value.split('#').nth(1).unwrap().trim_end_matches(';');
        let rgb = [0, 2, 4].map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap());
        assert!(
            1.05 / (luminance(rgb) + 0.05) >= 4.5,
            "contraste insuficiente em {key}"
        );
    }
}
