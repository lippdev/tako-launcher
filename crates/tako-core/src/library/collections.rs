//! Port de `Library/Collections.cs` (UX-003): coleções criadas pelo usuário, usadas como fileiras e filtros.

use super::query::{title_cmp, title_eq};
use super::{Clock, Game};
use crate::i18n::Lang;
use crate::presentation::tr;
use chrono::{DateTime, Utc};
use std::collections::BTreeSet;

/// Coleção do usuário: um nome e os jogos dela (por id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionInfo {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub game_ids: BTreeSet<String>,
}

impl CollectionInfo {
    pub fn contains(&self, game_id: &str) -> bool {
        self.game_ids.contains(game_id)
    }
}

pub const MAX_NAME_LENGTH: usize = 40;

/// Por que o nome não serve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    Empty,
    TooLong,
    Duplicate,
}

impl NameProblem {
    pub fn message(self, lang: Lang) -> String {
        match self {
            NameProblem::Empty => tr(lang, "library.collection.name_empty", &[]),
            NameProblem::TooLong => tr(
                lang,
                "library.collection.name_too_long",
                &[("n", &MAX_NAME_LENGTH.to_string())],
            ),
            NameProblem::Duplicate => tr(lang, "library.collection.name_taken", &[]),
        }
    }
}

/// Nome limpo (espaços nas pontas e repetidos saem).
pub fn clean_name(name: &str) -> String {
    name.split(' ')
        .filter(|w| !w.trim().is_empty())
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Problema do nome, ou `None` quando serve. `except_id` é a coleção sendo renomeada (pode manter o nome).
pub fn name_problem(
    name: &str,
    existing: &[CollectionInfo],
    except_id: Option<&str>,
) -> Option<NameProblem> {
    let clean = clean_name(name);
    if clean.is_empty() {
        Some(NameProblem::Empty)
    } else if clean.chars().count() > MAX_NAME_LENGTH {
        Some(NameProblem::TooLong)
    } else if existing
        .iter()
        .any(|c| Some(c.id.as_str()) != except_id && title_eq(&c.name, &clean))
    {
        Some(NameProblem::Duplicate)
    } else {
        None
    }
}

/// Onde as coleções ficam guardadas (o app usa SQLite; ver `persistence`).
pub trait CollectionStore {
    fn load_all(&mut self) -> Result<Vec<CollectionInfo>, String>;
    fn insert(&mut self, collection: &CollectionInfo) -> Result<(), String>;
    fn rename(&mut self, id: &str, name: &str) -> Result<(), String>;
    fn delete(&mut self, id: &str) -> Result<(), String>;
    fn add_game(
        &mut self,
        collection_id: &str,
        game_id: &str,
        at: DateTime<Utc>,
    ) -> Result<(), String>;
    fn remove_game(&mut self, collection_id: &str, game_id: &str) -> Result<(), String>;
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CollectionError {
    #[error("nome de coleção inválido: {0:?}")]
    Name(NameProblem),
    #[error("coleção '{0}' não existe")]
    NotFound(String),
    #[error("não foi possível gravar as coleções: {0}")]
    Store(String),
}

/// Coleções em memória, gravadas no [`CollectionStore`] a cada mudança.
pub struct CollectionLibrary {
    store: Box<dyn CollectionStore + Send>,
    clock: Clock,
    collections: Vec<CollectionInfo>,
    version: u64,
}

impl CollectionLibrary {
    pub fn new(store: Box<dyn CollectionStore + Send>, clock: Clock) -> CollectionLibrary {
        CollectionLibrary {
            store,
            clock,
            collections: Vec::new(),
            version: 0,
        }
    }

    pub fn load(&mut self) -> Result<(), CollectionError> {
        self.collections = self.store.load_all().map_err(CollectionError::Store)?;
        sort(&mut self.collections);
        Ok(())
    }

    /// Em ordem de nome.
    pub fn collections(&self) -> &[CollectionInfo] {
        &self.collections
    }

    /// Sobe a cada mudança (o evento `Changed` do C#).
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn find(&self, id: &str) -> Option<&CollectionInfo> {
        self.collections.iter().find(|c| c.id == id)
    }

    /// Coleções que têm o jogo.
    pub fn of(&self, game_id: &str) -> Vec<&CollectionInfo> {
        self.collections
            .iter()
            .filter(|c| c.contains(game_id))
            .collect()
    }

    /// Ids das coleções de cada jogo, para os filtros da biblioteca.
    pub fn apply_to(&self, games: &mut [Game]) {
        for game in games {
            game.collections = self
                .collections
                .iter()
                .filter(|c| c.contains(&game.id))
                .map(|c| c.id.clone())
                .collect();
        }
    }

    /// Cria a coleção (opcionalmente já com um jogo).
    pub fn create(
        &mut self,
        name: &str,
        first_game_id: Option<&str>,
    ) -> Result<CollectionInfo, CollectionError> {
        let clean = self.valid(name, None)?;
        let now = (self.clock)();
        let collection = CollectionInfo {
            id: uuid::Uuid::new_v4().simple().to_string(),
            name: clean,
            created_at: now,
            game_ids: first_game_id.map(str::to_owned).into_iter().collect(),
        };
        self.store
            .insert(&collection)
            .map_err(CollectionError::Store)?;
        if let Some(game) = first_game_id {
            self.store
                .add_game(&collection.id, game, now)
                .map_err(CollectionError::Store)?;
        }
        self.replace(&collection.id.clone(), Some(collection.clone()));
        Ok(collection)
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), CollectionError> {
        let collection = self
            .find(id)
            .cloned()
            .ok_or_else(|| CollectionError::NotFound(id.to_owned()))?;
        let clean = self.valid(name, Some(id))?;
        self.store
            .rename(id, &clean)
            .map_err(CollectionError::Store)?;
        self.replace(
            id,
            Some(CollectionInfo {
                name: clean,
                ..collection
            }),
        );
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> Result<(), CollectionError> {
        self.store.delete(id).map_err(CollectionError::Store)?;
        self.replace(id, None);
        Ok(())
    }

    /// Põe ou tira o jogo da coleção. Devolve `true` quando o jogo entrou.
    pub fn toggle(&mut self, collection_id: &str, game_id: &str) -> Result<bool, CollectionError> {
        let mut collection = self
            .find(collection_id)
            .cloned()
            .ok_or_else(|| CollectionError::NotFound(collection_id.to_owned()))?;
        let added = collection.game_ids.insert(game_id.to_owned());
        if added {
            self.store.add_game(collection_id, game_id, (self.clock)())
        } else {
            collection.game_ids.remove(game_id);
            self.store.remove_game(collection_id, game_id)
        }
        .map_err(CollectionError::Store)?;
        self.replace(collection_id, Some(collection));
        Ok(added)
    }

    fn valid(&self, name: &str, except_id: Option<&str>) -> Result<String, CollectionError> {
        match name_problem(name, &self.collections, except_id) {
            Some(p) => Err(CollectionError::Name(p)),
            None => Ok(clean_name(name)),
        }
    }

    fn replace(&mut self, id: &str, collection: Option<CollectionInfo>) {
        self.collections.retain(|c| c.id != id);
        self.collections.extend(collection);
        sort(&mut self.collections);
        self.version += 1;
    }
}

fn sort(collections: &mut [CollectionInfo]) {
    collections.sort_by(|a, b| title_cmp(&a.name, &b.name).then(a.created_at.cmp(&b.created_at)));
}

#[cfg(test)]
mod tests {
    use super::super::{LibraryCriteria, LibraryFilter, LibrarySort, query, sample, system_clock};
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Armazenamento em memória compartilhado, para "reabrir" como se fosse o disco.
    #[derive(Clone, Default)]
    struct Memory(Arc<Mutex<HashMap<String, CollectionInfo>>>);

    impl CollectionStore for Memory {
        fn load_all(&mut self) -> Result<Vec<CollectionInfo>, String> {
            Ok(self.0.lock().unwrap().values().cloned().collect())
        }
        fn insert(&mut self, c: &CollectionInfo) -> Result<(), String> {
            self.0.lock().unwrap().insert(
                c.id.clone(),
                CollectionInfo {
                    game_ids: BTreeSet::new(),
                    ..c.clone()
                },
            );
            Ok(())
        }
        fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .get_mut(id)
                .ok_or("sem coleção")?
                .name = name.into();
            Ok(())
        }
        fn delete(&mut self, id: &str) -> Result<(), String> {
            self.0.lock().unwrap().remove(id);
            Ok(())
        }
        fn add_game(&mut self, c: &str, g: &str, _: DateTime<Utc>) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .get_mut(c)
                .ok_or("sem coleção")?
                .game_ids
                .insert(g.into());
            Ok(())
        }
        fn remove_game(&mut self, c: &str, g: &str) -> Result<(), String> {
            self.0
                .lock()
                .unwrap()
                .get_mut(c)
                .ok_or("sem coleção")?
                .game_ids
                .remove(g);
            Ok(())
        }
    }

    fn collection(id: &str, name: &str) -> CollectionInfo {
        CollectionInfo {
            id: id.into(),
            name: name.into(),
            created_at: DateTime::UNIX_EPOCH,
            game_ids: BTreeSet::new(),
        }
    }

    fn open(store: &Memory) -> CollectionLibrary {
        let mut lib = CollectionLibrary::new(Box::new(store.clone()), system_clock());
        lib.load().unwrap();
        lib
    }

    #[test]
    fn collection_names_are_checked() {
        let existing = [collection("1", "RPG"), collection("2", "acao")];
        for (name, problem) in [
            ("", Some("Dê um nome à coleção.")),
            ("   ", Some("Dê um nome à coleção.")),
            ("rpg", Some("Já existe uma coleção com esse nome.")),
            ("Ação", Some("Já existe uma coleção com esse nome.")), // sem acento/caixa: "acao"
            (
                "Para jogar com a família no sábado à noite",
                Some("Use até 40 letras."),
            ),
            ("  Corrida   rápida ", None),
        ] {
            assert_eq!(
                name_problem(name, &existing, None)
                    .map(|p| p.message(Lang::PtBr))
                    .as_deref(),
                problem,
                "{name}"
            );
        }
        assert_eq!(
            NameProblem::TooLong.message(Lang::EnUs),
            "Use up to 40 characters."
        );
    }

    #[test]
    fn renaming_may_keep_the_same_name() {
        let existing = [collection("1", "RPG")];
        assert_eq!(name_problem("rpg", &existing, Some("1")), None);
        assert_eq!(clean_name("  Corrida   rápida "), "Corrida rápida");
    }

    #[test]
    fn collections_are_created_renamed_filled_and_deleted_in_the_store() {
        let store = Memory::default();
        let mut library = open(&store);
        let start = library.version();

        let rpg = library.create("  RPG ", Some("steam:570")).unwrap();
        let racing = library.create("Corrida", None).unwrap();
        assert!(library.toggle(&racing.id, "steam:730").unwrap());
        assert!(library.toggle(&racing.id, "steam:570").unwrap());
        assert!(!library.toggle(&racing.id, "steam:730").unwrap()); // tirou
        library.rename(&rpg.id, "Aventura").unwrap();
        assert_eq!(
            library.create("corrida", None),
            Err(CollectionError::Name(NameProblem::Duplicate))
        );
        assert_eq!(
            library.rename(&rpg.id, " "),
            Err(CollectionError::Name(NameProblem::Empty))
        );
        assert!(matches!(
            library.toggle("nao-existe", "x"),
            Err(CollectionError::NotFound(_))
        ));

        let mut reopened = open(&store);
        let names: Vec<_> = reopened
            .collections()
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, ["Aventura", "Corrida"]); // em ordem de nome
        assert_eq!(
            reopened.find(&rpg.id).unwrap().game_ids,
            BTreeSet::from(["steam:570".to_owned()])
        );
        assert_eq!(
            reopened.find(&racing.id).unwrap().game_ids,
            BTreeSet::from(["steam:570".to_owned()])
        );
        assert_eq!(reopened.of("steam:570").len(), 2);
        assert!(reopened.of("steam:730").is_empty());
        assert_eq!(library.version() - start, 6);

        reopened.delete(&racing.id).unwrap();
        let third = open(&store);
        assert_eq!(
            third
                .collections()
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            ["Aventura"]
        );
    }

    #[test]
    fn collections_filter_the_library_with_the_other_filters() {
        let mut games = sample::games();
        let mut library = open(&Memory::default());
        let couch = library.create("Sofá", Some(&games[0].id)).unwrap();
        library.toggle(&couch.id, &games[1].id).unwrap();
        let solo = library.create("Sozinho", Some(&games[2].id)).unwrap();
        library.apply_to(&mut games);

        let criteria = LibraryCriteria {
            sort: LibrarySort::Name,
            ..Default::default()
        };
        let only_couch = query::apply(
            &games,
            LibraryFilter::All,
            &criteria.toggle_collection(&couch.id),
        );
        let either = query::apply(
            &games,
            LibraryFilter::All,
            &criteria
                .toggle_collection(&couch.id)
                .toggle_collection(&solo.id),
        );

        let mut got: Vec<_> = only_couch.iter().map(|g| g.id.clone()).collect();
        got.sort();
        let mut want = vec![games[0].id.clone(), games[1].id.clone()];
        want.sort();
        assert_eq!(got, want);
        assert_eq!(either.len(), 3);
        assert_eq!(
            criteria.toggle_collection(&couch.id).active_filter_count(),
            1
        );
        assert!(
            criteria
                .toggle_collection(&couch.id)
                .clear_filters()
                .collections
                .is_empty()
        );
        assert!(
            criteria
                .toggle_collection("apagada")
                .keep_collections([couch.id.as_str()])
                .collections
                .is_empty()
        );
        assert!(games[0].collections.contains(&couch.id));
        assert!(games[3].collections.is_empty());
    }
}
