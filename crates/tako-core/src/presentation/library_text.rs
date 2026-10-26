//! Port de `Presentation/LibraryText.cs`: textos da busca, dos filtros, da ordenação e da procura de jogos.

use super::game_text::{store_of, store_via, via_name};
use super::{tr, trn};
use crate::i18n::Lang;
use crate::library::{Game, GameStore, InstallFilter, LibrarySort, PlayStatus};

/// Ordem das opções de ordenação no painel e no botão "Ordenar".
pub const SORT_ORDER: [LibrarySort; 6] = [
    LibrarySort::RecentlyPlayed,
    LibrarySort::Name,
    LibrarySort::PlayTime,
    LibrarySort::Rating,
    LibrarySort::RecentlyAdded,
    LibrarySort::Size,
];

pub fn sort_label(lang: Lang, sort: LibrarySort) -> String {
    let key = match sort {
        LibrarySort::Name => "name",
        LibrarySort::PlayTime => "play_time",
        LibrarySort::RecentlyAdded => "recently_added",
        LibrarySort::Rating => "rating",
        LibrarySort::RecentlyPlayed => "recently_played",
        LibrarySort::Size => "size",
    };
    tr(lang, &format!("library.sort.{key}"), &[])
}

pub fn install_label(lang: Lang, install: InstallFilter) -> String {
    let key = match install {
        InstallFilter::Installed => "installed",
        InstallFilter::NotInstalled => "not_installed",
        InstallFilter::Any => "any",
    };
    tr(lang, &format!("library.install.{key}"), &[])
}

/// "Jogando", "Zerado"...; `None` = "Sem status".
pub fn status_label(lang: Lang, status: Option<PlayStatus>) -> String {
    let key = status.map_or("none", PlayStatus::key);
    tr(lang, &format!("library.status.{key}"), &[])
}

/// A opção seguinte da lista de ordenação (volta ao início no fim).
pub fn next_sort(sort: LibrarySort) -> LibrarySort {
    let i = SORT_ORDER.iter().position(|s| *s == sort).unwrap_or(0);
    SORT_ORDER[(i + 1) % SORT_ORDER.len()]
}

/// "Nenhum jogo", "1 jogo", "12 jogos".
pub fn game_count(lang: Lang, count: usize) -> String {
    if count == 0 {
        tr(lang, "library.no_games", &[])
    } else {
        trn(lang, "common.games", count as i64, &[])
    }
}

/// "Filtros", "Filtros (2)".
pub fn filters_label(lang: Lang, active: usize) -> String {
    if active == 0 {
        tr(lang, "library.filters", &[])
    } else {
        tr(
            lang,
            "library.filters_active",
            &[("n", &active.to_string())],
        )
    }
}

/// "a, b e c" / "a, b and c".
fn join_list(lang: Lang, items: &[String], last_key: &str) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => tr(
            lang,
            last_key,
            &[("list", &rest.join(", ")), ("last", last)],
        ),
    }
}

/// Resultado de "Procurar jogos instalados" pelas lojas ligadas (ADR-0019): "12 jogos da Steam e 3 do Xbox
/// encontrados". Lojas sem jogos não aparecem.
pub fn scan_summary(lang: Lang, enabled: &[(GameStore, usize)]) -> String {
    let found: Vec<_> = enabled.iter().filter(|(_, n)| *n > 0).collect();
    if found.is_empty() {
        if enabled.is_empty() {
            return tr(lang, "library.scan.all_disabled", &[]);
        }
        let stores: Vec<String> = enabled.iter().map(|(s, _)| store_of(lang, *s)).collect();
        return tr(
            lang,
            "library.scan.none_found",
            &[("stores", &join_list(lang, &stores, "library.list_or"))],
        );
    }
    let parts: Vec<String> = found
        .iter()
        .enumerate()
        .map(|(i, (s, n))| {
            let count = if i == 0 {
                game_count(lang, *n)
            } else {
                n.to_string()
            };
            format!("{count} {}", store_of(lang, *s))
        })
        .collect();
    let total: usize = found.iter().map(|(_, n)| n).sum();
    trn(
        lang,
        "library.scan.found",
        total as i64,
        &[("list", &join_list(lang, &parts, "library.list_and"))],
    )
}

/// Como o jogo abre, para o aviso "Abrindo…": " pela Steam", " pelo GOG Galaxy"; nada quando abre direto pelo
/// executável (adicionados, Xbox e GOG sem Galaxy).
pub fn launch_via(lang: Lang, game: &Game) -> String {
    match game.store() {
        GameStore::Manual | GameStore::Xbox => String::new(),
        GameStore::Gog => {
            let galaxy = game
                .launch_target
                .as_deref()
                .is_some_and(|t| t.to_ascii_lowercase().ends_with("galaxyclient.exe"));
            if galaxy {
                format!(" {}", via_name(lang, "GOG Galaxy"))
            } else {
                String::new()
            }
        }
        store => format!(" {}", store_via(lang, store)),
    }
}

/// Falha ao abrir um jogo que depende do cliente da loja (STORE-011/012): o aviso da última leitura ("EA app não
/// encontrado…") quando o cliente falta; `None` nos demais casos.
pub fn client_missing(game: &Game, client_installed: bool, notice: Option<&str>) -> Option<String> {
    let needs_client = matches!(
        game.store(),
        GameStore::Epic
            | GameStore::Ubisoft
            | GameStore::Ea
            | GameStore::BattleNet
            | GameStore::Amazon
    );
    match notice {
        Some(n) if needs_client && !client_installed => Some(format!("{n} ({})", game.title)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_options_cycle_in_panel_order() {
        let mut sort = SORT_ORDER[0];
        let mut seen = Vec::new();
        for _ in SORT_ORDER {
            seen.push(sort);
            sort = next_sort(sort);
        }
        assert_eq!(seen, SORT_ORDER);
        assert_eq!(sort, SORT_ORDER[0]);
        let mut all = SORT_ORDER;
        all.sort();
        assert_eq!(all, LibrarySort::ALL);
        assert_eq!(sort_label(Lang::PtBr, LibrarySort::Rating), "Nota");
        assert_eq!(sort_label(Lang::EnUs, LibrarySort::Rating), "Rating");
        assert_eq!(filters_label(Lang::PtBr, 2), "Filtros (2)");
        assert_eq!(game_count(Lang::PtBr, 0), "Nenhum jogo");
        assert_eq!(game_count(Lang::EnUs, 12), "12 games");
        assert_eq!(
            install_label(Lang::PtBr, InstallFilter::NotInstalled),
            "Não instalados"
        );
    }

    #[test]
    fn scan_summary_lists_only_stores_with_games() {
        use GameStore::*;
        assert_eq!(
            scan_summary(Lang::PtBr, &[(Steam, 12), (Epic, 0), (Xbox, 3)]),
            "12 jogos da Steam e 3 do Xbox encontrados"
        );
        assert_eq!(
            scan_summary(Lang::PtBr, &[(Steam, 12), (Epic, 2), (Xbox, 3)]),
            "12 jogos da Steam, 2 da Epic Games e 3 do Xbox encontrados"
        );
        assert_eq!(
            scan_summary(Lang::PtBr, &[(Gog, 1)]),
            "1 jogo do GOG encontrado"
        );
        assert_eq!(
            scan_summary(Lang::PtBr, &[(Steam, 0)]),
            "Nenhum jogo da Steam encontrado neste PC"
        );
        assert_eq!(
            scan_summary(Lang::PtBr, &[(Steam, 0), (Epic, 0), (Gog, 0)]),
            "Nenhum jogo da Steam, da Epic Games ou do GOG encontrado neste PC"
        );
        assert_eq!(
            scan_summary(Lang::PtBr, &[]),
            "Todas as lojas estão desligadas nas Configurações"
        );
        assert_eq!(
            scan_summary(Lang::EnUs, &[(Steam, 12), (Xbox, 3)]),
            "12 games from Steam and 3 from Xbox found"
        );
    }

    #[test]
    fn launch_via_and_client_missing() {
        let mut g = Game {
            store: Some(GameStore::Gog),
            title: "Farol".into(),
            ..Game::default()
        };
        assert_eq!(launch_via(Lang::PtBr, &g), "");
        g.launch_target = Some(r"C:\GOG Galaxy\GalaxyClient.exe".into());
        assert_eq!(launch_via(Lang::PtBr, &g), " pelo GOG Galaxy");
        g.store = Some(GameStore::Steam);
        assert_eq!(launch_via(Lang::PtBr, &g), " pela Steam");
        assert_eq!(client_missing(&g, false, Some("Aviso")), None); // a Steam abre sozinha
        g.store = Some(GameStore::Ea);
        assert_eq!(
            client_missing(&g, false, Some("EA app não encontrado")).as_deref(),
            Some("EA app não encontrado (Farol)")
        );
        assert_eq!(client_missing(&g, true, Some("x")), None);
    }
}
