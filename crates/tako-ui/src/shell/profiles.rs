//! Perfis locais (UX-004) na interface: Configurações › Geral › Perfil (criar, renomear, apagar, trocar e
//! perguntar ao abrir) e o "Quem está jogando?" ao abrir o Tako. Trocar grava o perfil em uso e reabre o Tako
//! com os dados dele (`Effect::SwitchProfile`).

use super::keyboard::KeyboardLayer;
use super::layer::{Flow, MenuItem, MenuLayer};
use super::{Ctx, Effect};
use crate::model::PanelRow;
use tako_core::profiles::{self, MAIN, Profile, ProfileError, Profiles};

fn load(ctx: &Ctx) -> Profiles {
    Profiles::load(&ctx.paths.machine)
}

/// Nome mostrado: o principal sem nome aparece como "Principal".
pub fn name(ctx: &Ctx, p: &Profile) -> String {
    if p.name.is_empty() {
        ctx.t("profiles.main")
    } else {
        p.name.clone()
    }
}

pub fn active_name(ctx: &Ctx) -> String {
    name(ctx, &load(ctx).active())
}

fn save(ctx: &mut Ctx, p: &Profiles) -> bool {
    match p.save(&ctx.paths.machine) {
        Ok(()) => {
            ctx.active_profile = p.active();
            true
        }
        Err(e) => {
            tracing::warn!("perfis não gravados: {e}");
            ctx.toast(ctx.t("profiles.save_failed"));
            false
        }
    }
}

fn error_text(ctx: &Ctx, e: ProfileError) -> String {
    ctx.t(match e {
        ProfileError::InvalidName => "profiles.invalid_name",
        ProfileError::DuplicateName => "profiles.duplicate_name",
        ProfileError::TooMany => "profiles.too_many",
        ProfileError::Unknown | ProfileError::Protected => "profiles.save_failed",
    })
}

/// Usa o perfil `id`: grava como o perfil em uso e pede para reabrir o Tako com ele.
pub fn switch(ctx: &mut Ctx, id: &str) -> Flow {
    let mut p = load(ctx);
    if p.active == id {
        return Flow::CloseAll;
    }
    // A sessão do jogo aberto (tempo de jogo, tela e áudio) é do perfil atual.
    if ctx.running.is_some() || ctx.launching {
        ctx.toast(ctx.t("profiles.busy"));
        return Flow::Stay;
    }
    if p.set_active(id).is_err() || !save(ctx, &p) {
        return Flow::Stay;
    }
    profiles::mark_chosen(&ctx.paths.machine);
    ctx.effect(Effect::SwitchProfile);
    Flow::CloseAll
}

fn name_keyboard(ctx: &Ctx, title: String, text: &str, id: Option<String>) -> KeyboardLayer {
    KeyboardLayer::new(ctx, title, text, move |ctx, text| {
        let mut p = load(ctx);
        let done = match &id {
            Some(id) => p.rename(id, text),
            None => p.add(text).map(|_| ()),
        };
        match done {
            Ok(()) if save(ctx, &p) => Flow::Close,
            Ok(()) => Flow::Stay,
            Err(e) => {
                ctx.toast(error_text(ctx, e));
                Flow::Stay
            }
        }
    })
    .hint(ctx.tf(
        "profiles.name_hint",
        &[("max", &profiles::MAX_NAME.to_string())],
    ))
    .check(|ctx, text| {
        profiles::clean_name(text)
            .is_none()
            .then(|| ctx.t("profiles.invalid_name"))
    })
}

/// Configurações › Geral › Perfil.
pub fn panel(ctx: &Ctx) -> MenuLayer {
    MenuLayer::panel(ctx.t("profiles.title"), |ctx| {
        let p = load(ctx);
        let mut v: Vec<MenuItem> = p
            .list
            .iter()
            .map(|x| {
                let id = x.id.clone();
                let desc = if x.id == p.active {
                    ctx.t("profiles.in_use")
                } else {
                    String::new()
                };
                MenuItem::new(
                    PanelRow::radio(x.id.clone(), name(ctx, x), x.id == p.active).desc(desc),
                )
                .on(move |ctx| Flow::Push(Box::new(manage(ctx, &id))))
            })
            .collect();
        v.push(
            MenuItem::new(
                PanelRow::action("new", ctx.t("profiles.new"))
                    .desc(ctx.t("profiles.new_desc"))
                    .enabled(p.list.len() < profiles::MAX_PROFILES),
            )
            .on(|ctx| {
                Flow::Push(Box::new(name_keyboard(
                    ctx,
                    ctx.t("profiles.new"),
                    "",
                    None,
                )))
            }),
        );
        v.push(
            MenuItem::new(
                PanelRow::toggle("ask", ctx.t("profiles.ask"), p.ask_on_start)
                    .desc(ctx.t("profiles.ask_desc")),
            )
            .on(|ctx| {
                let mut p = load(ctx);
                p.ask_on_start = !p.ask_on_start;
                save(ctx, &p);
                Flow::Stay
            }),
        );
        v
    })
    .text(ctx.t("profiles.text"))
}

/// Editar perfil (TelaPerfil): o diálogo do perfil em uso.
pub fn manage_active(ctx: &Ctx) -> MenuLayer {
    manage(ctx, &load(ctx).active)
}

/// Um perfil: usar, renomear ou apagar (o principal e o em uso não se apagam).
fn manage(ctx: &Ctx, id: &str) -> MenuLayer {
    let p = load(ctx);
    let Some(profile) = p.find(id).cloned() else {
        return MenuLayer::dialog(ctx.t("profiles.title"), |_| Vec::new());
    };
    let title = name(ctx, &profile);
    let id = id.to_owned();
    MenuLayer::dialog(title, move |ctx| {
        let p = load(ctx);
        let (use_id, rename_id, remove_id) = (id.clone(), id.clone(), id.clone());
        let current = p.find(&id).map(|x| x.name.clone()).unwrap_or_default();
        vec![
            MenuItem::new(PanelRow::button("use", ctx.t("profiles.use")).enabled(p.active != id))
                .on(move |ctx| switch(ctx, &use_id)),
            MenuItem::new(PanelRow::button("rename", ctx.t("profiles.rename"))).on(move |ctx| {
                Flow::Replace(Box::new(name_keyboard(
                    ctx,
                    ctx.t("profiles.rename"),
                    &current,
                    Some(rename_id.clone()),
                )))
            }),
            MenuItem::new(
                PanelRow::button("remove", ctx.t("profiles.remove"))
                    .danger()
                    .enabled(id != MAIN && p.active != id),
            )
            .on(move |ctx| Flow::Replace(Box::new(confirm_remove(ctx, &remove_id)))),
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
        ]
    })
    .text(if profile.id == MAIN {
        ctx.t("profiles.main_desc")
    } else {
        String::new()
    })
}

fn confirm_remove(ctx: &Ctx, id: &str) -> MenuLayer {
    let title = load(ctx).find(id).map(|p| name(ctx, p)).unwrap_or_default();
    let id = id.to_owned();
    MenuLayer::dialog(
        ctx.tf("profiles.remove_title", &[("name", &title)]),
        move |ctx| {
            let id = id.clone();
            vec![
                MenuItem::new(PanelRow::button("remove", ctx.t("profiles.remove")).danger()).on(
                    move |ctx| {
                        let mut p = load(ctx);
                        if let Err(e) = p.remove(&id) {
                            ctx.toast(error_text(ctx, e));
                            return Flow::Close;
                        }
                        if !save(ctx, &p) {
                            return Flow::Close;
                        }
                        let (machine, id) = (ctx.paths.machine.clone(), id.clone());
                        crate::tasks::run(
                            ctx,
                            move || profiles::delete_data(&machine, &id),
                            |ctx, r| match r {
                                Ok(()) => ctx.toast(ctx.t("profiles.removed")),
                                Err(e) => {
                                    tracing::warn!("dados do perfil não apagados: {e}");
                                    ctx.toast(ctx.t("profiles.remove_failed"));
                                }
                            },
                        );
                        Flow::Close
                    },
                ),
                MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel")))
                    .on(|_| Flow::Close),
            ]
        },
    )
    .text(ctx.t("profiles.remove_text"))
}

/// "Quem está jogando?" ao abrir, quando ligado e com mais de um perfil. Não pergunta na volta de um jogo, com a
/// Central, com um link nem logo depois de uma troca (marcador de `switch`).
pub fn chooser_on_start(ctx: &Ctx, plain_start: bool) -> Option<MenuLayer> {
    let chosen = profiles::take_chosen(&ctx.paths.machine);
    let p = load(ctx);
    if chosen || !plain_start || !p.ask_on_start || p.list.len() < 2 {
        return None;
    }
    let active = p.list.iter().position(|x| x.id == p.active).unwrap_or(0);
    Some(
        MenuLayer::dialog(ctx.t("profiles.who"), |ctx| {
            let p = load(ctx);
            p.list
                .iter()
                .map(|x| {
                    let id = x.id.clone();
                    MenuItem::new(PanelRow::radio(
                        x.id.clone(),
                        name(ctx, x),
                        x.id == p.active,
                    ))
                    .on(move |ctx| switch(ctx, &id))
                })
                .collect()
        })
        .focus_at(active),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::Cmd;
    use crate::shell::layer::Layer;

    #[test]
    fn creating_switching_and_removing_profiles_from_settings() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        let layer = panel(&ctx);
        let rows = layer.render(&ctx).rows;
        assert_eq!(rows[0].title, ctx.t("profiles.main"));
        assert!(rows[0].on);

        // Novo perfil pelo teclado: grava a lista.
        let mut kb = name_keyboard(&ctx, String::new(), "Ana", None);
        assert!(matches!(kb.handle(Cmd::Menu, &mut ctx), Flow::Close));
        let mut kb = name_keyboard(&ctx, String::new(), "ana", None);
        assert!(matches!(kb.handle(Cmd::Menu, &mut ctx), Flow::Stay));
        assert_eq!(
            ctx.toast.take().unwrap().0,
            ctx.t("profiles.duplicate_name")
        );
        let ana = Profiles::load(dir.path()).list[1].id.clone();
        assert_eq!(layer.render(&ctx).rows[1].title, "Ana");
        // Renomear pelo mesmo teclado.
        let mut kb = name_keyboard(&ctx, String::new(), "Ana Clara", Some(ana.clone()));
        kb.handle(Cmd::Menu, &mut ctx);
        assert_eq!(layer.render(&ctx).rows[1].title, "Ana Clara");

        // Trocar grava o perfil em uso, marca a escolha e pede para reabrir.
        assert!(matches!(switch(&mut ctx, &ana), Flow::CloseAll));
        assert_eq!(Profiles::load(dir.path()).active, ana);
        assert!(ctx.effects.contains(&Effect::SwitchProfile));
        assert!(profiles::take_chosen(dir.path()));

        // O perfil em uso não se apaga; o outro sim, com a pasta dele.
        let mut remove = confirm_remove(&ctx, &ana);
        remove.handle(Cmd::Accept, &mut ctx);
        assert!(Profiles::load(dir.path()).find(&ana).is_some());
        let mut p = Profiles::load(dir.path());
        p.set_active(MAIN).unwrap();
        p.save(dir.path()).unwrap();
        let folder = dir.path().join("profiles").join(&ana);
        std::fs::create_dir_all(&folder).unwrap();
        let mut remove = confirm_remove(&ctx, &ana);
        remove.handle(Cmd::Accept, &mut ctx);
        assert!(Profiles::load(dir.path()).find(&ana).is_none());
        assert!(!folder.exists());
    }

    #[test]
    fn who_is_playing_only_on_a_plain_start_with_two_profiles() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        assert!(chooser_on_start(&ctx, true).is_none());
        let mut p = Profiles::load(dir.path());
        let bia = p.add("Bia").unwrap();
        p.ask_on_start = true;
        p.save(dir.path()).unwrap();
        assert!(chooser_on_start(&ctx, false).is_none());
        profiles::mark_chosen(dir.path());
        assert!(chooser_on_start(&ctx, true).is_none());
        let mut who = chooser_on_start(&ctx, true).expect("pergunta");
        let rows = who.render(&ctx).rows;
        assert_eq!(rows.len(), 2);
        assert!(rows[0].on);
        // Escolher o perfil em uso só fecha; outro troca.
        assert!(matches!(who.handle(Cmd::Accept, &mut ctx), Flow::CloseAll));
        assert!(ctx.effects.is_empty());
        who.handle(Cmd::Down, &mut ctx);
        who.handle(Cmd::Accept, &mut ctx);
        assert_eq!(Profiles::load(dir.path()).active, bia);
        assert!(ctx.effects.contains(&Effect::SwitchProfile));
    }
}
