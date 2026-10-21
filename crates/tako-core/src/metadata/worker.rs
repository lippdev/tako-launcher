//! Fila de metadados em segundo plano (port da rodada de `MetadataService.cs`): uma thread própria,
//! um jogo a cada 1,5 s (limite da loja Steam), pausável, resultados por `mpsc`.

use super::GameMetadata;
use super::agent::{MetadataAgent, PinnedIds};
use super::cache::MetadataCache;
use crate::library::Game;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

/// Espaço entre jogos consultados (~200 consultas a cada 5 minutos na Steam).
pub const SPACING: Duration = Duration::from_millis(1500);

/// Metadados novos de um jogo, já gravados no cache.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataUpdated {
    pub game_id: String,
    pub metadata: GameMetadata,
}

enum Command {
    Refresh {
        games: Vec<Game>,
        force: bool,
        pinned: HashMap<String, PinnedIds>,
    },
    Pause,
    Resume,
    Stop,
}

pub struct MetadataWorker {
    tx: Sender<Command>,
    progress: Arc<[AtomicUsize; 2]>,
}

impl MetadataWorker {
    /// Sobe a thread. `spacing` = [`SPACING`] (os testes usam zero).
    pub fn spawn(
        agent: MetadataAgent,
        cache: MetadataCache,
        spacing: Duration,
    ) -> (MetadataWorker, Receiver<MetadataUpdated>) {
        let (tx, rx) = mpsc::channel();
        let (out, results) = mpsc::channel();
        let progress = Arc::new([AtomicUsize::new(0), AtomicUsize::new(0)]);
        let shared = progress.clone();
        let spawned = std::thread::Builder::new()
            .name("tako-metadata".into())
            .spawn(move || {
                Loop {
                    agent,
                    cache,
                    spacing,
                    rx,
                    out,
                    paused: false,
                    pending: None,
                    progress: shared,
                }
                .run();
            });
        if let Err(e) = spawned {
            tracing::error!("Metadados: não foi possível iniciar a fila: {e}");
        }
        (MetadataWorker { tx, progress }, results)
    }

    /// Atualiza os jogos sem dados ou com dados velhos (`force` = todos). `pinned` = ids fixados
    /// por jogo ("Fontes e IDs"). Durante uma rodada, a nova entra quando ela termina.
    pub fn refresh(&self, games: Vec<Game>, force: bool, pinned: HashMap<String, PinnedIds>) {
        let _ = self.tx.send(Command::Refresh {
            games,
            force,
            pinned,
        });
    }

    /// Jogos já consultados e total da rodada em curso (AtividadeNaBarra); `feitos == total` = parada.
    pub fn progress(&self) -> (usize, usize) {
        (
            self.progress[0].load(Ordering::Relaxed),
            self.progress[1].load(Ordering::Relaxed),
        )
    }

    /// Para depois do jogo atual (ex.: durante uma partida); [`resume`](Self::resume) continua.
    pub fn pause(&self) {
        let _ = self.tx.send(Command::Pause);
    }

    pub fn resume(&self) {
        let _ = self.tx.send(Command::Resume);
    }
}

impl Drop for MetadataWorker {
    fn drop(&mut self) {
        // Sem `join`: uma consulta em andamento (até o tempo-limite) não segura quem fecha.
        let _ = self.tx.send(Command::Stop);
    }
}

type Round = (Vec<Game>, bool, HashMap<String, PinnedIds>);

struct Loop {
    agent: MetadataAgent,
    cache: MetadataCache,
    spacing: Duration,
    rx: Receiver<Command>,
    out: Sender<MetadataUpdated>,
    paused: bool,
    pending: Option<Round>,
    /// Feitos e total da rodada, lidos por [`MetadataWorker::progress`].
    progress: Arc<[AtomicUsize; 2]>,
}

impl Loop {
    fn run(mut self) {
        loop {
            let round = match self.pending.take() {
                Some(r) if !self.paused => r,
                other => {
                    self.pending = other;
                    match self.rx.recv() {
                        Ok(c) => {
                            if self.handle(c) {
                                continue;
                            }
                            return;
                        }
                        Err(_) => return,
                    }
                }
            };
            if !self.round(round) {
                return;
            }
        }
    }

    /// Falso = parar.
    fn handle(&mut self, c: Command) -> bool {
        match c {
            Command::Refresh {
                games,
                force,
                pinned,
            } => {
                // Rodada nova pedida: junta com a pendente (força vale se qualquer uma pediu).
                let force = force || self.pending.as_ref().is_some_and(|p| p.1);
                self.pending = Some((games, force, pinned));
            }
            Command::Pause => self.paused = true,
            Command::Resume => self.paused = false,
            Command::Stop => return false,
        }
        true
    }

    /// Recebe comandos até `deadline` (e enquanto pausado). Falso = parar.
    fn wait(&mut self, deadline: Instant) -> bool {
        loop {
            let timeout = if self.paused {
                Duration::from_secs(3600)
            } else {
                let now = Instant::now();
                if now >= deadline {
                    return true;
                }
                deadline - now
            };
            match self.rx.recv_timeout(timeout) {
                Ok(c) => {
                    if !self.handle(c) {
                        return false;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return false,
            }
        }
    }

    fn round(&mut self, (games, force, pinned): Round) -> bool {
        let none = PinnedIds::new();
        let pins = |id: &str| pinned.get(id).unwrap_or(&none);
        let due: Vec<Game> = games
            .into_iter()
            .filter(|g| {
                self.agent
                    .needs_update(g, self.cache.read(&g.id).as_ref(), force, pins(&g.id))
            })
            .collect();
        if due.is_empty() {
            return true;
        }
        // O andamento vai antes do lote do IGDB: a pílula aparece já no começo da rodada.
        self.progress[0].store(0, Ordering::Relaxed);
        self.progress[1].store(due.len(), Ordering::Relaxed);
        // IGDB: a biblioteca inteira em poucas consultas em lote, antes de ir jogo a jogo.
        self.agent.prefetch(&due);
        for game in &due {
            if !self.wait(Instant::now()) {
                return false;
            }
            let cached = self.cache.read(&game.id);
            let update = self
                .agent
                .update(game, cached.as_ref(), force, pins(&game.id));
            if let Some(m) = update.metadata {
                if m.is_empty() {
                    tracing::info!(title = %game.title, sources = %m.source, "Metadados: nenhuma correspondência");
                }
                if let Err(e) = self.cache.write(&game.id, &m) {
                    tracing::warn!(game = %game.id, "Metadados: não foi possível gravar o cache: {e}");
                }
                let _ = self.out.send(MetadataUpdated {
                    game_id: game.id.clone(),
                    metadata: m,
                });
            }
            self.progress[0].fetch_add(1, Ordering::Relaxed);
            if update.queried && !self.wait(Instant::now() + self.spacing) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Lang;
    use crate::library::GameStore;
    use crate::metadata::agent::tests::{FakeProvider, game};
    use crate::metadata::test_support::{TestClock, now};
    use std::sync::Arc;

    fn steam_meta(id: &str) -> Option<GameMetadata> {
        Some(GameMetadata {
            steam_app_id: id.parse().ok(),
            source_ids: [("Steam".to_owned(), id.to_owned())].into(),
            title: Some("Jogo".into()),
            ..GameMetadata::new("Steam")
        })
    }

    #[test]
    fn updates_due_games_in_the_background_and_writes_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let steam = Arc::new(FakeProvider::with("Steam", Box::new(steam_meta)).exact("Steam"));
        let mut agent = MetadataAgent::new(vec![steam.clone()], Arc::new(TestClock::at(now())));
        agent.lang = Some(Lang::PtBr);
        let (worker, results) =
            MetadataWorker::spawn(agent, MetadataCache::new(dir.path()), Duration::ZERO);
        let games = vec![
            game(GameStore::Steam, "A", Some(1)),
            Game {
                id: "steam:2".into(),
                ..game(GameStore::Steam, "B", Some(2))
            },
        ];
        worker.refresh(games.clone(), false, HashMap::new());
        let mut got: Vec<String> = (0..2)
            .map(|_| {
                results
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                    .game_id
            })
            .collect();
        got.sort();
        assert_eq!(got, ["steam:1", "steam:2"]);
        assert!(MetadataCache::new(dir.path()).read("steam:2").is_some());
        // O andamento da rodada (AtividadeNaBarra) termina com todos feitos.
        let start = Instant::now();
        while worker.progress() != (2, 2) && start.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(worker.progress(), (2, 2));

        // Cache em dia: a próxima rodada não consulta ninguém.
        worker.refresh(games, false, HashMap::new());
        assert!(results.recv_timeout(Duration::from_millis(300)).is_err());
        assert_eq!(steam.fetched().len(), 2);
    }

    #[test]
    fn paused_worker_waits_for_resume() {
        let dir = tempfile::tempdir().unwrap();
        let steam = Arc::new(FakeProvider::with("Steam", Box::new(steam_meta)).exact("Steam"));
        let agent = MetadataAgent::new(vec![steam], Arc::new(TestClock::at(now())));
        let (worker, results) =
            MetadataWorker::spawn(agent, MetadataCache::new(dir.path()), Duration::ZERO);
        worker.pause();
        worker.refresh(
            vec![game(GameStore::Steam, "A", Some(1))],
            false,
            HashMap::new(),
        );
        assert!(results.recv_timeout(Duration::from_millis(300)).is_err());
        worker.resume();
        assert_eq!(
            results
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .game_id,
            "steam:1"
        );
        drop(worker); // para a thread sem travar
    }
}
