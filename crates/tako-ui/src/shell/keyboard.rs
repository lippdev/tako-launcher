//! Teclado virtual (UX-001 do C#): busca e campos de texto. A digita, X apaga, Y = espaço, ☰ = pronto,
//! B = fechar. O teclado físico digita direto. A busca atualiza os resultados a cada tecla.

use super::layer::{Flow, Layer};
use super::{Cmd, Ctx};
use crate::model::{Hint, Key, Overlay};
use std::rc::Rc;
use tako_core::presentation::virtual_keyboard::{self as vk, VirtualKey, VirtualKeyKind};

type OnChange = Rc<dyn Fn(&mut Ctx, &str)>;
type OnDone = Rc<dyn Fn(&mut Ctx, &str) -> Flow>;
type Check = Rc<dyn Fn(&Ctx, &str) -> Option<String>>;
type OnCancel = Rc<dyn Fn(&mut Ctx)>;

pub struct KeyboardLayer {
    title: String,
    hint: String,
    text: String,
    secret: bool,
    rows: Vec<Vec<VirtualKey>>,
    row: usize,
    col: usize,
    on_change: Option<OnChange>,
    on_done: OnDone,
    check: Option<Check>,
    on_cancel: Option<OnCancel>,
    edited: bool,
    /// Campos livres (argumentos, chaves de API) aceitam qualquer caractere; a busca segue as regras dela.
    free_text: bool,
}

impl KeyboardLayer {
    pub fn new(
        ctx: &Ctx,
        title: impl Into<String>,
        text: &str,
        on_done: impl Fn(&mut Ctx, &str) -> Flow + 'static,
    ) -> Self {
        KeyboardLayer {
            title: title.into(),
            hint: String::new(),
            text: text.to_owned(),
            secret: false,
            rows: vk::rows(ctx.lang()),
            row: 1,
            col: 0,
            on_change: None,
            on_done: Rc::new(on_done),
            check: None,
            on_cancel: None,
            edited: false,
            free_text: true,
        }
    }

    pub fn search(ctx: &Ctx, text: &str, on_change: impl Fn(&mut Ctx, &str) + 'static) -> Self {
        let mut k = Self::new(ctx, ctx.t("search.title"), text, |_, _| Flow::Close);
        k.hint = ctx.t("search.hint");
        k.on_change = Some(Rc::new(on_change));
        k.free_text = false;
        k
    }

    pub fn hint(mut self, h: impl Into<String>) -> Self {
        self.hint = h.into();
        self
    }

    pub fn secret(mut self) -> Self {
        self.secret = true;
        self
    }

    pub fn check(mut self, f: impl Fn(&Ctx, &str) -> Option<String> + 'static) -> Self {
        self.check = Some(Rc::new(f));
        self
    }

    pub fn on_cancel(mut self, f: impl Fn(&mut Ctx) + 'static) -> Self {
        self.on_cancel = Some(Rc::new(f));
        self
    }

    fn set_text(&mut self, text: String, ctx: &mut Ctx) {
        self.edited = true;
        self.text = text;
        if let Some(f) = self.on_change.clone() {
            f(ctx, &self.text);
        }
    }

    fn typed(&self, typed: &str) -> String {
        if self.free_text {
            let mut t = self.text.clone();
            t.extend(typed.chars().filter(|c| !c.is_control()));
            t.chars().take(512).collect()
        } else {
            vk::type_text(&self.text, typed)
        }
    }

    fn done(&mut self, ctx: &mut Ctx) -> Flow {
        if let Some(check) = &self.check
            && check(ctx, &self.text).is_some()
        {
            return Flow::Stay;
        }
        let f = self.on_done.clone();
        f(ctx, self.text.trim())
    }

    fn press(&mut self, ctx: &mut Ctx) -> Flow {
        let Some(key) = self
            .rows
            .get(self.row)
            .and_then(|r| r.get(self.col))
            .cloned()
        else {
            return Flow::Stay;
        };
        match key.kind {
            VirtualKeyKind::Done => self.done(ctx),
            VirtualKeyKind::Character | VirtualKeyKind::Space => {
                let t = self.typed(&key.text);
                self.set_text(t, ctx);
                Flow::Stay
            }
            _ => {
                let t = vk::apply(&self.text, &key);
                self.set_text(t, ctx);
                Flow::Stay
            }
        }
    }

    /// Coluna inicial de cada tecla da linha (as largas ocupam várias).
    fn starts(row: &[VirtualKey]) -> Vec<usize> {
        let mut x = 0;
        row.iter()
            .map(|k| {
                let s = x;
                x += k.width as usize;
                s
            })
            .collect()
    }

    /// Ao mudar de linha, fica na tecla que cobre a mesma coluna.
    fn move_row(&mut self, to: usize) {
        let x = Self::starts(&self.rows[self.row])[self.col];
        let starts = Self::starts(&self.rows[to]);
        self.col = starts.iter().rposition(|s| *s <= x).unwrap_or(0);
        self.row = to;
    }
}

impl Layer for KeyboardLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        match cmd {
            Cmd::Up if self.row > 0 => self.move_row(self.row - 1),
            Cmd::Down if self.row + 1 < self.rows.len() => self.move_row(self.row + 1),
            Cmd::Left if self.col > 0 => self.col -= 1,
            Cmd::Right if self.col + 1 < self.rows[self.row].len() => self.col += 1,
            Cmd::Accept => return self.press(ctx),
            Cmd::Favorite => {
                let t = vk::backspace(&self.text);
                self.set_text(t, ctx);
            }
            Cmd::Search => {
                let t = self.typed(" ");
                self.set_text(t, ctx);
            }
            Cmd::Menu => return self.done(ctx),
            Cmd::Back => {
                if let Some(ref cancel) = self.on_cancel {
                    cancel(ctx);
                }
                return Flow::Close;
            }
            _ => {}
        }
        Flow::Stay
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        let mut keys = Vec::new();
        let mut focus = 0;
        for (r, row) in self.rows.iter().enumerate() {
            for (c, (k, start)) in row.iter().zip(Self::starts(row)).enumerate() {
                if r == self.row && c == self.col {
                    focus = keys.len() as i32;
                }
                keys.push(Key {
                    label: k.label.clone(),
                    row: r as i32,
                    col: start as i32,
                    span: k.width as i32,
                });
            }
        }
        let problem = if self.edited || !self.text.is_empty() {
            self.check.as_ref().and_then(|c| c(ctx, &self.text))
        } else {
            None
        };
        let valid = self.check.as_ref().map_or(true, |c| c(ctx, &self.text).is_none());
        Overlay {
            kind: "keyboard".into(),
            title: self.title.clone(),
            text: problem.unwrap_or_else(|| self.hint.clone()),
            keys,
            key_cols: vk::COLUMNS as i32,
            sel: focus,
            input: self.text.clone(),
            input_hint: ctx.t("search.placeholder"),
            secret: self.secret,
            wide: valid,
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let valid = self.check.as_ref().map_or(true, |c| c(ctx, &self.text).is_none());
        let mut hints = vec![
            ctx.hint(Cmd::Accept, &ctx.t("keyboard.type")),
            ctx.hint(Cmd::Favorite, &ctx.t("keyboard.delete")),
            ctx.hint(Cmd::Search, &ctx.t("keyboard.space")),
        ];
        if valid {
            hints.push(ctx.hint(Cmd::Menu, &ctx.t("keyboard.done")));
        }
        hints.push(ctx.hint(Cmd::Back, &ctx.t("hint.close")));
        hints
    }

    fn typing(&self) -> bool {
        true
    }

    fn text(&mut self, text: &str, ctx: &mut Ctx) {
        let t = self.typed(text);
        self.set_text(t, ctx);
    }

    fn erase(&mut self, ctx: &mut Ctx) {
        let t = vk::backspace(&self.text);
        self.set_text(t, ctx);
    }

    fn replace_text(&mut self, text: &str, ctx: &mut Ctx) {
        // Campo do modo desktop: o texto inteiro, com as mesmas regras de quem digita tecla a tecla.
        let t = if self.free_text {
            text.chars().filter(|c| !c.is_control()).take(512).collect()
        } else {
            vk::type_text("", text)
        };
        self.set_text(t, ctx);
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        match zone {
            "done" => return self.done(ctx),
            "cancel" => {
                if let Some(ref cancel) = self.on_cancel {
                    cancel(ctx);
                }
                return Flow::Close;
            }
            _ => {}
        }
        if zone == "key" {
            self.hover(zone, index);
            return self.press(ctx);
        }
        Flow::Stay
    }

    fn hover(&mut self, zone: &str, index: usize) {
        if zone != "key" {
            return;
        }
        let mut n = 0;
        for (r, row) in self.rows.iter().enumerate() {
            if index < n + row.len() {
                self.row = r;
                self.col = index - n;
                return;
            }
            n += row.len();
        }
    }
}
