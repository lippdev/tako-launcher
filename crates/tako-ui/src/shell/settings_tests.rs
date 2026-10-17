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
        overlays.push(controls::ChordLayer::new().render(&ctx));
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
    let mut layer = controls::ChordLayer::new();
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
    assert_eq!(ctx.button_map(), tako_core::input::ButtonMap::default());
    assert_eq!(ctx.glyph(Cmd::Accept), "✕");
}
