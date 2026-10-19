//! Detalhes do jogo (o `DetailsViewModel` do WPF): arte, informações e as ações do jogo.

use super::{Cmd, Ctx, Effect, Nav, Out, Page, dialogs, tiles};
use crate::model::{Details, Hint, PanelRow, Stat};
use tako_core::library::{Game, GameStore, InstallOffer, InstallProgress};
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
    } else if let Some((p, store)) = install_busy(ctx, &g) {
        // A loja já está instalando (ou o pedido está na fila): abrir o cliente outra vez não muda nada, então o
        // aviso mostra o andamento.
        ctx.toast(installing_label(ctx, p, store));
    } else if install_offers(&g, ctx.simulated()).is_empty() {
        ctx.toast(ctx.t(if g.store() == GameStore::Manual {
            "game.missing_file"
        } else {
            "game.not_installed"
        }));
    } else {
        install_many(ctx, std::slice::from_ref(&g.id));
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
/// elas, a loja do jogo, pelo endereço que a fonte deu (Epic, GOG, Amazon) ou o da Steam. Endereços fora da lista de
/// `is_install_uri` ficam de fora.
pub(crate) fn install_offers(g: &Game, demo: bool) -> Vec<InstallOffer> {
    let listed: Vec<InstallOffer> = g
        .install_offers
        .iter()
        .filter(|o| is_install_uri(&o.install_uri, demo))
        .take(MAX_OFFERS)
        .cloned()
        .collect();
    if !listed.is_empty() {
        return listed;
    }
    let uri = match (&g.install_uri, g.store(), g.steam_app_id) {
        (Some(uri), ..) if is_install_uri(uri, demo) => uri.clone(),
        (_, GameStore::Steam, Some(id)) => format!("steam://install/{id}"),
        _ => return Vec::new(),
    };
    vec![InstallOffer {
        store: g.store(),
        game_id: g.id.clone(),
        install_uri: uri,
        size: g.install_size,
    }]
}

/// Só os endereços de instalação que as fontes geram, por prefixo exato (nada de `steam://run`, `steam://uninstall`,
/// `file://`, caminhos ou executáveis). Defesa em profundidade: o endereço vem de arquivos das lojas e o sistema o
/// abre como está. `demo` = o app roda em demonstração, onde o `demo://install/<n>` não tem programa registrado.
pub(crate) fn is_install_uri(uri: &str, demo: bool) -> bool {
    let word = |s: &str, extra: &[char]| {
        !s.is_empty()
            && !s.chars().all(|c| c == '.')
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || extra.contains(&c))
    };
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if let Some(id) = uri.strip_prefix("steam://install/") {
        digits(id)
    } else if let Some(rest) = uri.strip_prefix("com.epicgames.launcher://apps/") {
        // `<Namespace>%3A<CatalogItemId>%3A<AppName>` já escapado pela fonte.
        rest.strip_suffix("?action=install")
            .is_some_and(|ids| word(ids, &['%', '.', '_', '-', '~']))
    } else if let Some(id) = uri.strip_prefix("goggalaxy://openGameView/") {
        word(id, &['.', '_', '-'])
    } else if let Some(id) = uri.strip_prefix("amazon-games://install/") {
        word(id, &['.', '_', '-'])
    } else if let Some(rest) = uri.strip_prefix("heroic://launch/") {
        // `<runner>/<appName>`: os runners do Heroic que a fonte lê, sem query, `/` extra nem argumentos.
        rest.split_once('/').is_some_and(|(runner, id)| {
            ["legendary", "gog", "nile", "sideload"].contains(&runner) && word(id, &['.', '_', '-'])
        })
    } else if let Some(n) = uri.strip_prefix("demo://install/") {
        demo && digits(n)
    } else {
        false
    }
}

/// A oferta que vale para o jogo: a loja escolhida; sem escolha (ou se ela saiu da lista), a loja do próprio jogo;
/// por último, a primeira.
pub(crate) fn chosen_offer<'a>(
    g: &Game,
    offers: &'a [InstallOffer],
    pick: Option<GameStore>,
) -> Option<&'a InstallOffer> {
    pick.into_iter()
        .chain([g.store()])
        .find_map(|s| offers.iter().find(|o| o.store == s))
        .or(offers.first())
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
pub(crate) fn install_stage(ctx: &Ctx, g: &Game) -> Option<InstallStage> {
    if g.is_installed || g.is_upcoming() {
        return None;
    }
    let offers = install_offers(g, ctx.simulated());
    let o = chosen_offer(g, &offers, ctx.install_pick.get(&g.id).copied())?;
    let lang = ctx.lang();
    let at = game_text::store_in(lang, o.store);
    if let Some((p, store)) = install_busy(ctx, g) {
        return Some(InstallStage {
            label: installing_label(ctx, p, store),
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

/// A instalação em andamento do jogo: a do próprio item (loja dele) ou a da cópia da loja escolhida, que pode ser
/// outra (`gog:7` com a Steam escolhida deixa `steam:7` instalando). Conta também o pedido ainda na fila.
fn install_busy(ctx: &Ctx, g: &Game) -> Option<(InstallProgress, GameStore)> {
    if let Some(p) = g.installing {
        return Some((p, g.store()));
    }
    let offers = install_offers(g, ctx.simulated());
    let o = chosen_offer(g, &offers, ctx.install_pick.get(&g.id).copied())?;
    let id = if o.game_id.is_empty() {
        &g.id
    } else {
        &o.game_id
    };
    let progress = ctx.library.find(id).and_then(|c| c.installing);
    let pending = ctx.installing.iter().any(|(i, _)| i == id)
        || ctx.install_queue.iter().any(|(_, q)| q.game_id == *id);
    (progress.is_some() || pending).then(|| (progress.unwrap_or_default(), o.store))
}

/// "Baixando… 42%" ou, sem número, "Instalando na Steam…". A fração vem da loja: fora de 0..1 ou NaN conta como
/// "sem número".
fn installing_label(ctx: &Ctx, p: InstallProgress, store: GameStore) -> String {
    match p.fraction.filter(|f| f.is_finite()) {
        Some(f) => ctx.tf(
            "desktop.install.progress",
            &[(
                "pct",
                &((f.clamp(0.0, 1.0) * 100.0).round() as u32).to_string(),
            )],
        ),
        None => ctx.tf(
            "desktop.install.installing",
            &[("in", &game_text::store_in(ctx.lang(), store))],
        ),
    }
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

fn watch_install(ctx: &mut Ctx, id: &str) {
    ctx.installing.retain(|(i, _)| i != id);
    ctx.installing
        .push((id.to_owned(), std::time::Instant::now() + Ctx::INSTALL_WAIT));
}

/// Instalar um ou vários de uma vez (menu, seleção múltipla, destaque; SRC-005): um pedido por jogo na loja
/// escolhida (sem escolha, a do próprio jogo), espaçados para o cliente da loja não perder nenhum e sem repetir o
/// mesmo endereço. Os pedidos entram na fila do `Ctx` (`run_install_queue`). Devolve quantos foram pedidos.
pub(crate) fn install_many(ctx: &mut Ctx, ids: &[String]) -> usize {
    let mut seen: std::collections::HashSet<String> = ctx
        .install_queue
        .iter()
        .map(|(_, o)| o.install_uri.clone())
        .collect();
    let mut n = 0;
    for id in ids {
        let Some(g) = ctx.library.find(id).cloned() else {
            continue;
        };
        if g.is_installed || install_busy(ctx, &g).is_some() {
            continue;
        }
        let offers = install_offers(&g, ctx.simulated());
        let Some(o) = chosen_offer(&g, &offers, ctx.install_pick.get(id).copied()) else {
            continue;
        };
        if !seen.insert(o.install_uri.clone()) {
            continue;
        }
        let mut o = o.clone();
        if o.game_id.is_empty() {
            o.game_id = g.id.clone();
        }
        let at = std::time::Instant::now() + INSTALL_GAP * n as u32;
        ctx.install_queue.push((at, o));
        n += 1;
    }
    n
}

/// Manda à loja os pedidos de instalação que já deram a hora e passa a vigiar a cópia da loja escolhida; se o
/// sistema não abre o endereço, o aviso aparece na tela. Chamado a cada ciclo da interface (thread da UI).
pub fn run_install_queue(ctx: &mut Ctx, now: std::time::Instant) {
    if ctx.install_queue.is_empty() {
        return;
    }
    let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut ctx.install_queue)
        .into_iter()
        .partition(|(at, _)| *at <= now);
    ctx.install_queue = later;
    for (_, o) in due {
        match ctx.platform.processes.launch(&o.install_uri, None, None) {
            Ok(_) => watch_install(ctx, &o.game_id),
            Err(e) => ctx.toast(e.to_string()),
        }
    }
}

/// Intervalo entre dois pedidos de instalação seguidos.
const INSTALL_GAP: std::time::Duration = std::time::Duration::from_millis(1500);

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
            game_id: format!("{}:1", store.id_prefix()),
            install_uri: uri.into(),
            size,
        }
    }

    #[test]
    fn only_the_install_addresses_the_sources_make_are_opened() {
        for ok in [
            "steam://install/1",
            "com.epicgames.launcher://apps/a%3Ab%3Ac?action=install",
            "goggalaxy://openGameView/1",
            "amazon-games://install/amzn1.adg.product.1-2_3",
            "heroic://launch/legendary/Fennec",
            "heroic://launch/gog/1207658930",
            "heroic://launch/nile/amzn1.adg.product.2222-3333",
            "heroic://launch/sideload/meu_jogo-1.0",
        ] {
            assert!(is_install_uri(ok, false), "{ok}");
        }
        for bad in [
            "file:///x",
            "C:\\x.exe",
            "/usr/bin/x",
            "steam://",
            "steam://install/",
            "steam://install/12abc",
            "steam://install/1/extra",
            "steam://run/730//+exec x",
            "steam://uninstall/730",
            "steam://rungameid/730",
            "STEAM://install/1",
            " steam://install/1",
            "heroic://launch/epic/Fennec",
            "heroic://launch/Gog/1",
            "heroic://launch/gog",
            "heroic://launch/gog/",
            "heroic://launch/gog/1/extra",
            "heroic://launch/gog/1?arg=--x",
            "heroic://launch/gog/1&x=y",
            "heroic://launch/gog/a b",
            "heroic://launch//1",
            "heroic://launch/gog/..",
            "heroic://launch/gog/../x",
            "heroic://install/gog/1",
            "heroic://settings",
            "com.epicgames.launcher://apps/a%3Ab%3Ac?action=launch",
            "com.epicgames.launcher://apps/a?x=1&y=2?action=install",
            "com.epicgames.launcher://apps/a b?action=install",
            "goggalaxy://openGameView/1?x=y",
            "goggalaxy://openGameView/../x",
            "goggalaxy://openGameView/..",
            "amazon-games://play/x",
            "amazon-games://install/",
            "amazon-games://install/a b",
            "https://x",
            "steam://install/1\nfile:///x",
            "demo://install/1",
            "",
        ] {
            assert!(!is_install_uri(bad, false), "{bad:?}");
        }
        // O endereço da demonstração só vale com o app em demonstração.
        assert!(is_install_uri("demo://install/3", true));
        assert!(!is_install_uri("demo://install/x", true));
        assert!(!is_install_uri("demo://3", true));
        // Valor ruim vindo da loja não vira oferta, nem pelo `install_uri` do jogo.
        let mut g = game();
        g.install_offers = vec![
            offer(GameStore::Gog, "file:///x", None),
            offer(GameStore::Epic, "C:\\x.exe", None),
        ];
        g.install_uri = Some("file:///etc/passwd".into());
        let offers = install_offers(&g, false);
        assert_eq!(offers.len(), 1);
        assert_eq!(offers[0].install_uri, "steam://install/1");
    }

    #[test]
    fn the_default_offer_is_the_items_own_store_not_the_first_of_the_list() {
        let offers = [
            offer(GameStore::Steam, "steam://install/1", None),
            offer(GameStore::Gog, "goggalaxy://openGameView/1", None),
        ];
        let mut gog = game();
        gog.id = "gog:1".into();
        gog.store = Some(GameStore::Gog);
        let store = |g: &Game, pick| chosen_offer(g, &offers, pick).map(|o| o.store);
        assert_eq!(store(&gog, None), Some(GameStore::Gog));
        assert_eq!(store(&game(), None), Some(GameStore::Steam));
        // A escolha vale por loja, não por posição; loja que sumiu da lista volta à do item.
        assert_eq!(store(&game(), Some(GameStore::Gog)), Some(GameStore::Gog));
        assert_eq!(store(&gog, Some(GameStore::Epic)), Some(GameStore::Gog));
        // O item de uma loja que não está na lista cai na primeira.
        let mut epic = game();
        epic.store = Some(GameStore::Epic);
        assert_eq!(store(&epic, None), Some(GameStore::Steam));
    }

    #[test]
    fn offers_come_from_the_account_or_from_the_games_own_store() {
        assert_eq!(
            install_offers(&game(), false)[0].install_uri,
            "steam://install/1"
        );
        let mut g = game();
        g.install_offers = vec![
            offer(GameStore::Gog, "  ", None),
            offer(GameStore::Gog, "goggalaxy://openGameView/x", None),
        ];
        // Endereço em branco vindo da loja não vira opção.
        assert_eq!(install_offers(&g, false).len(), 1);
        g.install_offers = (0..50)
            .map(|i| {
                offer(
                    GameStore::Epic,
                    &format!("com.epicgames.launcher://apps/a%3Ab%3A{i}?action=install"),
                    None,
                )
            })
            .collect();
        assert_eq!(install_offers(&g, false).len(), MAX_OFFERS);
        let mut none = game();
        none.store = Some(GameStore::Manual);
        none.steam_app_id = None;
        assert!(install_offers(&none, false).is_empty());
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
        let s = install_stage(&ctx, &g).unwrap();
        assert_eq!(s.label, "Instalar");
        assert!(s.sub.starts_with("Steam · 9"));
        assert!(!s.split && !s.warn);
        assert!(s.note.contains("Steam"), "{}", s.note);

        ctx.free_space = Some(tako_core::devices::FreeSpace {
            drive: "C".into(),
            bytes: 120 << 30,
        });
        let s = install_stage(&ctx, &g).unwrap();
        assert!(s.note.starts_with("Livre no C: 120"), "{}", s.note);
        assert!(!s.warn);

        ctx.free_space = Some(tako_core::devices::FreeSpace {
            drive: "C".into(),
            bytes: 6 << 30,
        });
        let s = install_stage(&ctx, &g).unwrap();
        assert!(s.warn);
        assert!(s.note.starts_with("Faltam 3"), "{}", s.note);
        assert!(s.note.ends_with("no C:"), "{}", s.note);

        g.installing = Some(InstallProgress {
            fraction: Some(0.42),
        });
        let s = install_stage(&ctx, &g).unwrap();
        assert!(s.installing && s.note.is_empty());
        assert_eq!(s.label, "Baixando… 42%");
        for bad in [f32::NAN, f32::INFINITY, -1.0, 7.0] {
            g.installing = Some(InstallProgress {
                fraction: Some(bad),
            });
            let label = install_stage(&ctx, &g).unwrap().label;
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
            install_stage(&ctx, &g).unwrap().label,
            "Instalando na Steam…"
        );
    }

    #[test]
    fn two_stores_make_a_split_button_and_installed_games_have_no_stage() {
        let (_d, ctx) = stage_ctx();
        let mut g = game();
        g.install_offers = vec![
            offer(GameStore::Steam, "steam://install/1", Some(9 << 30)),
            offer(GameStore::Gog, "goggalaxy://openGameView/1", Some(8 << 30)),
        ];
        let mut ctx = ctx;
        assert!(install_stage(&ctx, &g).unwrap().split);
        // Sem escolha vale a loja do próprio jogo (Steam); escolhida a GOG, a pílula passa a ela.
        assert!(install_stage(&ctx, &g).unwrap().sub.starts_with("Steam"));
        ctx.install_pick.insert(g.id.clone(), GameStore::Gog);
        assert!(install_stage(&ctx, &g).unwrap().sub.starts_with("GOG"));
        g.is_installed = true;
        assert!(install_stage(&ctx, &g).is_none());
    }
}
