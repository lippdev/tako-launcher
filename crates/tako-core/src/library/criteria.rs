//! Port de `Library/LibraryCriteria.cs`: busca, filtros do painel e ordenação da biblioteca (UX-001/002/004).
//! Imutável: cada mudança no painel gera um novo critério. Dentro de fontes, gêneros ou coleções vale qualquer um
//! dos marcados; entre filtros diferentes, todos precisam valer.

use super::query::LibrarySort;
use super::search_text::{Query, normalize};
use super::{Game, PlayStatus};
use crate::i18n;
use crate::presentation::game_text;
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;
use std::time::Duration;

/// Filtro de instalação do painel de filtros.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InstallFilter {
    #[default]
    Any,
    Installed,
    NotInstalled,
}

/// O que os filtros e a ordenação precisam saber de um item. Serve aos jogos do Tako ([`LibraryFacts::of`]) e aos
/// que o adaptador de temas Playnite mostra, para os dois usarem a mesma regra.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LibraryFacts {
    pub title: String,
    pub is_installed: bool,
    pub is_favorite: bool,
    pub is_broken: bool,
    /// Fonte como aparece para o usuário ("Steam", "Adicionado").
    pub source: String,
    pub genres: Vec<String>,
    pub last_played_at: Option<DateTime<Utc>>,
    pub play_time: Duration,
    /// Nota de 0 a 100 (Metacritic), quando existe.
    pub score: Option<i32>,
    pub added_at: Option<DateTime<Utc>>,
    /// Ids das coleções do usuário com o jogo (só os jogos do Tako têm).
    pub collections: BTreeSet<String>,
    /// Tags do usuário (LIB-002; só os jogos do Tako têm).
    pub tags: BTreeSet<String>,
    pub play_status: Option<PlayStatus>,
}

impl LibraryFacts {
    /// Fatos de um jogo do Tako (fonte no idioma atual da interface).
    pub fn of(game: &Game) -> LibraryFacts {
        LibraryFacts {
            title: game.title.clone(),
            is_installed: game.is_installed,
            is_favorite: game.is_favorite,
            is_broken: game.is_broken,
            source: game_text::store_name(i18n::lang(), game.store()),
            genres: game.genres.clone(),
            last_played_at: game.last_played_at,
            play_time: game.play_time,
            score: game.metacritic_score,
            added_at: game.added_at,
            collections: game.collections.clone(),
            tags: game.tags.clone(),
            play_status: game.play_status,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryCriteria {
    /// Texto da busca (sem acento e sem diferenciar maiúsculas; ver [`super::search_text`]).
    pub search: String,
    pub install: InstallFilter,
    pub favorites_only: bool,
    /// Só jogos já jogados (a mesma regra da fileira "Jogados recentemente" da Início).
    pub played_only: bool,
    /// Só jogos adicionados cujo arquivo ou pasta sumiu (LIB-004).
    pub broken_only: bool,
    /// Valores comparados sem diferenciar maiúsculas.
    pub sources: BTreeSet<String>,
    pub genres: BTreeSet<String>,
    /// Ids de coleções do usuário (UX-003): o jogo precisa estar em alguma das marcadas.
    pub collections: BTreeSet<String>,
    /// Tags marcadas (LIB-002): o jogo precisa ter alguma delas.
    pub tags: BTreeSet<String>,
    /// Status marcados (LIB-001): o jogo precisa ter algum deles.
    pub statuses: BTreeSet<PlayStatus>,
    pub sort: LibrarySort,
}

/// `OrdinalIgnoreCase` do C#.
fn same(a: &str, b: &str) -> bool {
    a == b || a.to_lowercase() == b.to_lowercase()
}

fn has(set: &BTreeSet<String>, value: &str) -> bool {
    set.iter().any(|v| same(v, value))
}

fn toggle(set: &BTreeSet<String>, value: &str) -> BTreeSet<String> {
    let mut copy = set.clone();
    if value.trim().is_empty() {
        return copy;
    }
    let before = copy.len();
    copy.retain(|v| !same(v, value));
    if copy.len() == before {
        copy.insert(value.to_owned());
    }
    copy
}

impl LibraryCriteria {
    pub fn has_search(&self) -> bool {
        !self.search.trim().is_empty()
    }

    /// Quantos filtros do painel estão ligados (a busca e a ordenação não contam).
    pub fn active_filter_count(&self) -> usize {
        usize::from(self.install != InstallFilter::Any)
            + usize::from(self.favorites_only)
            + usize::from(self.played_only)
            + usize::from(self.broken_only)
            + self.sources.len()
            + self.genres.len()
            + self.collections.len()
            + self.tags.len()
            + self.statuses.len()
    }

    pub fn has_filters(&self) -> bool {
        self.active_filter_count() > 0
    }

    pub fn has_source(&self, source: &str) -> bool {
        has(&self.sources, source)
    }

    pub fn has_genre(&self, genre: &str) -> bool {
        has(&self.genres, genre)
    }

    pub fn has_collection(&self, id: &str) -> bool {
        has(&self.collections, id)
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        has(&self.tags, tag)
    }

    pub fn toggle_tag(&self, tag: &str) -> Self {
        Self {
            tags: toggle(&self.tags, tag),
            ..self.clone()
        }
    }

    pub fn matches(&self, game: &LibraryFacts) -> bool {
        self.matches_query(game, &Query::parse(&self.search))
    }

    /// Versão para listas: a busca já normalizada uma vez.
    pub fn matches_query(&self, game: &LibraryFacts, query: &Query) -> bool {
        match self.install {
            InstallFilter::Installed if !game.is_installed => return false,
            InstallFilter::NotInstalled if game.is_installed => return false,
            _ => {}
        }
        (!self.favorites_only || game.is_favorite)
            && (!self.played_only || game.last_played_at.is_some())
            && (!self.broken_only || game.is_broken)
            && (self.sources.is_empty() || self.has_source(&game.source))
            && (self.genres.is_empty() || game.genres.iter().any(|g| self.has_genre(g)))
            && (self.collections.is_empty()
                || game.collections.iter().any(|c| self.has_collection(c)))
            && (self.tags.is_empty() || game.tags.iter().any(|t| self.has_tag(t)))
            && (self.statuses.is_empty()
                || game.play_status.is_some_and(|s| self.statuses.contains(&s)))
            && query.matches(&normalize(&game.title))
    }

    /// Todos → Instalados → Não instalados → Todos.
    pub fn next_install(&self) -> Self {
        let install = match self.install {
            InstallFilter::Any => InstallFilter::Installed,
            InstallFilter::Installed => InstallFilter::NotInstalled,
            InstallFilter::NotInstalled => InstallFilter::Any,
        };
        Self {
            install,
            ..self.clone()
        }
    }

    pub fn toggle_source(&self, source: &str) -> Self {
        Self {
            sources: toggle(&self.sources, source),
            ..self.clone()
        }
    }

    pub fn toggle_genre(&self, genre: &str) -> Self {
        Self {
            genres: toggle(&self.genres, genre),
            ..self.clone()
        }
    }

    pub fn toggle_collection(&self, collection_id: &str) -> Self {
        Self {
            collections: toggle(&self.collections, collection_id),
            ..self.clone()
        }
    }

    pub fn toggle_status(&self, status: PlayStatus) -> Self {
        let mut statuses = self.statuses.clone();
        if !statuses.remove(&status) {
            statuses.insert(status);
        }
        Self {
            statuses,
            ..self.clone()
        }
    }

    /// Tira coleções que não existem mais (ex.: excluída com o filtro ligado).
    pub fn keep_collections<'a>(&self, existing_ids: impl IntoIterator<Item = &'a str>) -> Self {
        let existing: Vec<&str> = existing_ids.into_iter().collect();
        let collections = self
            .collections
            .iter()
            .filter(|c| existing.iter().any(|e| same(e, c)))
            .cloned()
            .collect();
        Self {
            collections,
            ..self.clone()
        }
    }

    /// Desliga os filtros do painel; a busca e a ordenação continuam.
    pub fn clear_filters(&self) -> Self {
        Self {
            search: self.search.clone(),
            sort: self.sort,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clearing_filters_keeps_search_and_sort() {
        let criteria = LibraryCriteria {
            search: "god".into(),
            sort: LibrarySort::Rating,
            favorites_only: true,
            played_only: true,
            ..Default::default()
        }
        .next_install()
        .toggle_source("Steam")
        .toggle_genre("Ação");
        assert_eq!(criteria.active_filter_count(), 5);

        let cleared = criteria.clear_filters();
        assert!(!cleared.has_filters());
        assert_eq!(cleared.search, "god");
        assert_eq!(cleared.sort, LibrarySort::Rating);
    }

    #[test]
    fn status_filter_keeps_games_with_any_marked_status() {
        let facts = |s| LibraryFacts {
            play_status: s,
            ..Default::default()
        };
        let c = LibraryCriteria::default()
            .toggle_status(PlayStatus::Playing)
            .toggle_status(PlayStatus::Paused);
        assert_eq!(c.active_filter_count(), 2);
        assert!(c.matches(&facts(Some(PlayStatus::Paused))));
        assert!(!c.matches(&facts(Some(PlayStatus::Beaten))));
        assert!(!c.matches(&facts(None)));
        let c = c
            .toggle_status(PlayStatus::Playing)
            .toggle_status(PlayStatus::Paused);
        assert!(c.matches(&facts(None)));
    }

    #[test]
    fn tag_filter_keeps_games_with_any_marked_tag() {
        let facts = |tags: &[&str]| LibraryFacts {
            tags: tags.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        };
        let c = LibraryCriteria::default()
            .toggle_tag("Coop")
            .toggle_tag("Sofá");
        assert_eq!(c.active_filter_count(), 2);
        assert!(c.matches(&facts(&["sofá"])));
        assert!(!c.matches(&facts(&["Solo"])));
        assert!(!c.matches(&facts(&[])));
        assert!(c.toggle_tag("coop").toggle_tag("Sofá").matches(&facts(&[])));
    }

    #[test]
    fn blank_toggle_and_missing_collections_are_ignored() {
        let c = LibraryCriteria::default()
            .toggle_source("  ")
            .toggle_collection("apagada");
        assert!(c.sources.is_empty());
        assert!(c.keep_collections(["outra"]).collections.is_empty());
        assert!(c.keep_collections(["APAGADA"]).has_collection("apagada"));
    }
}
