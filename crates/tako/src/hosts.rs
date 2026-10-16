//! Hosts da interface: com supervisor (o jogo continua com ele e a interface fecha) ou sozinha
//! (`--no-supervisor`: a janela some durante o jogo e o acompanhamento roda aqui mesmo).

use crate::protocol::{self, UiMsg};
use std::time::{Duration, Instant};
use tako_ui::app::{Host, HostAction};
use tako_ui::gamepad::PadState;
use tako_ui::session::{self, Running, Started};
use tako_ui::shell::{Ctx, Effect, RunningGame};

/// Abre o jogo e, se ele puder ser acompanhado, devolve o pedido de acompanhamento.
fn launch(ctx: &mut Ctx, id: &str) -> Option<session::TrackRequest> {
    match session::start(ctx, id) {
        Ok(Started::Tracking(req)) => Some(req),
        Ok(Started::Untracked) => {
            let title = ctx
                .library
                .find(id)
                .map(|g| g.title.clone())
                .unwrap_or_default();
            ctx.toast(ctx.tf("session.opening", &[("game", &title)]));
            None
        }
        Err(e) => {
            ctx.toast(e);
            None
        }
    }
}

/// Interface aberta pelo supervisor: conversa pela saída padrão.
pub struct SupervisedHost {
    out: std::io::Stdout,
    /// O supervisor sumiu (canal fechado): a interface segue sozinha, sem insistir.
    lost: bool,
    seq: u64,
    last_beat: Instant,
}

impl SupervisedHost {
    pub fn new() -> Self {
        SupervisedHost {
            out: std::io::stdout(),
            lost: false,
            seq: 0,
            last_beat: Instant::now() - protocol::HEARTBEAT,
        }
    }

    fn send(&mut self, msg: &UiMsg) {
        if self.lost {
            return;
        }
        if let Err(e) = protocol::write(&mut self.out.lock(), msg) {
            tracing::warn!("supervisor fora do ar ({e}); seguindo sem ele");
            self.lost = true;
        }
    }
}

impl Host for SupervisedHost {
    fn handle(&mut self, effect: &Effect, ctx: &mut Ctx) -> HostAction {
        match effect {
            Effect::Launch(id) => {
                if let Some(req) = launch(ctx, id) {
                    self.send(&UiMsg::Track { req });
                    return HostAction::Close;
                }
            }
            Effect::ResumeGame => {
                self.send(&UiMsg::Resume);
                return HostAction::Close;
            }
            Effect::EndGame => self.send(&UiMsg::EndGame),
            Effect::GoHomeKeepingGame => {}
            Effect::PointerChanged => self.send(&UiMsg::Pointer {
                hide: ctx.platform.cursor.is_hidden(),
                mouse: ctx.controller_mouse,
            }),
            Effect::ExitToDesktop => {
                self.send(&UiMsg::Closing {
                    reason: "exit".into(),
                });
                return HostAction::Close;
            }
            Effect::ExitSafeMode => {
                self.send(&UiMsg::Closing {
                    reason: "normal".into(),
                });
                return HostAction::Close;
            }
            other => tracing::info!("pedido sem tratamento: {other:?}"),
        }
        HostAction::Stay
    }

    fn tick(&mut self, _ctx: &mut Ctx, _pad: &PadState, _now: Duration) -> HostAction {
        if self.last_beat.elapsed() >= protocol::HEARTBEAT {
            self.last_beat = Instant::now();
            self.seq += 1;
            let seq = self.seq;
            self.send(&UiMsg::Heartbeat { seq });
        }
        HostAction::Stay
    }
}

/// Sem supervisor: esconde a janela durante o jogo e acompanha daqui.
#[derive(Default)]
pub struct LocalHost {
    running: Option<Running>,
    last_poll: Option<Instant>,
    last_tick: Option<Duration>,
}

impl Host for LocalHost {
    fn handle(&mut self, effect: &Effect, ctx: &mut Ctx) -> HostAction {
        match effect {
            Effect::Launch(id) => {
                if let Some(req) = launch(ctx, id) {
                    match Running::new(req.clone()) {
                        Ok(mut r) => {
                            r.begin(&ctx.platform);
                            ctx.running = Some(RunningGame {
                                id: req.game_id.clone(),
                                title: req.title.clone(),
                            });
                            ctx.controller_mouse = req.controller_mouse;
                            self.running = Some(r);
                            return HostAction::Hide;
                        }
                        Err(e) => ctx.toast(e),
                    }
                }
            }
            Effect::ResumeGame => return HostAction::Hide,
            Effect::EndGame => {
                if let Some(r) = &self.running
                    && let Err(e) = r.request_close(&ctx.platform)
                {
                    ctx.toast(e);
                }
            }
            Effect::PointerChanged => {
                let hide = ctx.platform.cursor.is_hidden();
                if let Some(r) = self.running.as_mut() {
                    r.set_pointer(&ctx.platform, hide, ctx.controller_mouse);
                }
            }
            Effect::GoHomeKeepingGame => {}
            Effect::ExitToDesktop => return HostAction::Close,
            Effect::ExitSafeMode => return restart_normal(ctx),
            other => tracing::info!("pedido sem tratamento: {other:?}"),
        }
        HostAction::Stay
    }

    fn tick(&mut self, ctx: &mut Ctx, pad: &PadState, now: Duration) -> HostAction {
        let Some(running) = self.running.as_mut() else {
            return HostAction::Stay;
        };
        let dt = self
            .last_tick
            .map(|t| (now - t).as_secs_f64())
            .unwrap_or(0.016);
        self.last_tick = Some(now);
        running.pointer_step(&ctx.platform, pad, now, dt);
        if self.last_poll.is_some_and(|t| t.elapsed() < Running::POLL) {
            return HostAction::Stay;
        }
        self.last_poll = Some(Instant::now());
        let Some(played) = running.poll(&ctx.platform) else {
            return HostAction::Stay;
        };
        running.finish(&ctx.platform, &ctx.paths);
        let id = running.req.game_id.clone();
        self.running = None;
        if let Err(e) = ctx.library.record_session(&id, played) {
            tracing::warn!("tempo de jogo: {e}");
        }
        ctx.running = None;
        ctx.open_details = None;
        ctx.welcome_back = Some((id, played));
        HostAction::Show
    }
}

/// Sem supervisor: abre outro Tako sem `--safe-mode` (ele espera este soltar a trava) e fecha este.
fn restart_normal(ctx: &mut Ctx) -> HostAction {
    let spawned = std::env::current_exe().and_then(|exe| {
        std::process::Command::new(exe)
            .args(
                std::env::args()
                    .skip(1)
                    .filter(|a| a != tako_core::supervision::args::SAFE_MODE),
            )
            .env(crate::RESTART_ENV, "1")
            .spawn()
    });
    match spawned {
        Ok(_) => HostAction::Close,
        Err(e) => {
            ctx.toast(e.to_string());
            HostAction::Stay
        }
    }
}
