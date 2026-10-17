//! "Abrindo…" depois de Jogar: a arte do jogo em tela cheia até a janela dele aparecer (o supervisor avisa e a
//! interface some aos poucos, ver `app::game_shown`), em vez de mostrar a área de trabalho enquanto o jogo carrega.
//! B volta ao Tako com o jogo marcado como aberto (a loja pode estar atualizando o jogo).

use super::layer::{Flow, Layer};
use super::{Cmd, Ctx};
use crate::model::{Hint, Overlay};
use std::time::{Duration, Instant};

/// Depois disso o texto explica a demora.
pub const SLOW: Duration = Duration::from_secs(20);

pub struct LaunchSplash {
    title: String,
    since: Instant,
}

impl LaunchSplash {
    pub fn new(ctx: &mut Ctx) -> Self {
        let since = Instant::now();
        ctx.redraw_at = Some(since + SLOW);
        ctx.launching = true;
        // Por cima até a janela do jogo cobrir a tela; depois de SLOW, não esconde mais diálogos da loja.
        ctx.launch_on_top_until = Some(since + SLOW);
        LaunchSplash {
            title: ctx
                .running
                .as_ref()
                .map(|r| r.title.clone())
                .unwrap_or_default(),
            since,
        }
    }
}

impl Layer for LaunchSplash {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        if cmd == Cmd::Back {
            ctx.launching = false;
            return Flow::Close;
        }
        Flow::Stay
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        Overlay {
            kind: "splash".into(),
            title: self.title.clone(),
            overline: ctx.t("session.launching"),
            text: if self.since.elapsed() >= SLOW {
                ctx.t("session.launching_slow")
            } else {
                String::new()
            },
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![ctx.hint(Cmd::Back, &ctx.t("session.back_to_tako"))]
    }

    fn click(&mut self, _zone: &str, _index: usize, _ctx: &mut Ctx) -> Flow {
        Flow::Stay
    }

    fn ends_with_game(&self) -> bool {
        true
    }
}
