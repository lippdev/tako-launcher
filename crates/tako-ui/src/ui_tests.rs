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
    // Relógio das animações: o real, ou o da captura quadro a quadro (desenhar 1600 × 900 leva mais que um quadro).
    fn duration_since_start(&self) -> std::time::Duration {
        CLOCK.with(|c| c.get()).unwrap_or_else(|| START.elapsed())
    }
}

thread_local! {
    static CLOCK: std::cell::Cell<Option<std::time::Duration>> = const { std::cell::Cell::new(None) };
}
static START: std::sync::LazyLock<Instant> = std::sync::LazyLock::new(Instant::now);

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
fn big_picture_intro_covers_the_ready_first_screen_and_leaves_with_the_flag() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let theme = crate::theme::install_builtin(dir.path()).unwrap();
        let def = crate::theme::compile(&theme.join("main.slint"), &theme).unwrap();
        let ui = def.create().unwrap();
        ui.show().unwrap();
        window.set_size(slint::PhysicalSize::new(1280, 720));
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.ctx.intro = true;
        let draw = |shell: &mut Shell| {
            for (name, value) in shell.render(Instant::now()).props {
                ui.set_property(name, value).unwrap();
            }
            let mut pixels = vec![Rgb8Pixel::default(); 1280 * 720];
            ui.window().request_redraw();
            assert!(window.draw_if_needed(|r| {
                r.render(&mut pixels, 1280);
            }));
            pixels
        };
        // Com a entrada, o véu preto cobre tudo no 1º quadro (sem esperar a primeira tela).
        let veiled = draw(&mut shell);
        assert_eq!(veiled[0], Rgb8Pixel::new(0, 0, 0));
        // O véu pega o clique enquanto a entrada está ligada, mas não durante a saída (fade): aí o clique é da tela.
        let zones = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let seen = zones.clone();
        ui.set_callback("clicked", move |args| {
            if let Some(Value::String(z)) = args.first() {
                seen.borrow_mut().push(z.to_string());
            }
            Value::Void
        })
        .unwrap();
        let click = || {
            use slint::platform::{PointerEventButton, WindowEvent};
            let position = slint::LogicalPosition::new(640., 360.);
            let button = PointerEventButton::Left;
            window.dispatch_event(WindowEvent::PointerPressed { position, button });
            window.dispatch_event(WindowEvent::PointerReleased { position, button });
        };
        click();
        assert_eq!(zones.borrow().as_slice(), ["intro"]);
        zones.borrow_mut().clear();
        // Pulou: depois da saída (200 ms) a primeira tela, que estava pronta por baixo, aparece.
        shell.end_intro();
        let t = START.elapsed();
        CLOCK.with(|c| c.set(Some(t)));
        draw(&mut shell);
        slint::platform::update_timers_and_animations();
        CLOCK.with(|c| c.set(Some(t + std::time::Duration::from_millis(50))));
        slint::platform::update_timers_and_animations();
        click();
        assert!(!zones.borrow().iter().any(|z| z == "intro"), "{zones:?}");
        CLOCK.with(|c| c.set(Some(t + std::time::Duration::from_millis(400))));
        slint::platform::update_timers_and_animations();
        let shown = draw(&mut shell);
        let lit = |px: &[Rgb8Pixel]| {
            px.iter()
                .filter(|p| p.r != 0 || p.g != 0 || p.b != 0)
                .count()
        };
        assert!(
            lit(&shown) > lit(&veiled) * 2,
            "{} {}",
            lit(&shown),
            lit(&veiled)
        );
        ui.hide().unwrap();
    })
    .join()
    .unwrap();
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
            for scale in [1.0, 1.25, 1.5] {
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
fn fita_theme_binds_and_renders_in_the_slint_software_backend() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let fita = dir.path().join("tako-fita");
        let def = crate::theme::compile(&fita.join("main.slint"), &builtin).unwrap();
        let ui = def.create().unwrap();
        crate::theme::apply_tokens(&ui, Some(&fita));
        ui.show().unwrap();
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        let mut all_games = crate::demo::games();
        for i in 12..60 {
            all_games.push(tako_core::library::Game {
                id: format!("steam:extra{i}"),
                title: format!("Game Extra {i}"),
                store: Some(tako_core::library::GameStore::Steam),
                accent: "#3B4A6B".into(),
                is_installed: true,
                launch_target: Some(format!("demo://extra{i}")),
                added_at: Some(chrono::Utc::now()),
                last_played_at: Some(chrono::Utc::now()),
                play_time: std::time::Duration::from_secs(3600),
                ..Default::default()
            });
        }
        ctx.library = tako_core::library::GameLibrary::in_memory(
            all_games,
            tako_core::library::system_clock(),
        );
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        let game = shell.ctx.library.games()[0].id.clone();
        for lang in [Lang::PtBr, Lang::EnUs] {
            i18n::set_lang(lang);
            for reduce_motion in [false, true] {
                ui.set_global_property("Ui", "reduce-motion", Value::Bool(reduce_motion))
                    .unwrap();
                for scale in [1.0, 1.25, 1.5] {
                    ui.set_global_property("Ui", "text-scale", Value::Number(scale))
                        .unwrap();
                    for (width, height) in [(1280, 720), (1920, 1080)] {
                        window.set_size(slint::PhysicalSize::new(width, height));
                        for route in ["home", "library:all", "settings", "details"] {
                            shell.open_route(if route == "details" {
                                "library:all"
                            } else {
                                route
                            });
                            if route == "home" {
                                shell.handle(crate::shell::Cmd::Down);
                            }
                            if route == "details" {
                                shell.ctx.open_details = Some(game.clone());
                                shell.after_action();
                            }
                            for (name, value) in shell.render(Instant::now()).props {
                                crate::bridge::set(&ui, name, value);
                            }
                            let mut pixels = vec![Rgb8Pixel::default(); (width * height) as usize];
                            ui.window().request_redraw();
                            assert!(window.draw_if_needed(|renderer| {
                                renderer.render(&mut pixels, width as usize);
                            }));
                            assert!(
                                pixels.iter().any(|p| p.r != 0 || p.g != 0 || p.b != 0),
                                "tela vazia no fita: {route}/{width}/{lang:?}"
                            );
                        }
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
fn desktop_search_shortcuts_focus_the_visible_field_and_leave_returns_to_content() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        use slint::platform::WindowEvent;
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_desktop(&builtin)
            .unwrap()
            .create()
            .unwrap();
        window.set_size(slint::PhysicalSize::new(1600, 900));
        ui.show().unwrap();
        let edits = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let captured = edits.clone();
        ui.set_callback("edited", move |args| {
            if let Value::String(text) = &args[1] {
                captured.borrow_mut().push(text.to_string());
            }
            Value::Void
        })
        .unwrap();
        let key = |text: &str| {
            window.dispatch_event(WindowEvent::KeyPressed { text: text.into() });
            window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
        };
        for (page, view) in [
            ("desk-library", "list"),
            ("desk-library", "grid"),
            ("desk-library", "categories"),
            ("settings", "list"),
            ("desk-profile", "list"),
        ] {
            ui.set_property("page", Value::String(page.into())).unwrap();
            ui.set_property("desk-view", Value::String(view.into()))
                .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(150));
            slint::platform::update_timers_and_animations();
            for shortcut in ["k", "f"] {
                window.dispatch_event(WindowEvent::KeyPressed {
                    text: "\u{11}".into(),
                });
                key(shortcut);
                window.dispatch_event(WindowEvent::KeyReleased {
                    text: "\u{11}".into(),
                });
                std::thread::sleep(std::time::Duration::from_millis(3));
                slint::platform::update_timers_and_animations();
                edits.borrow_mut().clear();
                key("z");
                assert_eq!(edits.borrow().len(), 1, "{page}/{view}: Ctrl+{shortcut}");
                key("\u{1b}"); // clear
                assert_eq!(edits.borrow().last().unwrap(), "");
                key("\n"); // leave
                let n = edits.borrow().len();
                key("x");
                assert_eq!(edits.borrow().len(), n, "Enter leaves {page}/{view}");
            }
        }
    })
    .join()
    .unwrap();
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
/// TelaCarregamento: `TAKO_SNAP_OUT=<pasta>` grava `splash-<progresso>.png` (440 × 600) para cada valor de
/// `TAKO_SNAP_PROGRESS` (padrão `-1,0.42,1`; negativo = barra indeterminada), depois dos tentáculos assentarem.
#[test]
#[ignore = "ferramenta manual de captura"]
fn splash_snapshot() {
    let Ok(out_dir) = std::env::var("TAKO_SNAP_OUT") else {
        return;
    };
    std::thread::spawn(move || {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_splash(&builtin)
            .unwrap_or_else(|e| panic!("{e}"))
            .create()
            .unwrap();
        i18n::set_lang(Lang::PtBr);
        ui.show().unwrap();
        let (w, h) = (440, 600);
        window.set_size(slint::PhysicalSize::new(w, h));
        let progress = std::env::var("TAKO_SNAP_PROGRESS").unwrap_or_else(|_| "-1,0.42,1".into());
        for p in progress.split(',').filter_map(|p| p.parse::<f32>().ok()) {
            let state = if p < 0.0 {
                crate::splash::state(true, 0)
            } else if p >= 1.0 {
                crate::splash::state(false, 1000)
            } else {
                crate::splash::state(true, (p * 1000.0) as u32)
            };
            crate::splash::set(&ui, state);
            let mut pixels = vec![Rgb8Pixel::default(); (w * h) as usize];
            for _ in 0..30 {
                slint::platform::update_timers_and_animations();
                ui.window().request_redraw();
                window.draw_if_needed(|r| {
                    r.render(&mut pixels, w as usize);
                });
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            let raw: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            image::RgbImage::from_raw(w, h, raw)
                .unwrap()
                .save(format!("{out_dir}/splash-{p}.png"))
                .unwrap();
        }
        ui.hide().unwrap();
    })
    .join()
    .unwrap();
}

/// Captura da tela cheia do tema (Início e Biblioteca): `TAKO_SNAP_OUT=<pasta>`.
#[test]
#[ignore = "ferramenta manual de captura"]
fn fullscreen_snapshot() {
    let Ok(out_dir) = std::env::var("TAKO_SNAP_OUT") else {
        return;
    };
    std::thread::spawn(move || {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let theme_name = std::env::var("TAKO_SNAP_THEME").unwrap_or_else(|_| "tako-fita".into());
        let theme_dir = dir.path().join(&theme_name);
        let entry = theme_dir.join("main.slint");
        let def = crate::theme::compile(&entry, &builtin).unwrap_or_else(|e| panic!("{e:?}"));
        let ui = def.create().unwrap();
        crate::theme::apply_tokens(&ui, Some(&theme_dir));
        ui.show().unwrap();
        ui.set_global_property("Ui", "reduce-motion", Value::Bool(true))
            .unwrap();
        let art = std::env::var_os("TAKO_SNAP_ART")
            .map(|p| slint::Image::load_from_path(std::path::Path::new(&p)).unwrap());

        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);

        let sizes: Vec<(u32, u32)> = std::env::var("TAKO_SNAP_SIZES")
            .unwrap_or_else(|_| "1920x1080,1280x720,1600x900,960x600".into())
            .split(',')
            .filter_map(|s| s.split_once('x'))
            .filter_map(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
            .collect();

        // Arte clara simulada para demonstrar contraste (>= 4.5:1)
        let mut bright_art = image::RgbImage::new(1920, 1080);
        for (x, y, pixel) in bright_art.enumerate_pixels_mut() {
            let r = (235 + (x % 20)) as u8;
            let g = (240 + (y % 15)) as u8;
            let b = 250;
            *pixel = image::Rgb([r, g, b]);
        }
        let bright_art_path = dir.path().join("bright_art.png");
        bright_art.save(&bright_art_path).unwrap();
        let art_img = slint::Image::load_from_path(&bright_art_path).unwrap();

        let game = shell.ctx.library.games()[0].id.clone();
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev_lang = i18n::lang();
        for lang in [Lang::EnUs, Lang::PtBr] {
            i18n::set_lang(lang);
            let lang_suffix = match lang {
                Lang::EnUs => "",
                Lang::PtBr => "-pt-BR",
            };
            for (page_name, route, scales, focus_ribbon) in [
                ("inicio", "home", vec![1.0, 1.25, 1.5], true),
                ("biblioteca", "library:all", vec![1.0, 1.25, 1.5], false),
                ("configuracoes", "settings", vec![1.0, 1.25, 1.5], false),
                ("detalhes", "details", vec![1.0, 1.25, 1.5], false),
            ] {
                for scale in &scales {
                    let scale = *scale;
                    ui.set_global_property("Ui", "text-scale", Value::Number(scale))
                        .unwrap();
                    shell.open_route(if route == "details" {
                        "library:all"
                    } else {
                        route
                    });
                    if focus_ribbon {
                        shell.handle(crate::shell::Cmd::Down);
                    }
                    if route == "details" {
                        shell.ctx.open_details = Some(game.clone());
                        shell.after_action();
                    }
                    for &(w, h) in &sizes {
                        window.set_size(slint::PhysicalSize::new(w, h));
                        if let Ok(Value::Number(n)) = ui.get_property("library-columns") {
                            shell.ctx.grid_columns = (n as usize).max(1);
                        }
                        let out = shell.render(Instant::now());
                        for (name, value) in &out.props {
                            crate::bridge::set(&ui, name, value.clone());
                        }
                        if let Some(art) = &art {
                            ui.set_property("backdrop", Value::Image(art.clone()))
                                .unwrap();
                        } else {
                            ui.set_property("backdrop", Value::Image(art_img.clone()))
                                .unwrap();
                        }
                        let id = shell.focused_game();
                        let g = id.as_deref().and_then(|id| shell.ctx.library.find(id));
                        let p = tako_core::presentation::color::AccentPalette::for_accent(
                            g.map(|g| g.accent.as_str()),
                            true,
                        );
                        for (name, c) in [("accent", p.accent), ("accent-deep", p.deep)] {
                            let c = slint::Color::from_rgb_u8(c.r, c.g, c.b);
                            crate::bridge::set_global(&ui, "Ui", name, Value::Brush(c.into()));
                        }
                        for _ in 0..10 {
                            slint::platform::update_timers_and_animations();
                        }
                        let mut pixels = vec![Rgb8Pixel::default(); (w * h) as usize];
                        ui.window().request_redraw();
                        window.draw_if_needed(|r| {
                            r.render(&mut pixels, w as usize);
                        });
                        let raw: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
                        let img = image::RgbImage::from_raw(w, h, raw).unwrap();
                        let file_name = if (scale - 1.0).abs() < 0.01 {
                            format!("{out_dir}/fita-{page_name}{lang_suffix}-{w}x{h}.png")
                        } else {
                            format!("{out_dir}/fita-{page_name}{lang_suffix}-{scale}-{w}x{h}.png")
                        };
                        img.save(&file_name).unwrap();
                    }
                }
            }
        }
        i18n::set_lang(prev_lang);
        ui.hide().unwrap();
    })
    .join()
    .unwrap();
}

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
            slint::Image::from_rgba8(
                slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
                    img.as_raw(),
                    img.width(),
                    img.height(),
                ),
            )
        };
        let load_art = |p: String| {
            let img = image::open(p).unwrap();
            let (blur, lum) = crate::images::glass_copy(&img);
            let lum: Vec<Value> = lum.into_iter().map(|l| Value::Number(l.into())).collect();
            (
                to_image(&img.to_rgba8()),
                to_image(&blur),
                crate::values::model(lum),
            )
        };
        let art = std::env::var("TAKO_SNAP_ART").ok().map(load_art);
        // `TAKO_SNAP_SWITCH=game:3` (com `TAKO_SNAP_ART2=<imagem>`): depois da captura, troca de jogo e grava
        // `TAKO_SNAP_FRAMES` quadros (padrão 24) em `desktop-<w>x<h>-sNN.png`, com o accent de cada jogo;
        // `TAKO_SNAP_SWITCH=intro` grava a entrada do app em `iNN`.
        let switch = std::env::var("TAKO_SNAP_SWITCH").ok();
        let art2 = std::env::var("TAKO_SNAP_ART2").ok().map(load_art);
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        i18n::set_lang(Lang::PtBr);
        // `TAKO_SNAP_NI=1`: acrescenta jogos não instalados de três lojas, um baixando (NaoInstalados).
        if std::env::var("TAKO_SNAP_NI").is_ok_and(|v| v == "1") {
            ctx.found
                .lock()
                .unwrap()
                .extend(crate::demo::not_installed());
        }
        ctx.library.refresh();
        // `TAKO_SNAP_UNIFIED=1` liga a barra lateral unida; `TAKO_SNAP_NARROW=1`, a largura Estreita.
        if std::env::var("TAKO_SNAP_UNIFIED").is_ok_and(|v| v == "1") {
            ctx.settings
                .set_bool(crate::shell::ctx::keys::SIDEBAR_UNIFIED, true);
        }
        if std::env::var("TAKO_SNAP_NARROW").is_ok_and(|v| v == "1") {
            ctx.settings
                .set(crate::shell::ctx::keys::SIDEBAR_WIDTH, "narrow");
        }
        // `TAKO_SNAP_TEXT=125`: tamanho do texto (ListaTexto125).
        if let Ok(t) = std::env::var("TAKO_SNAP_TEXT")
            && !t.is_empty()
        {
            ctx.settings.set(crate::navigation_feedback::TEXT_SIZE, &t);
            // O app passa a escala ao tema fora do `render`: aqui vai direto.
            let scale = crate::navigation_feedback::text_size(&ctx.settings) as f64 / 100.;
            ui.set_global_property("Ui", "text-scale", Value::Number(scale))
                .unwrap();
        }
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        match std::env::var("TAKO_SNAP_PAGE").as_deref() {
            Ok("sunshine-preview") => {
                shell.open_route("settings");
                if let Some(index) = crate::shell::settings::categories()
                    .iter()
                    .position(|item| item.id == "integrations")
                {
                    shell.click("category", index);
                }
                let layer = crate::shell::settings::sunshine::preview_dialog(&shell.ctx, false);
                shell.push_layer(Box::new(layer));
            }
            Ok("settings") => {
                shell.open_route("settings");
                if let Ok(category) = std::env::var("TAKO_SNAP_SETTINGS_CATEGORY")
                    && let Some(index) = crate::shell::settings::categories()
                        .iter()
                        .position(|item| item.id == category)
                {
                    shell.click("category", index);
                }
            }
            Ok("appearance") => {
                shell.open_route("settings");
                shell.click("category", 3);
            }
            Ok("theme-edit") => {
                shell.open_route("settings");
                shell.click("category", 3);
                let layer = crate::shell::settings::edit_theme_panel(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("settings-api") => {
                shell.open_route("settings");
                shell
                    .ctx
                    .settings
                    .set_bool(crate::shell::ctx::keys::API_ENABLED, true);
                shell.ctx.settings.set(
                    crate::shell::ctx::keys::API_TOKEN,
                    "0123456789abcdef0123456789abcdef",
                );
                let layer = crate::shell::settings::api_panel(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            // PainelDownloads (DL-001): downloads de demonstração e o painel aberto; `TAKO_SNAP_NI=1` traz a
            // instalação da Steam no mesmo grupo. `downloads-settings`: Configurações › Downloads ligada.
            Ok("downloads") => {
                crate::shell::downloads::demo(&mut shell.ctx);
                crate::shell::downloads::open(&mut shell.ctx);
                shell.after_action();
            }
            Ok("downloads-settings") => {
                crate::shell::downloads::demo(&mut shell.ctx);
                shell.open_route("settings:downloads");
            }
            Ok("grid") => shell.click("view", 1),
            Ok("categories") => shell.click("view", 2),
            Ok("update-ready") | Ok("update-downloading") => {
                crate::app_updates::snapshot(
                    &mut shell.ctx,
                    std::env::var("TAKO_SNAP_PAGE").as_deref() == Ok("update-downloading"),
                );
                shell.after_action();
            }
            Ok("update-menu") => {
                shell.ctx.app_update.check = crate::app_updates::Check::Searching;
                shell.click("desk-brand", 0);
            }
            // `TAKO_SNAP_PAGE=profile` (ou `profile-sessions`, `profile-collections`): a TelaPerfil com sessões
            // de demonstração na semana.
            Ok(p) if p.starts_with("profile") => {
                crate::demo::sessions(&mut shell.ctx);
                shell.open_route(crate::shell::profile::ROUTE);
                match p {
                    "profile-sessions" => shell.click("profile-tab", 1),
                    "profile-collections" => shell.click("profile-tab", 2),
                    _ => {}
                }
            }
            Ok("onb-covers") => {
                // A comparação precisa de um jogo com capa: a arte da captura vira a capa do primeiro.
                if let Ok(p) = std::env::var("TAKO_SNAP_ART") {
                    if let Some(g) = shell.ctx.found.lock().unwrap().first_mut() {
                        g.cover_uri = Some(format!("file://{p}"));
                    }
                    shell.ctx.library.refresh();
                }
                let layer = crate::shell::first_run::covers(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("first-run" | "onb-stores") => {
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
            Ok("art") => {
                let id = shell.ctx.library.games()[0].id.clone();
                let layer = crate::shell::art::picker(&shell.ctx, &id);
                shell.push_layer(layer);
            }
            // Confirmação destrutiva, erro do jogo, aviso e abrindo (PadraoPopup, ErroDoJogo, Aviso, JogarEAbrir).
            Ok("exit") => {
                let layer = crate::shell::dialogs::confirm_exit(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("failed") => {
                let id = shell.ctx.library.games()[0].id.clone();
                let layer = crate::shell::dialogs::game_ended_badly(
                    &shell.ctx,
                    &id,
                    tako_core::session::GameEnd::Failed(3),
                    Some(std::time::Duration::from_secs(12)),
                );
                shell.push_layer(Box::new(layer));
            }
            Ok("toast") => shell.ctx.toast("Cupom copiado"),
            // BRAND-001: biblioteca vazia (ListaVazia com o polvo) e o aviso "Bem-vindo de volta".
            Ok("empty") => {
                shell.ctx.found.lock().unwrap().clear();
                shell.ctx.library.refresh();
            }
            Ok("welcome-back") => shell
                .ctx
                .toast_welcome_back("Bem-vindo de volta · Hades · 1 h 20 min"),
            Ok("opening") | Ok("opening-slow") => {
                let g = shell.ctx.library.games()[0].clone();
                shell.ctx.running = Some(crate::shell::ctx::RunningGame {
                    id: g.id,
                    title: g.title,
                    pid: None,
                    started: None,
                });
                shell.ctx.launching = true;
                if std::env::var("TAKO_SNAP_PAGE").as_deref() == Ok("opening-slow") {
                    shell.ctx.launch_since =
                        Some(Instant::now() - std::time::Duration::from_secs(30));
                }
            }
            // FilaDeCapas: achados só pelo nome, com nomes mais e menos parecidos, nas Configurações › Capas e metadados.
            Ok("covers") => {
                use tako_core::metadata::GameMetadata;
                let games: Vec<_> = shell.ctx.library.games().iter().take(7).cloned().collect();
                let variants = [
                    "{} Remastered",
                    "Outro Jogo Totalmente Diferente",
                    "{}: Edition",
                    "{} 2",
                    "{} Remake",
                    "{}!",
                    "Algo Quase Igual a {}",
                ];
                for (g, v) in games.iter().zip(variants) {
                    let part = GameMetadata {
                        source_ids: [("IGDB".to_owned(), "77".to_owned())].into(),
                        title: Some(v.replace("{}", &g.title)),
                        matched_by_title: true,
                        cover_url: Some("https://x/c.png".into()),
                        ..GameMetadata::new("IGDB")
                    };
                    let cached = GameMetadata {
                        parts: vec![part],
                        ..GameMetadata::new("IGDB")
                    };
                    crate::metadata::cache(&shell.ctx)
                        .write(&g.id, &cached)
                        .unwrap();
                }
                shell.open_route("settings");
                shell.click("category", 2);
                // A fila é conferida numa thread: espera ela terminar.
                for _ in 0..40 {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if crate::shell::art_review::pending(&shell.ctx).is_some() {
                        break;
                    }
                }
            }
            // EtiquetasDoJogo: etiquetas nos primeiros jogos e o editor do primeiro (`TAKO_SNAP_TYPE=<texto>` digita).
            Ok("tags") => {
                let ids: Vec<String> = (0..3)
                    .map(|i| shell.ctx.library.games()[i].id.clone())
                    .collect();
                for (tag, n) in [
                    ("Zerado", 3),
                    ("Co-op", 2),
                    ("Com mods", 1),
                    ("Para zerar depois", 1),
                ] {
                    shell.ctx.library.set_tag(&ids[..n], tag, true).unwrap();
                }
                shell.ctx.library.set_tag(&ids[..1], "Sofá", true).unwrap();
                let layer = crate::shell::dialogs::tags_layer(&shell.ctx, ids[..1].to_vec(), None);
                shell.push_layer(layer);
                if let Ok(t) = std::env::var("TAKO_SNAP_TYPE") {
                    shell.edited("overlay-input", &t);
                }
            }
            // ConfiguracoesDoJogo com ferramentas junto do jogo (EXT-001).
            Ok("game-settings") => {
                let id = shell.ctx.library.games()[0].id.clone();
                shell.ctx.game_overrides.insert(
                    id.clone(),
                    tako_core::session::settings::GameSettingsOverride {
                        before_command: Some("montar.bat".into()),
                        tools: Some("C:\\Ferramentas\\fps.exe".into()),
                        close_tools: Some(true),
                        ..Default::default()
                    },
                );
                let layer = crate::shell::game_settings::panel(&shell.ctx, &id);
                shell.push_layer(layer);
            }
            // SRC-005, destaque de jogo não instalado: `install` (uma loja), `install-multi` (Steam e GOG),
            // `install-nospace`, `install-busy` (42%), `install-busy-nofrac` e `install-menu` (o "Instalar pela" aberto).
            // O jogo é o 5º da fixture; clique nele com `TAKO_SNAP_CLICK=game:<posição na lista>`.
            Ok(p) if p.starts_with("install") => {
                use tako_core::library::{GameStore, InstallOffer, InstallProgress};
                const GIB: f64 = (1u64 << 30) as f64;
                let need = (9.1 * GIB) as u64;
                let offer = |store, uri: &str, gib: f64| InstallOffer {
                    store,
                    game_id: String::new(),
                    install_uri: uri.into(),
                    size: Some((gib * GIB) as u64),
                };
                {
                    let mut found = shell.ctx.found.lock().unwrap();
                    let g = &mut found[4];
                    g.title = "Disco Elysium".into();
                    g.store = Some(GameStore::Steam);
                    g.steam_app_id = Some(632470);
                    g.install_size = Some(need);
                    g.play_time = std::time::Duration::ZERO;
                    g.last_played_at = None;
                    if matches!(p, "install-multi" | "install-menu") {
                        g.install_offers = vec![
                            offer(GameStore::Steam, "steam://install/632470", 9.1),
                            offer(GameStore::Gog, "goggalaxy://openGameView/1", 8.8),
                        ];
                    }
                    if p.starts_with("install-busy") {
                        g.installing = Some(InstallProgress {
                            fraction: (p == "install-busy").then_some(0.42),
                        });
                    }
                }
                shell.ctx.library.refresh();
                shell.ctx.free_space =
                    (p != "install-nofree").then(|| tako_core::devices::FreeSpace {
                        drive: "C:".into(),
                        bytes: if p == "install-nospace" {
                            need - (3.2 * GIB) as u64
                        } else {
                            120 << 30
                        },
                    });
            }
            Ok("ask") => {
                shell.ctx.pads_connected = 1;
                shell.ask_big_picture = true;
                crate::shell::desktop::ask_big_picture(&mut shell, std::time::Duration::ZERO);
            }
            _ => {}
        }
        // `TAKO_SNAP_CLICK=zona:índice,…`: cliques antes da captura (ex.: `desk-brand:0`, `game:3`).
        for click in std::env::var("TAKO_SNAP_CLICK")
            .unwrap_or_default()
            .split(',')
        {
            if let Some((zone, i)) = click.split_once(':') {
                shell.click(zone, i.parse().unwrap_or(0));
            }
        }
        if let Some(t) = std::env::var("TAKO_SNAP_TEXT")
            .ok()
            .and_then(|t| t.parse::<f64>().ok())
        {
            ui.set_global_property("Ui", "text-scale", Value::Number(t / 100.))
                .unwrap();
        }
        // Entrada do app: apagado desde o primeiro quadro, como na abertura.
        if switch.as_deref() == Some("intro") {
            ui.set_property("desk-intro", Value::Bool(true)).unwrap();
        }
        ui.show().unwrap();
        // `TAKO_SNAP_SIZES=1920x1080,2048x1100`: outros tamanhos além dos das prévias.
        let sizes: Vec<(u32, u32)> = std::env::var("TAKO_SNAP_SIZES")
            .unwrap_or_else(|_| "1600x900,960x600".into())
            .split(',')
            .filter_map(|s| s.split_once('x'))
            .filter_map(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
            .collect();
        let apply = |shell: &mut Shell, art: Option<&(slint::Image, slint::Image, Value)>| {
            // Como o app: a grade conta linhas por coluna, e as colunas vêm da largura da janela.
            if let Ok(Value::Number(n)) = ui.get_property("library-columns") {
                shell.ctx.grid_columns = (n as usize).max(1);
            }
            let out = shell.render(Instant::now());
            for (name, value) in &out.props {
                let mut value = value.clone();
                // `TAKO_SNAP_LOADING=1`: biblioteca lendo (EstadoCarregando).
                if std::env::var("TAKO_SNAP_LOADING").is_ok() {
                    match (*name, &mut value) {
                        ("desk-loading", v) => *v = Value::Bool(true),
                        ("desk-game", Value::Struct(s)) => {
                            s.set_field("has-game".into(), Value::Bool(false));
                        }
                        _ => {}
                    }
                }
                if let (Some((art, blur, lum)), "desk-game", Value::Struct(s)) =
                    (art, *name, &mut value)
                {
                    s.set_field("art".into(), Value::Image(art.clone()));
                    s.set_field("art-blur".into(), Value::Image(blur.clone()));
                    s.set_field("art-lum".into(), lum.clone());
                }
                // `TAKO_SNAP_TITLE` / `TAKO_SNAP_DESC`: textos longos no destaque.
                if let ("desk-game", Value::Struct(s)) = (*name, &mut value) {
                    for (var, field) in [
                        ("TAKO_SNAP_TITLE", "title"),
                        ("TAKO_SNAP_DESC", "description"),
                    ] {
                        if let Some(text) = std::env::var(var).ok().filter(|t| !t.is_empty()) {
                            s.set_field(field.into(), Value::String(text.into()));
                        }
                    }
                    // `TAKO_SNAP_LOGO=<png>`: o título vira esse logo (TituloDoJogo).
                    if let Some(p) = std::env::var("TAKO_SNAP_LOGO")
                        .ok()
                        .filter(|p| !p.is_empty())
                    {
                        let logo = to_image(&image::open(p).unwrap().to_rgba8());
                        s.set_field("logo".into(), Value::Image(logo));
                        s.set_field("show-logo".into(), Value::Bool(true));
                    }
                }
                crate::bridge::set(&ui, name, value);
            }
            if switch.is_some() {
                let id = shell.focused_game();
                let g = id.as_deref().and_then(|id| shell.ctx.library.find(id));
                let p = tako_core::presentation::color::AccentPalette::for_accent(
                    g.map(|g| g.accent.as_str()),
                    true,
                );
                for (name, c) in [("accent", p.accent), ("accent-deep", p.deep)] {
                    let c = slint::Color::from_rgb_u8(c.r, c.g, c.b);
                    crate::bridge::set_global(&ui, "Ui", name, Value::Brush(c.into()));
                }
            }
        };
        for (w, h) in sizes {
            window.set_size(slint::PhysicalSize::new(w, h));
            apply(&mut shell, art.as_ref());
            // As entradas animam (Timer de 1 ms e `page-time`): a captura espera elas terminarem
            // (`TAKO_SNAP_WAIT=<ms>` fotografa no meio delas).
            let wait: u64 = std::env::var("TAKO_SNAP_WAIT")
                .ok()
                .and_then(|w| w.parse().ok())
                .unwrap_or(800);
            // Desenha a cada passo: o que está dentro de um `if` só nasce (e liga o Timer dele) ao ser desenhado.
            let mut scratch = vec![Rgb8Pixel::default(); (w * h) as usize];
            for _ in 0..wait / 50 {
                slint::platform::update_timers_and_animations();
                ui.window().request_redraw();
                window.draw_if_needed(|r| {
                    r.render(&mut scratch, w as usize);
                });
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            slint::platform::update_timers_and_animations();
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
            // Quadros de 40 ms no relógio das animações, a partir de agora.
            let capture = |prefix: &str, pixels: &mut Vec<Rgb8Pixel>| {
                let frames: u32 = std::env::var("TAKO_SNAP_FRAMES")
                    .ok()
                    .and_then(|f| f.parse().ok())
                    .unwrap_or(24);
                let t0 = START.elapsed();
                for f in 0..frames {
                    let t = 40 * f;
                    CLOCK.with(|c| c.set(Some(t0 + std::time::Duration::from_millis(t.into()))));
                    slint::platform::update_timers_and_animations();
                    ui.window().request_redraw();
                    window.draw_if_needed(|r| {
                        r.render(pixels, w as usize);
                    });
                    let raw: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
                    image::RgbImage::from_raw(w, h, raw)
                        .unwrap()
                        .save(format!("{out_dir}/desktop-{w}x{h}-{prefix}{f:02}.png"))
                        .unwrap();
                }
                CLOCK.with(|c| c.set(None));
            };
            // `TAKO_SNAP_SWITCH=intro`: a entrada do app (primeiro quadro apagado, depois `iNN`).
            if switch.as_deref() == Some("intro") {
                ui.set_property("desk-intro", Value::Bool(false)).unwrap();
                capture("i", &mut pixels);
            } else if let Some((zone, i)) = switch.as_deref().and_then(|s| s.split_once(':')) {
                shell.click(zone, i.parse().unwrap_or(0));
                apply(&mut shell, art2.as_ref().or(art.as_ref()));
                capture("s", &mut pixels);
            }
        }
        ui.hide().unwrap();
    })
    .join()
    .unwrap();
}

/// Custo da rolagem da lista do desktop (CPU, renderizador de software): `TAKO_BENCH_FRAMES=<n>` quadros de roda do
/// mouse sobre a lista; imprime o tempo médio do quadro e do `render` do Rust que o `scrolled` dispara.
/// `cargo test -p tako-ui --release desktop_scroll_bench -- --ignored --nocapture`.
#[test]
#[ignore = "medição manual"]
fn desktop_scroll_bench() {
    std::thread::spawn(|| {
        use std::cell::RefCell;
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_desktop(&builtin)
            .unwrap()
            .create()
            .unwrap();
        let (w, h) = (1600u32, 620u32);
        window.set_size(slint::PhysicalSize::new(w, h));
        ui.show().unwrap();
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        i18n::set_lang(Lang::PtBr);
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        let shell = Rc::new(RefCell::new(shell));
        let rust = Rc::new(RefCell::new((0u32, std::time::Duration::ZERO)));
        for (name, value) in shell.borrow_mut().render(Instant::now()).props {
            crate::bridge::set(&ui, name, value);
        }
        {
            let (shell, rust, ui2) = (shell.clone(), rust.clone(), ui.clone_strong());
            ui.set_callback("scrolled", move |args| {
                let Some(Value::Number(first)) = args.first() else {
                    return Value::Void;
                };
                let t = Instant::now();
                shell.borrow_mut().ctx.list_first = *first as usize;
                for (name, value) in shell.borrow_mut().render(Instant::now()).props {
                    crate::bridge::set(&ui2, name, value);
                }
                let mut r = rust.borrow_mut();
                r.0 += 1;
                r.1 += t.elapsed();
                Value::Void
            })
            .unwrap();
        }
        let mut pixels = vec![Rgb8Pixel::default(); (w * h) as usize];
        let mut draw = || {
            slint::platform::update_timers_and_animations();
            window.draw_if_needed(|r| {
                r.render(&mut pixels, w as usize);
            });
        };
        for _ in 0..20 {
            draw();
        }
        let frames: u32 = std::env::var("TAKO_BENCH_FRAMES")
            .ok()
            .and_then(|f| f.parse().ok())
            .unwrap_or(400);
        let start = Instant::now();
        for f in 0..frames {
            // Desce e sobe: a lista inteira passa pela tela várias vezes.
            let dy = if (f / 40) % 2 == 0 { -24.0 } else { 24.0 };
            window.dispatch_event(slint::platform::WindowEvent::PointerScrolled {
                position: slint::LogicalPosition::new(200.0, 400.0),
                delta_x: 0.0,
                delta_y: dy,
            });
            draw();
        }
        let total = start.elapsed();
        let (calls, rust) = *rust.borrow();
        println!(
            "quadros: {frames}, média {:.3} ms/quadro; scrolled: {calls} chamadas, média {:.3} ms",
            total.as_secs_f64() * 1000.0 / f64::from(frames),
            rust.as_secs_f64() * 1000.0 / f64::from(calls.max(1)),
        );
        assert!(calls > 0, "a lista não rolou");
        // `TAKO_BENCH_OUT=<png>`: o último quadro (a lista rolada), para conferir a ordem das linhas.
        if let Ok(path) = std::env::var("TAKO_BENCH_OUT") {
            ui.window().request_redraw();
            draw();
            let raw: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            image::RgbImage::from_raw(w, h, raw)
                .unwrap()
                .save(path)
                .unwrap();
        }
        ui.hide().unwrap();
    })
    .join()
    .unwrap();
}

mod fita;
