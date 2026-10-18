//! Editor de etiquetas do desktop (EtiquetasDoJogo, LIB-002): as etiquetas do jogo como chips com ×, um campo
//! para digitar e, embaixo, as sugestões (as que já existem, com quantos jogos têm) e `Criar "…"`.
//! Enter escolhe, Esc fecha e Backspace no campo vazio tira a última. A tela cheia segue com a lista de marcar.

use super::layer::{Flow, Layer};
use super::{Cmd, Ctx};
use crate::model::{Hint, Overlay, PanelRow};
use tako_core::i18n::tn;
use tako_core::library::collections::clean_name;
use tako_core::library::query::{FacetValue, title_cmp, title_eq};
use tako_core::library::search_text::normalize;
use tako_core::library::{Game, tags};

/// Estimativas do chip e da linha do painel (o Slint não quebra linha sozinho).
const CHIP_PAD: usize = 44;
const CHAR_W: usize = 9;
const LINE_W: usize = 380;
const CHIP_GAP: usize = 8;

pub struct TagsLayer {
    ids: Vec<String>,
    overline: String,
    text: String,
    sel: usize,
}

/// O que a lista de baixo mostra: etiquetas existentes que combinam e, no fim, a que seria criada.
#[derive(Debug, PartialEq)]
pub struct Suggestions {
    pub existing: Vec<FacetValue>,
    pub create: Option<String>,
}

/// Etiquetas que todos os jogos têm (os chips).
pub fn applied(games: &[&Game]) -> Vec<String> {
    let Some(first) = games.first() else {
        return Vec::new();
    };
    let mut v: Vec<String> = first
        .tags
        .iter()
        .filter(|t| games.iter().all(|g| g.tags.contains(*t)))
        .cloned()
        .collect();
    v.sort_by(|a, b| title_cmp(a, b));
    v
}

pub fn suggest(all: &[FacetValue], applied: &[String], typed: &str) -> Suggestions {
    let q = normalize(typed);
    let existing = all
        .iter()
        .filter(|f| !applied.iter().any(|a| title_eq(a, &f.name)))
        .filter(|f| normalize(&f.name).contains(&q))
        .cloned()
        .collect();
    let clean = clean_name(typed);
    let create = (tags::name_problem(typed).is_none()
        && !all.iter().any(|f| title_eq(&f.name, &clean)))
    .then_some(clean);
    Suggestions { existing, create }
}

/// Chips em linhas pelo tamanho estimado: a linha de cada um e quantas linhas deu.
pub fn wrap(chips: &[String]) -> (Vec<usize>, usize) {
    let (mut line, mut used) = (0, 0);
    let lines = chips
        .iter()
        .map(|c| {
            let w = (c.chars().count() * CHAR_W + CHIP_PAD).min(LINE_W);
            if used > 0 && used + CHIP_GAP + w > LINE_W {
                line += 1;
                used = 0;
            }
            used += w + CHIP_GAP;
            line
        })
        .collect();
    (lines, if chips.is_empty() { 0 } else { line + 1 })
}

impl TagsLayer {
    pub fn new(ids: Vec<String>, overline: String) -> Self {
        TagsLayer {
            ids,
            overline,
            text: String::new(),
            sel: 0,
        }
    }

    fn state(&self, ctx: &Ctx) -> (Vec<String>, Suggestions) {
        let games: Vec<&Game> = self
            .ids
            .iter()
            .filter_map(|id| ctx.library.find(id))
            .collect();
        let chips = applied(&games);
        let s = suggest(&tags::all(ctx.library.games()), &chips, &self.text);
        (chips, s)
    }

    fn set(&self, ctx: &mut Ctx, name: &str, on: bool) {
        if let Err(e) = ctx.library.set_tag(&self.ids, name, on) {
            ctx.toast(e.to_string());
        }
        ctx.dirty = true;
    }

    /// Enter / clique numa sugestão (a última linha pode ser o `Criar`): aplica e limpa o campo.
    fn pick(&mut self, index: usize, ctx: &mut Ctx) {
        let (_, s) = self.state(ctx);
        let name = match s.existing.get(index) {
            Some(f) => f.name.clone(),
            None => match s.create {
                Some(c) if index == s.existing.len() => c,
                _ => return,
            },
        };
        self.set(ctx, &name, true);
        self.text.clear();
        self.sel = 0;
    }
}

impl Layer for TagsLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let (_, s) = self.state(ctx);
        let n = s.existing.len() + usize::from(s.create.is_some());
        match cmd {
            Cmd::Up => self.sel = self.sel.saturating_sub(1),
            Cmd::Down => self.sel = (self.sel + 1).min(n.saturating_sub(1)),
            Cmd::Accept => self.pick(self.sel, ctx),
            Cmd::Back | Cmd::Menu => return Flow::Close,
            _ => {}
        }
        Flow::Stay
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        let (chips, s) = self.state(ctx);
        let mut rows: Vec<PanelRow> = s
            .existing
            .iter()
            .map(|f| {
                PanelRow::action(f.name.clone(), f.name.clone()).value(tn(
                    "common.games",
                    f.count as i64,
                    &[],
                ))
            })
            .collect();
        if let Some(c) = &s.create {
            rows.push(
                PanelRow::action("create", ctx.tf("tags.create", &[("name", c)])).icon("plus"),
            );
        }
        let (lines, count) = wrap(&chips);
        Overlay {
            kind: "tags".into(),
            title: ctx.t("tags.title"),
            overline: self.overline.to_uppercase(),
            text: ctx.t("tags.text_desktop"),
            sel: self.sel.min(rows.len().saturating_sub(1)) as i32,
            rows,
            tabs: chips,
            tabs2: lines.iter().map(usize::to_string).collect(),
            thumb_cols: count as i32,
            input: self.text.clone(),
            input_hint: ctx.t("tags.placeholder"),
            preview_text: ctx.t("tags.remove"),
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![ctx.hint(Cmd::Back, &ctx.t("hint.close"))]
    }

    fn typing(&self) -> bool {
        true
    }

    fn replace_text(&mut self, text: &str, _ctx: &mut Ctx) {
        self.text = text.chars().filter(|c| !c.is_control()).take(40).collect();
        self.sel = 0;
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        match zone {
            "chip" => {
                if let Some(t) = self.state(ctx).0.get(index).cloned() {
                    self.set(ctx, &t, false);
                }
            }
            "row" => self.pick(index, ctx),
            "done" => self.pick(self.sel, ctx),
            "move" => return self.handle(if index == 0 { Cmd::Up } else { Cmd::Down }, ctx),
            "backspace" => {
                if let Some(last) = self.state(ctx).0.pop() {
                    self.set(ctx, &last, false);
                }
            }
            "outside" | "cancel" => return Flow::Close,
            _ => {}
        }
        Flow::Stay
    }

    fn hover(&mut self, zone: &str, index: usize) {
        if zone == "row" {
            self.sel = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fv(name: &str, count: usize) -> FacetValue {
        FacetValue {
            name: name.into(),
            count,
        }
    }

    #[test]
    fn suggestions_filter_ignoring_case_and_accents_and_offer_to_create() {
        let all = [fv("Co-op", 3), fv("Sofá", 2), fv("Zerado", 5)];
        let s = suggest(&all, &["Zerado".into()], "SOFA");
        assert_eq!(s.existing, [fv("Sofá", 2)]);
        // Já existe (mesmo sem acento): não oferece criar.
        assert_eq!(s.create, None);
        let s = suggest(&all, &[], "Com mods");
        assert!(s.existing.is_empty());
        assert_eq!(s.create.as_deref(), Some("Com mods"));
        // Vazio lista o que falta e não cria nada; a já aplicada sai da lista.
        let s = suggest(&all, &["Co-op".into()], "");
        assert_eq!(s.existing, [fv("Sofá", 2), fv("Zerado", 5)]);
        assert_eq!(s.create, None);
    }

    #[test]
    fn chips_wrap_into_lines() {
        let chips: Vec<String> = (0..6).map(|_| "Para zerar".to_owned()).collect();
        let (lines, count) = wrap(&chips);
        assert!(count > 1 && lines[0] == 0 && lines.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(wrap(&[]), (vec![], 0));
    }

    #[test]
    fn typing_adds_creates_and_removes_tags() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        ctx.library.refresh();
        let id = ctx.library.games()[0].id.clone();
        let mut layer = TagsLayer::new(vec![id.clone()], "x".into());
        let tags_of = |ctx: &Ctx| ctx.library.find(&id).unwrap().tags.clone();

        // Cria pelo texto digitado e Enter.
        layer.replace_text("Com mods", &mut ctx);
        assert_eq!(layer.render(&ctx).rows.last().unwrap().key, "create");
        layer.handle(Cmd::Accept, &mut ctx);
        assert!(tags_of(&ctx).contains("Com mods"));
        // O campo limpou e o chip apareceu.
        let ov = layer.render(&ctx);
        assert_eq!(ov.input, "");
        assert_eq!(ov.tabs, ["Com mods"]);

        // × tira, e a etiqueta sem jogo nenhum some da lista.
        layer.click("chip", 0, &mut ctx);
        assert!(tags_of(&ctx).is_empty());
        assert!(tags::all(ctx.library.games()).is_empty());

        // Backspace no campo vazio tira a última.
        for t in ["a", "b"] {
            layer.replace_text(t, &mut ctx);
            layer.handle(Cmd::Accept, &mut ctx);
        }
        assert_eq!(tags_of(&ctx).len(), 2);
        layer.click("backspace", 0, &mut ctx);
        assert_eq!(tags_of(&ctx).len(), 1);
    }
}
