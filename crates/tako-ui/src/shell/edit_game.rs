//! Editar jogo adicionado (port de `EditGameViewModel.cs`, MVP-024): nome, argumentos, pasta de trabalho e fim do
//! jogo pelo teclado virtual; o alvo (executável, atalho ou URI) só aparece: trocar o arquivo é adicionar outro jogo.

use super::Ctx;
use super::keyboard::KeyboardLayer;
use super::layer::{Flow, Layer, MenuItem, MenuLayer, Pick};
use crate::model::PanelRow;
use std::cell::RefCell;
use std::rc::Rc;
use tako_core::library::manual_edit::{EditDraft, EditError};
use tako_core::library::{GameTrackingStrategy, ManualGameRecord};

const TRACKING: [(Option<GameTrackingStrategy>, &str); 4] = [
    (None, "edit_game.track.auto"),
    (
        Some(GameTrackingStrategy::Process),
        "edit_game.track.process",
    ),
    (
        Some(GameTrackingStrategy::InstallFolder),
        "edit_game.track.folder",
    ),
    (Some(GameTrackingStrategy::None), "edit_game.track.none"),
];

#[derive(Clone)]
struct Draft {
    record: ManualGameRecord,
    name: String,
    arguments: String,
    folder: String,
    tracking: usize,
}

type Shared = Rc<RefCell<Draft>>;

/// Grava o rascunho; `Err` = o que falta (nome vazio, pasta que não existe).
fn save(ctx: &mut Ctx, d: &Draft) -> Result<(), String> {
    let record = d
        .record
        .edited(&EditDraft {
            title: d.name.clone(),
            arguments: d.arguments.clone(),
            folder: d.folder.clone(),
            track_by: TRACKING[d.tracking].0,
        })
        .map_err(|e| match e {
            EditError::NameRequired => ctx.t("edit_game.name_required"),
            EditError::FolderMissing(f) => {
                ctx.tf("game_settings.folder_missing", &[("folder", &f)])
            }
        })?;
    ctx.library
        .update_manual(record)
        .map_err(|e| e.to_string())?;
    ctx.sync_collections();
    crate::local_api::sync(ctx);
    Ok(())
}

fn text_field(
    ctx: &Ctx,
    d: &Shared,
    title: String,
    get: fn(&Draft) -> &String,
    set: fn(&mut Draft, String),
    folder: bool,
) -> Flow {
    let text = get(&d.borrow()).clone();
    let d = d.clone();
    let k = KeyboardLayer::new(ctx, title, &text, move |ctx, t| {
        set(&mut d.borrow_mut(), t.to_owned());
        ctx.dirty = true;
        Flow::Close
    });
    let k = if folder {
        k.hint(ctx.t("game_settings.folder_hint"))
            .check(super::game_settings::folder_problem)
    } else {
        k
    };
    Flow::Push(Box::new(k))
}

pub fn panel(ctx: &Ctx, id: &str) -> Box<dyn Layer> {
    let Some(record) = ctx.library.find_manual(id).cloned() else {
        return Box::new(MenuLayer::dialog(ctx.t("edit_game.title"), |_| Vec::new()));
    };
    let draft: Shared = Rc::new(RefCell::new(Draft {
        name: record.title.clone(),
        arguments: record.arguments.clone().unwrap_or_default(),
        folder: record.working_directory.clone().unwrap_or_default(),
        tracking: TRACKING
            .iter()
            .position(|t| t.0 == record.track_by)
            .unwrap_or(0),
        record,
    }));
    let layer = MenuLayer::dialog(ctx.t("edit_game.title"), move |ctx| rows(ctx, &draft)).wide();
    Box::new(layer)
}

fn rows(ctx: &Ctx, draft: &Shared) -> Vec<MenuItem> {
    let d = draft.borrow().clone();
    let none = ctx.t("game_settings.none");
    let or = |v: &str, empty: &str| {
        if v.trim().is_empty() {
            empty.to_owned()
        } else {
            v.to_owned()
        }
    };
    let (a, b, c, t1, t2, s) = (
        draft.clone(),
        draft.clone(),
        draft.clone(),
        draft.clone(),
        draft.clone(),
        draft.clone(),
    );
    let cycle = move |d: &Shared, dir: i32| {
        let mut d = d.borrow_mut();
        d.tracking = (d.tracking as i32 + dir).rem_euclid(TRACKING.len() as i32) as usize;
        Flow::Stay
    };
    vec![
        MenuItem::new(PanelRow::choice(
            "name",
            ctx.t("edit_game.name"),
            or(&d.name, ""),
        ))
        .on(move |ctx| {
            let title = ctx.t("edit_game.name");
            text_field(ctx, &a, title, |d| &d.name, |d, t| d.name = t, false)
        }),
        MenuItem::new(PanelRow::choice(
            "args",
            ctx.t("edit_game.arguments"),
            or(&d.arguments, &none),
        ))
        .on(move |ctx| {
            let title = ctx.t("edit_game.arguments");
            text_field(
                ctx,
                &b,
                title,
                |d| &d.arguments,
                |d, t| d.arguments = t,
                false,
            )
        }),
        MenuItem::new(PanelRow::choice(
            "folder",
            ctx.t("game_settings.folder"),
            or(&d.folder, &ctx.t("game_settings.exe_folder")),
        ))
        .on(move |ctx| {
            let title = ctx.t("game_settings.folder");
            text_field(ctx, &c, title, |d| &d.folder, |d, t| d.folder = t, true)
        }),
        MenuItem::new(PanelRow::choice(
            "tracking",
            ctx.t("edit_game.tracking"),
            ctx.t(TRACKING[d.tracking].1),
        ))
        .on(move |_| cycle(&t1, 1))
        .on_adjust(move |_, dir| cycle(&t2, dir))
        .options({
            let d = draft.clone();
            move |ctx| {
                let picks = TRACKING
                    .iter()
                    .map(|(_, label)| Pick::new(ctx.t(label)))
                    .collect();
                (picks, d.borrow().tracking)
            }
        }),
        MenuItem::new(PanelRow::info(
            "target",
            ctx.t("edit_game.target"),
            d.record.path.clone(),
        )),
        MenuItem::new(PanelRow::button("save", ctx.t("common.save"))).on(move |ctx| {
            let d = s.borrow().clone();
            match save(ctx, &d) {
                Ok(()) => {
                    ctx.toast(ctx.tf("edit_game.saved", &[("game", d.name.trim())]));
                    Flow::Close
                }
                Err(e) => {
                    ctx.toast(e);
                    Flow::Stay
                }
            }
        }),
        MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::Cmd;

    #[test]
    fn edit_saves_the_record_and_rejects_missing_folders() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        let exe = dir.path().join("jogo.sh");
        std::fs::write(&exe, "").unwrap();
        let record = ManualGameRecord::create("Jogo", &exe.to_string_lossy(), chrono::Utc::now());
        let id = record.id.clone();
        ctx.library.add_manual(record).unwrap();

        let mut layer = panel(&ctx, &id);
        let o = layer.render(&ctx);
        assert_eq!(o.rows[0].value, "Jogo");
        assert_eq!(o.rows[3].value, "Automático");
        assert_eq!(o.rows[4].value, exe.to_string_lossy());

        // Nome pelo teclado virtual (o físico digita direto).
        let Flow::Push(mut k) = layer.handle(Cmd::Accept, &mut ctx) else {
            panic!("teclado");
        };
        k.erase(&mut ctx);
        k.erase(&mut ctx);
        k.erase(&mut ctx);
        k.erase(&mut ctx);
        k.text("Meu Jogo", &mut ctx);
        assert!(matches!(k.handle(Cmd::Menu, &mut ctx), Flow::Close));
        // Fim do jogo: "Processo iniciado".
        layer.hover("row", 3);
        layer.handle(Cmd::Accept, &mut ctx);
        assert_eq!(layer.render(&ctx).rows[3].value, "Processo iniciado");
        layer.hover("row", 5);
        assert!(matches!(layer.handle(Cmd::Accept, &mut ctx), Flow::Close));

        let saved = ctx.library.find_manual(&id).unwrap();
        assert_eq!(saved.title, "Meu Jogo");
        assert_eq!(saved.track_by, Some(GameTrackingStrategy::Process));
        assert_eq!(
            saved.working_directory.as_deref(),
            Some(&*dir.path().to_string_lossy())
        );
        assert_eq!(ctx.library.find(&id).unwrap().title, "Meu Jogo");

        let mut bad = Draft {
            record: saved.clone(),
            name: "X".into(),
            arguments: String::new(),
            folder: "/nao/existe".into(),
            tracking: 0,
        };
        assert!(save(&mut ctx, &bad).is_err());
        bad.folder.clear();
        bad.name = "  ".into();
        assert!(save(&mut ctx, &bad).is_err());
    }
}
