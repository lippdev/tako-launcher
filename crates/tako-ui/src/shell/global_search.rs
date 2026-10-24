//! Busca geral do desktop, independente dos filtros e da aba da biblioteca.

use super::layer::{Flow, Layer};
use super::{Cmd, Ctx};
use crate::model::{Overlay, Tile};
use tako_core::library::{LibraryCriteria, LibraryFilter, query};

pub(super) struct GlobalSearch {
    text: String,
    selected: usize,
}

impl GlobalSearch {
    pub(super) fn new() -> Self {
        Self {
            text: String::new(),
            selected: 0,
        }
    }

    fn results(&self, ctx: &Ctx) -> Vec<Tile> {
        let criteria = LibraryCriteria {
            search: self.text.clone(),
            ..Default::default()
        };
        query::apply(ctx.library.games(), LibraryFilter::All, &criteria)
            .into_iter()
            .map(|game| Tile {
                id: game.id.clone(),
                title: game.title.clone(),
                initial: game
                    .title
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().collect())
                    .unwrap_or_default(),
                meta: [
                    game.publisher
                        .as_deref()
                        .filter(|publisher| !publisher.trim().is_empty())
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            tako_core::presentation::game_text::store_name(ctx.lang(), game.store())
                        }),
                    tako_core::presentation::game_text::status(ctx.lang(), game),
                ]
                .join(" · "),
                cover: super::tiles::image(game.shown_cover(), (80, 120)),
                accent: super::desktop::game_card_accent(ctx, &game.accent),
                installed: game.is_installed,
                favorite: game.is_favorite,
                running: ctx
                    .running
                    .as_ref()
                    .is_some_and(|running| running.id == game.id),
            })
            .collect()
    }

    fn select(&self, ctx: &mut Ctx) -> Flow {
        if let Some(game) = self.results(ctx).get(self.selected) {
            ctx.open_details = Some(game.id.clone());
        }
        Flow::Close
    }
}

impl Layer for GlobalSearch {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        match cmd {
            Cmd::Back => Flow::Close,
            Cmd::Up => {
                self.selected = self.selected.saturating_sub(1);
                Flow::Stay
            }
            Cmd::Down => {
                self.selected = (self.selected + 1).min(self.results(ctx).len().saturating_sub(1));
                Flow::Stay
            }
            Cmd::Accept => self.select(ctx),
            _ => Flow::Stay,
        }
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        Overlay {
            kind: "search".into(),
            sel: self.selected as i32,
            input: self.text.clone(),
            input_hint: ctx.t("desktop.global_search_hint"),
            search_hint: ctx.t("desktop.global_search_hint"),
            search_results: self.results(ctx),
            search_library_label: ctx.t("desktop.global_search_library"),
            search_empty_text: ctx.t("desktop.global_search_empty"),
            ..Default::default()
        }
    }

    fn hints(&self, _ctx: &Ctx) -> Vec<crate::model::Hint> {
        Vec::new()
    }

    fn typing(&self) -> bool {
        true
    }

    fn replace_text(&mut self, text: &str, _ctx: &mut Ctx) {
        self.text = text.to_owned();
        self.selected = 0;
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        match zone {
            "search-select" => {
                self.selected = index;
                self.select(ctx)
            }
            "search-move" => {
                self.selected = index;
                Flow::Stay
            }
            _ => Flow::Close,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shell::Shell;
    use std::time::Instant;
    use tako_core::input::shortcut::KeyModifiers;
    use tako_core::library::{Game, GameStore, PlayStatus};

    fn shell() -> (tempfile::TempDir, Shell) {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.library = tako_core::library::GameLibrary::in_memory(
            vec![
                Game {
                    id: "steam:hades".into(),
                    title: "Hades".into(),
                    store: Some(GameStore::Steam),
                    publisher: Some("Supergiant Games".into()),
                    is_installed: true,
                    play_status: Some(PlayStatus::Beaten),
                    ..Default::default()
                },
                Game {
                    id: "steam:tunic".into(),
                    title: "Tunic".into(),
                    store: Some(GameStore::Steam),
                    ..Default::default()
                },
            ],
            tako_core::library::system_clock(),
        );
        let mut shell = Shell::new(ctx);
        shell.enter_desktop();
        (dir, shell)
    }

    #[test]
    fn global_search_filters_all_games_and_returns_status_without_list_headers() {
        let (_dir, shell) = shell();
        let mut search = GlobalSearch::new();
        search.text = "Hades".into();
        let rendered = search.render(&shell.ctx);
        assert_eq!(rendered.search_results.len(), 1);
        assert_eq!(rendered.search_results[0].id, "steam:hades");
        assert_eq!(rendered.search_results[0].initial, "H");
        assert_eq!(
            rendered.search_results[0].meta,
            "Supergiant Games · Instalado"
        );
        search.text.clear();
        assert_eq!(
            search.render(&shell.ctx).search_results[1].meta,
            "Steam · Não instalado"
        );
        assert_eq!(rendered.search_hint, rendered.input_hint);
        assert_eq!(
            rendered.search_library_label,
            shell.ctx.t("desktop.global_search_library")
        );
    }

    #[test]
    fn clearing_search_restores_all_results_and_escape_closes_the_layer() {
        let (_dir, mut shell) = shell();
        let mut search = GlobalSearch::new();
        search.replace_text("Hades", &mut shell.ctx);
        assert_eq!(search.render(&shell.ctx).search_results.len(), 1);
        search.replace_text("", &mut shell.ctx);
        assert_eq!(search.render(&shell.ctx).search_results.len(), 2);
        assert!(matches!(
            search.handle(Cmd::Back, &mut shell.ctx),
            Flow::Close
        ));
    }

    #[test]
    fn enter_opens_the_selected_game_and_ctrl_k_opens_search_from_profile() {
        let (_dir, mut shell) = shell();
        shell.click("desk-nav", 3);
        assert_eq!(shell.page().kind(), super::super::profile::KIND);
        assert!(shell.key("k", KeyModifiers::CTRL));
        assert!(shell.has_layers());
        shell.edited("overlay-search", "Tunic");
        shell.handle(Cmd::Accept);
        assert!(!shell.has_layers());
        let rendered = shell.render(Instant::now());
        let game = rendered
            .props
            .iter()
            .find_map(|(name, value)| match (*name, value) {
                ("desk-game", slint_interpreter::Value::Struct(game)) => game.get_field("title"),
                _ => None,
            });
        assert_eq!(
            game,
            Some(&slint_interpreter::Value::String("Tunic".into()))
        );
    }

    #[test]
    fn library_ctrl_k_keeps_its_embedded_search_and_the_gamepad_moves_results() {
        let (_dir, mut shell) = shell();
        assert!(!shell.key("k", KeyModifiers::CTRL));
        shell.click("global-search", 0);
        shell.edited("overlay-search", "");
        shell.handle(Cmd::Down);
        let overlay = shell.layers.last().unwrap().render(&shell.ctx);
        assert_eq!(overlay.sel, 1);
        shell.handle(Cmd::Back);
        assert!(!shell.has_layers());
    }
}
