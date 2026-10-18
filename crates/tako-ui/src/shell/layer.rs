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
/// Controle deslizante do desktop: o valor novo (0–100) e se a pessoa soltou.
pub type Slide = Rc<dyn Fn(&mut Ctx, i32, bool) -> Flow>;
/// As opções de uma escolha e a atual.
pub type Options = Rc<dyn Fn(&Ctx) -> (Vec<Pick>, usize)>;

/// Uma opção do select do desktop (CampoDeSelecao). Sem `set`, escolher anda o ajuste da linha (→) até ela.
#[derive(Clone)]
pub struct Pick {
    pub label: String,
    /// Nota curta à direita ("Nativa", "Não suportada").
    pub note: String,
    pub enabled: bool,
    pub set: Option<Action>,
}

impl Pick {
    pub fn new(label: impl Into<String>) -> Self {
        Pick {
            label: label.into(),
            note: String::new(),
            enabled: true,
            set: None,
        }
    }
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = note.into();
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn set(mut self, f: impl Fn(&mut Ctx) -> Flow + 'static) -> Self {
        self.set = Some(Rc::new(f));
        self
    }
}

/// Linha de menu: o que mostra e o que A, ← e → fazem.
#[derive(Clone)]
pub struct MenuItem {
    pub row: PanelRow,
    pub accept: Option<Action>,
    /// ← e → ajustam (volume, valores que ciclam).
    pub adjust: Option<Adjust>,
    /// No desktop, a escolha abre um select com estas opções em vez de ciclar.
    pub options: Option<Options>,
    /// No desktop, o controle deslizante (arrastar, trilha, roda, Home/End) vai direto a um valor.
    pub slide: Option<Slide>,
    /// Desktop: linha da fila de capas (duas ações dentro da linha: `accept` e `alt`).
    pub cover: Option<crate::model::CoverRow>,
    pub alt: Option<Action>,
}

/// Rodapé do popup (PadraoPopup): de 1 a 3 ações, todas no fim da lista; mais que isso segue empilhado.
pub(crate) fn footer_fits(rows: &[PanelRow]) -> bool {
    let n = rows.iter().filter(|r| r.kind == "button").count();
    let tail = rows.len() - n;
    (1..=3).contains(&n) && rows[tail..].iter().all(|r| r.kind == "button")
}

/// A principal é a primeira ação (ordem do Tako) que não é link; só uma por popup. Destrutivo nunca vira a pílula
/// branca: a ação fica em `error` no vidro (PadraoPopup).
pub(crate) fn mark_primary(rows: &mut [PanelRow]) {
    if rows
        .iter()
        .any(|r| r.style == "primary" || r.style == "danger")
    {
        return;
    }
    if let Some(r) = rows
        .iter_mut()
        .find(|r| r.kind == "button" && r.style != "link")
    {
        r.style = "primary".into();
    }
}

impl MenuItem {
    /// A linha como o tema a desenha: no desktop, "choice" sem lista (um texto que se digita) é navegação, valor e ›.
    pub fn shown(self, ctx: &Ctx) -> PanelRow {
        let mut row = self.row;
        if ctx.desktop && row.kind == "choice" && self.options.is_none() {
            row.kind = "action".into();
        }
        row
    }
    pub fn new(row: PanelRow) -> Self {
        MenuItem {
            row,
            accept: None,
            adjust: None,
            options: None,
            slide: None,
            cover: None,
            alt: None,
        }
    }
    pub fn cover(
        mut self,
        c: crate::model::CoverRow,
        alt: impl Fn(&mut Ctx) -> Flow + 'static,
    ) -> Self {
        self.cover = Some(c);
        self.alt = Some(Rc::new(alt));
        self
    }
    pub fn on_slide(mut self, f: impl Fn(&mut Ctx, i32, bool) -> Flow + 'static) -> Self {
        self.slide = Some(Rc::new(f));
        self
    }

    /// Zona do controle deslizante: "row-slide:<valor>" enquanto arrasta, "row-drop:<valor>" ao soltar.
    pub fn slide_zone(&self, zone: &str, ctx: &mut Ctx) -> Option<Flow> {
        let (value, done) = zone
            .strip_prefix("row-slide:")
            .map(|v| (v, false))
            .or_else(|| zone.strip_prefix("row-drop:").map(|v| (v, true)))?;
        let slide = self.slide.clone().filter(|_| self.row.enabled)?;
        Some(slide(ctx, value.parse::<i32>().ok()?.clamp(0, 100), done))
    }

    /// Teclas do controle deslizante focado: Shift+← → e Page Up/Down andam 10, Home/End vão a 0 e 100.
    pub fn slide_key(&self, text: &str, shift: bool, ctx: &mut Ctx) -> Option<Flow> {
        use super::keys::{LEFT, RIGHT};
        let slide = self.slide.clone().filter(|_| self.row.enabled)?;
        let now = self.row.value.parse::<i32>().unwrap_or(0);
        let next = match text.chars().next()? {
            LEFT if shift => now - 10,
            RIGHT if shift => now + 10,
            '\u{F72C}' => now + 10, // Page Up
            '\u{F72D}' => now - 10, // Page Down
            '\u{F729}' => 0,        // Home
            '\u{F72B}' => 100,      // End
            _ => return None,
        };
        Some(slide(ctx, next.clamp(0, 100), true))
    }
    pub fn options(mut self, f: impl Fn(&Ctx) -> (Vec<Pick>, usize) + 'static) -> Self {
        self.options = Some(Rc::new(f));
        self
    }

    /// O select do desktop (só quando a linha tem opções e está ativa): a lista junto da pílula, com ✓ na atual.
    pub fn select(&self, ctx: &Ctx) -> Option<Box<dyn Layer>> {
        if !ctx.desktop || !self.row.enabled {
            return None;
        }
        let options = self.options.clone()?;
        // Sem `set` na opção: anda o ajuste (ou o A) da linha para a frente até chegar nela.
        let step: Option<Action> = match (&self.adjust, &self.accept) {
            (Some(a), _) => {
                let a = a.clone();
                Some(Rc::new(move |ctx: &mut Ctx| a(ctx, 1)))
            }
            (None, Some(f)) => Some(f.clone()),
            _ => None,
        };
        let current = options(ctx).1;
        let build = move |ctx: &Ctx| {
            let (picks, cur) = options(ctx);
            picks
                .into_iter()
                .enumerate()
                .map(|(k, p)| {
                    let row = PanelRow::button(format!("opt-{k}"), p.label)
                        .icon(if k == cur { "tick" } else { "" })
                        .value(p.note)
                        .enabled(p.enabled)
                        .emphasized(k == cur);
                    let (options, step, set) = (options.clone(), step.clone(), p.set);
                    MenuItem::new(row).on(move |ctx| {
                        if let Some(set) = &set {
                            return match set(ctx) {
                                Flow::Stay => Flow::Close,
                                Flow::Push(layer) => Flow::Replace(layer),
                                other => other,
                            };
                        }
                        let (list, cur) = options(ctx);
                        let n = list.len().max(1);
                        if let Some(step) = &step {
                            for _ in 0..(k + n - cur.min(n - 1)) % n {
                                step(ctx);
                            }
                        }
                        Flow::Close
                    })
                })
                .collect()
        };
        Some(Box::new(
            MenuLayer::menu("", build).wide().focus_at(current),
        ))
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
    /// Miniatura ao lado do título (desktop: a capa do jogo em Configurações deste jogo).
    pub image: slint::Image,
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
            image: slint::Image::default(),
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

    pub fn image(mut self, image: slint::Image) -> Self {
        self.image = image;
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

    pub(crate) fn items(&self, ctx: &Ctx) -> Vec<MenuItem> {
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
            // No desktop um select só muda pela lista (← → não trocam o valor sem querer).
            Cmd::Left | Cmd::Right if ctx.desktop && items[cur].options.is_some() => {}
            Cmd::Left | Cmd::Right => {
                if let Some(adj) = &items[cur].adjust {
                    return adj(ctx, if cmd == Cmd::Left { -1 } else { 1 });
                }
            }
            Cmd::Accept => {
                let item = &items[cur];
                if let Some(layer) = item.select(ctx) {
                    return Flow::Push(layer);
                }
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
        let mut rows: Vec<PanelRow> = items.into_iter().map(|i| i.shown(ctx)).collect();
        let footer = ctx.desktop
            && matches!(self.kind, MenuKind::Dialog)
            && !self.full
            && self.steps.0.is_empty()
            && footer_fits(&rows);
        if footer {
            mark_primary(&mut rows);
        }
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
            footer,
            // Confirmação curta (S, 440): só título, uma ou duas frases e as ações.
            compact: footer
                && rows.iter().all(|r| r.kind == "button")
                && self.text.chars().count() <= 100,
            rows,
            sel: focus,
            wide: self.wide,
            tabs: self.steps.0.clone(),
            tab: self.steps.1 as i32,
            // Onboarding em tela inteira: marca, nota e as capas da parede ao fundo.
            tab2: i32::from(self.full),
            // Onboarding: o "n de 5" vira "‹ Voltar" no hover.
            tabs2: if self.full {
                vec![ctx.t("first_run.full.back")]
            } else {
                Vec::new()
            },
            input_hint: self.note.clone(),
            image: self.image.clone(),
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
        // Controle deslizante do desktop.
        if let Some(flow) = self
            .items(ctx)
            .get(index)
            .and_then(|i| i.slide_zone(zone, ctx))
        {
            return flow;
        }
        self.handle(Cmd::Accept, ctx)
    }

    fn hover(&mut self, zone: &str, index: usize) {
        if zone == "row" {
            self.focus = index;
        }
    }
}

#[cfg(test)]
mod footer_tests {
    use super::*;

    fn b(key: &str) -> PanelRow {
        PanelRow::button(key, key)
    }

    #[test]
    fn footer_takes_up_to_three_trailing_actions() {
        assert!(footer_fits(&[PanelRow::info("a", "a", "b"), b("ok")]));
        assert!(footer_fits(&[b("a"), b("b"), b("c")]));
        assert!(!footer_fits(&[b("a"), b("b"), b("c"), b("d")]));
        assert!(!footer_fits(&[b("a"), PanelRow::info("a", "a", "b")]));
        assert!(!footer_fits(&[]));
    }

    #[test]
    fn primary_is_the_first_non_link_and_never_beside_a_destructive_action() {
        let mut rows = vec![b("later").link(), b("now"), b("other")];
        mark_primary(&mut rows);
        assert_eq!(rows[1].style, "primary");
        assert_eq!(rows[2].style, "");
        let mut rows = vec![b("delete").danger(), b("cancel")];
        mark_primary(&mut rows);
        assert!(rows.iter().all(|r| r.style != "primary"));
    }
}
