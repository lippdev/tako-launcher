//! Port de `Library/LibraryQuery.cs`: rota da barra lateral, ordenação, facetas e as fileiras da Início.

use super::criteria::{LibraryCriteria, LibraryFacts};
use super::search_text::{Query, without_marks};
use super::{Game, GameStore};
use std::cmp::{Ordering, Reverse};
use std::collections::HashMap;

/// Filtro da biblioteca (barra lateral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LibraryFilter {
    #[default]
    All,
    Installed,
    Favorites,
    Store(GameStore),
}

impl LibraryFilter {
    pub fn matches(self, game: &Game) -> bool {
        match self {
            LibraryFilter::All => true,
            LibraryFilter::Installed => game.is_installed,
            LibraryFilter::Favorites => game.is_favorite,
            LibraryFilter::Store(store) => game.store() == store,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, serde::Serialize, serde::Deserialize,
)]
pub enum LibrarySort {
    #[default]
    RecentlyPlayed,
    Name,
    PlayTime,
    RecentlyAdded,
    /// Nota (Metacritic), maior primeiro; jogos sem nota no fim.
    Rating,
    /// Tamanho instalado (LIB-005), maior primeiro; jogos sem tamanho no fim.
    Size,
}

impl LibrarySort {
    /// Ordem de declaração (o `Enum.GetValues` do C#).
    pub const ALL: [LibrarySort; 6] = [
        LibrarySort::RecentlyPlayed,
        LibrarySort::Name,
        LibrarySort::PlayTime,
        LibrarySort::RecentlyAdded,
        LibrarySort::Rating,
        LibrarySort::Size,
    ];
}

/// Um valor de filtro (fonte ou gênero) e quantos jogos têm esse valor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetValue {
    pub name: String,
    pub count: usize,
}

/// Chave de ordenação de [`title_key`]: (classe do caractere, caractere).
pub type TitleKey = Vec<(u8, char)>;

/// Chave da ordem de nomes: sem diferenciar maiúsculas nem acentos ("Él" junto de "El"). Como no ICU, espaço e
/// pontuação vêm antes dos algarismos, e os algarismos antes das letras ("007" antes de "Avowed").
pub fn title_key(title: &str) -> TitleKey {
    without_marks(title)
        .flat_map(char::to_lowercase)
        .map(|c| {
            let class = if c.is_whitespace() {
                0
            } else if c.is_numeric() {
                2
            } else if c.is_alphabetic() {
                3
            } else {
                1
            };
            (class, c)
        })
        .collect()
}

/// Comparação de títulos da biblioteca (o `TitleComparer` do C#).
pub fn title_cmp(a: &str, b: &str) -> Ordering {
    title_key(a).cmp(&title_key(b))
}

/// Mesmo título para a biblioteca (sem caixa nem acento).
pub fn title_eq(a: &str, b: &str) -> bool {
    title_cmp(a, b) == Ordering::Equal
}

/// Biblioteca do Tako: rota da barra lateral + busca, filtros do painel e ordenação.
pub fn apply<'a>(
    games: &'a [Game],
    filter: LibraryFilter,
    criteria: &LibraryCriteria,
) -> Vec<&'a Game> {
    apply_by(
        games.iter().filter(|g| filter.matches(g)),
        |g| LibraryFacts::of(g),
        criteria,
    )
}

/// Filtra e ordena qualquer lista (os jogos do Tako ou os de um tema Playnite) pelos fatos de cada item. Empates
/// saem em ordem de nome.
pub fn apply_by<T>(
    items: impl IntoIterator<Item = T>,
    facts: impl Fn(&T) -> LibraryFacts,
    criteria: &LibraryCriteria,
) -> Vec<T> {
    let query = Query::parse(&criteria.search);
    let mut rows: Vec<(T, LibraryFacts, TitleKey)> = items
        .into_iter()
        .filter_map(|item| {
            let f = facts(&item);
            criteria.matches_query(&f, &query).then(|| {
                let key = title_key(&f.title);
                (item, f, key)
            })
        })
        .collect();
    rows.sort_by(|(_, a, ka), (_, b, kb)| {
        let primary = match criteria.sort {
            LibrarySort::Name => Ordering::Equal,
            LibrarySort::PlayTime => b.play_time.cmp(&a.play_time),
            LibrarySort::RecentlyAdded => b.added_at.cmp(&a.added_at),
            LibrarySort::Rating => newest_first(a.score, b.score),
            LibrarySort::Size => newest_first(a.size_bytes, b.size_bytes),
            // Jogados primeiro (mais recente no topo); nunca jogados depois, por nome.
            LibrarySort::RecentlyPlayed => newest_first(a.last_played_at, b.last_played_at),
        };
        primary.then_with(|| ka.cmp(kb))
    });
    rows.into_iter().map(|(item, _, _)| item).collect()
}

/// Maior primeiro, sem valor no fim.
fn newest_first<T: Ord>(a: Option<T>, b: Option<T>) -> Ordering {
    a.is_none().cmp(&b.is_none()).then_with(|| b.cmp(&a))
}

/// Fontes presentes na lista (Steam, Adicionado...), com quantos jogos cada uma tem, em ordem de nome.
pub fn sources<T>(
    items: impl IntoIterator<Item = T>,
    facts: impl Fn(&T) -> LibraryFacts,
) -> Vec<FacetValue> {
    facet(items.into_iter().map(|i| vec![facts(&i).source]))
}

/// Gêneros presentes na lista (vêm dos metadados), com quantos jogos cada um tem, em ordem de nome.
pub fn genres<T>(
    items: impl IntoIterator<Item = T>,
    facts: impl Fn(&T) -> LibraryFacts,
) -> Vec<FacetValue> {
    facet(items.into_iter().map(|i| facts(&i).genres))
}

fn facet(values: impl Iterator<Item = Vec<String>>) -> Vec<FacetValue> {
    let mut out: Vec<FacetValue> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for item in values {
        let mut seen = Vec::new();
        for v in item.iter().map(|v| v.trim()).filter(|v| !v.is_empty()) {
            let fold = v.to_lowercase();
            if seen.contains(&fold) {
                continue;
            }
            match index.get(&fold) {
                Some(&i) => out[i].count += 1,
                None => {
                    index.insert(fold.clone(), out.len());
                    out.push(FacetValue {
                        name: v.to_owned(),
                        count: 1,
                    });
                }
            }
            seen.push(fold);
        }
    }
    out.sort_by_cached_key(|f| title_key(&f.name));
    out
}

/// Fileira "Jogados recentemente" da Início: só jogos já jogados, do mais recente ao mais antigo.
pub fn recently_played(games: &[Game]) -> Vec<&Game> {
    let mut out: Vec<&Game> = games
        .iter()
        .filter(|g| g.last_played_at.is_some())
        .collect();
    out.sort_by_key(|g| Reverse(g.last_played_at));
    out
}

pub fn recently_added(games: &[Game], count: usize) -> Vec<&Game> {
    let mut out: Vec<&Game> = games.iter().collect();
    out.sort_by_key(|g| Reverse(g.added_at));
    out.truncate(count);
    out
}

#[cfg(test)]
mod tests {
    use super::super::sample;
    use super::*;
    use crate::library::InstallFilter;

    fn ids(games: &[&Game]) -> Vec<String> {
        games.iter().map(|g| g.id.clone()).collect()
    }

    fn sorted(mut v: Vec<String>) -> Vec<String> {
        v.sort();
        v
    }

    fn with_search(search: &str, sort: LibrarySort) -> LibraryCriteria {
        LibraryCriteria {
            search: search.into(),
            sort,
            ..Default::default()
        }
    }

    fn by_sort(sort: LibrarySort) -> LibraryCriteria {
        with_search("", sort)
    }

    // ---- LibraryTests

    #[test]
    fn recently_played_excludes_never_played_and_is_newest_first() {
        let games = sample::games();
        let recent = recently_played(&games);
        assert!(recent.iter().all(|g| g.last_played_at.is_some()));
        assert!(
            recent
                .windows(2)
                .all(|w| w[0].last_played_at >= w[1].last_played_at)
        );
        assert_eq!(recent[0].id, "control");
    }

    #[test]
    fn store_filter_matches_only_that_store() {
        let games = sample::games();
        for (store, expected) in [
            (GameStore::Xbox, 4),
            (GameStore::Manual, 2),
            (GameStore::Gog, 1),
        ] {
            let result = apply(
                &games,
                LibraryFilter::Store(store),
                &by_sort(LibrarySort::Name),
            );
            assert_eq!(result.len(), expected, "{store:?}");
            assert!(result.iter().all(|g| g.store() == store));
        }
    }

    #[test]
    fn name_sort_is_alphabetical_and_recent_sort_puts_unplayed_last() {
        let games = sample::games();
        let by_name = apply(&games, LibraryFilter::All, &by_sort(LibrarySort::Name));
        let by_recent = apply(
            &games,
            LibraryFilter::All,
            &by_sort(LibrarySort::RecentlyPlayed),
        );
        assert!(
            by_name
                .windows(2)
                .all(|w| title_cmp(&w[0].title, &w[1].title) != Ordering::Greater)
        );
        assert!(
            by_recent
                .iter()
                .skip_while(|g| g.last_played_at.is_some())
                .all(|g| g.last_played_at.is_none())
        );
    }

    // ---- LibrarySearchTests

    #[test]
    fn search_finds_games_live_by_partial_title() {
        let games = sample::games();
        let gow = apply(
            &games,
            LibraryFilter::All,
            &with_search("god of", LibrarySort::Name),
        );
        let circle = apply(
            &games,
            LibraryFilter::All,
            &with_search("CIRCULO", LibrarySort::RecentlyPlayed),
        );
        let none = apply(
            &games,
            LibraryFilter::All,
            &with_search("zzz", LibrarySort::RecentlyPlayed),
        );
        assert_eq!(ids(&gow), ["gow3", "ragnarok"]);
        assert_eq!(ids(&circle), ["indiana"]);
        assert!(none.is_empty());
    }

    #[test]
    fn search_combines_with_the_sidebar_route() {
        let games = sample::games();
        let steam = apply(
            &games,
            LibraryFilter::Store(GameStore::Steam),
            &with_search("god", LibrarySort::RecentlyPlayed),
        );
        assert_eq!(ids(&steam), ["ragnarok"]); // o God of War III é adicionado manualmente
    }

    #[test]
    fn install_filter_cycles_and_splits_the_library() {
        let games = sample::games();
        let criteria = LibraryCriteria::default().next_install();
        assert_eq!(criteria.install, InstallFilter::Installed);
        let installed = apply(&games, LibraryFilter::All, &criteria);
        let criteria = criteria.next_install();
        assert_eq!(criteria.install, InstallFilter::NotInstalled);
        let not_installed = apply(&games, LibraryFilter::All, &criteria);

        assert!(installed.iter().all(|g| g.is_installed));
        assert_eq!(sorted(ids(&not_installed)), ["ds2", "phantom"]);
        assert_eq!(games.len(), installed.len() + not_installed.len());
        assert_eq!(criteria.next_install().install, InstallFilter::Any);
    }

    #[test]
    fn panel_filters_are_or_within_a_group_and_and_between_groups() {
        let games = sample::games();
        let xbox_or_gog = LibraryCriteria {
            sort: LibrarySort::Name,
            ..Default::default()
        }
        .toggle_source("Xbox")
        .toggle_source("GOG");
        let sources = apply(&games, LibraryFilter::All, &xbox_or_gog);
        let favorite_sources = apply(
            &games,
            LibraryFilter::All,
            &LibraryCriteria {
                favorites_only: true,
                ..xbox_or_gog.clone()
            },
        );
        let rpg_not_installed = apply(
            &games,
            LibraryFilter::All,
            &LibraryCriteria {
                install: InstallFilter::NotInstalled,
                ..LibraryCriteria::default().toggle_genre("rpg")
            },
        );

        assert_eq!(
            ids(&sources),
            ["avowed", "clair", "forza5", "silksong", "indiana"]
        ); // "Hollow" antes de "Indiana"
        assert_eq!(ids(&favorite_sources), ["clair", "silksong"]);
        assert_eq!(ids(&rpg_not_installed), ["phantom"]); // gênero sem diferenciar maiúsculas
        assert_eq!(xbox_or_gog.active_filter_count(), 2);
        assert_eq!(xbox_or_gog.toggle_source("xbox").active_filter_count(), 1); // desmarcar
    }

    #[test]
    fn played_only_matches_the_home_recently_played_row() {
        let games = sample::games();
        let played = apply(
            &games,
            LibraryFilter::All,
            &LibraryCriteria {
                played_only: true,
                ..Default::default()
            },
        );
        assert_eq!(sorted(ids(&recently_played(&games))), sorted(ids(&played)));
    }

    #[test]
    fn sorts_by_name_last_played_play_time_rating_and_date_added() {
        let mut games = sample::games();
        for (id, score) in [("ragnarok", 94), ("clair", 92), ("avowed", 80)] {
            games
                .iter_mut()
                .find(|g| g.id == id)
                .unwrap()
                .metacritic_score = Some(score);
        }
        let sorted = |sort| ids(&apply(&games, LibraryFilter::All, &by_sort(sort)));

        let by_name = sorted(LibrarySort::Name);
        assert_eq!(by_name[..4], ["first-light", "avowed", "bf6", "bloodborne"]); // "007" vem antes das letras
        let recent = sorted(LibrarySort::RecentlyPlayed);
        assert_eq!(recent[0], "control");
        assert_eq!(recent[recent.len() - 3..], ["avowed", "ds2", "phantom"]); // nunca jogados, por nome
        assert_eq!(sorted(LibrarySort::PlayTime)[0], "forza5");
        assert_eq!(
            sorted(LibrarySort::Rating)[..3],
            ["ragnarok", "clair", "avowed"]
        ); // sem nota no fim
        assert_eq!(sorted(LibrarySort::RecentlyAdded)[0], "control");
    }

    #[test]
    fn name_sort_ignores_case_and_accents() {
        let mut titles = ["Écho", "eagle", "Zelda", "ábaco"];
        titles.sort_by(|a, b| title_cmp(a, b));
        assert_eq!(titles, ["ábaco", "eagle", "Écho", "Zelda"]);
    }

    #[test]
    fn facets_list_present_sources_and_genres_with_counts() {
        let games = sample::games();
        let sources = sources(&games, |g| LibraryFacts::of(g));
        let genres = genres(&games, |g| LibraryFacts::of(g));

        let names: Vec<_> = sources.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Adicionado", "Epic Games", "GOG", "Steam", "Xbox"]);
        assert_eq!(
            sources.iter().find(|s| s.name == "Steam").unwrap().count,
            10
        );
        assert_eq!(sources.iter().map(|s| s.count).sum::<usize>(), games.len());
        assert!(genres.iter().any(|g| g.name == "RPG" && g.count == 5));
        assert!(
            genres
                .windows(2)
                .all(|w| title_cmp(&w[0].name, &w[1].name) != Ordering::Greater)
        );
    }

    #[test]
    fn generic_apply_filters_any_item_type_with_the_same_rules() {
        let items = [("Pokémon", true), ("Pikmin", false), ("Zelda", true)];
        let facts = |i: &&(&str, bool)| LibraryFacts {
            title: i.0.into(),
            is_installed: i.1,
            source: "Playnite".into(),
            ..Default::default()
        };
        let result = apply_by(items.iter(), facts, &with_search("POKE", LibrarySort::Name));
        let installed = apply_by(
            items.iter(),
            facts,
            &LibraryCriteria {
                install: InstallFilter::Installed,
                sort: LibrarySort::Name,
                ..Default::default()
            },
        );
        assert_eq!(result.iter().map(|i| i.0).collect::<Vec<_>>(), ["Pokémon"]);
        assert_eq!(
            installed.iter().map(|i| i.0).collect::<Vec<_>>(),
            ["Pokémon", "Zelda"]
        );
    }
}
