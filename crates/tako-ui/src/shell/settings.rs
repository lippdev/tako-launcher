//! Configurações (ADR-0020): coluna de categorias à esquerda e as linhas da categoria à direita. LB/RB trocam
//! de categoria; → ou A entram nas linhas; B nas linhas volta à categoria e B na coluna sai.

use super::layer::{Flow, MenuItem};
use super::{Cmd, Ctx, Nav, Out, Page};
use crate::model::{Hint, NavItem, PanelRow};

/// Uma categoria: id, ícone e as linhas (reconstruídas a cada desenho).
pub struct Category {
    pub id: &'static str,
    pub icon: &'static str,
    pub rows: fn(&Ctx) -> Vec<MenuItem>,
}

pub fn categories() -> Vec<Category> {
    vec![Category {
        id: "about",
        icon: "info",
        rows: about,
    }]
}

fn about(ctx: &Ctx) -> Vec<MenuItem> {
    vec![
        MenuItem::new(PanelRow::info(
            "version",
            ctx.t("settings.about.version"),
            ctx.version.clone(),
        )),
        MenuItem::new(
            PanelRow::action("source", ctx.t("settings.about.source"))
                .value("github.com/lippdev/tako-launcher"),
        )
        .on(|ctx| {
            ctx.effect(super::Effect::OpenUrl(
                "https://github.com/lippdev/tako-launcher".into(),
            ));
            Flow::Stay
        }),
    ]
}

pub struct SettingsPage {
    cats: Vec<Category>,
    cat: usize,
    /// `None` = coluna de categorias.
    row: Option<usize>,
    /// Camada que uma linha abriu (o Shell a recebe pelo `pending_layer`).
    _priv: (),
}

impl Default for SettingsPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPage {
    pub fn new() -> SettingsPage {
        SettingsPage {
            cats: categories(),
            cat: 0,
            row: None,
            _priv: (),
        }
    }

    pub fn at(category: &str) -> SettingsPage {
        let mut p = Self::new();
        p.cat = p.cats.iter().position(|c| c.id == category).unwrap_or(0);
        p
    }

    fn items(&self, ctx: &Ctx) -> Vec<MenuItem> {
        (self.cats[self.cat].rows)(ctx)
    }

    fn step(items: &[MenuItem], from: usize, d: i32) -> usize {
        let mut i = from as i32;
        loop {
            i += d;
            if i < 0 || i >= items.len() as i32 {
                return from;
            }
            if items[i as usize].row.is_focusable() {
                return i as usize;
            }
        }
    }

    fn first(items: &[MenuItem]) -> Option<usize> {
        items.iter().position(|i| i.row.is_focusable())
    }

    fn run(flow: Flow, ctx: &mut Ctx) -> Nav {
        match flow {
            Flow::Push(l) | Flow::Replace(l) => Nav::Layer(l),
            _ => {
                ctx.dirty = true;
                Nav::Stay
            }
        }
    }
}

impl Page for SettingsPage {
    fn route(&self) -> String {
        "settings".into()
    }

    fn kind(&self) -> &'static str {
        "settings"
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        let items = self.items(ctx);
        let n = self.cats.len();
        match (self.row, cmd) {
            (_, Cmd::PrevSection) => {
                self.cat = (self.cat + n - 1) % n;
                self.row = None;
            }
            (_, Cmd::NextSection) => {
                self.cat = (self.cat + 1) % n;
                self.row = None;
            }
            (None, Cmd::Up) => self.cat = self.cat.saturating_sub(1),
            (None, Cmd::Down) => self.cat = (self.cat + 1).min(n - 1),
            (None, Cmd::Right) | (None, Cmd::Accept) => self.row = Self::first(&items),
            (None, Cmd::Left) | (None, Cmd::Back) => return Nav::Sidebar,
            (Some(r), Cmd::Up) => self.row = Some(Self::step(&items, r, -1)),
            (Some(r), Cmd::Down) => self.row = Some(Self::step(&items, r, 1)),
            (Some(_), Cmd::Back) => self.row = None,
            (Some(r), Cmd::Left) | (Some(r), Cmd::Right) => {
                if let Some(adj) = items.get(r).and_then(|i| i.adjust.clone()) {
                    return Self::run(adj(ctx, if cmd == Cmd::Left { -1 } else { 1 }), ctx);
                }
                if cmd == Cmd::Left {
                    self.row = None;
                }
            }
            (Some(r), Cmd::Accept) => {
                let Some(item) = items.get(r) else {
                    return Nav::Stay;
                };
                if !item.row.enabled {
                    if !item.row.description.is_empty() {
                        ctx.toast(item.row.description.clone());
                    }
                    return Nav::Stay;
                }
                if let Some(f) = item.accept.clone() {
                    return Self::run(f(ctx), ctx);
                }
                if let Some(adj) = item.adjust.clone() {
                    return Self::run(adj(ctx, 1), ctx);
                }
            }
            _ => {}
        }
        Nav::Stay
    }

    fn render(&mut self, ctx: &Ctx, out: &mut Out) {
        let items = self.items(ctx);
        if let Some(r) = self.row
            && r >= items.len()
        {
            self.row = Self::first(&items);
        }
        out.set("settings-title", ctx.t("nav.settings"));
        let cats: Vec<NavItem> = self
            .cats
            .iter()
            .enumerate()
            .map(|(i, c)| NavItem {
                id: c.id.into(),
                label: ctx.t(&format!("settings.{}.title", c.id)),
                icon: c.icon.into(),
                selected: i == self.cat,
                ..Default::default()
            })
            .collect();
        out.set("settings-categories", cats);
        out.set("settings-category-focus", self.cat as i32);
        let id = self.cats[self.cat].id;
        out.set("settings-heading", ctx.t(&format!("settings.{id}.title")));
        out.set(
            "settings-subtitle",
            ctx.t(&format!("settings.{id}.subtitle")),
        );
        let rows: Vec<PanelRow> = items.into_iter().map(|i| i.row).collect();
        out.set("settings-rows", rows);
        out.set(
            "settings-row-focus",
            self.row.map(|r| r as i32).unwrap_or(-1),
        );
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![
            ctx.hint(Cmd::Accept, &ctx.t("hint.select")),
            ctx.hint(
                Cmd::Back,
                &ctx.t(if self.row.is_some() {
                    "hint.categories"
                } else {
                    "hint.back"
                }),
            ),
            ctx.hint(Cmd::PrevSection, &ctx.t("hint.category")),
        ]
    }

    fn hover(&mut self, zone: &str, index: usize, _ctx: &Ctx) {
        match zone {
            "category" => {
                self.cat = index.min(self.cats.len() - 1);
                self.row = None;
            }
            "row" => self.row = Some(index),
            _ => {}
        }
    }
}
