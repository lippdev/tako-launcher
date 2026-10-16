//! Biblioteca em grade (o `LibraryViewModel` do WPF): rota da barra lateral, busca, filtros e ordenação.
//! Os critérios ficam só na memória, como nas alphas.

use super::keyboard::KeyboardLayer;
use super::{Cmd, Ctx, Nav, Out, Page, details, dialogs, tiles};
use crate::model::{Hint, PanelRow};
use std::cell::RefCell;
use std::rc::Rc;
use tako_core::library::{GameStore, LibraryCriteria, LibraryFilter, query};
use tako_core::presentation::{game_text, library_text};

pub type Criteria = Rc<RefCell<LibraryCriteria>>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Chip {
    Search,
    Filters,
    Sort,
    Clear,
}

pub struct LibraryPage {
    route: String,
    filter: LibraryFilter,
    pub criteria: Criteria,
    /// `None` = cabeçalho (chips).
    focus: Option<usize>,
    chip: usize,
}

pub fn filter_of(route: &str) -> LibraryFilter {
    match route {
        "library:installed" => LibraryFilter::Installed,
        "library:favorites" => LibraryFilter::Favorites,
        r => r
            .strip_prefix("library:store:")
            .and_then(GameStore::from_key)
            .map(LibraryFilter::Store)
            .unwrap_or(LibraryFilter::All),
    }
}

impl LibraryPage {
    pub fn for_route(route: &str) -> LibraryPage {
        Self::with_criteria(route, LibraryCriteria::default())
    }

    pub fn with_criteria(route: &str, criteria: LibraryCriteria) -> LibraryPage {
        LibraryPage {
            route: route.into(),
            filter: filter_of(route),
            criteria: Rc::new(RefCell::new(criteria)),
            focus: Some(0),
            chip: 0,
        }
    }

    /// Biblioteca já com o teclado da busca aberto (Y na Início).
    pub fn searching(route: &str, ctx: &mut Ctx) -> LibraryPage {
        let p = Self::for_route(route);
        ctx.pending_layer = Some(p.search_layer(ctx));
        p
    }

    fn chips(&self) -> Vec<Chip> {
        let c = self.criteria.borrow();
        let mut v = vec![Chip::Search, Chip::Filters, Chip::Sort];
        if c.has_search() || c.has_filters() {
            v.push(Chip::Clear);
        }
        v
    }

    fn ids(&self, ctx: &Ctx) -> Vec<String> {
        query::apply(ctx.library.games(), self.filter, &self.criteria.borrow())
            .into_iter()
            .map(|g| g.id.clone())
            .collect()
    }

    fn title(&self, ctx: &Ctx) -> String {
        match self.filter {
            LibraryFilter::All => ctx.t("nav.all"),
            LibraryFilter::Installed => ctx.t("nav.installed"),
            LibraryFilter::Favorites => ctx.t("nav.favorites"),
            LibraryFilter::Store(GameStore::Manual) => ctx.t("nav.manual"),
            LibraryFilter::Store(s) => game_text::store_name(ctx.lang(), s),
        }
    }

    fn search_layer(&self, ctx: &Ctx) -> Box<dyn super::Layer> {
        let criteria = self.criteria.clone();
        let text = criteria.borrow().search.clone();
        Box::new(KeyboardLayer::search(ctx, &text, move |ctx, t| {
            criteria.borrow_mut().search = t.to_owned();
            ctx.dirty = true;
        }))
    }

    fn activate_chip(&mut self, ctx: &mut Ctx) -> Nav {
        let chips = self.chips();
        match chips.get(self.chip) {
            Some(Chip::Search) => Nav::Layer(self.search_layer(ctx)),
            Some(Chip::Filters) => Nav::Layer(Box::new(dialogs::library_filters(
                ctx,
                self.criteria.clone(),
                self.filter,
            ))),
            Some(Chip::Sort) => {
                let mut c = self.criteria.borrow_mut();
                c.sort = library_text::next_sort(c.sort);
                Nav::Stay
            }
            Some(Chip::Clear) => {
                let mut c = self.criteria.borrow_mut();
                let sort = c.sort;
                *c = LibraryCriteria {
                    sort,
                    ..Default::default()
                };
                drop(c);
                self.chip = 0;
                Nav::Stay
            }
            None => Nav::Stay,
        }
    }
}

impl Page for LibraryPage {
    fn route(&self) -> String {
        self.route.clone()
    }

    fn kind(&self) -> &'static str {
        "library"
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        let ids = self.ids(ctx);
        let cols = ctx.grid_columns.max(1);
        if let Some(f) = self.focus
            && f >= ids.len()
        {
            self.focus = if ids.is_empty() {
                None
            } else {
                Some(ids.len() - 1)
            };
        }
        match (self.focus, cmd) {
            (_, Cmd::Search) => return Nav::Layer(self.search_layer(ctx)),
            (None, Cmd::Left) => {
                if self.chip == 0 {
                    return Nav::Sidebar;
                }
                self.chip -= 1;
            }
            (None, Cmd::Right) => self.chip = (self.chip + 1).min(self.chips().len() - 1),
            (None, Cmd::Down) => {
                if !ids.is_empty() {
                    self.focus = Some(0);
                }
            }
            (None, Cmd::Accept) => return self.activate_chip(ctx),
            (None, Cmd::Back) => return Nav::Sidebar,
            (Some(i), Cmd::Up) => {
                self.focus = if i < cols { None } else { Some(i - cols) };
            }
            (Some(i), Cmd::Down) => {
                if i + cols < ids.len() {
                    self.focus = Some(i + cols);
                } else if i / cols < (ids.len() - 1) / cols {
                    // Última linha incompleta: vai para a última capa.
                    self.focus = Some(ids.len() - 1);
                }
            }
            (Some(i), Cmd::Left) => {
                if i % cols == 0 {
                    return Nav::Sidebar;
                }
                self.focus = Some(i - 1);
            }
            (Some(i), Cmd::Right) => {
                if i % cols + 1 < cols && i + 1 < ids.len() {
                    self.focus = Some(i + 1);
                }
            }
            (Some(i), Cmd::Accept) => {
                if let Some(id) = ids.get(i) {
                    return Nav::Push(Box::new(details::DetailsPage::new(id)));
                }
            }
            (Some(i), Cmd::Favorite) => {
                if let Some(id) = ids.get(i) {
                    dialogs::toggle_favorite(ctx, id);
                }
            }
            (Some(i), Cmd::Menu) => {
                if let Some(id) = ids.get(i) {
                    return Nav::Layer(Box::new(dialogs::quick_actions(ctx, id)));
                }
            }
            (Some(_), Cmd::Back) => {
                self.focus = None;
                self.chip = 0;
            }
            _ => {}
        }
        Nav::Stay
    }

    fn render(&mut self, ctx: &Ctx, out: &mut Out) {
        let ids = self.ids(ctx);
        let c = self.criteria.borrow();
        let lang = ctx.lang();
        let total = ctx
            .library
            .games()
            .iter()
            .filter(|g| self.filter.matches(g))
            .count();
        out.set("library-title", self.title(ctx));
        out.set(
            "library-count",
            if c.has_search() || c.has_filters() {
                ctx.tf(
                    "library.count_of",
                    &[
                        ("n", &ids.len().to_string()),
                        ("total", &library_text::game_count(lang, total)),
                    ],
                )
            } else {
                library_text::game_count(lang, total)
            },
        );
        let chips: Vec<PanelRow> =
            self.chips()
                .into_iter()
                .map(|chip| match chip {
                    Chip::Search => PanelRow::button(
                        "search",
                        if c.has_search() {
                            c.search.clone()
                        } else {
                            ctx.t("library.search")
                        },
                    )
                    .icon("search"),
                    Chip::Filters => PanelRow::button(
                        "filters",
                        library_text::filters_label(lang, c.active_filter_count()),
                    )
                    .icon("filter"),
                    Chip::Sort => PanelRow::button("sort", library_text::sort_label(lang, c.sort))
                        .icon("sort"),
                    Chip::Clear => PanelRow::button("clear", ctx.t("library.clear")).icon("close"),
                })
                .collect();
        drop(c);
        out.set("library-chips", chips);
        out.set(
            "library-chip-focus",
            if self.focus.is_none() {
                self.chip as i32
            } else {
                -1
            },
        );
        let tiles: Vec<_> = ids
            .iter()
            .filter_map(|id| ctx.library.find(id))
            .map(|g| tiles::tile(ctx, g))
            .collect();
        out.set("library-tiles", tiles);
        out.set("library-focus", self.focus.map(|f| f as i32).unwrap_or(-1));
        let filtered = {
            let c = self.criteria.borrow();
            c.has_search() || c.has_filters()
        };
        out.set(
            "library-empty-title",
            ctx.t(if filtered {
                "library.none_found"
            } else {
                "library.empty"
            }),
        );
        out.set(
            "library-empty-text",
            ctx.t(if filtered {
                "library.none_found_text"
            } else {
                "library.empty_text"
            }),
        );
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let mut h = vec![ctx.hint(
            Cmd::Accept,
            &ctx.t(if self.focus.is_some() {
                "hint.details"
            } else {
                "hint.select"
            }),
        )];
        if self.focus.is_some() {
            h.push(ctx.hint(Cmd::Favorite, &ctx.t("hint.favorite")));
            h.push(ctx.hint(Cmd::Menu, &ctx.t("hint.actions")));
        }
        h.push(ctx.hint(Cmd::Search, &ctx.t("hint.search")));
        h.push(ctx.hint(Cmd::PrevSection, &ctx.t("hint.section")));
        h
    }

    fn hover(&mut self, zone: &str, index: usize, _ctx: &Ctx) {
        match zone {
            "grid" => self.focus = Some(index),
            "chip" => {
                self.focus = None;
                self.chip = index;
            }
            _ => {}
        }
    }

    fn focused_game(&self, ctx: &Ctx) -> Option<String> {
        self.focus.and_then(|f| self.ids(ctx).get(f).cloned())
    }

    fn focus_game(&mut self, id: &str, ctx: &Ctx) {
        if let Some(i) = self.ids(ctx).iter().position(|x| x == id) {
            self.focus = Some(i);
        }
    }
}
