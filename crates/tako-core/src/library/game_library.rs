//! Port de `IGameLibrary`, `LocalGameLibrary` e `InMemoryGameLibrary` (MVP-023): jogos encontrados nas lojas mais
//! os adicionados pelo usuário, com favorito, "jogado por último" e tempo guardados num [`LibraryStore`].

use super::{
    Clock, Game, GameStateRecord, GameStore, ManualGameRecord, PlaySession, PlayStatus, duplicates,
};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Sessões guardadas por jogo: as mais antigas saem para a tabela não crescer sem fim.
pub const MAX_SESSIONS: usize = 100;

/// O que fica guardado da biblioteca: jogos adicionados e estado por jogo.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LibraryData {
    pub manual: Vec<ManualGameRecord>,
    pub state: HashMap<String, GameStateRecord>,
}

/// Onde a biblioteca do usuário fica guardada (o app usa SQLite; ver `persistence`).
pub trait LibraryStore {
    /// Dado corrompido não é erro: recomeça vazio (quem implementa guarda uma cópia).
    fn load(&mut self) -> LibraryData;
    /// Grava tudo de forma atômica.
    fn save(&mut self, data: &LibraryData) -> Result<(), String>;
}

/// Guarda só em memória (testes e demonstração).
#[derive(Debug, Clone, Default)]
pub struct MemoryLibraryStore(pub LibraryData);

impl LibraryStore for MemoryLibraryStore {
    fn load(&mut self) -> LibraryData {
        self.0.clone()
    }

    fn save(&mut self, data: &LibraryData) -> Result<(), String> {
        self.0 = data.clone();
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LibraryError {
    #[error("jogo '{0}' não está na biblioteca")]
    NotFound(String),
    #[error("jogo '{0}' não foi adicionado manualmente")]
    NotManual(String),
    #[error("não foi possível gravar a biblioteca: {0}")]
    Store(String),
}

/// Procura os jogos instalados (normalmente as fontes de `tako-sources`).
pub type Discover = Box<dyn FnMut() -> Vec<Game> + Send>;
/// Completa o jogo com metadados em cache (descrição, gêneros, arte de jogos manuais).
pub type Enrich = Box<dyn Fn(&mut Game) + Send + Sync>;
/// Mudança no estado de um jogo durante uma importação (recebe o estado atual, se houver).
pub type StateChange = Box<dyn FnOnce(Option<GameStateRecord>) -> GameStateRecord>;

pub struct GameLibrary {
    discover: Discover,
    store: Box<dyn LibraryStore + Send>,
    clock: Clock,
    enrich: Option<Enrich>,
    data: LibraryData,
    games: Vec<Game>,
    version: u64,
}

impl GameLibrary {
    /// Biblioteca vazia; chame [`GameLibrary::load`] para ler o armazenamento e procurar os jogos.
    pub fn new(
        discover: Discover,
        store: Box<dyn LibraryStore + Send>,
        clock: Clock,
    ) -> GameLibrary {
        GameLibrary {
            discover,
            store,
            clock,
            enrich: None,
            data: LibraryData::default(),
            games: Vec::new(),
            version: 0,
        }
    }

    /// Biblioteca fixa em memória, já carregada (testes e demonstração).
    pub fn in_memory(games: Vec<Game>, clock: Clock) -> GameLibrary {
        let mut lib = GameLibrary::new(
            Box::new(move || games.clone()),
            Box::new(MemoryLibraryStore::default()),
            clock,
        );
        lib.load();
        lib
    }

    pub fn with_enrich(mut self, enrich: Enrich) -> Self {
        self.enrich = Some(enrich);
        self
    }

    pub fn games(&self) -> &[Game] {
        &self.games
    }

    /// Para quem completa os jogos depois da leitura (ex.: coleções em `Game::collections`).
    pub fn games_mut(&mut self) -> &mut [Game] {
        &mut self.games
    }

    /// Sobe a cada mudança (o evento `Changed` do C#): a UI compara com o que já mostrou.
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn find(&self, id: &str) -> Option<&Game> {
        self.games.iter().find(|g| g.id == id)
    }

    pub fn find_manual(&self, id: &str) -> Option<&ManualGameRecord> {
        self.data.manual.iter().find(|m| m.id == id)
    }

    /// Lê o armazenamento e procura os jogos instalados.
    pub fn load(&mut self) {
        self.data = self.store.load();
        self.rebuild();
    }

    /// Procura jogos instalados de novo.
    pub fn refresh(&mut self) {
        self.rebuild();
        self.version += 1;
    }

    pub fn toggle_favorite(&mut self, id: &str) -> Result<(), LibraryError> {
        let game = self.find_mut(id)?;
        game.is_favorite = !game.is_favorite;
        let favorite = game.is_favorite;
        self.update_state(id, |s| s.is_favorite = favorite)
    }

    /// Muda o status do jogo (LIB-001); `None` tira o status.
    pub fn set_status(&mut self, id: &str, status: Option<PlayStatus>) -> Result<(), LibraryError> {
        self.find_mut(id)?.play_status = status;
        self.update_state(id, |s| s.play_status = status)
    }

    /// Registra o fim de uma sessão: o jogo vai para o topo de "Jogados recentemente" e a sessão entra no
    /// histórico (duração zero é só a abertura, sem sessão).
    pub fn record_session(&mut self, id: &str, duration: Duration) -> Result<(), LibraryError> {
        let now = (self.clock)();
        let game = self.find_mut(id)?;
        game.last_played_at = Some(now);
        game.play_time += duration;
        let started_at = now - chrono::TimeDelta::from_std(duration).unwrap_or_default();
        self.update_state(id, |s| {
            s.last_played_at = Some(now);
            s.play_minutes += duration.as_secs_f64() / 60.0;
            if !duration.is_zero() {
                s.sessions.push(PlaySession {
                    started_at,
                    duration,
                });
                let extra = s.sessions.len().saturating_sub(MAX_SESSIONS);
                s.sessions.drain(..extra);
            }
        })
    }

    /// Sessões do jogo, da mais antiga para a mais recente.
    pub fn sessions(&self, id: &str) -> &[PlaySession] {
        self.data.state.get(id).map_or(&[], |s| &s.sessions)
    }

    /// O jogo foi aberto pelo launcher agora (a duração chega com o acompanhamento da sessão).
    pub fn record_launch(&mut self, id: &str) -> Result<(), LibraryError> {
        self.record_session(id, Duration::ZERO)
    }

    pub fn add_manual(&mut self, record: ManualGameRecord) -> Result<&Game, LibraryError> {
        let id = record.id.clone();
        self.data.manual.push(record);
        self.save()?;
        self.refresh();
        self.find(&id).ok_or(LibraryError::NotFound(id))
    }

    pub fn update_manual(&mut self, record: ManualGameRecord) -> Result<(), LibraryError> {
        let slot = self
            .data
            .manual
            .iter_mut()
            .find(|m| m.id == record.id)
            .ok_or_else(|| LibraryError::NotManual(record.id.clone()))?;
        *slot = record;
        self.save()?;
        self.refresh();
        Ok(())
    }

    pub fn remove_manual(&mut self, id: &str) -> Result<(), LibraryError> {
        self.data.manual.retain(|m| m.id != id);
        self.data.state.remove(id);
        self.save()?;
        self.refresh();
        Ok(())
    }

    /// Importação em lote (STORE-009): adiciona os jogos e muda o estado dos indicados, gravando tudo de uma vez.
    /// Jogo já adicionado com o mesmo id é ignorado.
    pub fn import(
        &mut self,
        added: Vec<ManualGameRecord>,
        state: Vec<(String, StateChange)>,
    ) -> Result<(), LibraryError> {
        for record in added {
            if !self.data.manual.iter().any(|m| m.id == record.id) {
                self.data.manual.push(record);
            }
        }
        for (id, change) in state {
            let next = change(self.data.state.remove(&id));
            self.data.state.insert(id, next);
        }
        self.save()?;
        self.refresh();
        Ok(())
    }

    fn find_mut(&mut self, id: &str) -> Result<&mut Game, LibraryError> {
        self.games
            .iter_mut()
            .find(|g| g.id == id)
            .ok_or_else(|| LibraryError::NotFound(id.to_owned()))
    }

    fn update_state(
        &mut self,
        id: &str,
        change: impl FnOnce(&mut GameStateRecord),
    ) -> Result<(), LibraryError> {
        change(self.data.state.entry(id.to_owned()).or_default());
        self.version += 1;
        self.save()
    }

    fn save(&mut self) -> Result<(), LibraryError> {
        self.store.save(&self.data).map_err(LibraryError::Store)
    }

    fn rebuild(&mut self) {
        let mut seen = HashSet::new();
        let mut games = Vec::new();
        let discovered = (self.discover)();
        for mut game in discovered
            .into_iter()
            .chain(self.data.manual.iter().map(ManualGameRecord::to_game))
        {
            if !seen.insert(game.id.clone()) {
                continue;
            }
            if let Some(state) = self.data.state.get(&game.id) {
                game.is_favorite = state.is_favorite;
                game.play_status = state.play_status;
                if game.last_played_at.is_none() || state.last_played_at > game.last_played_at {
                    game.last_played_at = state.last_played_at;
                }
                // A Steam já soma o próprio tempo; o registrado pelo launcher vale para os demais.
                if game.store() != GameStore::Steam {
                    game.play_time += Duration::from_secs_f64(state.play_minutes.max(0.0) * 60.0);
                }
            }
            games.push(game);
        }
        // Mesmo jogo em duas lojas: itens separados, ligados (STORE-010).
        duplicates::link(&mut games);
        if let Some(enrich) = &self.enrich {
            games.iter_mut().for_each(enrich);
        }
        self.games = games;
    }
}

#[cfg(test)]
mod tests {
    use super::super::duplicates::store_game;
    use super::super::{query, sample};
    use super::*;
    use chrono::{DateTime, TimeDelta, Utc};
    use std::sync::{Arc, Mutex};

    fn clock(now: &Arc<Mutex<DateTime<Utc>>>) -> Clock {
        let now = Arc::clone(now);
        Box::new(move || *now.lock().unwrap())
    }

    #[test]
    fn recording_a_session_moves_the_game_to_the_front() {
        let now = Arc::new(Mutex::new(sample::now()));
        let mut library = GameLibrary::in_memory(sample::games(), clock(&now));
        let before = library.find("gow3").unwrap().play_time;
        *now.lock().unwrap() = sample::now() + TimeDelta::minutes(30);

        library
            .record_session("gow3", Duration::from_secs(25 * 60))
            .unwrap();

        assert_eq!(query::recently_played(library.games())[0].id, "gow3");
        assert_eq!(
            library.find("gow3").unwrap().play_time,
            before + Duration::from_secs(25 * 60)
        );
        assert_eq!(
            library.sessions("gow3"),
            [PlaySession {
                started_at: sample::now() + TimeDelta::minutes(5),
                duration: Duration::from_secs(25 * 60),
            }]
        );
        library.record_launch("gow3").unwrap(); // abrir não conta como sessão
        for _ in 0..MAX_SESSIONS {
            library
                .record_session("gow3", Duration::from_secs(60))
                .unwrap();
        }
        let sessions = library.sessions("gow3");
        assert_eq!(sessions.len(), MAX_SESSIONS);
        assert!(
            sessions
                .iter()
                .all(|s| s.duration == Duration::from_secs(60))
        );
        assert_eq!(
            library.record_launch("nao-existe"),
            Err(LibraryError::NotFound("nao-existe".into()))
        );
    }

    #[test]
    fn toggling_favorite_raises_changed() {
        let now = Arc::new(Mutex::new(sample::now()));
        let mut library = GameLibrary::in_memory(sample::games(), clock(&now));
        let version = library.version();
        let was = library.find("ds2").unwrap().is_favorite;

        library.toggle_favorite("ds2").unwrap();

        assert!(library.version() > version);
        assert_ne!(was, library.find("ds2").unwrap().is_favorite);
        library.refresh(); // o estado sobrevive à nova leitura
        assert_ne!(was, library.find("ds2").unwrap().is_favorite);
    }

    #[test]
    fn library_links_duplicates_and_keeps_each_store_identity() {
        let now = Arc::new(Mutex::new(sample::now()));
        let mut library = GameLibrary::new(
            Box::new(|| {
                vec![
                    store_game("steam:7", "Farol", GameStore::Steam, Some(7)),
                    store_game("epic:Farol", "Farol do Fim", GameStore::Epic, None),
                    store_game("gog:7", "FAROL", GameStore::Gog, None),
                ]
            }),
            Box::new(MemoryLibraryStore::default()),
            clock(&now),
        );
        library.load();

        assert_eq!(library.games().len(), 3);
        assert_eq!(
            library.find("gog:7").unwrap().also_installed_in,
            [GameStore::Steam]
        );
        assert!(
            library
                .find("epic:Farol")
                .unwrap()
                .also_installed_in
                .is_empty()
        );
    }

    #[test]
    fn manual_games_and_import_are_saved_and_survive_a_reload() {
        let now = Arc::new(Mutex::new(sample::now()));
        let store = Arc::new(Mutex::new(MemoryLibraryStore::default()));
        struct Shared(Arc<Mutex<MemoryLibraryStore>>);
        impl LibraryStore for Shared {
            fn load(&mut self) -> LibraryData {
                self.0.lock().unwrap().load()
            }
            fn save(&mut self, data: &LibraryData) -> Result<(), String> {
                self.0.lock().unwrap().save(data)
            }
        }
        let open = || {
            let mut lib = GameLibrary::new(
                Box::new(|| vec![store_game("steam:1", "Steam", GameStore::Steam, Some(1))]),
                Box::new(Shared(Arc::clone(&store))),
                clock(&now),
            );
            lib.load();
            lib
        };

        let mut library = open();
        let record = ManualGameRecord::create("Meu Jogo", "steam://rungameid/9", sample::now());
        let id = record.id.clone();
        library.add_manual(record.clone()).unwrap();
        library
            .record_session(&id, Duration::from_secs(600))
            .unwrap();
        library
            .import(
                vec![record.clone()],
                vec![(
                    "steam:1".into(),
                    Box::new(|s: Option<GameStateRecord>| GameStateRecord {
                        is_favorite: true,
                        play_minutes: 99.0,
                        ..s.unwrap_or_default()
                    }) as StateChange,
                )],
            )
            .unwrap();
        assert_eq!(
            library.update_manual(ManualGameRecord {
                id: "x".into(),
                ..record.clone()
            }),
            Err(LibraryError::NotManual("x".into()))
        );

        let reopened = open();
        assert_eq!(reopened.games().len(), 2); // import não duplica
        assert_eq!(
            reopened.find(&id).unwrap().play_time,
            Duration::from_secs(600)
        );
        let steam = reopened.find("steam:1").unwrap();
        assert!(steam.is_favorite);
        assert_eq!(steam.play_time, Duration::ZERO); // a Steam soma o próprio tempo

        let mut reopened = reopened;
        reopened.remove_manual(&id).unwrap();
        assert!(reopened.find(&id).is_none() && reopened.find_manual(&id).is_none());
    }
}
