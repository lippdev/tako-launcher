//! Camadas sobre a página (diálogos, painéis laterais, teclado virtual): a de cima recebe a entrada.
//!
//! `MenuLayer` cobre a maioria dos casos (diálogos de opções, confirmações, painéis de filtros e de
//! configurações do jogo): as linhas são reconstruídas a cada desenho a partir do estado, então os valores
//! mostrados nunca ficam velhos.

use super::{Cmd, Ctx};
use crate::model::{Hint, Overlay, PanelRow};
use std::rc::Rc;

/// O que a camada pede depois de tratar um comando.
pub enum Flow {
    Stay,
    Close,
    /// Fecha esta e abre outra no lugar.
    Replace(Box<dyn Layer>),
    /// Abre outra por cima.
    Push(Box<dyn Layer>),
    /// Fecha todas as camadas.
    CloseAll,
}

pub trait Layer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow;
    fn render(&self, ctx: &Ctx) -> Overlay;
    fn hints(&self, ctx: &Ctx) -> Vec<Hint>;
    /// Há um campo de texto: letras do teclado físico viram texto.
    fn typing(&self) -> bool {
        false
    }
    fn text(&mut self, _text: &str, _ctx: &mut Ctx) {}
    fn erase(&mut self, _ctx: &mut Ctx) {}
    /// O campo de texto do modo desktop foi editado (texto inteiro: colar, apagar uma seleção).
    fn replace_text(&mut self, _text: &str, _ctx: &mut Ctx) {}
    /// Tecla física crua (`text` do Slint) com os modificadores, antes de virar comando: `Some` = a camada tratou
    /// (captura do atalho de teclado).
    fn key(
        &mut self,
        _text: &str,
        _mods: tako_core::input::shortcut::KeyModifiers,
        _ctx: &mut Ctx,
    ) -> Option<Flow> {
        None
    }
    /// Clique/toque numa zona da camada (índice da linha, tecla...). Padrão: focar e aceitar.
    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        let _ = (zone, index);
        self.handle(Cmd::Accept, ctx)
    }
    fn hover(&mut self, _zone: &str, _index: usize) {}
    /// A camada só faz sentido com o jogo aberto: sai quando ele termina.
    fn ends_with_game(&self) -> bool {
        false
    }
    /// Mostra medidas que mudam sem entrada (painel de desempenho): redesenhada a cada segundo.
    fn live(&self) -> bool {
        false
    }
}

pub type Action = Rc<dyn Fn(&mut Ctx) -> Flow>;
pub type Adjust = Rc<dyn Fn(&mut Ctx, i32) -> Flow>;

/// Linha de menu: o que mostra e o que A, ← e → fazem.
#[derive(Clone)]
pub struct MenuItem {
    pub row: PanelRow,
    pub accept: Option<Action>,
    /// ← e → ajustam (volume, valores que ciclam).
    pub adjust: Option<Adjust>,
}

impl MenuItem {
    pub fn new(row: PanelRow) -> Self {
        MenuItem {
            row,
            accept: None,
            adjust: None,
        }
    }
    pub fn on(mut self, f: impl Fn(&mut Ctx) -> Flow + 'static) -> Self {
        self.accept = Some(Rc::new(f));
        self
    }
    pub fn on_adjust(mut self, f: impl Fn(&mut Ctx, i32) -> Flow + 'static) -> Self {
        self.adjust = Some(Rc::new(f));
        self
    }
}

pub type Builder = Rc<dyn Fn(&Ctx) -> Vec<MenuItem>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    Dialog,
    Panel,
    /// Menu junto do ponteiro (modo desktop: ⋯ e botão direito).
    Menu,
}

pub struct MenuLayer {
    pub kind: MenuKind,
    pub title: String,
    pub overline: String,
    pub text: String,
    pub wide: bool,
    pub build: Builder,
    pub focus: usize,
    /// O que B faz além de fechar (ex.: devolver o foco a quem abriu).
    pub on_back: Option<Action>,
    pub accept_label: Option<String>,
    /// Indicador de passos de um assistente (nomes e o atual); o desktop desenha, a tela cheia usa a etiqueta.
    pub steps: (Vec<String>, usize),
    /// Onboarding do desktop em tela inteira (TelaOnboarding): coluna à esquerda, parede de capas ao fundo e uma
    /// nota de rodapé.
    pub full: bool,
    pub note: String,
}

impl MenuLayer {
    pub fn new(
        kind: MenuKind,
        title: impl Into<String>,
        build: impl Fn(&Ctx) -> Vec<MenuItem> + 'static,
    ) -> Self {
        MenuLayer {
            kind,
            title: title.into(),
            overline: String::new(),
            text: String::new(),
            wide: false,
            build: Rc::new(build),
            focus: usize::MAX,
            on_back: None,
            accept_label: None,
            steps: (Vec::new(), 0),
            full: false,
            note: String::new(),
        }
    }

    pub fn dialog(
        title: impl Into<String>,
        build: impl Fn(&Ctx) -> Vec<MenuItem> + 'static,
    ) -> Self {
        Self::new(MenuKind::Dialog, title, build)
    }

    pub fn panel(
        title: impl Into<String>,
        build: impl Fn(&Ctx) -> Vec<MenuItem> + 'static,
    ) -> Self {
        Self::new(MenuKind::Panel, title, build)
    }

    pub fn menu(title: impl Into<String>, build: impl Fn(&Ctx) -> Vec<MenuItem> + 'static) -> Self {
        Self::new(MenuKind::Menu, title, build)
    }

    pub fn text(mut self, t: impl Into<String>) -> Self {
        self.text = t.into();
        self
    }

    pub fn overline(mut self, t: impl Into<String>) -> Self {
        self.overline = t.into();
        self
    }

    /// Em tela inteira no desktop (onboarding), com a nota de rodapé.
    pub fn full(mut self, note: impl Into<String>) -> Self {
        self.full = true;
        self.note = note.into();
        self
    }

    pub fn steps(mut self, names: Vec<String>, current: usize) -> Self {
        self.steps = (names, current);
        self
    }

    pub fn wide(mut self) -> Self {
        self.wide = true;
        self
    }

    pub fn focus_at(mut self, index: usize) -> Self {
        self.focus = index;
        self
    }

    pub fn on_back(mut self, f: impl Fn(&mut Ctx) -> Flow + 'static) -> Self {
        self.on_back = Some(Rc::new(f));
        self
    }

    fn items(&self, ctx: &Ctx) -> Vec<MenuItem> {
        (self.build)(ctx)
    }

    /// Foco válido: o pedido, ou a primeira linha focável.
    fn current(&self, items: &[MenuItem]) -> Option<usize> {
        if items.get(self.focus).is_some_and(|i| i.row.is_focusable()) {
            return Some(self.focus);
        }
        let start = self.focus.min(items.len());
        (start..items.len())
            .chain(0..start)
            .find(|&i| items[i].row.is_focusable())
    }

    fn step(&self, items: &[MenuItem], from: usize, delta: i32) -> usize {
        let mut i = from as i32;
        loop {
            i += delta;
            if i < 0 || i >= items.len() as i32 {
                return from;
            }
            if items[i as usize].row.is_focusable() {
                return i as usize;
            }
        }
    }
}

impl Layer for MenuLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let items = self.items(ctx);
        let Some(cur) = self.current(&items) else {
            return if matches!(cmd, Cmd::Back | Cmd::Accept) {
                Flow::Close
            } else {
                Flow::Stay
            };
        };
        self.focus = cur;
        match cmd {
            Cmd::Up => self.focus = self.step(&items, cur, -1),
            Cmd::Down => self.focus = self.step(&items, cur, 1),
            Cmd::Left | Cmd::Right => {
                if let Some(adj) = &items[cur].adjust {
                    return adj(ctx, if cmd == Cmd::Left { -1 } else { 1 });
                }
            }
            Cmd::Accept => {
                let item = &items[cur];
                if item.row.enabled
                    && let Some(f) = &item.accept
                {
                    return f(ctx);
                }
                if item.accept.is_none() && item.adjust.is_some() {
                    return items[cur]
                        .adjust
                        .as_ref()
                        .map(|a| a(ctx, 1))
                        .unwrap_or(Flow::Stay);
                }
            }
            Cmd::Back => {
                if let Some(f) = &self.on_back {
                    return f(ctx);
                }
                return Flow::Close;
            }
            _ => {}
        }
        Flow::Stay
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        let items = self.items(ctx);
        let focus = self.current(&items).map(|i| i as i32).unwrap_or(-1);
        Overlay {
            kind: match self.kind {
                MenuKind::Panel => "panel",
                MenuKind::Dialog => "dialog",
                MenuKind::Menu => "menu",
            }
            .into(),
            title: self.title.clone(),
            overline: self.overline.clone(),
            text: self.text.clone(),
            rows: items.into_iter().map(|i| i.row).collect(),
            sel: focus,
            wide: self.wide,
            tabs: self.steps.0.clone(),
            tab: self.steps.1 as i32,
            // Onboarding em tela inteira: marca, nota e as capas da parede ao fundo.
            tab2: i32::from(self.full),
            input_hint: self.note.clone(),
            thumbs: if self.full {
                super::first_run::wall(ctx)
            } else {
                Vec::new()
            },
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let items = self.items(ctx);
        let adjustable = self
            .current(&items)
            .is_some_and(|i| items[i].adjust.is_some());
        let mut h = vec![
            ctx.hint(
                Cmd::Accept,
                self.accept_label
                    .as_deref()
                    .unwrap_or(&ctx.t("hint.select")),
            ),
        ];
        if adjustable {
            h.push(ctx.hint_raw("←→", &ctx.t("hint.adjust")));
        }
        h.push(ctx.hint(Cmd::Back, &ctx.t("hint.back")));
        h
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        if zone == "outside" {
            return self.handle(Cmd::Back, ctx);
        }
        self.focus = index;
        // ‹ › do modo desktop numa linha que se ajusta (volume, valores que ciclam).
        if zone == "row-dec" || zone == "row-inc" {
            // Sem ajuste, a linha só tem o "próximo valor" do A.
            let adjustable = self
                .items(ctx)
                .get(index)
                .is_some_and(|i| i.adjust.is_some());
            if !adjustable {
                return self.handle(Cmd::Accept, ctx);
            }
            return self.handle(
                if zone == "row-dec" {
                    Cmd::Left
                } else {
                    Cmd::Right
                },
                ctx,
            );
        }
        self.handle(Cmd::Accept, ctx)
    }

    fn hover(&mut self, zone: &str, index: usize) {
        if zone == "row" {
            self.focus = index;
        }
    }
}
