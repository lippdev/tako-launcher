//! Detalhes do jogo (o `DetailsViewModel` do WPF): arte, informações e as ações do jogo.

use super::{Cmd, Ctx, Effect, Nav, Out, Page, dialogs, tiles};
use crate::model::{Details, Hint, PanelRow, Stat};
use tako_core::library::{Game, GameStore};
use tako_core::presentation::{game_text, library_text};

pub struct DetailsPage {
    id: String,
    focus: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Primary,
    Favorite,
    Options,
    Collections,
    Settings,
}

const ACTIONS: [Action; 5] = [
    Action::Primary,
    Action::Favorite,
    Action::Options,
    Action::Collections,
    Action::Settings,
];

impl DetailsPage {
    pub fn new(id: &str) -> DetailsPage {
        DetailsPage {
            id: id.into(),
            focus: 0,
        }
    }
}

/// A ação principal de um jogo (Jogar, Voltar ao jogo, Instalar via loja...).
pub fn primary_action(ctx: &mut Ctx, id: &str) -> Nav {
    let Some(g) = ctx.library.find(id).cloned() else {
        return Nav::Stay;
    };
    if ctx.running.as_ref().is_some_and(|r| r.id == g.id) {
        ctx.effect(Effect::ResumeGame);
    } else if ctx.running.is_some() {
        return Nav::Layer(Box::new(dialogs::other_game_running(ctx, &g.id)));
    } else if g.is_broken {
        return Nav::Layer(Box::new(dialogs::broken_game(ctx, &g.id)));
    } else if g.can_play()
        && let Some(notice) = client_missing(ctx, &g)
    {
        return Nav::Layer(Box::new(dialogs::client_missing(ctx, &g, notice)));
    } else if g.can_play() && !crate::session::effective(ctx, &g.id).commands_approved {
        return Nav::Layer(Box::new(dialogs::approve_commands(ctx, &g.id)));
    } else if g.can_play() {
        ctx.effect(Effect::Launch(g.id.clone()));
    } else if g.is_upcoming() || g.is_rom() {
        ctx.toast(tiles::play_label(ctx, &g));
    } else if let Some(uri) = install_uri(&g) {
        ctx.effect(Effect::OpenUrl(uri));
    } else {
        ctx.toast(ctx.t(if g.store() == GameStore::Manual {
            "game.missing_file"
        } else {
            "game.not_installed"
        }));
    }
    Nav::Stay
}

/// O jogo precisa do app da loja e a última leitura não o achou (SRC-003): o aviso da loja.
fn client_missing(ctx: &Ctx, g: &Game) -> Option<String> {
    let report = ctx.sources.iter().find(|r| r.store == g.store());
    tako_sources::report::client_missing(g, report)
}

/// Verificação de arquivos da loja dona do jogo (SESSION-006). Só a Steam tem um caminho oficial para pedir
/// isso de fora; nas outras lojas a verificação fica dentro do app delas.
pub(crate) fn repair_uri(g: &Game) -> Option<String> {
    match (g.store(), g.steam_app_id) {
        (GameStore::Steam, Some(id)) if g.is_installed => Some(format!("steam://validate/{id}")),
        _ => None,
    }
}

/// Onde instalar um jogo que não está instalado (só a Steam tem um caminho oficial direto).
fn install_uri(g: &Game) -> Option<String> {
    match (g.store(), g.steam_app_id) {
        (GameStore::Steam, Some(id)) => Some(format!("steam://install/{id}")),
        _ => None,
    }
}

pub fn stats(ctx: &Ctx, g: &Game) -> Vec<Stat> {
    let lang = ctx.lang();
    let mut s = Vec::new();
    let mut add = |key: &str, value: String| {
        if !value.trim().is_empty() {
            s.push(Stat {
                label: ctx.t(key),
                value,
            });
        }
    };
    if !g.play_time.is_zero() {
        add("details.play_time", game_text::play_time(lang, g.play_time));
    }
    if g.last_played_at.is_some() {
        add(
            "details.last_played",
            game_text::last_played(lang, g.last_played_at, chrono::Utc::now()),
        );
    }
    match (g.metacritic_score, g.critic_score) {
        (Some(m), _) => add("details.metacritic", m.to_string()),
        (None, Some(c)) => add("details.critics", c.to_string()),
        _ => {}
    }
    if let Some(size) = &g.size_label {
        add("details.size", size.clone());
    }
    if let Some(d) = g.release_date {
        add("details.release", game_text::date(lang, d));
    }
    add("details.genres", game_text::genres(g));
    if let Some(d) = &g.developer {
        add("details.developer", d.clone());
    }
    if let Some(f) = &g.franchise {
        add("details.series", f.clone());
    }
    if let Some(p) = &g.platform {
        add("details.system", p.to_uppercase());
    }
    if let Some(p) = &g.players {
        add("details.players", p.clone());
    }
    add("details.status", game_text::status(lang, g));
    if !g.also_installed_in.is_empty() {
        let names: Vec<String> = g
            .also_installed_in
            .iter()
            .map(|s| game_text::store_name(lang, *s))
            .collect();
        add("details.also_in", names.join(", "));
    }
    s
}

impl Page for DetailsPage {
    fn route(&self) -> String {
        "details".into()
    }

    fn kind(&self) -> &'static str {
        "details"
    }

    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Nav {
        if ctx.library.find(&self.id).is_none() {
            return Nav::Back;
        }
        match cmd {
            Cmd::Left => self.focus = self.focus.saturating_sub(1),
            Cmd::Right => self.focus = (self.focus + 1).min(ACTIONS.len() - 1),
            Cmd::Back => return Nav::Back,
            Cmd::Favorite => dialogs::toggle_favorite(ctx, &self.id),
            Cmd::Menu => return Nav::Layer(Box::new(dialogs::game_options(ctx, &self.id))),
            Cmd::Accept => {
                return match ACTIONS[self.focus] {
                    Action::Primary => primary_action(ctx, &self.id),
                    Action::Favorite => {
                        dialogs::toggle_favorite(ctx, &self.id);
                        Nav::Stay
                    }
                    Action::Options => Nav::Layer(Box::new(dialogs::game_options(ctx, &self.id))),
                    Action::Collections => {
                        Nav::Layer(Box::new(dialogs::choose_collections(ctx, &self.id)))
                    }
                    Action::Settings => Nav::Layer(dialogs::game_settings(ctx, &self.id)),
                };
            }
            _ => {}
        }
        Nav::Stay
    }

    fn render(&mut self, ctx: &Ctx, out: &mut Out) {
        let Some(g) = ctx.library.find(&self.id) else {
            return;
        };
        let lang = ctx.lang();
        let mut meta = vec![game_text::store_name(lang, g.store())];
        if let Some(d) = g.release_date {
            meta.push(d.format("%Y").to_string());
        }
        if let Some(d) = &g.developer {
            meta.push(d.clone());
        }
        let mut overline = game_text::status(lang, g);
        if g.play_status.is_some() {
            overline = format!(
                "{overline} · {}",
                library_text::status_label(lang, g.play_status)
            );
        }
        let hint = if g.is_broken {
            ctx.t("game.broken_text")
        } else if g.description.is_empty()
            && !super::settings::metadata::providers(ctx).igdb.enabled
        {
            ctx.t("details.metadata_hint")
        } else {
            g.note.clone().unwrap_or_default()
        };
        out.set(
            "details",
            Details {
                title: g.title.clone(),
                overline: overline.to_uppercase(),
                meta: meta.join(" · "),
                description: g.description.clone(),
                art: tiles::image(g.shown_hero(), tiles::HERO),
                cover: tiles::image(g.shown_cover(), tiles::COVER),
                logo: tiles::image(g.shown_logo(), tiles::LOGO),
                hint,
            },
        );
        out.set("details-stats", stats(ctx, g));
        let n_cols = ctx.collections.of(&g.id).len();
        let actions = vec![
            PanelRow::button("primary", tiles::play_label(ctx, g)).icon("play"),
            PanelRow::button(
                "favorite",
                ctx.t(if g.is_favorite {
                    "details.unfavorite"
                } else {
                    "details.favorite"
                }),
            )
            .icon(if g.is_favorite { "starfill" } else { "star" }),
            PanelRow::button("options", ctx.t("details.options")).icon("more"),
            PanelRow::button(
                "collections",
                if n_cols > 0 {
                    ctx.tf("details.collections_n", &[("n", &n_cols.to_string())])
                } else {
                    ctx.t("details.collections")
                },
            )
            .icon("grid"),
            PanelRow::button("settings", ctx.t("details.game_settings")).icon("gear"),
        ];
        out.set("details-actions", actions);
        out.set("details-focus", self.focus as i32);
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![
            ctx.hint(Cmd::Accept, &ctx.t("hint.select")),
            ctx.hint(Cmd::Favorite, &ctx.t("hint.favorite")),
            ctx.hint(Cmd::Menu, &ctx.t("hint.options")),
            ctx.hint(Cmd::Back, &ctx.t("hint.back")),
        ]
    }

    fn hover(&mut self, zone: &str, index: usize, _ctx: &Ctx) {
        if zone == "action" {
            self.focus = index.min(ACTIONS.len() - 1);
        }
    }

    fn focused_game(&self, _ctx: &Ctx) -> Option<String> {
        Some(self.id.clone())
    }

    fn focus_game(&mut self, _id: &str, _ctx: &Ctx) {
        self.focus = 0;
    }
}
