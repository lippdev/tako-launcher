//! Importar do Playnite (Configurações › Biblioteca, só Windows), como `ShellViewModel.PlayniteImport.cs`:
//! consentimento com o que será lido, leitura só de leitura fora da UI, prévia e confirmação. Grava jogos novos,
//! favoritos, tempo jogado e as categorias como coleções. Nada muda nos arquivos do Playnite.

use crate::model::PanelRow;
use crate::shell::Ctx;
use crate::shell::layer::{Flow, MenuItem, MenuLayer};
use crate::tasks;
use std::path::PathBuf;
use std::rc::Rc;
use tako_core::library::game_library::StateChange;
use tako_core::library::{GameStateRecord, GameStore, ManualGameRecord};
use tako_sources::playnite;
use tako_sources::playnite::import::{self, PlayniteState};
use tako_sources::playnite::library::{PlayniteError, PlayniteLibrary};

/// Primeiro passo: achar a biblioteca e pedir o consentimento.
pub fn start(ctx: &mut Ctx) {
    let app_data = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_default();
    let Some(folder) = playnite::find(&app_data).filter(|f| f.join("games.db").exists()) else {
        ctx.toast(ctx.t("playnite.not_found"));
        return;
    };
    let shown = folder.display().to_string();
    let folder = Rc::new(folder);
    ctx.pending_layer = Some(Box::new(
        MenuLayer::dialog(ctx.t("playnite.consent_title"), move |ctx| {
            let folder = folder.clone();
            vec![
                MenuItem::new(PanelRow::button("read", ctx.t("playnite.read"))).on(move |ctx| {
                    read(ctx, (*folder).clone());
                    Flow::Close
                }),
                MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                    .on(|_| Flow::Close),
            ]
        })
        .text(ctx.tf("playnite.consent", &[("folder", &shown)])),
    ));
}

fn read(ctx: &mut Ctx, folder: PathBuf) {
    ctx.toast(ctx.t("playnite.reading"));
    tasks::run(
        ctx,
        move || PlayniteLibrary::read(&folder),
        |ctx, result| match result {
            Ok(lib) => {
                let names: Vec<String> = ctx
                    .collections
                    .collections()
                    .iter()
                    .map(|c| c.name.clone())
                    .collect();
                let plan = import::plan(&lib, ctx.library.games(), &names, chrono::Utc::now());
                ctx.pending_layer = Some(Box::new(preview(ctx, Rc::new(plan))));
            }
            Err(e) => {
                tracing::warn!("Playnite: {e}");
                ctx.toast(ctx.t(match e {
                    PlayniteError::NotFound(_) => "playnite.not_found",
                    PlayniteError::InvalidData(_) => "playnite.invalid",
                    PlayniteError::Io(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        "playnite.denied"
                    }
                    PlayniteError::Io(_) => "playnite.in_use",
                }));
            }
        },
    );
}

fn preview(ctx: &Ctx, plan: Rc<import::PlayniteImportPlan>) -> MenuLayer {
    let mut text = plan.summary();
    if !plan.new.is_empty() {
        let names: Vec<&str> = plan
            .new
            .iter()
            .take(5)
            .map(|g| g.record.title.as_str())
            .collect();
        let more = if plan.new.len() > 5 { "…" } else { "" };
        text.push_str(&format!(
            "\n\n{}{more}",
            ctx.tf("playnite.examples", &[("list", &names.join(", "))])
        ));
    }
    MenuLayer::dialog(ctx.t("playnite.preview_title"), move |ctx| {
        if plan.is_empty() {
            return vec![
                MenuItem::new(PanelRow::button("close", ctx.t("common.close"))).on(|_| Flow::Close),
            ];
        }
        let plan = plan.clone();
        vec![
            MenuItem::new(PanelRow::button("import", ctx.t("playnite.import"))).on(move |ctx| {
                apply(ctx, &plan);
                Flow::Close
            }),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
    .text(text)
}

fn change(source: &tako_sources::playnite::library::PlayniteGame, store: GameStore) -> StateChange {
    let source = source.clone();
    Box::new(move |current: Option<GameStateRecord>| {
        let sessions = current
            .as_ref()
            .map(|c| c.sessions.clone())
            .unwrap_or_default();
        let play_status = current.as_ref().and_then(|c| c.play_status);
        let tags = current.as_ref().map(|c| c.tags.clone()).unwrap_or_default();
        let current = current.map(|c| PlayniteState {
            is_favorite: c.is_favorite,
            last_played_at: c.last_played_at,
            play_minutes: c.play_minutes,
        });
        let s = import::merge_state(current.as_ref(), &source, store);
        GameStateRecord {
            is_favorite: s.is_favorite,
            play_status,
            last_played_at: s.last_played_at,
            play_minutes: s.play_minutes,
            sessions,
            tags,
        }
    })
}

/// Grava a importação confirmada.
fn apply(ctx: &mut Ctx, plan: &import::PlayniteImportPlan) {
    let mut state: Vec<(String, StateChange)> = Vec::new();
    let added: Vec<ManualGameRecord> = plan
        .new
        .iter()
        .map(|g| {
            state.push((g.record.id.clone(), change(&g.source, GameStore::Manual)));
            let r = &g.record;
            ManualGameRecord {
                id: r.id.clone(),
                title: r.title.clone(),
                path: r.path.clone(),
                arguments: r.arguments.clone(),
                working_directory: r.working_directory.clone(),
                track_by: None,
                cover_path: None,
                hero_path: None,
                added_at: r.added_at,
                platform: r.platform.clone(),
            }
        })
        .collect();
    for m in &plan.matched {
        let store = ctx
            .library
            .find(&m.tako_id)
            .map(|g| g.store())
            .unwrap_or(GameStore::Manual);
        state.push((m.tako_id.clone(), change(&m.source, store)));
    }
    if let Err(e) = ctx.library.import(added, state) {
        ctx.toast(e.to_string());
        return;
    }
    let memberships = plan
        .new
        .iter()
        .map(|g| (&g.record.id, &g.collections))
        .chain(plan.matched.iter().map(|g| (&g.tako_id, &g.collections)));
    for (id, names) in memberships {
        for name in names {
            let fold = |s: &str| tako_core::library::search_text::normalize(s);
            let existing = ctx
                .collections
                .collections()
                .iter()
                .find(|c| fold(&c.name) == fold(name))
                .map(|c| c.id.clone());
            let cid = match existing {
                Some(c) => c,
                None => match ctx.collections.create(name, None) {
                    Ok(c) => c.id,
                    Err(e) => {
                        tracing::warn!("coleção do Playnite {name}: {e}");
                        continue;
                    }
                },
            };
            if ctx.collections.find(&cid).is_some_and(|c| !c.contains(id)) {
                let _ = ctx.collections.toggle(&cid, id);
            }
        }
    }
    ctx.sync_collections();
    tracing::info!(
        new = plan.new.len(),
        matched = plan.matched.len(),
        "Playnite importado"
    );
    ctx.toast(match plan.new.len() {
        0 => ctx.t("playnite.done_state"),
        n => tako_core::i18n::tn("playnite.done", n as i64, &[]),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_sources::playnite::import::{PlayniteImportPlan, PlayniteNewGame, PlayniteRecord};

    #[test]
    fn apply_adds_games_state_and_collections() {
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(std::env::temp_dir().join("tako-playnite-test")),
            tako_platform::Platform::simulated(),
            true,
        );
        let new = |id: &str, cats: &[&str]| PlayniteNewGame {
            source: tako_sources::playnite::library::PlayniteGame {
                favorite: true,
                ..Default::default()
            },
            record: PlayniteRecord {
                id: id.into(),
                title: id.into(),
                path: "C:/g.exe".into(),
                arguments: None,
                working_directory: None,
                added_at: chrono::Utc::now(),
                platform: None,
            },
            platform_name: None,
            collections: cats.iter().map(|c| c.to_string()).collect(),
        };
        let plan = PlayniteImportPlan {
            new: vec![
                new("manual:playnite-a", &["RPG"]),
                new("manual:playnite-b", &["rpg"]),
            ],
            ..Default::default()
        };
        apply(&mut ctx, &plan);
        assert!(
            ctx.library
                .find("manual:playnite-a")
                .is_some_and(|g| g.is_favorite)
        );
        let rpg: Vec<_> = ctx
            .collections
            .collections()
            .iter()
            .filter(|c| c.name.eq_ignore_ascii_case("rpg"))
            .collect();
        assert_eq!(rpg.len(), 1);
        assert_eq!(rpg[0].game_ids.len(), 2);
    }
}
