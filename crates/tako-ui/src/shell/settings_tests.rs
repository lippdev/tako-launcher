use super::*;
use crate::shell::Layer;
use tako_core::input::GamepadButtons as B;
use tako_core::input::shortcut::chord;
use tako_core::library::GameStore;
use tako_core::paths::DataPaths;
use tako_platform::Platform;

use crate::LANG_LOCK as LOCK;

fn ctx() -> (tempfile::TempDir, Ctx) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
    i18n::set_lang(Lang::PtBr);
    (dir, ctx)
}

fn rows_of(ctx: &Ctx, id: &str) -> Vec<MenuItem> {
    let cat = categories().into_iter().find(|c| c.id == id).unwrap();
    (cat.rows)(ctx)
}

fn find(items: &[MenuItem], key: &str) -> MenuItem {
    items
        .iter()
        .find(|i| i.row.key == key)
        .cloned()
        .unwrap_or_else(|| panic!("sem a linha {key}"))
}

fn accept(ctx: &mut Ctx, item: &MenuItem) -> Flow {
    (item.accept.clone().unwrap())(ctx)
}

fn adjust(ctx: &mut Ctx, item: &MenuItem, dir: i32) {
    let _ = (item.adjust.clone().unwrap())(ctx, dir);
}

#[test]
fn navigates_between_categories_and_rows() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    let mut page = SettingsPage::new();
    let n = page.cats.len();
    assert_eq!(n, 8);
    page.handle(Cmd::NextSection, &mut ctx);
    assert_eq!(page.cats[page.cat].id, "library");
    page.handle(Cmd::PrevSection, &mut ctx);
    page.handle(Cmd::PrevSection, &mut ctx);
    assert_eq!(page.cats[page.cat].id, "about");
    page.handle(Cmd::Up, &mut ctx);
    assert_eq!(page.cats[page.cat].id, "system");
    // A entra nas linhas, pulando o título "Diagnóstico".
    page.handle(Cmd::Accept, &mut ctx);
    assert_eq!(page.row, Some(1));
    page.handle(Cmd::Down, &mut ctx);
    assert_eq!(page.row, Some(2));
    page.handle(Cmd::Back, &mut ctx);
    assert_eq!(page.row, None);
    assert!(matches!(page.handle(Cmd::Back, &mut ctx), Nav::Sidebar));
    assert_eq!(
        SettingsPage::at("controls").cats[SettingsPage::at("controls").cat].id,
        "controls"
    );
}

#[test]
fn text_size_and_navigation_volume_persist_and_are_bounded() {
    let dir = tempfile::tempdir().unwrap();
    let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
    let items = rows_of(&ctx, "appearance");
    let text = find(&items, "text-size");
    let sound = find(&items, "navigation-volume");
    adjust(&mut ctx, &text, -1);
    assert_eq!(crate::navigation_feedback::text_size(&ctx.settings), 125);
    for _ in 0..20 {
        adjust(&mut ctx, &sound, 1);
    }
    assert_eq!(crate::navigation_feedback::volume(&ctx.settings), 100);
    for _ in 0..8 {
        adjust(&mut ctx, &sound, -1);
    }
    drop(ctx);
    let ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), false);
    assert_eq!(crate::navigation_feedback::text_size(&ctx.settings), 125);
    assert_eq!(crate::navigation_feedback::volume(&ctx.settings), 20);
}

/// BarraLateralUnida: desligada e Normal por padrão (perfil sem a preferência); as duas escolhas ficam no perfil.
#[test]
fn sidebar_layout_defaults_off_and_normal_and_persists_per_profile() {
    let dir = tempfile::tempdir().unwrap();
    let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
    ctx.desktop = true;
    assert!(!ctx.settings.bool(keys::SIDEBAR_UNIFIED, false));
    assert_eq!(ctx.settings.string(keys::SIDEBAR_WIDTH), "");
    let items = rows_of(&ctx, "appearance");
    let unified = find(&items, "settings.appearance.sidebar_unified");
    let width = find(&items, "sidebar-width");
    assert!(!unified.row.on);
    assert_eq!(width.row.value, ctx.t("settings.appearance.sidebar_normal"));
    accept(&mut ctx, &unified);
    let mut layer = width.select(&ctx).expect("select");
    let _ = layer.click("row", 1, &mut ctx);
    drop(ctx);
    let mut ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), false);
    ctx.desktop = true;
    assert!(ctx.settings.bool(keys::SIDEBAR_UNIFIED, false));
    assert_eq!(ctx.settings.string(keys::SIDEBAR_WIDTH), "narrow");
    let items = rows_of(&ctx, "appearance");
    assert!(find(&items, "settings.appearance.sidebar_unified").row.on);
    assert_eq!(
        find(&items, "sidebar-width").row.value,
        ctx.t("settings.appearance.sidebar_narrow")
    );
}

#[test]
fn toggles_persist_in_settings() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    let items = rows_of(&ctx, "appearance");
    let motion = find(&items, "settings.appearance.motion");
    assert!(!motion.row.on);
    accept(&mut ctx, &motion);
    assert!(ctx.settings.bool(keys::REDUCED_MOTION, false));
    assert!(
        find(&rows_of(&ctx, "appearance"), "settings.appearance.motion")
            .row
            .on
    );
    // Cor dinâmica começa ligada.
    let dynamic = find(&items, "settings.appearance.dynamic");
    assert!(dynamic.row.on);
    accept(&mut ctx, &dynamic);
    assert!(!ctx.settings.bool(keys::DYNAMIC_COLOR, true));

    // Atualizações vão para o updates.json.
    let auto = find(&rows_of(&ctx, "general"), "settings.general.autoupdate");
    assert!(auto.row.on);
    accept(&mut ctx, &auto);
    assert!(!UpdatePreferences::load(&ctx.paths.updates()).auto_update);

    // Opções de desenvolvedor mostram "Recarregar tema".
    let has_reload = |ctx: &Ctx| {
        rows_of(ctx, "system")
            .iter()
            .any(|i| i.row.key == "settings.system.reload_theme")
    };
    assert!(!has_reload(&ctx));
    let dev = find(&rows_of(&ctx, "system"), "settings.system.developer");
    accept(&mut ctx, &dev);
    assert!(has_reload(&ctx));
    let reload = find(&rows_of(&ctx, "system"), "settings.system.reload_theme");
    accept(&mut ctx, &reload);
    assert_eq!(
        ctx.effects.last(),
        Some(&Effect::ApplyTheme("default".into()))
    );
}

#[test]
fn store_toggle_updates_disabled_stores() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(library::toggle_store("Epic, Gog", GameStore::Gog), "Epic");
    assert_eq!(
        library::toggle_store("Epic", GameStore::Steam),
        "Epic,Steam"
    );
    assert_eq!(library::toggle_store("", GameStore::Steam), "Steam");

    let (_d, mut ctx) = ctx();
    let steam = find(&rows_of(&ctx, "library"), "store:Steam");
    assert!(steam.row.on);
    accept(&mut ctx, &steam);
    assert_eq!(ctx.settings.string(keys::DISABLED_STORES), "Steam");
    assert_eq!(ctx.effects.last(), Some(&Effect::ReloadLibrary));
    let steam = find(&rows_of(&ctx, "library"), "store:Steam");
    assert!(!steam.row.on);
    assert!(!steam.row.description.is_empty());
    accept(&mut ctx, &steam);
    assert_eq!(ctx.settings.string(keys::DISABLED_STORES), "");
}

#[test]
fn language_switch_changes_texts() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    let mut page = SettingsPage::new();
    let heading = |page: &mut SettingsPage, ctx: &Ctx| {
        let mut out = Out::default();
        page.render(ctx, &mut out);
        out.props
            .into_iter()
            .find(|(n, _)| *n == "settings-heading")
            .map(|(_, v)| format!("{v:?}"))
            .unwrap()
    };
    assert!(heading(&mut page, &ctx).contains("Geral"));
    let lang = find(&rows_of(&ctx, "general"), "language");
    assert_eq!(lang.row.value, "Português (Brasil)");
    adjust(&mut ctx, &lang, 1);
    let after = (
        heading(&mut page, &ctx),
        i18n::lang(),
        ctx.settings.string(keys::LANGUAGE),
    );
    i18n::set_lang(Lang::PtBr);
    assert!(after.0.contains("General"), "{}", after.0);
    assert_eq!(after.1, Lang::EnUs);
    assert_eq!(after.2, "en-US");
}

/// Nenhuma linha mostra a chave crua (texto faltando no catálogo), nos dois idiomas.
#[test]
fn every_row_has_text() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    ctx.settings.set_bool(keys::DEVELOPER, true);
    ctx.settings.set(keys::GAME_SCREEN_ID, "DISPLAY2");
    ctx.collections.create("Favoritos do sofá", None).unwrap();
    ctx.pad_name = "Controle".into();
    for lang in [Lang::PtBr, Lang::EnUs] {
        i18n::set_lang(lang);
        let mut overlays = Vec::new();
        for cat in categories() {
            for f in ["title", "subtitle"] {
                let k = format!("settings.{}.{f}", cat.id);
                assert_ne!(ctx.t(&k), k);
            }
            for item in (cat.rows)(&ctx) {
                overlays.push(crate::model::Overlay {
                    rows: vec![item.row.clone()],
                    ..Default::default()
                });
                // Painéis e diálogos que as linhas abrem.
                if let Some(f) = item.accept.clone()
                    && let Flow::Push(l) = f(&mut ctx)
                {
                    overlays.push(l.render(&ctx));
                }
            }
        }
        overlays.push(controls::ChordLayer::new(&ctx).render(&ctx));
        overlays.push(controls::TestLayer.render(&ctx));
        for o in overlays {
            for text in [&o.title, &o.text, &o.overline] {
                assert!(!text.contains("settings."), "{lang:?}: {text}");
            }
            for r in o.rows {
                for text in [&r.title, &r.description, &r.value] {
                    assert!(!text.starts_with("settings."), "{lang:?}: {text}");
                }
            }
        }
    }
    i18n::set_lang(Lang::PtBr);
}

#[test]
fn provider_keys_go_to_the_vault() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    metadata::update(&mut ctx, |p| {
        p.igdb.enabled = true;
        p.igdb.client_id = "abc".into();
        p.sgdb.api_key = "k".into();
    });
    forget("settings.providers");
    let p = metadata::providers(&ctx);
    assert!(p.igdb.enabled);
    assert_eq!(p.igdb.client_id, "abc");
    assert_eq!(p.sgdb.api_key, "k");
    assert!(ctx.platform.secrets.get("metadata.igdb").is_some());
    // Sem chave, "Testar" explica o que falta sem ir à rede.
    let igdb = find(&rows_of(&ctx, "metadata"), "settings.metadata.igdb");
    let mut panel = match accept(&mut ctx, &igdb) {
        Flow::Push(l) => l,
        _ => panic!("painel"),
    };
    let rows = panel.render(&ctx).rows;
    let test = rows.iter().position(|r| r.key == "test").unwrap();
    for _ in 0..test {
        panel.handle(Cmd::Down, &mut ctx);
    }
    panel.handle(Cmd::Accept, &mut ctx);
    let desc = &panel.render(&ctx).rows[test].description;
    assert_eq!(desc, &ctx.t("metadata.igdb.problem.missing_client_secret"));
}

#[test]
fn game_screen_choices_cycle() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    let display = find(&screen::rows(&ctx), "display");
    adjust(&mut ctx, &display, 1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_ID), "DISPLAY1");
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_NAME), "Monitor");
    adjust(&mut ctx, &display, 1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_ID), "DISPLAY2");
    adjust(&mut ctx, &display, -1);
    let mode = find(&screen::rows(&ctx), "mode");
    adjust(&mut ctx, &mode, 1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_MODE), "1920x1080@144");
    adjust(&mut ctx, &mode, -1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_MODE), "");
    // O monitor de mesa não tem HDR; a TV tem.
    assert!(
        !screen::rows(&ctx)
            .iter()
            .any(|i| i.row.key == "settings.screen.hdr")
    );
    let hide = find(&screen::rows(&ctx), "hide");
    assert!(hide.row.on);
    accept(&mut ctx, &hide);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_HIDE_OTHERS), "None");
    let audio = find(&screen::rows(&ctx), "audio");
    adjust(&mut ctx, &audio, 1);
    assert_eq!(ctx.settings.string(keys::GAME_AUDIO_NAME), "Alto-falantes");
}

#[test]
fn chord_capture_saves_the_held_buttons() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    ctx.input = crate::shell::InputKind::Gamepad;
    let mut layer = controls::ChordLayer::new(&ctx);
    layer.render(&ctx); // nada segurado: espera
    ctx.pressed_buttons = (B::VIEW | B::X).0;
    layer.render(&ctx);
    // B no meio da espera não fecha enquanto segura.
    assert!(matches!(layer.handle(Cmd::Back, &mut ctx), Flow::Stay));
    std::thread::sleep(std::time::Duration::from_millis(1050));
    layer.render(&ctx);
    assert!(
        matches!(layer.handle(Cmd::Accept, &mut ctx), Flow::Stay),
        "só salva depois de soltar"
    );
    ctx.pressed_buttons = 0;
    layer.render(&ctx);
    assert!(ctx.live_pad.get());
    assert!(matches!(layer.handle(Cmd::Accept, &mut ctx), Flow::Close));
    assert_eq!(
        chord::parse(ctx.settings.get(keys::CONTROL_CENTER_CHORD)),
        B::VIEW | B::X
    );
}

#[test]
fn key_capture_refuses_conflicts_and_saves_the_pressed_shortcut() {
    use tako_core::input::shortcut::{KeyModifiers as M, KeyShortcut};
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    let mut layer = controls::KeyLayer::default();
    // Ctrl sozinho só mostra o que está pressionado.
    assert!(matches!(
        layer.key("\u{11}", M::CTRL, &mut ctx),
        Some(Flow::Stay)
    ));
    assert_eq!(layer.render(&ctx).rows[1].value, "Ctrl+…");
    // Shift+Tab (Backtab no Slint) é da Steam: recusa e explica.
    assert!(matches!(
        layer.key("\u{19}", M::SHIFT, &mut ctx),
        Some(Flow::Stay)
    ));
    assert!(layer.render(&ctx).rows[1].description.contains("Steam"));
    assert!(ctx.settings.get(keys::CONTROL_CENTER_KEY).is_none());
    assert!(matches!(
        layer.key("t", M::CTRL | M::ALT, &mut ctx),
        Some(Flow::Close)
    ));
    assert_eq!(
        KeyShortcut::parse(ctx.settings.get(keys::CONTROL_CENTER_KEY)),
        KeyShortcut::new(M::CTRL | M::ALT, "T")
    );
    // Esc sozinho cancela e mantém o atalho.
    let mut again = controls::KeyLayer::default();
    assert!(matches!(
        again.key("\u{1b}", M::NONE, &mut ctx),
        Some(Flow::Close)
    ));
    assert_eq!(ctx.toast.as_ref().unwrap().0, "Atalho mantido · Ctrl+Alt+T");
}

#[test]
fn storage_lists_the_largest_games_and_sorts_by_size() {
    use tako_core::library::{LibraryCriteria, LibraryFilter, LibrarySort, query};
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    ctx.library.refresh();
    let biggest = ctx
        .library
        .games()
        .iter()
        .max_by_key(|g| g.size_bytes)
        .map(|g| g.id.clone())
        .unwrap();

    let row = find(&rows_of(&ctx, "library"), "storage");
    assert_ne!(row.row.value, "");
    let mut panel = library::storage_panel(&ctx);
    let rows = panel.render(&ctx).rows;
    let first = rows.iter().position(|r| r.kind == "action").unwrap();
    assert_eq!(rows[first].key, biggest);
    assert!(matches!(
        panel.click("row", first, &mut ctx),
        Flow::CloseAll
    ));
    assert_eq!(ctx.open_details.as_deref(), Some(biggest.as_str()));

    let by_size = LibraryCriteria {
        sort: LibrarySort::Size,
        ..Default::default()
    };
    let sorted = query::apply(ctx.library.games(), LibraryFilter::All, &by_size);
    assert_eq!(sorted[0].id, biggest);
}

#[test]
fn themes_installed_from_an_address_offer_updates() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, ctx) = ctx();
    let rows = |ctx: &Ctx| {
        let layer = themes_panel(ctx);
        layer.render(ctx).rows
    };
    let keys: Vec<String> = rows(&ctx).into_iter().map(|r| r.key).collect();
    assert!(keys.contains(&"install-url".to_owned()));
    // Sem tema por endereço, nada a atualizar.
    assert!(!keys.contains(&"update-themes".to_owned()));
    assert!(matches!(install_from_url(&ctx), Flow::Push(_)));
    let root = ctx.paths.themes();
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join(".sources.json"),
        r#"{"Autor_Tema":{"repo":"https://github.com/autor/tema","tag":"v1"}}"#,
    )
    .unwrap();
    assert!(rows(&ctx).iter().any(|r| r.key == "update-themes"));
}

#[test]
fn tako_buttons_can_be_swapped_and_the_hints_follow() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    ctx.input = crate::shell::ctx::InputKind::Gamepad;
    let rows = rows_of(&ctx, "controls");
    assert_eq!(find(&rows, "button-accept").row.value, "A");
    assert!(!find(&rows, "buttons-reset").row.enabled);
    // Aceitar passa ao B; Voltar fica com o A.
    accept(&mut ctx, &find(&rows, "button-accept"));
    let rows = rows_of(&ctx, "controls");
    assert_eq!(find(&rows, "button-accept").row.value, "B");
    assert_eq!(find(&rows, "button-back").row.value, "A");
    assert_eq!(ctx.glyph(Cmd::Accept), "B");
    assert_eq!(ctx.glyph(Cmd::Back), "A");
    assert_eq!(ctx.glyph(Cmd::Guide), "⌂");
    // O aviso diz o que mudou.
    let toast = ctx.toast.clone().unwrap().0;
    assert!(
        toast.contains("Aceitar B") && toast.contains("Voltar A"),
        "{toast}"
    );
    ctx.pad = crate::gamepad::PadFamily::PlayStation;
    assert_eq!(ctx.glyph(Cmd::Accept), "○");
    // Restaurar volta ao padrão.
    let reset = find(&rows_of(&ctx, "controls"), "buttons-reset");
    assert!(reset.row.enabled);
    accept(&mut ctx, &reset);
    assert_eq!(
        ctx.profile().buttons,
        tako_core::input::ButtonMap::default()
    );
    assert_eq!(ctx.glyph(Cmd::Accept), "✕");
}

/// TelaControles no desktop: uma linha por controle (bateria só no em uso) e o grupo "Teste".
#[test]
fn desktop_lists_every_connected_controller_and_a_test_group() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    ctx.desktop = true;
    ctx.pads = vec![
        ("a".into(), "Xbox Wireless".into()),
        ("b".into(), "DualSense".into()),
    ];
    (ctx.pad_guid, ctx.pad_name, ctx.pad_battery) = ("b".into(), "DualSense".into(), Some(15));
    let rows = rows_of(&ctx, "controls");
    let pads: Vec<_> = rows.iter().filter(|r| r.row.key == "controller").collect();
    assert_eq!(pads.len(), 2);
    assert_eq!(pads[0].row.value, "");
    let in_use = &pads[1].row;
    assert_eq!(
        (in_use.value.as_str(), in_use.style.as_str()),
        ("15%", "warning")
    );
    let pos = |k: &str| rows.iter().position(|r| r.row.key == k).unwrap();
    assert_eq!(rows[pos("settings.controls.test") - 1].row.kind, "header");
    assert!(pos("settings.controls.test") < pos("reset"));
}

#[test]
fn each_controller_gets_its_own_profile() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    const PAD1: &str = "030000005e0400000b0b000000000000";
    const PAD2: &str = "050000004c050000e60c000000000000";
    ctx.input = crate::shell::ctx::InputKind::Gamepad;
    ctx.pads = vec![
        (PAD1.into(), "Xbox Wireless".into()),
        (PAD2.into(), "DualSense".into()),
    ];
    ctx.pads_connected = 2;
    (ctx.pad_guid, ctx.pad_name) = (PAD1.into(), "Xbox Wireless".into());
    let rows = rows_of(&ctx, "controls");
    assert_eq!(find(&rows, "pad-0").row.value, "Padrão");
    assert!(!find(&rows, "profile-rename").row.enabled);
    assert!(!find(&rows, "profile-delete").row.enabled);

    // Novo perfil: vale para o controle em uso.
    let Flow::Push(mut layer) = accept(&mut ctx, &find(&rows, "profile-new")) else {
        panic!("sem teclado");
    };
    assert!(
        matches!(layer.handle(Cmd::Menu, &mut ctx), Flow::Stay),
        "nome vazio"
    );
    layer.text("Arcade", &mut ctx);
    assert!(matches!(layer.handle(Cmd::Menu, &mut ctx), Flow::Close));
    assert_eq!(ctx.profile().name, "Arcade");
    // Os botões e a combinação mudam só no perfil Arcade.
    let rows = rows_of(&ctx, "controls");
    accept(&mut ctx, &find(&rows, "button-accept"));
    assert_eq!(ctx.glyph(Cmd::Accept), "B");
    let mut chord_layer = controls::ChordLayer::new(&ctx);
    chord_layer.render(&ctx);
    ctx.pressed_buttons = (B::GUIDE | B::X).0;
    chord_layer.render(&ctx);
    std::thread::sleep(std::time::Duration::from_millis(1050));
    chord_layer.render(&ctx);
    ctx.pressed_buttons = 0;
    chord_layer.render(&ctx);
    assert!(matches!(
        chord_layer.handle(Cmd::Accept, &mut ctx),
        Flow::Close
    ));
    assert_eq!(ctx.profile().chord, B::GUIDE | B::X);
    // O "Aperte A para salvar" segue o botão de aceitar do perfil.
    let accept_text = ctx.tf(
        "settings.controls.press_save",
        &[("accept", ctx.pad_glyph(Cmd::Accept))],
    );
    assert_eq!(accept_text, "Aperte B para salvar");

    // O outro controle continua no padrão.
    let input = ctx.profiles.read([(PAD2, B::B), (PAD1, B::B)]);
    assert_eq!(input.buttons, B::A | B::B);
    assert!(ctx.profiles.read([(PAD1, B::GUIDE | B::X)]).central);
    assert!(!ctx.profiles.read([(PAD2, B::GUIDE | B::X)]).central);
    (ctx.pad_guid, ctx.pad_name) = (PAD2.into(), "DualSense".into());
    assert_eq!(ctx.glyph(Cmd::Accept), "A");
    assert_eq!(ctx.profile().chord, chord::DEFAULT);

    // Escolher o perfil do DualSense pela linha dele.
    let rows = rows_of(&ctx, "controls");
    assert_eq!(find(&rows, "pad-1").row.value, "Padrão");
    accept(&mut ctx, &find(&rows, "pad-1"));
    assert_eq!(ctx.profile().name, "Arcade");
    assert!(
        ctx.toast
            .clone()
            .unwrap()
            .0
            .contains("DualSense: perfil Arcade")
    );

    // Renomear e excluir.
    let rows = rows_of(&ctx, "controls");
    let Flow::Push(mut layer) = accept(&mut ctx, &find(&rows, "profile-rename")) else {
        panic!("sem teclado");
    };
    layer.erase(&mut ctx);
    layer.text("o", &mut ctx);
    assert!(matches!(layer.handle(Cmd::Menu, &mut ctx), Flow::Close));
    assert_eq!(ctx.profile().name, "Arcado");
    let rows = rows_of(&ctx, "controls");
    let Flow::Push(dialog) = accept(&mut ctx, &find(&rows, "profile-delete")) else {
        panic!("sem confirmação");
    };
    let mut dialog = dialog;
    let delete = dialog.render(&ctx).rows;
    assert_eq!(delete[0].key, "delete");
    dialog.handle(Cmd::Accept, &mut ctx);
    assert!(ctx.profile().is_default());
    (ctx.pad_guid, ctx.pad_name) = (PAD1.into(), "Xbox Wireless".into());
    assert!(ctx.profile().is_default());
    assert_eq!(ctx.profiles.all().len(), 1);
    // Gravado nas configurações: relido, dá o mesmo.
    let again =
        tako_core::input::profiles::Profiles::load(|k| ctx.settings.get(k).map(str::to_owned));
    assert_eq!(again, ctx.profiles);
}

#[test]
fn emulators_panel_keeps_folders_executables_and_the_emulator_of_each_system() {
    use tako_sources::emulators::emulator::Emulator;
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (dir, mut ctx) = ctx();
    let cache = emulators::Cache::default();
    let rows = |ctx: &Ctx| emulators::rows(ctx, &cache);
    // A loja aparece com a entrada para o painel.
    let lib = rows_of(&ctx, "library");
    assert!(matches!(
        accept(&mut ctx, &find(&lib, "settings.library.emulators")),
        Flow::Push(_)
    ));
    // Pasta nova: uma subpasta por sistema; muda para SNES pelo diálogo e depois remove.
    let roms = dir.path().join("ROMs");
    emulators::add_folder_path(&mut ctx, roms.clone());
    assert!(ctx.effects.iter().any(|e| matches!(e, Effect::Rescan)));
    let folder = find(&rows(&ctx), "folder-0");
    assert_eq!(folder.row.value, "Uma subpasta por sistema");
    let mut dialog = emulators::folder_options(&ctx, 0);
    let items = dialog.render(&ctx).rows;
    // O foco começa em "Remover" e pula o título.
    let snes = items.iter().position(|r| r.key == "snes").unwrap();
    let downs = items[1..=snes]
        .iter()
        .filter(|r| r.kind != "header")
        .count();
    for _ in 0..downs {
        dialog.handle(Cmd::Down, &mut ctx);
    }
    dialog.handle(Cmd::Accept, &mut ctx);
    assert_eq!(find(&rows(&ctx), "folder-0").row.value, "Super Nintendo");
    // Executável escolhido à mão: vale e libera o emulador dos sistemas dele.
    let exe = dir.path().join("Dolphin.exe");
    std::fs::write(&exe, b"").unwrap();
    emulators::set_exe(&mut ctx, Emulator::Dolphin, exe.clone());
    let r = rows(&ctx);
    assert_eq!(find(&r, "emulator-dolphin").row.value, "Encontrado");
    let gc = find(&r, "system-gc");
    assert_eq!(gc.row.value, "Automático (Dolphin)");
    accept(&mut ctx, &gc);
    assert_eq!(find(&rows(&ctx), "system-gc").row.value, "Dolphin");
    let saved = crate::sources::emulator_config(&ctx.settings);
    assert_eq!(saved.by_system.get("gc"), Some(&Emulator::Dolphin));
    assert_eq!(saved.paths.get(&Emulator::Dolphin), Some(&exe));
    // Remover a pasta.
    let mut dialog = emulators::folder_options(&ctx, 0);
    dialog.handle(Cmd::Accept, &mut ctx);
    assert!(
        crate::sources::emulator_config(&ctx.settings)
            .rom_folders
            .is_empty()
    );
}

#[test]
fn hiding_the_controller_offers_the_driver_or_the_option() {
    use std::sync::Arc;
    use tako_core::devices::controller_hiding::{HIDHIDE_URL, SETTING};
    use tako_core::devices::fake::FakeHiding;
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    // Fora do Windows (e no simulado): nem aparece.
    let has = |ctx: &Ctx, key: &str| rows_of(ctx, "controls").iter().any(|r| r.row.key == key);
    assert!(!has(&ctx, "settings.controls.hidhide"));
    assert!(!has(&ctx, "settings.controls.hide_pads"));
    // Sem o driver: oferece a página oficial (não baixa nada).
    ctx.platform.controller_hiding = Arc::new(FakeHiding::new(false));
    let offer = find(&rows_of(&ctx, "controls"), "settings.controls.hidhide");
    accept(&mut ctx, &offer);
    assert!(
        ctx.effects
            .iter()
            .any(|e| matches!(e, Effect::OpenUrl(u) if u == HIDHIDE_URL))
    );
    // Com o driver: a opção, ligada por padrão.
    ctx.platform.controller_hiding = Arc::new(FakeHiding::new(true));
    let rows = rows_of(&ctx, "controls");
    assert!(!has(&ctx, "settings.controls.hidhide"));
    let option = find(&rows, "settings.controls.hide_pads");
    assert!(option.row.on);
    accept(&mut ctx, &option);
    assert!(!ctx.settings.bool(SETTING, true));
}

#[test]
fn each_monitor_gets_back_its_own_resolution_and_hdr() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    let display = find(&screen::rows(&ctx), "display");
    // TV: a maior resolução e HDR.
    adjust(&mut ctx, &display, -1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_ID), "DISPLAY2");
    let mode = find(&screen::rows(&ctx), "mode");
    adjust(&mut ctx, &mode, 1);
    let hdr = find(&screen::rows(&ctx), "settings.screen.hdr");
    accept(&mut ctx, &hdr);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_MODE), "3840x2160@120");
    assert!(ctx.settings.bool(keys::GAME_SCREEN_HDR, false));
    // Monitor sem preset: o padrão (manter a atual, sem HDR); depois, 60 Hz.
    adjust(&mut ctx, &display, -1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_ID), "DISPLAY1");
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_MODE), "");
    assert!(!ctx.settings.bool(keys::GAME_SCREEN_HDR, true));
    let mode = find(&screen::rows(&ctx), "mode");
    adjust(&mut ctx, &mode, 1);
    adjust(&mut ctx, &mode, 1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_MODE), "1920x1080@60");
    // De volta à TV: o preset dela volta; e ao monitor, o dele.
    adjust(&mut ctx, &display, 1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_MODE), "3840x2160@120");
    assert!(ctx.settings.bool(keys::GAME_SCREEN_HDR, false));
    adjust(&mut ctx, &display, 1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_MODE), "1920x1080@60");
    assert!(!ctx.settings.bool(keys::GAME_SCREEN_HDR, true));
    // Valor quebrado volta ao padrão: a tela troca sem preset.
    ctx.settings.set(keys::GAME_SCREEN_PRESETS, "{quebrado");
    adjust(&mut ctx, &display, 1);
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_ID), "DISPLAY2");
    assert_eq!(ctx.settings.string(keys::GAME_SCREEN_MODE), "");
    assert!(!ctx.settings.bool(keys::GAME_SCREEN_HDR, true));
}

#[test]
fn desktop_search_finds_rows_in_every_category_and_a_category_click_leaves_it() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
    let (_d, mut ctx) = ctx();
    ctx.desktop = true;
    let mut page = SettingsPage::new();
    page.edited("search", "transparencia", &mut ctx);
    let rows = page.items(&ctx);
    // Sem acento e sem maiúsculas: acha "Reduzir transparência" (em Aparência) sob o nome da categoria.
    assert_eq!(rows[0].row.kind, "header");
    assert!(
        rows.iter()
            .any(|i| i.row.key == "settings.appearance.transparency")
    );
    let appearance = page.cats.iter().position(|c| c.id == "appearance").unwrap();
    assert!(page.matches_in(&ctx, appearance) > 0);
    page.click("category", 0, &mut ctx);
    assert!(page.query.is_empty());
}

/// No desktop toda escolha é um select (CampoDeSelecao): cada linha "choice" tem a lista, e escolher qualquer opção
/// a deixa como a atual (inclusive as que andam o ciclo até ela).
#[test]
fn desktop_choices_are_selects_that_pick_each_option() {
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    ctx.desktop = true;
    for cat in categories() {
        let keys: Vec<String> = (cat.rows)(&ctx)
            .into_iter()
            .filter(|i| i.row.kind == "choice" && i.row.enabled)
            .map(|i| i.row.key)
            .collect();
        for key in keys {
            let item = find(&(cat.rows)(&ctx), &key);
            let options = item
                .options
                .clone()
                .unwrap_or_else(|| panic!("{}: {key} sem select", cat.id));
            let n = options(&ctx).0.len();
            for k in 0..n {
                let item = find(&(cat.rows)(&ctx), &key);
                if !options(&ctx).0[k].enabled {
                    continue;
                }
                let mut layer = item.select(&ctx).expect("select");
                let _ = layer.click("row", k, &mut ctx);
                let item = find(&(cat.rows)(&ctx), &key);
                let now = (item.options.unwrap())(&ctx).1;
                assert_eq!(now, k, "{}: {key} opção {k}", cat.id);
            }
        }
    }
    i18n::set_lang(Lang::PtBr);
}

#[test]
fn desktop_volume_is_a_slider_that_drags_mutes_and_jumps() {
    use crate::navigation_feedback::{SOUND_VOLUME, volume};
    use crate::shell::keys::RIGHT;
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (_d, mut ctx) = ctx();
    ctx.desktop = true;
    let row = |ctx: &Ctx| {
        categories()
            .iter()
            .flat_map(|c| (c.rows)(ctx))
            .find(|i| i.row.key == "navigation-volume")
            .expect("linha do volume")
    };
    assert_eq!(row(&ctx).row.kind, "slider");
    let _ = row(&ctx).slide_zone("row-slide:37", &mut ctx);
    assert_eq!(volume(&ctx.settings), 37);
    let _ = row(&ctx).slide_zone("row-drop:0", &mut ctx);
    assert_eq!(row(&ctx).row.value, ctx.t("accessibility.sound_muted"));
    let _ = row(&ctx).slide_key("\u{F72B}", false, &mut ctx);
    assert_eq!(volume(&ctx.settings), 100);
    ctx.settings.set(SOUND_VOLUME, "50");
    let _ = row(&ctx).slide_key(&RIGHT.to_string(), true, &mut ctx);
    assert_eq!(volume(&ctx.settings), 60);
    // Sem Shift, ← → são do ajuste comum (5 no desktop).
    assert!(
        row(&ctx)
            .slide_key(&RIGHT.to_string(), false, &mut ctx)
            .is_none()
    );
    let _ = (row(&ctx).adjust.unwrap())(&mut ctx, 1);
    assert_eq!(volume(&ctx.settings), 65);
}
