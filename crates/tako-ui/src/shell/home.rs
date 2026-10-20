//! Início (DESIGN.md › Layout): destaque com o jogo em foco e fileiras: jogados recentemente, uma por
//! coleção com jogos e adicionados recentemente. Vazia, explica como começar.

use super::{Cmd, Ctx, Effect, Nav, Out, Page, details, dialogs, library, tiles};
use crate::model::{Hero, Hint, HomeRow, PanelRow};
use std::collections::HashMap;
use tako_core::library::{Game, LibraryCriteria, LibrarySort, query};
use tako_core::presentation::game_text;

const ROW_LIMIT: usize = 20;
const ADDED_LIMIT: usize = 10;

#[derive(Clone, PartialEq)]
enum RowKind {
    Recent,
    Collection(String),
    Added,
}

struct Row {
    kind: RowKind,
    title: String,
    ids: Vec<String>,
}

pub struct HomePage {
    /// `None` = destaque; `Some(i)` = fileira i.
    row: Option<usize>,
    /// Coluna lembrada por fileira.
    cols: HashMap<usize, usize>,
    hero_button: usize,
    /// Jogo do destaque quando o foco não está numa capa.
    hero_id: Option<String>,
}

impl Default for HomePage {
    fn default() -> Self {
        Self::new()
    }
}

impl HomePage {
    pub fn new() -> HomePage {
        HomePage {
            row: None,
            cols: HashMap::new(),
            hero_button: 0,
            hero_id: None,
        }
    }

    fn rows(&self, ctx: &Ctx) -> Vec<Row> {
        let games = ctx.library.games();
        let mut rows = Vec::new();
        let recent: Vec<String> = query::recently_played(games)
            .into_iter()
            .take(ROW_LIMIT)
            .map(|g| g.id.clone())
            .collect();
        if !recent.is_empty() {
            rows.push(Row {
                kind: RowKind::Recent,
                title: ctx.t("home.recent"),
                ids: recent,
            });
        }
        for c in ctx.collections.collections() {
            let mut ids: Vec<&Game> = games.iter().filter(|g| c.contains(&g.id)).collect();
            if ids.is_empty() {
                continue;
            }
            ids.sort_by(|a, b| {
                b.last_played_at
                    .cmp(&a.last_played_at)
                    .then_with(|| query::title_cmp(&a.title, &b.title))
            });
            rows.push(Row {
                kind: RowKind::Collection(c.id.clone()),
                title: c.name.clone(),
                ids: ids
                    .into_iter()
                    .take(ROW_LIMIT)
                    .map(|g| g.id.clone())
                    .collect(),
            });
        }
        let added: Vec<String> = query::recently_added(games, ADDED_LIMIT)
            .into_iter()
            .map(|g| g.id.clone())
            .collect();
        if !added.is_empty() {
            rows.push(Row {
                kind: RowKind::Added,
                title: ctx.t("home.added"),
                ids: added,
            });
        }
        rows
    }

    fn col(&self, row: usize) -> usize {
        self.cols.get(&row).copied().unwrap_or(0)
    }

    /// Jogo do destaque: o focado numa fileira; senão o último jogado ("Continuar jogando") ou o mais novo.
    fn hero_game<'a>(&self, ctx: &'a Ctx, rows: &[Row]) -> Option<&'a Game> {
        if let Some(r) = self.row
            && let Some(row) = rows.get(r)
            && let Some(id) = row.ids.get(self.col(r))
        {
            return ctx.library.find(id);
        }
        if let Some(id) = &self.hero_id
            && let Some(g) = ctx.library.find(id)
        {
            return Some(g);
        }
        rows.first()
            .and_then(|r| r.ids.first())
            .and_then(|id| ctx.library.find(id))
    }

    fn see_all(&self, kind: &RowKind) -> Nav {
        let mut c = LibraryCriteria::default();
        match kind {
            RowKind::Recent => {
                c.sort = LibrarySort::RecentlyPlayed;
                c.played_only = true;
            }
            RowKind::Added => c.sort = LibrarySort::RecentlyAdded,
            RowKind::Collection(id) => {
                c.collections.insert(id.clone());
            }
        }
        Nav::Replace(Box::new(library::LibraryPage::with_criteria(
            "library:all",
            c,
        )))
    }
}

impl Page for HomePage {
    fn route(&self) -> String {
        "home".into()
    }

    fn kind(&self) -> &'static str {
        "home"
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        let rows = self.rows(ctx);
        if ctx.library.games().is_empty() {
            return match cmd {
                Cmd::Left | Cmd::Back => Nav::Sidebar,
                Cmd::Right => {
                    self.hero_button = 1;
                    Nav::Stay
                }
                Cmd::Accept => {
                    ctx.effect(if self.hero_button == 0 {
                        Effect::AddGame
                    } else {
                        Effect::Rescan
                    });
                    Nav::Stay
                }
                _ => {
                    if cmd == Cmd::Up || cmd == Cmd::Down {
                        self.hero_button = 0;
                    }
                    Nav::Stay
                }
            };
        }
        match (self.row, cmd) {
            (_, Cmd::Search) => {
                return Nav::Replace(Box::new(library::LibraryPage::searching(
                    "library:all",
                    ctx,
                )));
            }
            (None, Cmd::Left) => {
                if self.hero_button == 0 {
                    return Nav::Sidebar;
                }
                self.hero_button = 0;
            }
            (None, Cmd::Right) => self.hero_button = 1,
            (None, Cmd::Down) => {
                if !rows.is_empty() {
                    self.row = Some(0);
                }
            }
            (None, Cmd::Accept) => {
                let Some(g) = self.hero_game(ctx, &rows) else {
                    return Nav::Stay;
                };
                if self.hero_button == 1 {
                    return Nav::Push(Box::new(details::DetailsPage::new(&g.id)));
                }
                return details::primary_action(ctx, &g.id.clone());
            }
            (None, Cmd::Menu) | (Some(_), Cmd::Menu) => {
                if let Some(g) = self.hero_game(ctx, &rows) {
                    return Nav::Layer(Box::new(dialogs::quick_actions(ctx, &g.id)));
                }
            }
            (None, Cmd::Favorite) => {
                if let Some(id) = self.hero_game(ctx, &rows).map(|g| g.id.clone()) {
                    dialogs::toggle_favorite(ctx, &id);
                }
            }
            (None, Cmd::Back) => return Nav::Sidebar,
            (Some(r), Cmd::Up) => {
                self.hero_id = self.hero_game(ctx, &rows).map(|g| g.id.clone());
                self.row = if r == 0 { None } else { Some(r - 1) };
                if self.row.is_none() {
                    self.hero_button = 0;
                }
            }
            (Some(r), Cmd::Down) => {
                if r + 1 < rows.len() {
                    self.row = Some(r + 1);
                }
            }
            (Some(r), Cmd::Left) => {
                let c = self.col(r);
                if c == 0 {
                    return Nav::Sidebar;
                }
                self.cols.insert(r, c - 1);
            }
            (Some(r), Cmd::Right) => {
                let c = self.col(r);
                // Uma posição a mais: o "Ver todos" no fim da fileira.
                if let Some(row) = rows.get(r)
                    && c < row.ids.len()
                {
                    self.cols.insert(r, c + 1);
                }
            }
            (Some(r), Cmd::Accept) => {
                let Some(row) = rows.get(r) else {
                    return Nav::Stay;
                };
                let c = self.col(r);
                return match row.ids.get(c) {
                    Some(id) => Nav::Push(Box::new(details::DetailsPage::new(id))),
                    None => self.see_all(&row.kind),
                };
            }
            (Some(r), Cmd::Favorite) => {
                if let Some(id) = rows
                    .get(r)
                    .and_then(|row| row.ids.get(self.col(r)))
                    .cloned()
                {
                    dialogs::toggle_favorite(ctx, &id);
                }
            }
            (Some(_), Cmd::Back) => {
                self.row = None;
                self.hero_button = 0;
            }
            _ => {}
        }
        Nav::Stay
    }

    fn render(&mut self, ctx: &Ctx, out: &mut Out) {
        let rows = self.rows(ctx);
        let empty = ctx.library.games().is_empty();
        out.set("home-empty", empty);
        if empty {
            out.set("home-empty-title", ctx.t("home.empty_title"));
            out.set("home-empty-text", ctx.t("home.empty_text"));
            out.set(
                "home-empty-actions",
                vec![
                    PanelRow::button("add", ctx.t("home.add_game")).icon("more"),
                    PanelRow::button("rescan", ctx.t("home.rescan")),
                ],
            );
            out.set("hero-focus", self.hero_button as i32);
            out.set("home-rows", Vec::<HomeRow>::new());
            out.set("home-row-focus", -1);
            return;
        }
        if let Some(r) = self.row
            && r >= rows.len()
        {
            self.row = rows.len().checked_sub(1);
        }
        let hero = self.hero_game(ctx, &rows).map(|g| {
            let lang = ctx.lang();
            let last = rows
                .first()
                .filter(|r| r.kind == RowKind::Recent)
                .and_then(|r| r.ids.first());
            let overline = if ctx.running.as_ref().is_some_and(|r| r.id == g.id) {
                ctx.t("home.playing_now")
            } else if last == Some(&g.id) && self.row.is_none() {
                ctx.t("home.continue")
            } else if g.is_installed {
                ctx.t("home.installed")
            } else {
                String::new()
            };
            let mut meta = vec![game_text::store_name(lang, g.store())];
            if g.last_played_at.is_some() {
                meta.push(game_text::last_played(
                    lang,
                    g.last_played_at,
                    chrono::Utc::now(),
                ));
            }
            if !g.play_time.is_zero() {
                meta.push(game_text::play_time(lang, g.play_time));
            }
            Hero {
                has_game: true,
                title: g.title.clone(),
                overline: overline.to_uppercase(),
                meta: meta.join(" · "),
                description: g.description.clone(),
                art: tiles::image(g.shown_hero(), tiles::hero(ctx)),
                logo: tiles::image(g.shown_logo(), tiles::LOGO),
                play_label: tiles::play_label(ctx, g),
                running: ctx.running.as_ref().is_some_and(|r| r.id == g.id),
            }
        });
        out.set("hero", hero.unwrap_or_default());
        out.set("hero-focus", self.hero_button as i32);
        let home_rows: Vec<HomeRow> = rows
            .iter()
            .map(|r| HomeRow {
                title: r.title.clone(),
                count: if r.kind == RowKind::Recent {
                    String::new()
                } else {
                    r.ids.len().to_string()
                },
                tiles: r
                    .ids
                    .iter()
                    .filter_map(|id| ctx.library.find(id))
                    .map(|g| tiles::tile(ctx, g, true))
                    .collect(),
                see_all: true,
            })
            .collect();
        out.set("home-rows", home_rows);
        out.set("home-row-focus", self.row.map(|r| r as i32).unwrap_or(-1));
        out.set(
            "home-col-focus",
            self.row.map(|r| self.col(r) as i32).unwrap_or(0),
        );
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let on_game = !ctx.library.games().is_empty();
        let mut h = vec![ctx.hint(
            Cmd::Accept,
            &ctx.t(if self.row.is_some() {
                "hint.details"
            } else {
                "hint.select"
            }),
        )];
        if on_game {
            h.push(ctx.hint(Cmd::Favorite, &ctx.t("hint.favorite")));
            h.push(ctx.hint(Cmd::Menu, &ctx.t("hint.actions")));
        }
        h.push(ctx.hint(Cmd::Search, &ctx.t("hint.search")));
        h.push(ctx.hint(Cmd::PrevSection, &ctx.t("hint.section")));
        h
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Nav {
        self.hover(zone, index, ctx);
        if zone == "empty" {
            self.hero_button = index;
        }
        self.handle(Cmd::Accept, ctx)
    }

    fn hover(&mut self, zone: &str, index: usize, ctx: &Ctx) {
        if zone == "hero" || zone == "hero-keep" {
            // `hero-keep` (Fita): o destaque fica com o jogo apresentado, como o ↑ do teclado.
            if zone == "hero-keep" {
                self.hero_id = self.hero_game(ctx, &self.rows(ctx)).map(|g| g.id.clone());
            }
            self.row = None;
            self.hero_button = index;
        } else if let Some(r) = zone
            .strip_prefix("tab-")
            .and_then(|r| r.parse::<usize>().ok())
        {
            self.row = Some(r);
        } else if let Some(r) = zone
            .strip_prefix("row-")
            .and_then(|r| r.parse::<usize>().ok())
        {
            self.row = Some(r);
            self.cols.insert(r, index);
        }
    }

    fn focused_game(&self, ctx: &Ctx) -> Option<String> {
        let rows = self.rows(ctx);
        self.hero_game(ctx, &rows).map(|g| g.id.clone())
    }

    fn resumed(&mut self, _ctx: &mut Ctx) {}

    fn focus_game(&mut self, id: &str, _ctx: &Ctx) {
        // Depois do jogo: o jogo volta ao destaque e a fileira volta ao início (DESIGN.md › Início).
        self.row = None;
        self.cols.clear();
        self.hero_button = 0;
        self.hero_id = Some(id.to_owned());
    }
}
