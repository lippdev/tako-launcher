//! Barra lateral: Início, a biblioteca, as fontes com jogos, Configurações e Sair (sempre presente: o
//! princípio "recovery-first").

use super::Ctx;
use crate::model::NavItem;
use tako_core::library::GameStore;
use tako_core::presentation::game_text;

#[derive(Default)]
pub struct Sidebar {
    /// `Some` quando o foco está na barra.
    pub focus: Option<usize>,
}

impl Sidebar {
    pub fn index_of(&self, ctx: &Ctx, route: &str) -> usize {
        items(ctx).iter().position(|i| i.id == route).unwrap_or(0)
    }
}

fn item(id: &str, label: String, icon: &str) -> NavItem {
    NavItem {
        id: id.into(),
        label,
        icon: icon.into(),
        ..Default::default()
    }
}

pub fn store_icon(store: GameStore) -> &'static str {
    match store {
        GameStore::Steam => "store-steam",
        GameStore::Epic => "store-epic",
        GameStore::Gog => "store-gog",
        GameStore::Xbox => "gamepad",
        GameStore::Ubisoft => "store-ubisoft",
        GameStore::Ea => "store-ea",
        GameStore::BattleNet => "store-battlenet",
        GameStore::Amazon => "store-amazon",
        GameStore::Heroic | GameStore::Lutris => "grid",
        GameStore::Manual => "folder",
    }
}

pub fn route_of(store: GameStore) -> String {
    format!("library:store:{}", store.key().to_lowercase())
}

/// Itens na ordem da tela. Lojas sem jogos somem, menos a Steam e os adicionados.
pub fn items(ctx: &Ctx) -> Vec<NavItem> {
    let lang = ctx.lang();
    let games = ctx.library.games();
    let count =
        |f: &dyn Fn(&tako_core::library::Game) -> bool| games.iter().filter(|g| f(g)).count();
    let mut out = vec![item("home", ctx.t("nav.home"), "home")];
    let mut all = item("library:all", ctx.t("nav.all"), "grid");
    all.section = ctx.t("nav.section_library");
    all.count = games.len().to_string();
    out.push(all);
    let mut installed = item("library:installed", ctx.t("nav.installed"), "check");
    installed.count = count(&|g| g.is_installed).to_string();
    out.push(installed);
    let mut fav = item("library:favorites", ctx.t("nav.favorites"), "star");
    fav.count = count(&|g| g.is_favorite).to_string();
    out.push(fav);
    let mut first = true;
    for store in GameStore::ALL {
        let n = count(&|g| g.store() == store);
        if n == 0 && !matches!(store, GameStore::Steam | GameStore::Manual) {
            continue;
        }
        let label = if store == GameStore::Manual {
            ctx.t("nav.manual")
        } else {
            game_text::store_name(lang, store)
        };
        let mut it = item(&route_of(store), label, store_icon(store));
        it.count = n.to_string();
        if first {
            it.section = ctx.t("nav.section_sources");
            first = false;
        }
        out.push(it);
    }
    out.push(item("spacer", String::new(), ""));
    out.push(item("settings", ctx.t("nav.settings"), "gear"));
    out.push(item("exit", ctx.t("nav.exit"), "power"));
    out
}
