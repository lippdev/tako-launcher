//! Junta as fontes (port de `LibrarySources`): lê só as lojas ligadas e guarda como cada uma ficou
//! para as Configurações › Lojas (`LibrarySourceReport.Summary`).

use crate::{LibraryScan, LibrarySource};
use std::collections::{BTreeMap, HashSet};
use tako_core::i18n::{t, tf, tn};
use tako_core::library::{Game, GameStore};

/// Como uma loja ficou na última leitura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibrarySourceReport {
    pub store: GameStore,
    pub enabled: bool,
    pub game_count: usize,
    pub client_installed: bool,
    pub notice: Option<String>,
}

impl LibrarySourceReport {
    /// Linha de ajuda nas Configurações.
    pub fn summary(&self) -> String {
        let store = store_of(self.store);
        if !self.enabled {
            return tf("sources.summary.disabled", &[("store", &store)]);
        }
        let found = match self.game_count {
            0 => t("sources.summary.none"),
            n => tn("sources.summary.installed", n as i64, &[]),
        };
        // Sem o cliente, o aviso vem primeiro (é o que o usuário precisa resolver).
        match &self.notice {
            None => found,
            Some(n) if self.client_installed => format!("{found} · {n}"),
            Some(n) => format!("{n} · {found}"),
        }
    }
}

/// "da Steam", "do GOG" (pt-BR) / "Steam" (en-US), para frases como "os jogos da Steam".
pub fn store_of(store: GameStore) -> String {
    t(&format!("sources.store_of.{}", store.key()))
}

/// Página oficial para baixar o app da loja (SRC-003). Só endereços da própria loja: o Tako não baixa nem
/// instala nada sozinho.
pub fn client_download(store: GameStore) -> Option<&'static str> {
    match store {
        GameStore::Steam => Some("https://store.steampowered.com/about/"),
        GameStore::Epic => Some("https://store.epicgames.com/download"),
        GameStore::Gog => Some("https://www.gog.com/galaxy"),
        GameStore::Ubisoft => Some("https://ubisoftconnect.com/"),
        GameStore::Ea => Some("https://www.ea.com/ea-app"),
        GameStore::BattleNet => Some("https://download.battle.net/"),
        GameStore::Amazon => Some("https://gaming.amazon.com/"),
        // App Xbox na Microsoft Store.
        GameStore::Xbox => Some("ms-windows-store://pdp/?productid=9MV0B5HZVK9Z"),
        _ => None,
    }
}

/// Aviso de cliente ausente ou desatualizado ao falhar a abertura (port de `LibraryText.ClientMissing`):
/// só nas lojas que precisam do cliente para abrir o jogo.
pub fn client_missing(game: &Game, report: Option<&LibrarySourceReport>) -> Option<String> {
    let needs_client = matches!(
        game.store(),
        GameStore::Epic
            | GameStore::Ubisoft
            | GameStore::Ea
            | GameStore::BattleNet
            | GameStore::Amazon
    );
    match report {
        Some(r) if needs_client && !r.client_installed => {
            r.notice.as_ref().map(|n| format!("{n} ({})", game.title))
        }
        _ => None,
    }
}

/// As fontes registradas, na ordem de leitura.
pub struct LibrarySources {
    sources: Vec<Box<dyn LibrarySource>>,
    reports: BTreeMap<GameStore, LibrarySourceReport>,
}

impl LibrarySources {
    pub fn new(sources: Vec<Box<dyn LibrarySource>>) -> Self {
        LibrarySources {
            sources,
            reports: BTreeMap::new(),
        }
    }

    pub fn stores(&self) -> Vec<GameStore> {
        self.sources.iter().map(|s| s.store()).collect()
    }

    /// Última leitura de cada loja; ausente = ainda não lida.
    pub fn reports(&self) -> &BTreeMap<GameStore, LibrarySourceReport> {
        &self.reports
    }

    /// Jogos de todas as lojas ligadas (ids distintos; o primeiro vence).
    pub fn discover(&mut self, is_enabled: impl Fn(GameStore) -> bool) -> Vec<Game> {
        let mut games = Vec::new();
        let mut ids = HashSet::new();
        for source in &self.sources {
            let store = source.store();
            let enabled = is_enabled(store);
            let scan = if enabled {
                source.scan()
            } else {
                LibraryScan::default()
            };
            let before = games.len();
            games.extend(scan.games.into_iter().filter(|g| ids.insert(g.id.clone())));
            self.reports.insert(
                store,
                LibrarySourceReport {
                    store,
                    enabled,
                    game_count: games.len() - before,
                    client_installed: scan.client_installed,
                    notice: if enabled { scan.notice } else { None },
                },
            );
        }
        games
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Stub(GameStore, LibraryScan);

    impl LibrarySource for Stub {
        fn store(&self) -> GameStore {
            self.0
        }
        fn scan(&self) -> LibraryScan {
            self.1.clone()
        }
    }

    fn game(id: &str, store: GameStore) -> Game {
        Game {
            id: id.into(),
            title: id.into(),
            store: Some(store),
            is_installed: true,
            ..Default::default()
        }
    }

    fn scan(games: Vec<Game>, client: bool, notice: Option<&str>) -> LibraryScan {
        LibraryScan {
            games,
            client_installed: client,
            notice: notice.map(Into::into),
        }
    }

    #[test]
    fn sources_skip_disabled_stores_and_count_distinct_ids() {
        let gog_notice = "Sem GOG Galaxy: os jogos abrem direto pelo executável";
        let mut sources = LibrarySources::new(vec![
            Box::new(Stub(
                GameStore::Steam,
                scan(vec![game("steam:1", GameStore::Steam)], true, None),
            )),
            Box::new(Stub(
                GameStore::Gog,
                scan(
                    vec![game("gog:2", GameStore::Gog), game("gog:2", GameStore::Gog)],
                    false,
                    Some(gog_notice),
                ),
            )),
            Box::new(Stub(
                GameStore::Xbox,
                scan(vec![game("xbox:3", GameStore::Xbox)], true, None),
            )),
        ]);

        let games = sources.discover(|s| s != GameStore::Xbox);

        let ids: Vec<_> = games.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, ["steam:1", "gog:2"]);
        let r = sources.reports();
        assert_eq!(r[&GameStore::Steam].summary(), "1 jogo instalado");
        assert_eq!(
            r[&GameStore::Gog].summary(),
            "Sem GOG Galaxy: os jogos abrem direto pelo executável · 1 jogo instalado"
        );
        assert_eq!(
            r[&GameStore::Xbox].summary(),
            "Desligada: os jogos do Xbox não aparecem na biblioteca"
        );
        assert_eq!(
            sources.stores(),
            [GameStore::Steam, GameStore::Gog, GameStore::Xbox]
        );
    }

    #[test]
    fn summaries_name_each_store() {
        let report = |store, enabled, count, client, notice: Option<&str>| LibrarySourceReport {
            store,
            enabled,
            game_count: count,
            client_installed: client,
            notice: notice.map(Into::into),
        };
        assert_eq!(
            report(GameStore::Amazon, false, 0, false, None).summary(),
            "Desligada: os jogos da Amazon Games não aparecem na biblioteca"
        );
        assert_eq!(
            report(GameStore::Ea, true, 0, false, Some("Só o Origin")).summary(),
            "Só o Origin · Nenhum jogo instalado encontrado"
        );
        assert_eq!(
            report(GameStore::Steam, true, 3, true, None).summary(),
            "3 jogos instalados"
        );
    }

    #[test]
    fn launch_failure_shows_the_client_notice_only_for_client_stores() {
        let outdated = "Só o Origin está instalado e ele não abre mais os jogos: instale o EA app";
        let mut ea = game("ea:1", GameStore::Ea);
        ea.title = "Arena".into();
        let report = |store, client, notice: Option<&str>| LibrarySourceReport {
            store,
            enabled: true,
            game_count: 1,
            client_installed: client,
            notice: notice.map(Into::into),
        };
        assert_eq!(
            client_missing(&ea, Some(&report(GameStore::Ea, false, Some(outdated)))),
            Some(format!("{outdated} (Arena)"))
        );
        assert_eq!(
            client_missing(&ea, Some(&report(GameStore::Ea, true, None))),
            None
        );
        let gog = game("gog:1", GameStore::Gog);
        assert_eq!(
            client_missing(&gog, Some(&report(GameStore::Gog, false, Some("x")))),
            None
        );
    }
}
