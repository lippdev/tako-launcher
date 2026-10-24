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
fn desktop_shortcuts_focus_native_search_and_return_to_content() {
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
fn collapsed_list_expands_on_ctrl_k_only_and_not_when_returning_to_the_library() {
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
        let expands = Rc::new(std::cell::Cell::new(0));
        let counted = expands.clone();
        ui.set_callback("clicked", move |args| {
            if matches!(&args[0], Value::String(z) if z.as_str() == "collapse") {
                counted.set(counted.get() + 1);
            }
            Value::Void
        })
        .unwrap();
        let ctrl_k = |text: &str| {
            window.dispatch_event(WindowEvent::KeyPressed {
                text: "\u{11}".into(),
            });
            window.dispatch_event(WindowEvent::KeyPressed { text: text.into() });
            window.dispatch_event(WindowEvent::KeyReleased { text: text.into() });
            window.dispatch_event(WindowEvent::KeyReleased {
                text: "\u{11}".into(),
            });
            // O foco da busca passa por um timer; com a máquina carregada, 5 ms não bastavam.
            for _ in 0..10 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                slint::platform::update_timers_and_animations();
            }
        };
        let show = |page: &str| {
            ui.set_property("page", Value::String(page.into())).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(150));
            slint::platform::update_timers_and_animations();
        };
        ui.set_property("desk-collapsed", Value::Bool(true))
            .unwrap();
        show("desk-library");
        ctrl_k("k");
        assert_eq!(expands.get(), 1, "Ctrl+K expande a lista recolhida");
        // Sair e voltar à biblioteca não pede para expandir (a lista continua recolhida).
        show("settings");
        show("desk-library");
        assert_eq!(expands.get(), 1, "voltar à biblioteca não expande");
        // Ctrl+K em outra tela também não mexe na lista escondida.
        show("settings");
        ctrl_k("k");
        show("desk-library");
        assert_eq!(expands.get(), 1, "Ctrl+K nas Configurações não expande");
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
/// `TAKO_SNAP_ART=<imagem>` (arte do jogo em foco, e do destaque da Visão geral, com `TAKO_SNAP_LOGO`, `TAKO_SNAP_COVER`,
/// `TAKO_SNAP_BRIGHT` e `TAKO_SNAP_LOADING`), `TAKO_SNAP_PAGE=settings` e `TAKO_SNAP_CLICK` (opcionais).
/// `cargo test -p tako-ui desktop_snapshot -- --ignored`.
/// TelaCarregamento: `TAKO_SNAP_OUT=<pasta>` grava `splash-<progresso>.png` (440 × 600) para cada valor de
/// `TAKO_SNAP_PROGRESS` (padrão `-1,0.42,1`; negativo = barra indeterminada), depois dos tentáculos assentarem.
/// A página do jogo rola pelo tema: Page Down move o corpo e, passado o primário do destaque, a barra fina ganha o
/// botão Jogar (TelaJogoRolada). Pelos pixels, porque a rolagem mora no tema e não no Tako.
#[test]
fn game_page_scrolls_with_page_down_and_the_thin_bar_gets_the_play_button() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_desktop(&builtin)
            .unwrap_or_else(|e| panic!("{e}"))
            .create()
            .unwrap();
        ui.show().unwrap();
        let (w, h) = (1280usize, 800usize);
        window.set_size(slint::PhysicalSize::new(w as u32, h as u32));
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        i18n::set_lang(Lang::PtBr);
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        shell.click("game", 0);
        let draw = |shell: &mut Shell| {
            for (name, value) in shell.render(Instant::now()).props {
                let mut value = value;
                // Muitos parágrafos no Sobre: a página tem por onde rolar.
                if let ("desk-page", Value::Struct(s)) = (name, &mut value) {
                    let about = (0..30).map(|_| Value::String("Um parágrafo do Sobre.".into()));
                    s.set_field("about".into(), crate::values::model(about.collect()));
                }
                crate::bridge::set(&ui, name, value);
            }
            let mut pixels = vec![Rgb8Pixel::default(); w * h];
            ui.window().request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut pixels, w);
            });
            pixels
        };
        // Passa o tempo no relógio das animações (a rolagem anima em 200 ms).
        let settle = |shell: &mut Shell, ms: u64| {
            let t = START.elapsed() + std::time::Duration::from_millis(ms);
            CLOCK.with(|c| c.set(Some(t)));
            slint::platform::update_timers_and_animations();
            draw(shell)
        };
        draw(&mut shell);
        let top = settle(&mut shell, 100);
        // Page Down rolou 85% da janela: o corpo muda e a barra fina da direita deixa de ser só fundo.
        shell.key("\u{F72D}", tako_core::input::shortcut::KeyModifiers::NONE);
        draw(&mut shell);
        // A rolagem nasce no primeiro passo (o `changed` dispara) e anda no segundo.
        settle(&mut shell, 1000);
        let rolled = settle(&mut shell, 1600);
        // O primário branco da barra fina: pixels claros na faixa de 52 a 114, à direita (as pílulas do topo têm só o
        // texto claro).
        let light = |px: &[Rgb8Pixel]| {
            (60..110)
                .flat_map(|y| (900..1270).map(move |x| (x, y)))
                .filter(|(x, y)| px[y * w + x].g > 235)
                .count()
        };
        assert!(
            light(&top) < 300,
            "sem rolar, a barra fina não tem o primário"
        );
        assert!(
            light(&rolled) > 2000,
            "rolada, a barra fina mostra o primário: {}",
            light(&rolled)
        );
        assert!(top != rolled, "o corpo da página se moveu");
        // Page Up volta ao topo.
        shell.key("\u{F72C}", tako_core::input::shortcut::KeyModifiers::NONE);
        draw(&mut shell);
        settle(&mut shell, 2200);
        let back = settle(&mut shell, 3000);
        assert!(
            light(&back) < 300,
            "de volta ao topo, o primário sai da barra"
        );
        CLOCK.with(|c| c.set(None));
    })
    .join()
    .unwrap();
}

/// Trocar de jogo não recarrega a página: as partes do destaque saem, o outro jogo entra e elas voltam em escada. Com
/// Reduzir animações tudo troca na hora. Pelos pixels do nome do jogo (texto claro), no relógio manual das animações.
#[test]
fn game_page_swaps_parts_in_a_staircase_and_instantly_with_reduced_motion() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_desktop(&builtin)
            .unwrap_or_else(|e| panic!("{e}"))
            .create()
            .unwrap();
        ui.show().unwrap();
        let (w, h) = (1600usize, 900usize);
        window.set_size(slint::PhysicalSize::new(w as u32, h as u32));
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        i18n::set_lang(Lang::PtBr);
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        shell.click("game", 0);
        let draw = |shell: &mut Shell| {
            for (name, value) in shell.render(Instant::now()).props {
                crate::bridge::set(&ui, name, value);
            }
            let mut pixels = vec![Rgb8Pixel::default(); w * h];
            ui.window().request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut pixels, w);
            });
            pixels
        };
        let base = START.elapsed();
        let at = |shell: &mut Shell, ms: u64| {
            CLOCK.with(|c| c.set(Some(base + std::time::Duration::from_millis(ms))));
            slint::platform::update_timers_and_animations();
            draw(shell)
        };
        // O nome do jogo (texto claro) na coluna da identidade, embaixo à direita do destaque.
        let title = |px: &[Rgb8Pixel]| {
            (400..700)
                .flat_map(|y| (900..1560).map(move |x| (x, y)))
                .filter(|(x, y)| px[y * w + x].g > 215)
                .count()
        };
        draw(&mut shell);
        let before = at(&mut shell, 500);
        assert!(title(&before) > 500, "o nome do primeiro jogo aparece");
        shell.click("game", 1);
        draw(&mut shell);
        at(&mut shell, 510);
        // No fim da saída (200 ms) a cópia troca e as partes recomeçam de zero: quase nada à vista.
        let gap = at(&mut shell, 715);
        assert!(
            title(&gap) * 5 < title(&before),
            "as partes saem antes do outro jogo entrar"
        );
        let end = at(&mut shell, 2500);
        assert!(title(&end) > 500, "o outro jogo entrou");
        assert!(end != before, "é outro jogo");
        // Reduzir animações: troca na hora.
        crate::bridge::set_global(&ui, "Ui", "reduce-motion", Value::Bool(true));
        shell.click("game", 2);
        draw(&mut shell);
        at(&mut shell, 2600);
        let instant = at(&mut shell, 2650);
        assert!(
            title(&instant) > 500,
            "sem animação o jogo novo já está na tela"
        );
        CLOCK.with(|c| c.set(None));
    })
    .join()
    .unwrap();
}

/// Janela pequena: a coluna da identidade nunca fica mais estreita que a fileira de ações, então o botão primário tem
/// a mesma largura a 960 e a 1600 (antes saía cortado a 960). Pelos pixels: a corrida clara mais longa de uma linha é o
/// fundo do primário.
#[test]
fn game_page_primary_action_is_not_clipped_in_a_small_window() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_desktop(&builtin)
            .unwrap_or_else(|e| panic!("{e}"))
            .create()
            .unwrap();
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
        shell.click("game", 0);
        let mut longest_light_run = |w: usize, h: usize, scale: f64| {
            window.set_size(slint::PhysicalSize::new(w as u32, h as u32));
            ui.set_global_property("Ui", "text-scale", Value::Number(scale))
                .unwrap();
            for (name, value) in shell.render(Instant::now()).props {
                crate::bridge::set(&ui, name, value);
            }
            let mut pixels = vec![Rgb8Pixel::default(); w * h];
            for _ in 0..2 {
                ui.window().request_redraw();
                window.draw_if_needed(|r| {
                    r.render(&mut pixels, w);
                });
            }
            (0..h)
                .map(|y| {
                    let (mut best, mut run) = (0, 0);
                    for x in w / 2..w {
                        let p = pixels[y * w + x];
                        run = if p.r > 235 && p.g > 235 && p.b > 235 {
                            run + 1
                        } else {
                            0
                        };
                        best = best.max(run);
                    }
                    best
                })
                .max()
                .unwrap_or(0)
        };
        let wide = longest_light_run(1600, 900, 1.);
        let small = longest_light_run(960, 600, 1.);
        let large_text = longest_light_run(960, 600, 1.25);
        assert!(wide > 100, "o primário aparece a 1600: {wide}");
        assert!(
            small + 2 >= wide,
            "primário cortado a 960: {small} < {wide}"
        );
        assert!(
            large_text + 2 >= wide,
            "primário cortado a 960 com texto a 125%: {large_text} < {wide}"
        );
    })
    .join()
    .unwrap();
}

/// Focar uma capa de Parecidos pelo teclado rola a página até ela: a fileira fica abaixo da dobra num 960 × 600 e
/// passa a aparecer inteira (o quadro muda muito); ↑ volta ao topo, onde estavam as ações.
#[test]
fn focusing_similar_games_with_the_keyboard_scrolls_the_page_to_them() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let ui = crate::theme::compile_desktop(&builtin)
            .unwrap_or_else(|e| panic!("{e}"))
            .create()
            .unwrap();
        ui.show().unwrap();
        let (w, h) = (960usize, 600usize);
        window.set_size(slint::PhysicalSize::new(w as u32, h as u32));
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        i18n::set_lang(Lang::PtBr);
        ctx.library.refresh();
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        shell.click("game", 0);
        let draw = |shell: &mut Shell| {
            for (name, value) in shell.render(Instant::now()).props {
                crate::bridge::set(&ui, name, value);
            }
            let mut pixels = vec![Rgb8Pixel::default(); w * h];
            ui.window().request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut pixels, w);
            });
            pixels
        };
        let settle = |shell: &mut Shell, ms: u64| {
            let t = START.elapsed() + std::time::Duration::from_millis(ms);
            CLOCK.with(|c| c.set(Some(t)));
            slint::platform::update_timers_and_animations();
            draw(shell)
        };
        let differ =
            |a: &[Rgb8Pixel], b: &[Rgb8Pixel]| a.iter().zip(b).filter(|(x, y)| x != y).count();
        shell.key("\u{0009}", tako_core::input::shortcut::KeyModifiers::NONE);
        draw(&mut shell);
        let top = settle(&mut shell, 100);
        shell.handle(Cmd::Down);
        draw(&mut shell);
        settle(&mut shell, 1000);
        let rolled = settle(&mut shell, 1600);
        assert!(
            differ(&top, &rolled) > 20_000,
            "a página rolou até as capas: {}",
            differ(&top, &rolled)
        );
        shell.handle(Cmd::Up);
        draw(&mut shell);
        settle(&mut shell, 2200);
        let back = settle(&mut shell, 3000);
        assert!(
            differ(&top, &back) < differ(&top, &rolled) / 4,
            "↑ voltou ao topo: {}",
            differ(&top, &back)
        );
        CLOCK.with(|c| c.set(None));
    })
    .join()
    .unwrap();
}

/// O escurecimento da arte (UI-GLASS-001) não pode ter aresta dura: sobre fundo branco, a luminância muda de um pixel
/// para o outro só pelo degradê, inclusive no topo e na direita da elipse atrás do texto.
#[test]
fn art_scrim_has_no_hard_edge_around_the_ellipse() {
    std::thread::spawn(|| {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let entry = builtin.join("scrim_test.slint");
        std::fs::write(
            &entry,
            "import { ArtScrim } from \"desktop.slint\";\n\
             export component App inherits Window {\n\
                 in property <bool> small;\n\
                 background: white;\n\
                 ArtScrim { width: parent.width; height: parent.height; small: root.small; }\n\
             }\n",
        )
        .unwrap();
        let ui = crate::theme::compile(&entry, &builtin)
            .unwrap_or_else(|e| panic!("{e}"))
            .create()
            .unwrap();
        ui.show().unwrap();
        // (janela, pequena, centro em fração, raios em fração): 52% × 64% em 24% / 90%; 70% × 72% em 40% / 92%.
        let cases = [
            (
                (1600u32, 900u32),
                false,
                (0.24f32, 0.90f32),
                (0.52f32, 0.64f32),
            ),
            ((960, 600), true, (0.40, 0.92), (0.70, 0.72)),
        ];
        for ((w, h), small, (cx, cy), (rx, ry)) in cases {
            ui.set_property("small", Value::Bool(small)).unwrap();
            window.set_size(slint::PhysicalSize::new(w, h));
            let mut pixels = vec![Rgb8Pixel::default(); (w * h) as usize];
            ui.window().request_redraw();
            assert!(window.draw_if_needed(|r| {
                r.render(&mut pixels, w as usize);
            }));
            let at = |x: u32, y: u32| pixels[(y * w + x) as usize].r as i32;
            // Salto máximo entre pixels vizinhos: o degradê mais íngreme tem menos de 2 níveis por pixel; o círculo
            // que não chegava a 0 na borda deixava um degrau de dezenas de níveis.
            let jump = |from: (u32, u32), to: (u32, u32)| {
                let (dx, dy) = (to.0 as i32 - from.0 as i32, to.1 as i32 - from.1 as i32);
                let steps = dx.abs().max(dy.abs());
                (0..steps)
                    .map(|i| {
                        let p = |k: i32| {
                            at(
                                (from.0 as i32 + dx.signum() * k) as u32,
                                (from.1 as i32 + dy.signum() * k) as u32,
                            )
                        };
                        (p(i + 1) - p(i)).abs()
                    })
                    .max()
                    .unwrap()
            };
            let (wf, hf) = (w as f32, h as f32);
            let (x0, y0) = ((cx * wf) as u32, (cy * hf) as u32);
            // Topo da elipse: coluna do centro, de 60 pixels acima a 60 abaixo da borda.
            let top = y0 - (ry * hf) as u32;
            assert!(jump((x0, top - 60), (x0, top + 60)) <= 3, "topo, {w}x{h}");
            // Borda direita (só cabe na janela grande): linha do centro, 60 pixels de cada lado.
            let right = x0 + (rx * wf) as u32;
            if right + 60 < w {
                assert!(
                    jump((right - 60, y0), (right + 60, y0)) <= 3,
                    "direita, {w}x{h}"
                );
            }
        }
    })
    .join()
    .unwrap();
}

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
        let art_accent = std::env::var("TAKO_SNAP_ART")
            .ok()
            .and_then(|p| image::open(p).ok())
            .and_then(|img| crate::images::sample_accent(&img));
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
        // Visão geral: `TAKO_SNAP_SESSIONS=1` põe as sessões de demonstração (Sua semana) e `TAKO_SNAP_ATTENTION=1` um
        // download com erro e um jogo sem os arquivos (Precisa de atenção).
        if std::env::var("TAKO_SNAP_SESSIONS").is_ok_and(|v| v == "1") {
            crate::demo::sessions(&mut ctx);
        }
        if std::env::var("TAKO_SNAP_ATTENTION").is_ok_and(|v| v == "1") {
            let now = Instant::now();
            let list = &mut ctx.activity.list;
            list.start("download:1", tako_core::activities::Kind::Download, "x", "", now);
            list.fail(
                "download:1",
                "Não baixou Hades II",
                Some(tako_core::activities::Action::Retry),
                now,
            );
            if let Some(g) = ctx.library.games_mut().get_mut(3) {
                g.is_broken = true;
            }
        }
        // `TAKO_SNAP_DESK_THEME=tako|dark|midnight|oled|glass`, `TAKO_SNAP_ACCENT=#rrggbb` e `TAKO_SNAP_MOTION=1`
        // (Reduzir animações) escolhem o tema do desktop; o padrão é o TAKO. `TAKO_SNAP_TRANSPARENCY=1` liga Reduzir
        // transparência.
        for (var, key) in [
            ("TAKO_SNAP_DESK_THEME", crate::desk_theme::THEME),
            ("TAKO_SNAP_ACCENT", crate::desk_theme::ACCENT),
        ] {
            if let Ok(v) = std::env::var(var) {
                ctx.settings.set(key, &v);
            }
        }
        if std::env::var("TAKO_SNAP_MOTION").is_ok_and(|v| v == "1") {
            ctx.settings
                .set_bool(crate::shell::ctx::keys::REDUCED_MOTION, true);
        }
        if std::env::var("TAKO_SNAP_TRANSPARENCY").is_ok_and(|v| v == "1") {
            ctx.settings
                .set_bool(crate::shell::ctx::keys::REDUCED_TRANSPARENCY, true);
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
        shell.show_overview();
        // `TAKO_SNAP_RICH=1`: sessões de demonstração, e na página do jogo trailer, captura e recursos (o resto da
        // página só aparece com dado).
        if std::env::var("TAKO_SNAP_RICH").is_ok() {
            crate::demo::sessions(&mut shell.ctx);
        }
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
            Ok("api-regenerate") => {
                shell.open_route("settings");
                let layer = crate::shell::settings::confirm_regenerate_token(&shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("cancel-download") => {
                crate::shell::downloads::demo(&mut shell.ctx);
                if let Some(download) = shell
                    .ctx
                    .downloads
                    .manager
                    .as_ref()
                    .and_then(|manager| manager.list().get(1))
                {
                    let layer = crate::shell::downloads::confirm_cancel(&shell.ctx, download.id);
                    shell.push_layer(Box::new(layer));
                }
            }
            Ok("toast-success") => shell.ctx.toast_success(
                "Backup criado com sucesso e salvo na pasta escolhida para esta instalação do Tako",
            ),
            Ok("toast-error") => shell.ctx.toast_error(
                "Falha ao restaurar o backup: o arquivo está incompleto e a cópia anterior não pôde ser validada",
            ),
            Ok("integrations") => {
                shell.open_route("settings:integrations");
                if let Ok(st) = std::env::var("TAKO_SNAP_CONSOLE_MODE") {
                    match st.as_str() {
                        "verificando" => {
                            crate::shell::settings::integrations::set_checking_for_test(true)
                        }
                        "instalado" => crate::shell::settings::integrations::set_status_for_test(
                            tako_core::console_mode::ConsoleModeStatus::Instalado {
                                versao_api: 1,
                                capacidades: vec!["tv".into(), "fps".into()],
                            },
                        ),
                        "nao-instalado" => {
                            crate::shell::settings::integrations::set_status_for_test(
                                tako_core::console_mode::ConsoleModeStatus::NaoInstalado,
                            )
                        }
                        _ => {}
                    }
                }
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
                // `TAKO_SNAP_BATTERY=<%>`: dois controles conectados, o em uso com essa bateria (TelaControles).
                if let Some(level) = std::env::var("TAKO_SNAP_BATTERY")
                    .ok()
                    .and_then(|b| b.parse().ok())
                {
                    shell.ctx.pads = vec![
                        ("a".into(), "Controle sem fio Xbox".into()),
                        ("b".into(), "DualSense".into()),
                    ];
                    (shell.ctx.pad_guid, shell.ctx.pad_name) = ("b".into(), "DualSense".into());
                    shell.ctx.pad_battery = Some(level);
                }
                // `TAKO_SNAP_QUERY=<texto>`: digita na busca das configurações.
                if let Ok(query) = std::env::var("TAKO_SNAP_QUERY") {
                    shell.edited("search", &query);
                }
            }
            Ok("appearance") => {
                shell.open_route("settings");
                if let Some(index) = crate::shell::settings::categories()
                    .iter()
                    .position(|item| item.id == "appearance")
                {
                    shell.click("category", index);
                }
            }
            Ok("theme-edit") => {
                shell.open_route("settings");
                if let Some(index) = crate::shell::settings::categories()
                    .iter()
                    .position(|item| item.id == "appearance")
                {
                    shell.click("category", index);
                }
                let layer = crate::shell::settings::edit_theme_panel(&mut shell.ctx);
                shell.push_layer(Box::new(layer));
            }
            Ok("theme-create") => {
                shell.open_route("settings");
                if let Some(index) = crate::shell::settings::categories()
                    .iter()
                    .position(|item| item.id == "appearance")
                {
                    shell.click("category", index);
                }
                let layer = crate::shell::settings::create_theme_editor_panel(&shell.ctx, None);
                shell.push_layer(Box::new(layer));
            }
            Ok("theme-create-editor") => {
                shell.open_route("settings");
                if let Some(index) = crate::shell::settings::categories()
                    .iter()
                    .position(|item| item.id == "appearance")
                {
                    shell.click("category", index);
                }
                let layer = crate::shell::settings::create_theme_editor_panel(&shell.ctx, None);
                shell.push_layer(Box::new(layer));
            }
            Ok("theme-name") => {
                shell.open_route("settings");
                if let Some(index) = crate::shell::settings::categories()
                    .iter()
                    .position(|item| item.id == "appearance")
                {
                    shell.click("category", index);
                }
                let layer = crate::shell::settings::prompt_theme_name(&shell.ctx, None, false);
                shell.push_layer(Box::new(layer));
            }
            Ok("theme-overwrite") => {
                shell.open_route("settings");
                if let Some(index) = crate::shell::settings::categories()
                    .iter()
                    .position(|item| item.id == "appearance")
                {
                    shell.click("category", index);
                }
                let theme_id = "meu-tema";
                let t_dir = shell.ctx.paths.themes().join(theme_id);
                let _ = std::fs::create_dir_all(&t_dir);
                let _ = std::fs::write(
                    t_dir.join("theme.toml"),
                    r#"id = "meu-tema"
name = "Meu Tema"
version = "1.0.0"
author = "Tako"
api = 1
entry = "main.slint"
created-by-tako = true
"#,
                );
                let _ = std::fs::write(
                    t_dir.join("main.slint"),
                    r#"export { App, Ui, Hint, PanelRow } from "@tako/main.slint";
"#,
                );
                let flow = crate::shell::settings::finish_create_theme(
                    &mut shell.ctx,
                    "Meu Tema",
                    None,
                    None,
                    false,
                );
                if let crate::shell::layer::Flow::Push(layer) = flow {
                    shell.push_layer(layer);
                }
            }
            Ok("theme-export") => {
                shell.open_route("settings");
                shell.click("category", 3);
                for (id, name) in [("tema-um", "Tema 1"), ("tema-dois", "Tema 2")] {
                    let t_dir = shell.ctx.paths.themes().join(id);
                    let _ = std::fs::create_dir_all(&t_dir);
                    let _ = std::fs::write(
                        t_dir.join("theme.toml"),
                        format!(
                            "id = \"{id}\"\nname = \"{name}\"\nversion = \"1.0.0\"\nauthor = \"Tako\"\napi = 1\nentry = \"main.slint\"\ncreated-by-tako = true\n"
                        ),
                    );
                    let _ = std::fs::write(
                        t_dir.join("main.slint"),
                        r#"export { App, Ui, Hint, PanelRow } from "@tako/main.slint";
"#,
                    );
                    let _ = std::fs::write(t_dir.join("tokens.toml"), "accent = \"#6C5CE7\"\n");
                }
                crate::shell::settings::forget(crate::shell::settings::THEMES_CACHE);
                shell.ctx.reload_theme_cache();
                let data = crate::shell::settings::cached_themes(&shell.ctx);
                if data.exportable.len() >= 2 {
                    let flow = crate::shell::settings::export_theme_panel(&mut shell.ctx);
                    if let crate::shell::layer::Flow::Push(layer) = flow {
                        shell.push_layer(layer);
                    }
                }
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
            // AtividadeNaBarra (UX-006): `atividade-<estado>` = giro, download, concluida, erro, varias, pronta, lista; `aviso-concluida`
            // e `aviso-erro` deixam o Aviso do resultado na tela; `erro-lista` abre o popover com o erro.
            Ok(p) if p.starts_with("atividade-") || p.starts_with("aviso-") || p == "erro-lista" => {
                use tako_core::activities::Kind;
                let now = Instant::now();
                let list = &mut shell.ctx.activity.list;
                let page = p.strip_prefix("atividade-").unwrap_or(p);
                let spin = |l: &mut tako_core::activities::Activities| {
                    l.start("library", Kind::LibraryRefresh, "Atualizando biblioteca", "Steam · 3 de 5 lojas", now)
                };
                let download = |l: &mut tako_core::activities::Activities| {
                    l.start("download:1", Kind::Download, "Hades II", "", now);
                    l.progress("download:1", Some(42), "42% · faltam 4 min");
                };
                match page {
                    "giro" => spin(list),
                    "download" => download(list),
                    "concluida" | "aviso-concluida" => {
                        spin(list);
                        list.finish("library", "Biblioteca atualizada", "4 jogos novos", now);
                    }
                    "erro" | "aviso-erro" | "erro-lista" => {
                        // O caminho de produção: a busca explícita que falha, com o menu fechado.
                        shell.ctx.app_update.menu_open = false;
                        crate::app_updates::finish_check(
                            &mut shell.ctx,
                            crate::app_updates::Check::Failed,
                            true,
                            "updates.desktop.connection",
                        );
                    }
                    "pronta" => shell.ctx.update_ready = Some("snapshot-only".into()),
                    _ => {
                        spin(list);
                        download(list);
                        list.start("covers", Kind::Covers, "Buscando capas", "12 de 24 jogos", now);
                        list.progress("covers", None, "12 de 24 jogos");
                        list.start("cloud", Kind::CloudSync, "Sincronizando a nuvem", "Hades II", now);
                        list.finish("cloud", "Nuvem sincronizada", "Hades II", now);
                    }
                }
                if page == "pronta" || page.starts_with("aviso-") {
                    crate::shell::activities::tick(&mut shell.ctx, now);
                }
                if matches!(page, "varias" | "lista" | "erro-lista") {
                    shell.click("desk-activity", 0);
                    if page == "varias" {
                        shell.close_layers();
                    }
                }
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
            // `playing`: o primeiro jogo aberto há 42 minutos (EstadoJogando); `opening`, ainda abrindo.
            Ok("opening") | Ok("opening-slow") | Ok("playing") => {
                let g = shell.ctx.library.games()[0].clone();
                let playing = std::env::var("TAKO_SNAP_PAGE").as_deref() == Ok("playing");
                shell.ctx.running = Some(crate::shell::ctx::RunningGame {
                    id: g.id,
                    title: g.title,
                    pid: None,
                    started: playing.then(|| Instant::now() - std::time::Duration::from_secs(42 * 60)),
                });
                shell.ctx.launching = !playing;
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
                shell.open_route("settings:metadata");
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
            Ok("search-panel") => {
                shell.open_route(crate::shell::profile::ROUTE);
                shell.click("global-search", 0);
                shell.edited("overlay-search", "e");
            }
            Ok("restore-confirm") => {
                shell.open_route("settings");
                if let Some(index) = crate::shell::settings::categories()
                    .iter()
                    .position(|item| item.id == "system")
                {
                    shell.click("category", index);
                }
                let backup_path = shell
                    .ctx
                    .paths
                    .root
                    .join("tako-backup-2026-10-19.takobackup");
                let _ = std::fs::write(&backup_path, vec![0u8; 12 * 1024 * 1024]);
                let layer = crate::shell::settings::confirm_restore_layer(&shell.ctx, backup_path);
                shell.push_layer(Box::new(layer));
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
            match std::env::var("TAKO_SNAP_PAGE").as_deref() {
                Ok("toast-success") => shell.ctx.toast_success(
                    "Backup criado com sucesso e salvo na pasta escolhida para esta instalação do Tako",
                ),
                Ok("toast-error") => shell.ctx.toast_error(
                    "Falha ao restaurar o backup: o arquivo está incompleto e a cópia anterior não pôde ser validada",
                ),
                _ => {}
            }
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
                // Visão geral: `TAKO_SNAP_ART` e `TAKO_SNAP_LOGO` vão para o destaque dela; `TAKO_SNAP_COVER=<imagem>` vira a
                // capa de todas as linhas.
                if let ("desk-overview", Value::Struct(s)) = (*name, &mut value) {
                    // `TAKO_SNAP_BRIGHT=1`: arte clara no destaque (EstadoArteClara).
                    if std::env::var("TAKO_SNAP_BRIGHT").is_ok() {
                        s.set_field("hero-bright".into(), Value::Bool(true));
                    }
                    if std::env::var("TAKO_SNAP_LOADING").is_ok() {
                        s.set_field("loading".into(), Value::Bool(true));
                        for key in ["back", "attention", "installed", "week-groups"] {
                            s.set_field(key.into(), crate::values::model(Vec::new()));
                        }
                    }
                    if let Some(Value::Struct(mut hero)) = s.get_field("hero").cloned() {
                        if let Some((art, _, _)) = art {
                            hero.set_field("art".into(), Value::Image(art.clone()));
                        }
                        if let Some(p) = std::env::var("TAKO_SNAP_LOGO").ok().filter(|p| !p.is_empty()) {
                            let logo = to_image(&image::open(p).unwrap().to_rgba8());
                            hero.set_field("logo".into(), Value::Image(logo));
                            hero.set_field("show-logo".into(), Value::Bool(true));
                        }
                        s.set_field("hero".into(), Value::Struct(hero));
                    }
                    if let Some(p) = std::env::var("TAKO_SNAP_COVER").ok().filter(|p| !p.is_empty()) {
                        let cover = to_image(&image::open(p).unwrap().to_rgba8());
                        let with_cover = |v: &Value| match v {
                            Value::Struct(item) => {
                                let mut item = item.clone();
                                item.set_field("thumb".into(), Value::Image(cover.clone()));
                                Value::Struct(item)
                            }
                            other => other.clone(),
                        };
                        for key in ["back", "attention", "installed"] {
                            if let Some(Value::Model(m)) = s.get_field(key) {
                                let items: Vec<Value> = slint::Model::iter(m).map(|v| with_cover(&v)).collect();
                                s.set_field(key.into(), crate::values::model(items));
                            }
                        }
                        if let Some(best) = s.get_field("best").cloned() {
                            s.set_field("best".into(), with_cover(&best));
                        }
                    }
                }
                // `TAKO_SNAP_BRIGHT=1`: arte clara também na página do jogo.
                if let ("desk-page", Value::Struct(s)) = (*name, &mut value)
                    && std::env::var("TAKO_SNAP_BRIGHT").is_ok()
                {
                    s.set_field("bright".into(), Value::Bool(true));
                }
                if std::env::var("TAKO_SNAP_RICH").is_ok() {
                    match (*name, &mut value) {
                        ("desk-game", Value::Struct(s)) => {
                            s.set_field("trailer".into(), Value::Bool(true));
                        }
                        ("desk-page", Value::Struct(s)) => {
                            if let Some((art, ..)) = art {
                                s.set_field(
                                    "shots".into(),
                                    crate::values::model(vec![Value::Image(art.clone())]),
                                );
                            }
                            let modes = ["Um jogador", "Cooperativo", "Online"];
                            let features = modes.iter().map(|m| Value::String((*m).into()));
                            s.set_field("features".into(), crate::values::model(features.collect()));
                        }
                        _ => {}
                    }
                }
                // `TAKO_SNAP_ABOUT=<n>`: n parágrafos no Sobre da página do jogo (a rolagem e os dados fixos).
                if let ("desk-page", Value::Struct(s)) = (*name, &mut value)
                    && let Some(n) = std::env::var("TAKO_SNAP_ABOUT")
                        .ok()
                        .and_then(|n| n.parse::<usize>().ok())
                {
                    let text = "Kratos e Atreus partem numa jornada mítica atrás de respostas antes do Ragnarök, enfrentando deuses e monstros dos Nove Reinos enquanto as forças de Odin se preparam para a batalha profetizada que vai acabar com o mundo.";
                    let about = (0..n).map(|_| Value::String(text.into())).collect();
                    s.set_field("about".into(), crate::values::model(about));
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
            // O que o `App` entrega ao `Ui` fora do `render`.
            crate::desk_theme::apply(&ui, &shell.ctx.settings, art_accent);
            let motion = shell
                .ctx
                .settings
                .bool(crate::shell::ctx::keys::REDUCED_MOTION, false);
            crate::bridge::set_global(&ui, "Ui", "reduce-motion", Value::Bool(motion));
            let flat = (shell.ctx.settings).bool(crate::shell::ctx::keys::REDUCED_TRANSPARENCY, false);
            crate::bridge::set_global(&ui, "Ui", "reduce-transparency", Value::Bool(flat));
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
            // `TAKO_SNAP_PAGES=2`: com a página já na tela, Page Down duas vezes antes da captura (a página do jogo
            // rolada; negativo = Page Up).
            if let Some(n) = std::env::var("TAKO_SNAP_PAGES")
                .ok()
                .and_then(|n| n.parse::<i32>().ok())
            {
                let key = if n > 0 { "\u{F72D}" } else { "\u{F72C}" };
                for _ in 0..n.abs() {
                    shell.key(key, tako_core::input::shortcut::KeyModifiers::NONE);
                }
                apply(&mut shell, art.as_ref());
                for _ in 0..wait / 50 {
                    slint::platform::update_timers_and_animations();
                    ui.window().request_redraw();
                    window.draw_if_needed(|r| {
                        r.render(&mut scratch, w as usize);
                    });
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            // `TAKO_SNAP_CMDS=tab,down,right`: teclas antes da captura (foco do corpo da página do jogo). Use um tamanho por
            // execução: o estado do teclado se acumula de um tamanho para o outro.
            if let Ok(cmds) = std::env::var("TAKO_SNAP_CMDS") {
                for name in cmds.split(',') {
                    match name {
                        "tab" => {
                            shell.key("\u{0009}", tako_core::input::shortcut::KeyModifiers::NONE);
                        }
                        "up" => shell.handle(Cmd::Up),
                        "down" => shell.handle(Cmd::Down),
                        "left" => shell.handle(Cmd::Left),
                        "right" => shell.handle(Cmd::Right),
                        "accept" => shell.handle(Cmd::Accept),
                        _ => {}
                    }
                }
                apply(&mut shell, art.as_ref());
                for _ in 0..wait / 50 {
                    slint::platform::update_timers_and_animations();
                    ui.window().request_redraw();
                    window.draw_if_needed(|r| {
                        r.render(&mut scratch, w as usize);
                    });
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            slint::platform::update_timers_and_animations();
            let mut pixels = vec![Rgb8Pixel::default(); (w * h) as usize];
            ui.window().request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut pixels, w as usize);
            });
            let raw: Vec<u8> = pixels.iter().flat_map(|p| [p.r, p.g, p.b]).collect();
            let file_prefix = std::env::var("TAKO_SNAP_NAME").unwrap_or_else(|_| "desktop".into());
            let file_suffix = std::env::var("TAKO_SNAP_FILE_SUFFIX").unwrap_or_default();
            image::RgbImage::from_raw(w, h, raw)
                .unwrap()
                .save(format!("{out_dir}/{file_prefix}{file_suffix}-{w}x{h}.png"))
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
            } else if let Some(route) = switch.as_deref().and_then(|s| s.strip_prefix("route=")) {
                // `TAKO_SNAP_SWITCH=route=settings:<categoria>/<linha>`: abre por um link (a linha pisca nos quadros).
                shell.open_route(route);
                apply(&mut shell, art.as_ref());
                capture("s", &mut pixels);
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
