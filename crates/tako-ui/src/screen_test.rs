//! Testar a tela de jogo (Configurações › Tela de jogo), como o `TestGameScreenAsync` do C#: liga a tela de jogo,
//! leva o Tako para ela, pergunta "Você consegue ver esta tela?" e, com ou sem resposta, devolve as telas. Sem
//! resposta em 15 s, conta como "não". Confirmada, a combinação tela + modo não é perguntada no primeiro jogo.

use crate::model::PanelRow;
use crate::shell::Ctx;
use crate::shell::ctx::keys;
use crate::shell::layer::{Flow, MenuItem, MenuLayer};
use crate::tasks;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;
use tako_core::session::screen::{GameScreenOutcome, GameScreenSession};
use tako_core::session::settings::game_screen_confirmations;

const SECONDS: u32 = 15;

thread_local! {
    static TIMER: RefCell<Option<slint::Timer>> = const { RefCell::new(None) };
}

/// Estado do teste em andamento; `session` vira `None` quando as telas já voltaram.
struct Test {
    session: RefCell<Option<GameScreenSession>>,
    setup_key: String,
    left: Cell<u32>,
}

pub fn start(ctx: &mut Ctx) {
    if ctx.running.is_some() {
        ctx.toast(ctx.t("screen_test.in_use"));
        return;
    }
    let Some(settings) =
        tako_core::session::settings::resolve(&crate::session::global_settings(ctx), None)
            .game_screen
    else {
        ctx.toast(ctx.t("screen_test.choose_first"));
        return;
    };
    let displays = ctx.platform.displays.clone();
    ctx.toast(ctx.t("screen_test.switching"));
    tasks::run(
        ctx,
        move || {
            let mut session = GameScreenSession::default();
            let result = session.begin(displays.as_ref(), &settings, None);
            (session, result, settings.setup_key())
        },
        |ctx, (session, result, setup_key)| {
            if result.outcome != GameScreenOutcome::Applied {
                let reason = result
                    .reason
                    .unwrap_or_else(|| ctx.t("session.ui.screen_failed"));
                ctx.toast(ctx.tf("screen_test.failed", &[("reason", &reason)]));
                return;
            }
            ctx.move_window = result.target_bounds;
            let name = result.target.map(|t| t.name).unwrap_or_default();
            let test = Rc::new(Test {
                session: RefCell::new(Some(session)),
                setup_key,
                left: Cell::new(SECONDS),
            });
            ctx.pending_layer = Some(Box::new(dialog(ctx, &name, test.clone())));
            countdown(test);
        },
    );
}

fn dialog(ctx: &Ctx, name: &str, test: Rc<Test>) -> MenuLayer {
    let (yes, no, back) = (test.clone(), test.clone(), test.clone());
    MenuLayer::dialog(ctx.t("screen_test.title"), move |ctx| {
        let (yes, no) = (yes.clone(), no.clone());
        vec![
            MenuItem::new(PanelRow::button("yes", ctx.t("screen_test.yes"))).on(move |ctx| {
                finish(ctx, &yes, true);
                Flow::Close
            }),
            MenuItem::new(PanelRow::button(
                "no",
                ctx.tf("screen_test.no", &[("seconds", &no.left.get().to_string())]),
            ))
            .on(move |ctx| {
                finish(ctx, &no, false);
                Flow::Close
            }),
        ]
    })
    .text(ctx.tf("screen_test.text", &[("screen", name)]))
    .on_back(move |ctx| {
        finish(ctx, &back, false);
        Flow::Close
    })
}

fn countdown(test: Rc<Test>) {
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(1),
        move || {
            let Some(shell) = tasks::shell() else { return };
            let mut s = shell.borrow_mut();
            let left = test.left.get().saturating_sub(1);
            test.left.set(left);
            if left == 0 {
                s.pop_layer();
                finish(&mut s.ctx, &test, false);
            }
            s.ctx.dirty = true;
        },
    );
    TIMER.with(|t| *t.borrow_mut() = Some(timer));
}

/// Responde (uma vez só) e devolve as telas fora da thread da UI.
fn finish(ctx: &mut Ctx, test: &Test, seen: bool) {
    TIMER.with(|t| t.borrow_mut().take());
    let Some(mut session) = test.session.borrow_mut().take() else {
        return;
    };
    if seen {
        let saved = game_screen_confirmations::remember(
            ctx.settings.get(keys::GAME_SCREEN_CONFIRMED),
            &test.setup_key,
        );
        ctx.settings.set(keys::GAME_SCREEN_CONFIRMED, &saved);
    }
    let displays = ctx.platform.displays.clone();
    tasks::run(
        ctx,
        move || {
            session.end(displays.as_ref());
            displays
                .list()
                .unwrap_or_default()
                .into_iter()
                .find(|d| d.is_primary && d.is_active)
                .and_then(|d| d.bounds)
        },
        move |ctx, home| {
            ctx.move_window = home;
            ctx.toast(ctx.t(if seen {
                "screen_test.confirmed"
            } else {
                "screen_test.not_confirmed"
            }));
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answering_twice_restores_once() {
        let test = Test {
            session: RefCell::new(Some(GameScreenSession::default())),
            setup_key: "k".into(),
            left: Cell::new(SECONDS),
        };
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(std::env::temp_dir().join("tako-screen-test")),
            tako_platform::Platform::simulated(),
            true,
        );
        finish(&mut ctx, &test, true);
        assert!(test.session.borrow().is_none());
        assert!(
            ctx.settings
                .string(keys::GAME_SCREEN_CONFIRMED)
                .contains('k')
        );
        finish(&mut ctx, &test, false);
    }
}
