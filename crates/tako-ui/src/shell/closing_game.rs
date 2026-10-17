//! "Fechando o jogo…" depois de Encerrar jogo: a interface espera o jogo sair de verdade (o fim chega pelo
//! acompanhamento da sessão, que fecha esta camada no "Bem-vindo de volta"). Se o jogo não fechar em
//! [`PATIENCE`] (ex.: pergunta se quer salvar), oferece voltar ao jogo para responder ou forçar o encerramento.

use super::layer::{Flow, Layer};
use super::{Cmd, Ctx, Effect};
use crate::model::{Hint, Overlay, PanelRow};
use std::time::{Duration, Instant};

/// Quanto esperar o jogo fechar sozinho antes de oferecer forçar.
pub const PATIENCE: Duration = Duration::from_secs(10);

pub struct ClosingGame {
    title: String,
    since: Instant,
    focus: usize,
}

impl ClosingGame {
    pub fn new(ctx: &mut Ctx) -> Self {
        let since = Instant::now();
        ctx.redraw_at = Some(since + PATIENCE);
        ClosingGame {
            title: ctx
                .running
                .as_ref()
                .map(|r| r.title.clone())
                .unwrap_or_default(),
            since,
            focus: 0,
        }
    }

    fn stuck(&self) -> bool {
        self.since.elapsed() >= PATIENCE
    }

    fn rows(&self, ctx: &Ctx) -> Vec<PanelRow> {
        let mut rows = vec![PanelRow::button("resume", ctx.t("cc.back_to_game")).icon("play")];
        if self.stuck() {
            rows.push(
                PanelRow::button("force", ctx.t("session.force_close"))
                    .icon("power")
                    .danger(),
            );
        }
        rows
    }
}

impl Layer for ClosingGame {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let n = self.rows(ctx).len();
        match cmd {
            Cmd::Up => self.focus = self.focus.saturating_sub(1),
            Cmd::Down => self.focus = (self.focus + 1).min(n - 1),
            Cmd::Accept => match self.rows(ctx).get(self.focus).map(|r| r.key.as_str()) {
                Some("resume") => {
                    ctx.effect(Effect::ResumeGame);
                    return Flow::Close;
                }
                Some("force") => {
                    ctx.effect(Effect::ForceEndGame);
                    // Volta a esperar: o fim ainda vem pelo acompanhamento.
                    self.since = Instant::now();
                    self.focus = 0;
                    ctx.redraw_at = Some(self.since + PATIENCE);
                }
                _ => {}
            },
            // Sem sair daqui enquanto o jogo fecha: a home ainda mostraria o jogo como aberto.
            _ => {}
        }
        Flow::Stay
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        let rows = self.rows(ctx);
        Overlay {
            kind: "dialog".into(),
            title: ctx.tf("session.closing", &[("game", &self.title)]),
            text: ctx.t(if self.stuck() {
                "session.closing_stuck"
            } else {
                "session.closing_wait"
            }),
            sel: self.focus.min(rows.len() - 1) as i32,
            rows,
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![ctx.hint(Cmd::Accept, &ctx.t("hint.select"))]
    }

    fn click(&mut self, _zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        self.focus = index;
        self.handle(Cmd::Accept, ctx)
    }

    fn hover(&mut self, _zone: &str, index: usize) {
        self.focus = index;
    }

    fn ends_with_game(&self) -> bool {
        true
    }
}
