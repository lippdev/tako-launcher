//! "Adicionar jogo" (port de `ShellViewModel.Library.AddGame`): no Linux e no macOS, primeiro os jogos do sistema
//! (atalhos `.desktop`, apps `.app`); depois o seletor de arquivos do SO. ROMs pedem o sistema (ADR-0022).

use super::Ctx;
use super::layer::{Flow, Layer, MenuItem, MenuLayer};
use crate::model::PanelRow;
use std::path::{Path, PathBuf};
use tako_core::library::ManualGameRecord;
use tako_core::metadata::screenscraper_parse::{
    EmulatedSystem, SYSTEMS, candidates_for, looks_like_rom, rom_title,
};

/// Trata `Effect::AddGame`. Camadas abertas daqui vão para `ctx.pending_layer` (com janela, entram já).
pub fn add_game(ctx: &mut Ctx) {
    if let Some(layer) = system_games(ctx) {
        open_layer(ctx, layer);
        return;
    }
    pick_file(ctx);
}

/// Mostra a camada: o app só abre `pending_layer` depois de uma ação, e quem chama aqui é o app (fora dela).
pub(super) fn open_layer(ctx: &mut Ctx, layer: Box<dyn Layer>) {
    ctx.pending_layer = Some(layer);
    ctx.dirty = true;
    if crate::tasks::shell().is_some() {
        let _ = slint::invoke_from_event_loop(|| {
            if let Some(shell) = crate::tasks::shell()
                && let Ok(mut s) = shell.try_borrow_mut()
                && let Some(l) = s.ctx.pending_layer.take()
            {
                s.push_layer(l);
            }
        });
    }
}

/// Seletor de arquivos do SO para "Adicionar jogo".
pub fn pick_file(ctx: &mut Ctx) {
    let title = ctx.t("add_game.title");
    pick(
        ctx,
        move || rfd::FileDialog::new().set_title(&title).pick_file(),
        |ctx, path| add_path(ctx, &path),
    );
}

/// Abre um diálogo do SO (`choose`) e entrega o arquivo na thread da UI. Windows e Linux: numa thread (bloqueante);
/// macOS: na thread principal (AppKit exige), fora do tratamento atual.
pub fn pick(
    ctx: &mut Ctx,
    choose: impl FnOnce() -> Option<PathBuf> + Send + 'static,
    done: impl FnOnce(&mut Ctx, PathBuf) + 'static,
) {
    #[cfg(target_os = "macos")]
    {
        let _ = ctx;
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let (Some(path), Some(shell)) = (choose(), crate::tasks::shell()) {
                let mut s = shell.borrow_mut();
                done(&mut s.ctx, path);
                s.ctx.dirty = true;
            }
        });
    }
    #[cfg(not(target_os = "macos"))]
    crate::tasks::run(ctx, choose, |ctx, path| {
        if let Some(path) = path {
            done(ctx, path);
        }
    });
}

/// Jogos do sistema ainda fora da biblioteca, com "Outro arquivo…" no fim; `None` = direto ao seletor.
fn system_games(ctx: &Ctx) -> Option<Box<dyn Layer>> {
    let found = system_entries();
    let known = |target: &str| {
        ctx.library
            .games()
            .iter()
            .any(|g| g.launch_target.as_deref() == Some(target))
    };
    let entries: Vec<(String, PathBuf)> = found
        .into_iter()
        .filter(|(_, p)| !known(&p.to_string_lossy()))
        .collect();
    if entries.is_empty() {
        return None;
    }
    let layer = MenuLayer::dialog(ctx.t("add_game.system_games"), move |ctx| {
        let mut v: Vec<MenuItem> = entries
            .iter()
            .map(|(name, path)| {
                let path = path.clone();
                MenuItem::new(PanelRow::button(path.to_string_lossy(), name.clone())).on(
                    move |ctx| {
                        add_path(ctx, &path);
                        Flow::Close
                    },
                )
            })
            .collect();
        v.push(
            MenuItem::new(PanelRow::button("other", ctx.t("add_game.other_file"))).on(|ctx| {
                pick_file(ctx);
                Flow::Close
            }),
        );
        v.push(
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        );
        v
    });
    Some(Box::new(layer))
}

/// (nome, arquivo) dos jogos instalados que o SO lista.
fn system_entries() -> Vec<(String, PathBuf)> {
    #[cfg(target_os = "linux")]
    return tako_sources::desktop_entries::game_desktop_entries()
        .into_iter()
        .map(|e| (e.name, e.file))
        .collect();
    #[cfg(target_os = "macos")]
    return tako_sources::mac_apps::list_game_apps()
        .into_iter()
        .map(|a| (a.name, a.bundle))
        .collect();
    #[allow(unreachable_code)]
    Vec::new()
}

/// Adiciona o arquivo escolhido: `.desktop` e `.app` pelo que declaram; ROM pergunta o sistema; o resto vira jogo
/// de PC com o nome do produto (`.exe` no Windows) ou do arquivo.
pub fn add_path(ctx: &mut Ctx, path: &Path) {
    let text = path.to_string_lossy().into_owned();
    if let Some(existing) = ctx.library.games().iter().find(|g| {
        g.launch_target
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case(&text))
    }) {
        let title = existing.title.clone();
        ctx.toast(ctx.tf("add_game.already", &[("game", &title)]));
        return;
    }
    let now = chrono::Utc::now();
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let record = match ext.as_str() {
        "desktop" => {
            let Some(entry) = tako_sources::desktop_entries::read_desktop_file(path) else {
                ctx.toast(ctx.t("add_game.unreadable"));
                return;
            };
            let Some((target, args)) = entry.launch() else {
                ctx.toast(ctx.t("add_game.unreadable"));
                return;
            };
            let mut r = ManualGameRecord::create(&entry.name, &target, now);
            r.arguments = args;
            if entry.working_dir.is_some() {
                r.working_directory = entry.working_dir;
            }
            r
        }
        "app" => {
            let name = tako_sources::mac_apps::read_app(path)
                .map(|a| a.name)
                .unwrap_or_else(|| guess_title(path));
            ManualGameRecord::create(&name, &text, now)
        }
        _ if looks_like_rom(&text) => {
            let found = candidates_for(&text);
            let layer = choose_system(ctx, path.to_owned(), found);
            open_layer(ctx, layer);
            return;
        }
        _ => ManualGameRecord::create(&guess_title(path), &text, now),
    };
    add(ctx, record, None);
}

fn add(ctx: &mut Ctx, record: ManualGameRecord, system: Option<&str>) {
    match ctx.library.add_manual(record) {
        Ok(g) => {
            let title = match system {
                Some(s) => format!("{} ({s})", g.title),
                None => g.title.clone(),
            };
            // A fila de metadados vê a biblioteca mudar e busca capa e informações.
            ctx.sync_collections();
            ctx.toast(ctx.tf("add_game.added", &[("game", &title)]));
        }
        Err(e) => ctx.toast(e.to_string()),
    }
}

/// "De qual sistema é este jogo?": os prováveis pela extensão e "Outro sistema" (todos).
fn choose_system(ctx: &Ctx, path: PathBuf, found: Vec<&'static EmulatedSystem>) -> Box<dyn Layer> {
    let all = found.is_empty();
    let list: Vec<&'static EmulatedSystem> = if all { SYSTEMS.iter().collect() } else { found };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let layer = MenuLayer::dialog(ctx.t("add_game.which_system"), move |ctx| {
        let mut v: Vec<MenuItem> = list
            .iter()
            .map(|s| {
                let (path, s) = (path.clone(), *s);
                MenuItem::new(PanelRow::button(s.key, s.name)).on(move |ctx| {
                    let text = path.to_string_lossy();
                    let r = ManualGameRecord::create_rom(
                        &rom_title(&text),
                        &text,
                        s.key,
                        chrono::Utc::now(),
                    );
                    add(ctx, r, Some(s.name));
                    Flow::Close
                })
            })
            .collect();
        if !all {
            let path = path.clone();
            v.push(
                MenuItem::new(PanelRow::button("other", ctx.t("add_game.other_system")))
                    .on(move |ctx| Flow::Replace(choose_system(ctx, path.clone(), Vec::new()))),
            );
        }
        v.push(
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        );
        v
    })
    .text(format!("{name}\n{}", ctx.t("add_game.system_text")));
    Box::new(layer)
}

/// Nome do produto do `.exe` no Windows ("ELDEN RING™" → "ELDEN RING"); senão o nome do arquivo sem extensão nem
/// separadores ("hollow_knight.exe" → "hollow knight").
fn guess_title(path: &Path) -> String {
    #[cfg(target_os = "windows")]
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
        && let Some(t) = product_title(tako_platform::windows::version_info::names(path))
    {
        return t;
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let t = stem.replace(['_', '-'], " ").trim().to_owned();
    if t.is_empty() { stem } else { t }
}

/// Primeiro de ProductName e FileDescription que sirva de título: com mais de 2 letras, sem "launcher" (o .exe do
/// launcher da loja não é o jogo); tira ™ e ®.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn product_title(candidates: [Option<String>; 2]) -> Option<String> {
    candidates.into_iter().flatten().find_map(|c| {
        let ok =
            !c.trim().is_empty() && !c.to_lowercase().contains("launcher") && c.chars().count() > 2;
        ok.then(|| c.replace(['™', '®'], "").trim().to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_title_skips_launchers_and_short_names() {
        let s = |v: &str| Some(v.to_owned());
        assert_eq!(
            product_title([s("ELDEN RING™"), s("elden ring")]).as_deref(),
            Some("ELDEN RING")
        );
        assert_eq!(
            product_title([s("Rockstar Games Launcher"), s("GTA V®")]).as_deref(),
            Some("GTA V")
        );
        assert_eq!(product_title([None, s("Hades")]).as_deref(), Some("Hades"));
        assert_eq!(product_title([s("  "), s("UE")]), None);
        assert_eq!(product_title([None, None]), None);
    }

    #[test]
    fn adds_executables_once_and_asks_the_system_of_roms() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        let exe = dir.path().join("hollow_knight.exe");
        std::fs::write(&exe, "").unwrap();
        add_path(&mut ctx, &exe);
        assert_eq!(ctx.toast.as_ref().unwrap().0, "hollow knight adicionado");
        assert!(
            ctx.library
                .games()
                .iter()
                .any(|g| g.title == "hollow knight")
        );
        add_path(&mut ctx, &exe);
        assert_eq!(
            ctx.toast.as_ref().unwrap().0,
            "hollow knight já está na biblioteca"
        );

        let rom = dir.path().join("Super Mario World (USA).sfc");
        std::fs::write(&rom, "").unwrap();
        add_path(&mut ctx, &rom);
        let mut layer = ctx.pending_layer.take().expect("pergunta o sistema");
        assert_eq!(layer.render(&ctx).rows[0].title, "SNES");
        layer.handle(crate::shell::Cmd::Accept, &mut ctx);
        let g = ctx
            .library
            .games()
            .iter()
            .find(|g| g.title == "Super Mario World")
            .unwrap();
        assert_eq!(g.platform.as_deref(), Some("snes"));
    }

    #[test]
    fn desktop_files_use_name_and_exec() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        let file = dir.path().join("jogo.desktop");
        std::fs::write(
            &file,
            "[Desktop Entry]\nType=Application\nName=Jogo Livre\nExec=/opt/jogo/run --fullscreen %U\nCategories=Game;\n",
        )
        .unwrap();
        add_path(&mut ctx, &file);
        let g = ctx
            .library
            .games()
            .iter()
            .find(|g| g.title == "Jogo Livre")
            .unwrap();
        assert_eq!(g.launch_target.as_deref(), Some("/opt/jogo/run"));
        assert_eq!(g.launch_arguments.as_deref(), Some("--fullscreen"));
    }
}
