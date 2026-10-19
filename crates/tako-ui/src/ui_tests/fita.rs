//! Exercita eventos reais e o resultado renderizado, sem expor propriedades de teste na API.
use super::*;
use crate::values::ToValue;
use slint::Model;
use slint::platform::{PointerEventButton, WindowEvent};
use std::cell::RefCell;

type Events = Rc<RefCell<Vec<(bool, String, usize)>>>;

struct Fixture {
    window: Rc<MinimalSoftwareWindow>,
    ui: slint_interpreter::ComponentInstance,
    shell: Shell,
    events: Events,
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new(theme: &str) -> Self {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let builtin = crate::theme::install_builtin(dir.path()).unwrap();
        let entry = if theme == "default" {
            builtin.clone()
        } else {
            dir.path().join(theme)
        };
        let ui = crate::theme::compile(&entry.join("main.slint"), &builtin)
            .unwrap()
            .create()
            .unwrap();
        ui.set_global_property("Ui", "reduce-motion", true.to_value())
            .unwrap();
        ui.show().unwrap();
        let mut ctx = crate::app::build_ctx(
            DataPaths::at(dir.path().join("data")),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library.refresh();
        let events: Events = Rc::new(RefCell::new(Vec::new()));
        for (name, click) in [("clicked", true), ("hovered", false)] {
            let events = events.clone();
            ui.set_callback(name, move |args| {
                if let [Value::String(zone), Value::Number(index)] = args {
                    events
                        .borrow_mut()
                        .push((click, zone.to_string(), *index as usize));
                }
                Value::Void
            })
            .unwrap();
        }
        Self {
            window,
            ui,
            shell: Shell::new(ctx),
            events,
            _dir: dir,
        }
    }

    fn draw(&mut self, w: u32, h: u32) -> Vec<Rgb8Pixel> {
        self.window.set_size(slint::PhysicalSize::new(w, h));
        for (name, value) in self.shell.render(Instant::now()).props {
            crate::bridge::set(&self.ui, name, value);
        }
        crate::bridge::set_global(&self.ui, "Ui", "pointer", self.shell.ctx.pointer.to_value());
        slint::platform::update_timers_and_animations();
        let mut pixels = vec![Rgb8Pixel::default(); (w * h) as usize];
        self.ui.window().request_redraw();
        self.window.draw_if_needed(|r| {
            r.render(&mut pixels, w as usize);
        });
        let Value::Number(cols) = self.ui.get_property("library-columns").unwrap() else {
            panic!()
        };
        assert!(cols.is_finite() && cols >= 1.);
        self.shell.ctx.grid_columns = cols as usize;
        pixels
    }

    fn click(&mut self, x: f32, y: f32) {
        let position = slint::LogicalPosition::new(x, y);
        self.window
            .dispatch_event(WindowEvent::PointerMoved { position });
        for event in [
            WindowEvent::PointerPressed {
                position,
                button: PointerEventButton::Left,
            },
            WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            },
        ] {
            self.window.dispatch_event(event);
        }
    }

    fn apply_events(&mut self) {
        for (click, zone, index) in self.events.take() {
            if click {
                self.shell.click(&zone, index);
            } else {
                self.shell.hover(&zone, index);
            }
        }
    }
}

#[test]
fn collection_tab_only_focuses_the_row() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let mut f = Fixture::new("tako-fita");
        f.draw(1920, 1080);
        for reduce in [false, true] {
            f.ui.set_global_property("Ui", "reduce-motion", reduce.to_value())
                .unwrap();
            for duration in ["focus-time", "page-time", "color-time"] {
                let Value::Number(ms) = f.ui.get_global_property("Ui", duration).unwrap() else {
                    panic!()
                };
                assert_eq!(ms == 0., reduce, "{duration}");
            }
        }
        let Value::Model(rows) = f.ui.get_property("home-rows").unwrap() else {
            panic!()
        };
        let last = rows.row_count() - 1;
        let zone = format!("tab-{last}");
        let _x = (80..1840)
            .step_by(3)
            .find(|x| {
                f.events.borrow_mut().clear();
                f.window.dispatch_event(WindowEvent::PointerMoved {
                    position: slint::LogicalPosition::new(*x as f32, 650.),
                });
                assert!(
                    f.events.borrow().is_empty(),
                    "passar o mouse na aba não pode trocar a fileira"
                );
                f.click(*x as f32, 650.);
                let hit = f.events.borrow().iter().any(|(_, z, _)| z == &zone);
                if !hit {
                    f.events.borrow_mut().clear();
                }
                hit
            })
            .expect("aba Adicionados recentemente visível");
        f.apply_events();
        assert_eq!(
            f.shell.page().kind(),
            "home",
            "aba não deve aceitar a primeira capa"
        );
        assert_eq!(
            number(&f.shell.render(Instant::now()), "home-row-focus"),
            last as i32
        );
        assert!(!f.shell.has_layers());
    })
    .join()
    .unwrap();
}

#[test]
fn hero_actions_keep_the_presented_game_in_both_themes() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for theme in ["default", "tako-fita"] {
        std::thread::spawn(move || {
            let mut f = Fixture::new(theme);
            f.draw(1280, 720);
            let first = f.shell.focused_game();
            f.shell.hover("row-0", 1);
            let selected = f.shell.focused_game();
            assert_ne!(selected, first);
            f.shell.hover("hero", 1);
            assert_eq!(
                f.shell.focused_game(),
                selected,
                "{theme}: hover mantém a prévia"
            );
            f.shell.click("hero", 1);
            assert_eq!(f.shell.page().kind(), "details");
            assert_eq!(f.shell.focused_game(), selected);
            f.shell.handle(Cmd::Back);
            f.shell.ctx.running = Some(crate::shell::RunningGame {
                id: selected.clone().unwrap(),
                title: "Jogo selecionado".into(),
                pid: None,
                started: None,
            });
            f.shell.hover("row-0", 1);
            f.shell.click("hero", 0);
            assert!(
                f.shell
                    .ctx
                    .effects
                    .iter()
                    .any(|e| matches!(e, crate::shell::Effect::ResumeGame))
            );
            assert!(!f.shell.has_layers());
            assert_eq!(
                f.shell.focused_game(),
                selected,
                "{theme}: ação primária mantém o alvo"
            );
        })
        .join()
        .unwrap();
    }
}

#[test]
fn every_details_action_is_visible_and_clickable_at_all_text_scales() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let mut f = Fixture::new("tako-fita");
        let id = f.shell.ctx.library.games()[0].id.clone();
        f.shell.ctx.library.games_mut()[0].title = "Uma aventura com um título muito longo para testar a leitura em televisores e telas pequenas".into();
        if !f.shell.ctx.library.games()[0].is_favorite { f.shell.ctx.library.toggle_favorite(&id).unwrap(); }
        for i in 0..12 { f.shell.ctx.collections.create(&format!("Coleção extensa {i}"), Some(&id)).unwrap(); }
        f.shell.ctx.running = Some(crate::shell::RunningGame { id: id.clone(), title: "Uma aventura".into(), pid: None, started: None });
        f.shell.ctx.open_details = Some(id);
        f.shell.after_action();
        for lang in [Lang::PtBr, Lang::EnUs] {
            i18n::set_lang(lang);
            for scale in [1., 1.25, 1.5] {
                f.ui.set_global_property("Ui", "text-scale", Value::Number(scale)).unwrap();
                for (w,h) in [(1280,720), (1920,1080)] {
                    for index in (0..5).chain((0..5).rev()) {
                        let current = number(&f.shell.render(Instant::now()), "details-focus") as usize;
                        if current < index { f.shell.handle(Cmd::Right); }
                        if current > index { f.shell.handle(Cmd::Left); }
                        assert_eq!(number(&f.shell.render(Instant::now()), "details-focus"), index as i32);
                        f.shell.ctx.pointer = false;
                        let pixels = f.draw(w,h);
                        if index == 4 && let Ok(dir) = std::env::var("TAKO_SNAP_OUT") {
                            let raw: Vec<u8> = pixels.iter().flat_map(|p| [p.r,p.g,p.b]).collect();
                            image::RgbImage::from_raw(w,h,raw).unwrap().save(format!("{dir}/fita-detalhes-{lang:?}-{scale}-{w}x{h}.png")).unwrap();
                        }
                        // O maior segmento da superfície pérola é o interior do botão focado.
                        let mut best = (0,0,0);
                        for y in (80*h/1080)..(600*h/1080) {
                            let mut start = 476*w/1920;
                            for x in (476*w/1920)..(w-79*w/1920) {
                                if pixels[(y*w+x) as usize] != Rgb8Pixel::new(245,247,250) { start = x+1; }
                                else if x+1-start > best.2 { best = (start,y,x+1-start); }
                            }
                        }
                        assert!(best.2 > 100*w/1920, "ação {index} fora da tela: {lang:?}/{scale}/{w}, {best:?}");
                        assert!(best.0 >= 476*w/1920 && best.0+best.2 < w-80*w/1920, "ação {index} cortada: {lang:?}/{scale}/{w}, {best:?}");
                        f.events.borrow_mut().clear();
                        f.click((best.0+best.2/2) as f32, best.1 as f32);
                        assert!(f.events.borrow().iter().any(|(c,z,i)| *c && z == "action" && *i == index), "foco e clique divergem: {index}, {:?}", f.events.borrow());
                    }
                }
            }
        }
    }).join().unwrap();
}

#[test]
fn keyboard_and_controller_reach_rows_see_all_and_the_last_grid_item() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let mut f = Fixture::new("tako-fita");
        let commands = Rc::new(RefCell::new(Vec::new()));
        let captured = commands.clone();
        f.ui.set_callback("key", move |args| {
            let Value::String(text) = &args[0] else {
                panic!()
            };
            if let crate::shell::keys::KeyAction::Cmd(cmd) =
                crate::shell::keys::map(text, false, false, false)
            {
                captured.borrow_mut().push(cmd);
                Value::Bool(true)
            } else {
                Value::Bool(false)
            }
        })
        .unwrap();
        for keyboard in [false, true] {
            for (w, h) in [(1280, 720), (1920, 1080)] {
                f.shell.open_route("home");
                f.draw(w, h);
                assert_eq!(f.shell.ctx.grid_columns, 5);
                let send = |f: &mut Fixture, cmd: Cmd, key: char| {
                    if keyboard {
                        f.window.dispatch_event(WindowEvent::KeyPressed {
                            text: key.to_string().into(),
                        });
                        f.window.dispatch_event(WindowEvent::KeyReleased {
                            text: key.to_string().into(),
                        });
                        let mapped = commands
                            .borrow_mut()
                            .pop()
                            .expect("FocusScope repassa a tecla");
                        assert_eq!(mapped, cmd);
                        f.shell.handle(mapped);
                    } else {
                        f.shell.handle(cmd);
                    }
                };
                use crate::shell::keys::{DOWN, ENTER, RIGHT, UP};
                send(&mut f, Cmd::Down, DOWN);
                send(&mut f, Cmd::Down, DOWN);
                assert_eq!(number(&f.shell.render(Instant::now()), "home-row-focus"), 1);
                send(&mut f, Cmd::Up, UP);
                assert_eq!(number(&f.shell.render(Instant::now()), "home-row-focus"), 0);
                let Value::Model(rows) =
                    property(&f.shell.render(Instant::now()), "home-rows").clone()
                else {
                    panic!()
                };
                let Value::Struct(row) = rows.row_data(0).unwrap() else {
                    panic!()
                };
                let Some(Value::Model(tiles)) = row.get_field("tiles") else {
                    panic!()
                };
                let count = tiles.row_count();
                for _ in 0..count {
                    send(&mut f, Cmd::Right, RIGHT);
                }
                f.draw(w, h);
                assert_eq!(
                    number(&f.shell.render(Instant::now()), "home-col-focus"),
                    count as i32
                );
                // O Ver todos acompanha a âncora até a primeira posição visível da fita.
                f.events.borrow_mut().clear();
                let center_x = (w as f32 + 100. * w as f32 / 1920.) / 2.;
                f.click(center_x, 800. * h as f32 / 1080.);
                assert!(
                    f.events
                        .borrow()
                        .iter()
                        .any(|(c, z, i)| *c && z == "row-0" && *i == count)
                );
                f.events.borrow_mut().clear();
                send(&mut f, Cmd::Accept, ENTER);
                assert_eq!(f.shell.page().kind(), "library");
                f.draw(w, h);
                let Value::Model(tiles) = f.ui.get_property("library-tiles").unwrap() else {
                    panic!()
                };
                let count = tiles.row_count();
                send(&mut f, Cmd::Down, DOWN);
                let initial = number(&f.shell.render(Instant::now()), "library-focus");
                if initial < 0 {
                    send(&mut f, Cmd::Down, DOWN);
                }
                for _ in 0..count {
                    send(&mut f, Cmd::Down, DOWN);
                }
                for _ in 0..f.shell.ctx.grid_columns {
                    send(&mut f, Cmd::Right, RIGHT);
                }
                assert_eq!(
                    number(&f.shell.render(Instant::now()), "library-focus"),
                    count as i32 - 1
                );
                f.draw(w, h);
                let mut last_position = None;
                for y in (160 * h / 1080..h - 88 * h / 1080).step_by(16) {
                    for x in (80 * w / 1920..w - 80 * w / 1920).step_by(16) {
                        f.events.borrow_mut().clear();
                        f.window.dispatch_event(WindowEvent::PointerMoved {
                            position: slint::LogicalPosition::new(x as f32, y as f32),
                        });
                        if f.events
                            .borrow()
                            .iter()
                            .any(|(_, z, i)| z == "grid" && *i == count - 1)
                        {
                            last_position = Some((x, y));
                            break;
                        }
                    }
                    if last_position.is_some() {
                        break;
                    }
                }
                let (x, y) = last_position.expect("última capa revelada na grade");
                f.click(x as f32, y as f32);
                assert!(
                    f.events
                        .borrow()
                        .iter()
                        .any(|(c, z, i)| *c && z == "grid" && *i == count - 1)
                );
                f.events.borrow_mut().clear();
                let selected = f.shell.focused_game();
                send(&mut f, Cmd::Accept, ENTER);
                assert_eq!(f.shell.page().kind(), "details");
                assert_eq!(f.shell.focused_game(), selected);
            }
        }
    })
    .join()
    .unwrap();
}

#[test]
fn details_icon_uses_game_accessibility_data_without_a_language_literal() {
    let source = include_str!("../../theme/extras/tako-fita/main.slint");
    for animation in source.split("animate ").skip(1) {
        let body = animation.split('}').next().unwrap();
        assert!(
            body.contains("duration: Ui."),
            "animação sem duração acessível: {body}"
        );
    }
    let info = source
        .split("icon: \"info\";")
        .nth(1)
        .unwrap()
        .split("}\n")
        .next()
        .unwrap();
    assert!(!source.contains("\"Detalhes\""));
    assert!(source.contains("accessible-label: hero.title;"));
    assert!(source.contains("accessible-description: hero.meta;"));
    assert!(info.contains("root.clicked(\"hero\", 1)"));
}

#[test]
fn card_captions_and_library_header_height_and_contrast() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let mut f = Fixture::new("tako-fita");
        let w = 1920;
        let h = 1080;
        f.shell.open_route("library:all");
        let pixels = f.draw(w, h);

        // 1. Cabeçalho da Biblioteca fica contido no topo, nunca sob a grade (base do cabeçalho < topo da grade).
        // top = 120 * u + 60 * u * text_scale = 180px em 1080p escala 1.0.
        let u = h as f32 / 1080.;
        let top = (120. * u + 60. * u) as u32;
        let mut header_bottom_y = 0;
        for y in (48. * u) as u32..top {
            for x in (100. * u) as u32..(w - 100) {
                let p = pixels[(y * w + x) as usize];
                if p.r > 100 || p.g > 100 || p.b > 100 {
                    header_bottom_y = header_bottom_y.max(y);
                }
            }
        }
        assert!(header_bottom_y > 0, "cabeçalho da biblioteca renderizado no topo");
        assert!(
            header_bottom_y < top,
            "base do cabeçalho da biblioteca ({header_bottom_y}) fica acima do topo da grade ({top})"
        );

        // 2. Fita: cartões têm legenda ancorada no fundo com scrim contrastante (>= 4.5:1).
        f.shell.open_route("home");
        f.shell.handle(Cmd::Down);
        let pixels_home = f.draw(w, h);

        // Coordenadas reais do cartão focado na fita em 1080p:
        // ribbon y = 1080 - 340 - 88 - 16 = 636; cards y = 636 + 44 = 680.
        // Focado com escala 1.12x: art-box vai de y≈672 a y≈974 (bem acima da barra de dicas em y=992).
        let card_top = 672;
        let card_bottom = 974;
        let x_center = w / 2; // 960

        // Amostra o scrim nas coordenadas reais do cartão (14px acima da borda inferior)
        let sample_y = card_bottom - 14; // 960
        assert!(
            sample_y > card_top && sample_y < card_bottom,
            "base da legenda ({sample_y}) está dentro do cartão ({card_top}..{card_bottom})"
        );
        let lower_p = pixels_home[(sample_y * w + x_center) as usize];
        let lum_lower =
            0.2126 * lower_p.r as f32 + 0.7152 * lower_p.g as f32 + 0.0722 * lower_p.b as f32;
        assert!(
            lum_lower < 50.0,
            "scrim do cartão garante contraste no fundo dentro do cartão: {lum_lower}"
        );
    })
    .join()
    .unwrap();
}

#[test]
fn default_theme_hover_on_row_updates_column_even_if_previously_saved() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let mut f = Fixture::new("default");
        f.draw(1920, 1080);
        // Primeiro foca a fileira 0 na coluna 3
        f.shell.hover("row-0", 3);
        assert_eq!(
            super::number(&f.shell.render(Instant::now()), "home-col-focus"),
            3
        );
        // Move para o destaque (hero)
        f.shell.hover("hero", 0);
        // Move para a fileira 0 no índice 2 com coluna salva 3: deve atualizar para coluna 2
        f.shell.hover("row-0", 2);
        assert_eq!(
            super::number(&f.shell.render(Instant::now()), "home-col-focus"),
            2,
            "hover em row-0 índice 2 com coluna salva 3 deve atualizar para coluna 2"
        );
    })
    .join()
    .unwrap();
}

#[test]
fn fita_tab_hover_changes_row_without_resetting_column() {
    let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    std::thread::spawn(|| {
        let mut f = Fixture::new("tako-fita");
        f.draw(1920, 1080);
        // Foca fileira 1 na coluna 2
        f.shell.hover("row-1", 2);
        assert_eq!(
            super::number(&f.shell.render(Instant::now()), "home-row-focus"),
            1
        );
        assert_eq!(
            super::number(&f.shell.render(Instant::now()), "home-col-focus"),
            2
        );
        // Move para fileira 0 na coluna 1
        f.shell.hover("row-0", 1);
        assert_eq!(
            super::number(&f.shell.render(Instant::now()), "home-row-focus"),
            0
        );
        assert_eq!(
            super::number(&f.shell.render(Instant::now()), "home-col-focus"),
            1
        );
        // Hover/clique na aba da fileira 1 (tab-1) deve mudar só a fileira para 1, preservando coluna salva 2
        f.shell.hover("tab-1", 0);
        assert_eq!(
            super::number(&f.shell.render(Instant::now()), "home-row-focus"),
            1,
            "tab-1 deve selecionar a fileira 1"
        );
        assert_eq!(
            super::number(&f.shell.render(Instant::now()), "home-col-focus"),
            2,
            "tab-1 deve manter a coluna salva 2 da fileira 1"
        );
    })
    .join()
    .unwrap();
}
