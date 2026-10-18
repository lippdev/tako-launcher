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
            for scale in [1.0, 1.25] {
                ui.set_global_property("Ui", "text-scale", Value::Number(scale))
                    .unwrap();
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

#[test]
fn desktop_journey_renders_selection_search_tabs_actions_and_controller_question() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        use crate::mode::{ASK_SETTING, UiMode};
        use crate::shell::{Effect, Settings};
        use std::time::Duration;
        use tako_core::persistence::{SettingsStore, TakoDatabase};

        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_desktop(&builtin)
            .unwrap()
            .create()
            .unwrap();
        window.set_size(slint::PhysicalSize::new(1280, 800));
        ui.show().unwrap();
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library.refresh();
        let store = SettingsStore::new(TakoDatabase::new(dir.path().join("question.db")));
        ctx.settings = Settings::new(Some(store.clone()));
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        let draw = |shell: &mut Shell| {
            let out = shell.render(Instant::now());
            for (name, value) in &out.props {
                crate::bridge::set(&ui, name, value.clone());
            }
            let mut pixels = vec![Rgb8Pixel::default(); 1280 * 800];
            ui.window().request_redraw();
            assert!(window.draw_if_needed(|r| {
                r.render(&mut pixels, 1280);
            }));
            assert!(pixels.iter().any(|p| p.r != 0 || p.g != 0 || p.b != 0));
            out
        };
        for lang in [Lang::PtBr, Lang::EnUs] {
            i18n::set_lang(lang);
            shell.click("tab", 0);
            shell.click("game", 2);
            let selected = shell.focused_game().unwrap();
            assert_eq!(number(&draw(&mut shell), "desk-selected"), 2);
            let favorite = shell.ctx.library.find(&selected).unwrap().is_favorite;
            shell.click("action", 1);
            assert_ne!(
                shell.ctx.library.find(&selected).unwrap().is_favorite,
                favorite
            );
            shell.click("action", 0);
            assert!(
                shell
                    .ctx
                    .effects
                    .contains(&Effect::Launch(selected.clone()))
            );
            shell.ctx.effects.clear();
            shell.click("action", 2);
            assert!(shell.has_layers());
            draw(&mut shell);
            shell.close_layers();
            let title = shell.ctx.library.find(&selected).unwrap().title.clone();
            shell.edited("search", &title);
            draw(&mut shell);
            assert!(
                shell
                    .desktop_library()
                    .unwrap()
                    .games(&shell.ctx)
                    .iter()
                    .all(|g| g.title.contains(&title))
            );
            shell.edited("search", "sem resultado 987654321");
            assert_eq!(number(&draw(&mut shell), "desk-selected"), -1);
            shell.edited("search", "");
            shell.click("tab", 1);
            assert_eq!(number(&draw(&mut shell), "desk-tab"), 1);
            assert!(
                shell
                    .desktop_library()
                    .unwrap()
                    .games(&shell.ctx)
                    .iter()
                    .all(|g| g.is_favorite)
            );
            shell.click("tab", 0);
            shell
                .desktop_library()
                .unwrap()
                .criteria
                .borrow_mut()
                .favorites_only = true;
            assert_eq!(
                property(&draw(&mut shell), "desk-filtered"),
                &Value::Bool(true)
            );
            assert!(
                shell
                    .desktop_library()
                    .unwrap()
                    .games(&shell.ctx)
                    .iter()
                    .all(|g| g.is_favorite)
            );
            shell
                .desktop_library()
                .unwrap()
                .criteria
                .borrow_mut()
                .favorites_only = false;
            shell.click("tab", 2);
            if shell.ctx.collections.collections().is_empty() {
                shell
                    .ctx
                    .collections
                    .create("Sofá", Some(&selected))
                    .unwrap();
            }
            shell.ctx.sync_collections();
            shell.click("collection", 0);
            assert_eq!(shell.focused_game(), Some(selected));
            draw(&mut shell);
            shell.click("desk-nav", 1);
            // A rota também é usada pelos links externos.
            shell.open_route("settings");
            draw(&mut shell);
            shell.open_route(crate::shell::desktop::ROUTE);
        }

        shell.ask_big_picture = true;
        crate::shell::desktop::ask_big_picture(&mut shell, Duration::ZERO);
        assert!(!shell.has_layers(), "sem controle não pergunta");
        shell.ctx.pads_connected = 1;
        crate::shell::desktop::ask_big_picture(&mut shell, Duration::from_millis(100));
        assert!(shell.has_layers());
        draw(&mut shell);
        shell.click("overlay-row", 2);
        assert!(!shell.has_layers());
        assert_eq!(store.get(ASK_SETTING).unwrap().as_deref(), Some("false"));
        shell.ctx.settings = Settings::new(Some(store));
        shell.ask_big_picture = true;
        crate::shell::desktop::ask_big_picture(&mut shell, Duration::from_millis(200));
        assert!(
            !shell.has_layers(),
            "a preferência persiste após recarregar"
        );
        shell.ctx.settings.set_bool(ASK_SETTING, true);
        shell.ask_big_picture = true;
        crate::shell::desktop::ask_big_picture(&mut shell, Duration::ZERO);
        shell.click("overlay-row", 0);
        assert_eq!(
            shell.ctx.effects.pop(),
            Some(Effect::SwitchMode(UiMode::BigPicture))
        );
        shell.ask_big_picture = true;
        crate::shell::desktop::ask_big_picture(
            &mut shell,
            crate::mode::ASK_WINDOW + Duration::from_secs(1),
        );
        assert!(
            !shell.has_layers(),
            "conectar depois da abertura não pergunta"
        );
        shell.ask_big_picture = true;
        shell.enter_big_picture();
        crate::shell::desktop::ask_big_picture(&mut shell, Duration::ZERO);
        assert!(!shell.has_layers());
        assert!(!crate::mode::start(&["tako".into(), "--big-picture".into()]).ask);
        assert_eq!(
            crate::mode::start(&["tako".into(), "--big-picture".into()]).mode,
            UiMode::BigPicture
        );
        ui.hide().unwrap();
    })
    .join()
    .unwrap();
}

/// Captura da janela desktop para conferir com as prévias do design: `TAKO_SNAP_OUT=<pasta>` (obrigatório),
/// `TAKO_SNAP_ART=<imagem>` (arte do jogo em foco), `TAKO_SNAP_PAGE=settings` e `TAKO_SNAP_CLICK` (opcionais).
/// `cargo test -p tako-ui desktop_snapshot -- --ignored`.
#[test]
#[ignore = "ferramenta manual de captura"]
fn desktop_snapshot() {
    let Ok(out_dir) = std::env::var("TAKO_SNAP_OUT") else {
        return;
    };
    std::thread::spawn(move || {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_desktop(&builtin)
            .unwrap_or_else(|e| panic!("{e}"))
            .create()
            .unwrap();
        let to_image = |img: &image::RgbaImage| {
            slint::Image::from_rgba8(slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                img.as_raw(),
                img.width(),
                img.height(),
            ))
        };
        let art = std::env::var("TAKO_SNAP_ART").ok().map(|p| {
            let img = image::open(p).unwrap();
            let (blur, lum) = crate::images::glass_copy(&img);
            let lum: Vec<Value> = lum.into_iter().map(|l| Value::Number(l.into())).collect();
            (to_image(&img.to_rgba8()), to_image(&blur), crate::values::model(lum))
        });
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        i18n::set_lang(Lang::PtBr);
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        match std::env::var("TAKO_SNAP_PAGE").as_deref() {
            Ok("settings") => shell.open_route("settings"),
            Ok("first-run") => {
                let layer = crate::shell::first_run::welcome(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("onb-emu") => {
                let layer = crate::shell::first_run::emulators(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("onb-mode") => {
                let layer = crate::shell::first_run::mode(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("onb-ready") => {
                let layer = crate::shell::first_run::ready(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("ask") => {
                shell.ctx.pads_connected = 1;
                shell.ask_big_picture = true;
                crate::shell::desktop::ask_big_picture(&mut shell, std::time::Duration::ZERO);
            }
            _ => {}
        }
        // `TAKO_SNAP_CLICK=zona:índice,…`: cliques antes da captura (ex.: `desk-brand:0`, `game:3`).
        for click in std::env::var("TAKO_SNAP_CLICK").unwrap_or_default().split(',') {
            if let Some((zone, i)) = click.split_once(':') {
                shell.click(zone, i.parse().unwrap_or(0));
            }
        }
        ui.show().unwrap();
        for (w, h) in [(1600u32, 900u32), (960, 600)] {
            window.set_size(slint::PhysicalSize::new(w, h));
            let out = shell.render(Instant::now());
            for (name, value) in &out.props {
                let mut value = value.clone();
                if let (Some((art, blur, lum)), "desk-game", Value::Struct(s)) =
                    (&art, *name, &mut value)
                {
                    s.set_field("art".into(), Value::Image(art.clone()));
                    s.set_field("art-blur".into(), Value::Image(blur.clone()));
                    s.set_field("art-lum".into(), lum.clone());
                }
                crate::bridge::set(&ui, name, value);
            }
            let mut pixels = vec![Rgb8Pixel::default(); (w * h) as usize];
            ui.window().request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut pixels, w as usize);
            });
            let raw: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            image::RgbImage::from_raw(w, h, raw)
                .unwrap()
                .save(format!("{out_dir}/desktop-{w}x{h}.png"))
                .unwrap();
        }
        ui.hide().unwrap();
    })
    .join()
    .unwrap();
}
