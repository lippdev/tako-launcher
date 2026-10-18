//! Configurações › Biblioteca › Emuladores e ROMs (SRC-001): as pastas de ROMs do usuário, o executável de cada
//! emulador (achado sozinho ou escolhido) e o emulador de cada sistema. Toda mudança grava e relê a biblioteca.

use super::{Ctx, Flow, MenuItem};
use crate::model::PanelRow;
use crate::shell::{Effect, MenuLayer};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use tako_sources::emulators::emulator::{Emulator, Env};
use tako_sources::emulators::{Config, Fs, Installed, RomFolder, systems};

fn config(ctx: &Ctx) -> Config {
    crate::sources::emulator_config(&ctx.settings)
}

/// Grava e relê a biblioteca (os jogos dos emuladores mudam).
fn save(ctx: &mut Ctx, c: &Config) {
    for (k, v) in c.save() {
        ctx.settings.set(k, &v);
    }
    ctx.effect(Effect::Rescan);
    ctx.dirty = true;
}

/// O que foi achado no disco; a busca olha várias pastas, então roda ao abrir o painel e depois de cada mudança
/// (não a cada desenho).
pub(crate) type Cache = Rc<RefCell<Option<(Config, Installed)>>>;

fn installed(cache: &Cache, ctx: &Ctx) -> Installed {
    let now = config(ctx);
    let mut c = cache.borrow_mut();
    match c.as_ref() {
        Some((cfg, found)) if *cfg == now => found.clone(),
        _ => {
            let found = Installed::detect(&Env::current(), &now, &Fs::real());
            *c = Some((now, found.clone()));
            found
        }
    }
}

/// O onboarding mostra o passo dos emuladores só quando há algo: um emulador achado, uma pasta de ROMs ou jogos
/// de emuladores na biblioteca.
pub(crate) fn anything_found(ctx: &Ctx) -> bool {
    let cfg = config(ctx);
    if !cfg.rom_folders.is_empty()
        || ctx
            .library
            .games()
            .iter()
            .any(|g| g.store() == tako_core::library::GameStore::Emulators)
    {
        return true;
    }
    let found = Installed::detect(&Env::current(), &cfg, &Fs::real());
    !found.programs.is_empty() || found.retroarch.is_some()
}

pub(crate) fn panel(ctx: &Ctx) -> MenuLayer {
    let cache: Cache = Rc::default();
    MenuLayer::panel(ctx.t("settings.emulators.title"), move |ctx| {
        rows(ctx, &cache)
    })
    .text(ctx.t("settings.emulators.text"))
}

pub(crate) fn rows(ctx: &Ctx, cache: &Cache) -> Vec<MenuItem> {
    let cfg = config(ctx);
    let found = installed(cache, ctx);
    let mut v = vec![MenuItem::new(PanelRow::header(
        ctx.t("settings.emulators.folders_header"),
    ))];
    for (i, f) in cfg.rom_folders.iter().enumerate() {
        let system = f.system.as_deref().and_then(systems::by_id).map_or_else(
            || ctx.t("settings.emulators.folder_auto"),
            |s| s.name.to_owned(),
        );
        v.push(
            MenuItem::new(
                PanelRow::action(format!("folder-{i}"), f.path.to_string_lossy())
                    .value(system)
                    .icon("folder"),
            )
            .on(move |ctx| Flow::Push(Box::new(folder_options(ctx, i)))),
        );
    }
    v.push(
        MenuItem::new(
            PanelRow::action("add-folder", ctx.t("settings.emulators.add_folder"))
                .desc(ctx.t("settings.emulators.add_folder_desc"))
                .icon("more"),
        )
        .on(|ctx| {
            add_folder(ctx);
            Flow::Stay
        }),
    );

    v.push(MenuItem::new(PanelRow::header(
        ctx.t("settings.emulators.emulators_header"),
    )));
    for emu in Emulator::ALL {
        let chosen = cfg.paths.get(&emu);
        let program = found.programs.get(&emu);
        let (value, desc) = match (program, chosen) {
            (Some(p), Some(_)) => (
                ctx.t("settings.emulators.found"),
                ctx.tf(
                    "settings.emulators.chosen_desc",
                    &[("path", &p.target.to_string_lossy())],
                ),
            ),
            (Some(p), None) => (
                ctx.t("settings.emulators.found"),
                match &p.flatpak {
                    Some(id) => format!("Flatpak · {id}"),
                    None => p.target.to_string_lossy().into_owned(),
                },
            ),
            (None, _) => (
                ctx.t("settings.emulators.missing"),
                ctx.t("settings.emulators.missing_desc"),
            ),
        };
        v.push(
            MenuItem::new(
                PanelRow::action(format!("emulator-{}", emu.key()), emu.name())
                    .value(value)
                    .desc(desc),
            )
            .on(move |ctx| {
                if config(ctx).paths.contains_key(&emu) {
                    Flow::Push(Box::new(exe_options(emu)))
                } else {
                    choose_exe(ctx, emu);
                    Flow::Stay
                }
            }),
        );
    }

    let os = tako_core::paths::Os::current();
    let fs = Fs::real();
    let mut by_system: Vec<MenuItem> = Vec::new();
    for s in systems::SYSTEMS {
        let mut options: Vec<Emulator> = s
            .standalone
            .iter()
            .copied()
            .filter(|e| found.programs.contains_key(e))
            .collect();
        if found.programs.contains_key(&Emulator::RetroArch) && found.has_core(s, os, &fs) {
            options.push(Emulator::RetroArch);
        }
        if options.is_empty() {
            continue;
        }
        let value = match cfg.by_system.get(s.id) {
            Some(e) => e.name().to_owned(),
            None => ctx.tf(
                "settings.emulators.auto_with",
                &[("emulator", options[0].name())],
            ),
        };
        let id = s.id;
        let cycle = move |ctx: &mut Ctx, dir: i32| {
            let mut c = config(ctx);
            // Automático e as opções deste PC, em roda.
            let mut all: Vec<Option<Emulator>> = vec![None];
            all.extend(options.iter().copied().map(Some));
            let now = all
                .iter()
                .position(|o| *o == c.by_system.get(id).copied())
                .unwrap_or(0) as i32;
            let next = all[(now + dir).rem_euclid(all.len() as i32) as usize];
            match next {
                Some(e) => c.by_system.insert(id.to_owned(), e),
                None => c.by_system.remove(id),
            };
            save(ctx, &c);
            Flow::Stay
        };
        let cycle2 = cycle.clone();
        by_system.push(
            MenuItem::new(PanelRow::choice(format!("system-{id}"), s.name, value))
                .on(move |ctx| cycle(ctx, 1))
                .on_adjust(move |ctx, d| cycle2(ctx, d)),
        );
    }
    if !by_system.is_empty() {
        v.push(MenuItem::new(PanelRow::header(
            ctx.t("settings.emulators.by_system_header"),
        )));
        v.extend(by_system);
    }
    v
}

fn add_folder(ctx: &mut Ctx) {
    let title = ctx.t("settings.emulators.add_folder");
    crate::shell::add_game::pick(
        ctx,
        move || rfd::FileDialog::new().set_title(&title).pick_folder(),
        add_folder_path,
    );
}

pub(crate) fn add_folder_path(ctx: &mut Ctx, path: PathBuf) {
    let mut c = config(ctx);
    if !c.rom_folders.iter().any(|f| f.path == path) {
        c.rom_folders.push(RomFolder {
            path: path.clone(),
            system: None,
        });
        save(ctx, &c);
    }
    ctx.toast(ctx.tf(
        "settings.emulators.folder_added",
        &[("path", &path.to_string_lossy())],
    ));
}

/// Uma pasta: o sistema dela (ou uma subpasta por sistema) e remover.
pub(crate) fn folder_options(ctx: &Ctx, index: usize) -> MenuLayer {
    let title = config(ctx)
        .rom_folders
        .get(index)
        .map(|f| f.path.to_string_lossy().into_owned())
        .unwrap_or_default();
    MenuLayer::dialog(title, move |ctx| {
        let current = config(ctx)
            .rom_folders
            .get(index)
            .and_then(|f| f.system.clone());
        let set = move |system: Option<&'static str>| {
            move |ctx: &mut Ctx| {
                let mut c = config(ctx);
                if let Some(f) = c.rom_folders.get_mut(index) {
                    f.system = system.map(str::to_owned);
                }
                save(ctx, &c);
                Flow::Close
            }
        };
        let mut v = vec![
            MenuItem::new(
                PanelRow::button("remove", ctx.t("settings.emulators.remove_folder")).danger(),
            )
            .on(move |ctx| {
                let mut c = config(ctx);
                if index < c.rom_folders.len() {
                    c.rom_folders.remove(index);
                }
                save(ctx, &c);
                ctx.toast(ctx.t("settings.emulators.folder_removed"));
                Flow::Close
            }),
            MenuItem::new(PanelRow::header(ctx.t("settings.emulators.system_header"))),
            MenuItem::new(PanelRow::radio(
                "auto",
                ctx.t("settings.emulators.folder_auto"),
                current.is_none(),
            ))
            .on(set(None)),
        ];
        for s in systems::SYSTEMS
            .iter()
            .filter(|s| !s.extensions.is_empty() || s.id == "ps3")
        {
            v.push(
                MenuItem::new(PanelRow::radio(
                    s.id,
                    s.name,
                    current.as_deref() == Some(s.id),
                ))
                .on(set(Some(s.id))),
            );
        }
        v
    })
}

/// Executável escolhido: escolher outro ou voltar a procurar sozinho.
fn exe_options(emu: Emulator) -> MenuLayer {
    MenuLayer::dialog(emu.name(), move |ctx| {
        vec![
            MenuItem::new(PanelRow::button(
                "choose",
                ctx.t("settings.emulators.choose_exe"),
            ))
            .on(move |ctx| {
                choose_exe(ctx, emu);
                Flow::Close
            }),
            MenuItem::new(PanelRow::button(
                "detect",
                ctx.t("settings.emulators.use_detected"),
            ))
            .on(move |ctx| {
                let mut c = config(ctx);
                c.paths.remove(&emu);
                save(ctx, &c);
                Flow::Close
            }),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
}

fn choose_exe(ctx: &mut Ctx, emu: Emulator) {
    let title = ctx.tf(
        "settings.emulators.choose_title",
        &[("emulator", emu.name())],
    );
    crate::shell::add_game::pick(
        ctx,
        move || rfd::FileDialog::new().set_title(&title).pick_file(),
        move |ctx, path| set_exe(ctx, emu, path),
    );
}

pub(crate) fn set_exe(ctx: &mut Ctx, emu: Emulator, path: PathBuf) {
    let mut c = config(ctx);
    c.paths.insert(emu, path.clone());
    save(ctx, &c);
    ctx.toast(ctx.tf(
        "settings.emulators.exe_set",
        &[("emulator", emu.name()), ("path", &path.to_string_lossy())],
    ));
}
