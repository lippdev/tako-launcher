//! Detalhes do jogo (o `DetailsViewModel` do WPF): arte, informações e as ações do jogo.

use super::{Cmd, Ctx, Effect, Nav, Out, Page, dialogs, tiles};
use crate::model::{Details, Hint, PanelRow, Stat};
use tako_core::library::{Game, GameStore, InstallOffer};
use tako_core::metadata::art::format_bytes;
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
    primary_action_with(ctx, id, 0)
}

/// Como `primary_action`; `offer` é a loja escolhida no destaque (a posição em `install_offers`).
pub fn primary_action_with(ctx: &mut Ctx, id: &str, offer: usize) -> Nav {
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
    } else if g.is_upcoming() {
        ctx.toast(tiles::play_label(ctx, &g));
    } else if g.installing.is_some() {
        // A loja já está instalando: abrir o cliente outra vez não muda nada.
    } else if !install(ctx, &g, offer) {
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

/// Mais lojas que isso numa lista vinda de fora não cabe num menu.
const MAX_OFFERS: usize = 8;

/// Por onde instalar um jogo que não está instalado: as lojas da conta (a da compra mais antiga primeiro) ou, sem
/// elas, a loja do jogo, pelo endereço que a fonte deu (Epic, GOG, Amazon, Heroic) ou o da Steam.
pub(crate) fn install_offers(g: &Game) -> Vec<InstallOffer> {
    let listed: Vec<InstallOffer> = g
        .install_offers
        .iter()
        .filter(|o| !o.install_uri.trim().is_empty())
        .take(MAX_OFFERS)
        .cloned()
        .collect();
    if !listed.is_empty() {
        return listed;
    }
    let uri = match (&g.install_uri, g.store(), g.steam_app_id) {
        (Some(uri), ..) if !uri.trim().is_empty() => uri.clone(),
        (_, GameStore::Steam, Some(id)) => format!("steam://install/{id}"),
        _ => return Vec::new(),
    };
    vec![InstallOffer {
        store: g.store(),
        install_uri: uri,
        size: g.install_size,
    }]
}

/// Tamanho de uma oferta: o dela ou, na loja do próprio jogo, o que o jogo guarda. Zero = não informado.
pub(crate) fn offer_size(g: &Game, o: &InstallOffer) -> Option<u64> {
    o.size
        .or_else(|| (o.store == g.store()).then_some(g.install_size).flatten())
        .filter(|b| *b > 0)
}

/// O que o destaque mostra de um jogo não instalado: a pílula (rótulo, loja e tamanho, ⌄ para escolher a loja) e a
/// linha de baixo (espaço livre e por onde abre, ou o aviso de falta de espaço).
pub(crate) struct InstallStage {
    pub label: String,
    pub sub: String,
    pub split: bool,
    pub installing: bool,
    pub note: String,
    pub warn: bool,
}

/// `None` = o jogo não tem o que instalar (já instalado, por lançar ou sem endereço de instalação).
pub(crate) fn install_stage(ctx: &Ctx, g: &Game, pick: usize) -> Option<InstallStage> {
    if g.is_installed || g.is_upcoming() {
        return None;
    }
    let offers = install_offers(g);
    let o = offers.get(pick).or(offers.first())?;
    let lang = ctx.lang();
    let at = game_text::store_in(lang, o.store);
    if let Some(p) = g.installing {
        // A fração vem da loja: fora de 0..1 ou NaN conta como "sem número".
        let label = match p.fraction.filter(|f| f.is_finite()) {
            Some(f) => ctx.tf(
                "desktop.install.progress",
                &[(
                    "pct",
                    &((f.clamp(0.0, 1.0) * 100.0).round() as u32).to_string(),
                )],
            ),
            None => ctx.tf("desktop.install.installing", &[("in", &at)]),
        };
        return Some(InstallStage {
            label,
            sub: String::new(),
            split: false,
            installing: true,
            note: String::new(),
            warn: false,
        });
    }
    let size = offer_size(g, o);
    let sub = std::iter::once(game_text::store_name(lang, o.store))
        .chain(size.map(size_text))
        .collect::<Vec<_>>()
        .join(" · ");
    let (note, warn) = match &ctx.free_space {
        Some(free) => {
            let drive = drive_label(&free.drive);
            match size {
                Some(need) if need > free.bytes => (
                    ctx.tf(
                        "desktop.install.short",
                        &[("size", &size_text(need - free.bytes)), ("drive", &drive)],
                    ),
                    true,
                ),
                _ => {
                    let opens = ctx.tf("desktop.install.opens", &[("in", &at)]);
                    let free = ctx.tf(
                        "desktop.install.free",
                        &[("size", &size_text(free.bytes)), ("drive", &drive)],
                    );
                    (format!("{free} · {opens}"), false)
                }
            }
        }
        None => (ctx.tf("desktop.install.opens_alone", &[("in", &at)]), false),
    };
    Some(InstallStage {
        label: ctx.t("desktop.install.label"),
        sub,
        split: offers.len() > 1,
        installing: false,
        note,
        warn,
    })
}

/// "9,1 GB" (pt-BR) ou "9.1 GB"; sem a casa quando ela é zero ("120 GB").
pub(crate) fn size_text(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".into();
    }
    format_bytes(bytes).replace(",0 ", " ").replace(".0 ", " ")
}

/// A letra do disco com os dois pontos ("C:"); ponto de montagem fica como veio. Texto de fora: sem limite aqui
/// o aviso estouraria a tela.
fn drive_label(drive: &str) -> String {
    let d: String = drive.trim().chars().take(24).collect();
    let letter = d.trim_end_matches(':');
    if letter.chars().count() == 1 && letter.chars().all(|c| c.is_ascii_alphabetic()) {
        format!("{}:", letter.to_ascii_uppercase())
    } else {
        d
    }
}

/// Abre o cliente da loja escolhida na tela de instalação e fica de olho até o jogo aparecer instalado.
fn install(ctx: &mut Ctx, g: &Game, offer: usize) -> bool {
    let offers = install_offers(g);
    let Some(o) = offers.get(offer).or(offers.first()) else {
        return false;
    };
    ctx.effect(Effect::OpenUrl(o.install_uri.clone()));
    watch_install(ctx, &g.id);
    true
}

fn watch_install(ctx: &mut Ctx, id: &str) {
    ctx.installing.retain(|(i, _)| i != id);
    ctx.installing
        .push((id.to_owned(), std::time::Instant::now() + Ctx::INSTALL_WAIT));
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
                art: tiles::image(g.shown_hero(), tiles::hero(ctx)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::library::InstallProgress;

    fn game() -> Game {
        Game {
            id: "steam:1".into(),
            title: "Jogo".into(),
            store: Some(GameStore::Steam),
            steam_app_id: Some(1),
            ..Default::default()
        }
    }

    fn offer(store: GameStore, uri: &str, size: Option<u64>) -> InstallOffer {
        InstallOffer {
            store,
            install_uri: uri.into(),
            size,
        }
    }

    #[test]
    fn offers_come_from_the_account_or_from_the_games_own_store() {
        assert_eq!(install_offers(&game())[0].install_uri, "steam://install/1");
        let mut g = game();
        g.install_offers = vec![
            offer(GameStore::Gog, "  ", None),
            offer(GameStore::Gog, "goggalaxy://x", None),
        ];
        // Endereço em branco vindo da loja não vira opção.
        assert_eq!(install_offers(&g).len(), 1);
        g.install_offers = (0..50)
            .map(|i| offer(GameStore::Epic, &format!("e://{i}"), None))
            .collect();
        assert_eq!(install_offers(&g).len(), MAX_OFFERS);
        let mut none = game();
        none.store = Some(GameStore::Manual);
        none.steam_app_id = None;
        assert!(install_offers(&none).is_empty());
    }

    #[test]
    fn size_and_drive_texts_are_short_and_safe() {
        assert_eq!(size_text(0), "0 B");
        assert_eq!(size_text(120 << 30).replace('.', ","), "120 GB");
        assert_eq!(drive_label("c"), "C:");
        assert_eq!(drive_label("C:"), "C:");
        assert_eq!(drive_label("/home"), "/home");
        assert_eq!(drive_label(&"x".repeat(100)).chars().count(), 24);
    }

    fn stage_ctx() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let (_l, ctx) = crate::metadata::test_ctx(dir.path(), false);
        (dir, ctx)
    }

    #[test]
    fn the_stage_warns_when_the_disk_is_short_and_follows_the_progress() {
        let (_d, mut ctx) = stage_ctx();
        let mut g = game();
        g.install_size = Some(9 << 30);
        // Sem espaço livre conhecido: só por onde abre.
        let s = install_stage(&ctx, &g, 0).unwrap();
        assert_eq!(s.label, "Instalar");
        assert!(s.sub.starts_with("Steam · 9"));
        assert!(!s.split && !s.warn);
        assert!(s.note.contains("Steam"), "{}", s.note);

        ctx.free_space = Some(tako_core::devices::FreeSpace {
            drive: "C".into(),
            bytes: 120 << 30,
        });
        let s = install_stage(&ctx, &g, 0).unwrap();
        assert!(s.note.starts_with("Livre no C: 120"), "{}", s.note);
        assert!(!s.warn);

        ctx.free_space = Some(tako_core::devices::FreeSpace {
            drive: "C".into(),
            bytes: 6 << 30,
        });
        let s = install_stage(&ctx, &g, 0).unwrap();
        assert!(s.warn);
        assert!(s.note.starts_with("Faltam 3"), "{}", s.note);
        assert!(s.note.ends_with("no C:"), "{}", s.note);

        g.installing = Some(InstallProgress {
            fraction: Some(0.42),
        });
        let s = install_stage(&ctx, &g, 0).unwrap();
        assert!(s.installing && s.note.is_empty());
        assert_eq!(s.label, "Baixando… 42%");
        for bad in [f32::NAN, f32::INFINITY, -1.0, 7.0] {
            g.installing = Some(InstallProgress {
                fraction: Some(bad),
            });
            let label = install_stage(&ctx, &g, 0).unwrap().label;
            assert!(
                matches!(
                    label.as_str(),
                    "Instalando na Steam…" | "Baixando… 0%" | "Baixando… 100%"
                ),
                "{label}"
            );
        }
        g.installing = Some(InstallProgress { fraction: None });
        assert_eq!(
            install_stage(&ctx, &g, 0).unwrap().label,
            "Instalando na Steam…"
        );
    }

    #[test]
    fn two_stores_make_a_split_button_and_installed_games_have_no_stage() {
        let (_d, ctx) = stage_ctx();
        let mut g = game();
        g.install_offers = vec![
            offer(GameStore::Steam, "steam://install/1", Some(9 << 30)),
            offer(GameStore::Gog, "goggalaxy://1", Some(8 << 30)),
        ];
        assert!(install_stage(&ctx, &g, 0).unwrap().split);
        assert!(install_stage(&ctx, &g, 1).unwrap().sub.starts_with("GOG"));
        g.is_installed = true;
        assert!(install_stage(&ctx, &g, 0).is_none());
    }
}
