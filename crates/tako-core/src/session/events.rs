//! Eventos de sessão (EXT-001): quem quiser saber quando um jogo abre e fecha (API local do EXT-002, plugins do
//! EXT-003) assina aqui. Nada sai do processo: cada assinante recebe uma cópia num canal só dele.
//!
//! Por processo: `GameStarting` e `GameStarted` saem de quem abre o jogo (a interface) e `GameExited` de quem o
//! acompanha (o supervisor, ou a interface sem ele). Juntar os dois lados pelo IPC fica para quem consumir.

use crate::library::GameStore;
use chrono::{DateTime, Utc};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

/// O jogo da sessão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionGame {
    pub game_id: String,
    pub title: String,
    pub store: GameStore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// O usuário mandou jogar; antes das ferramentas, do comando de antes e do jogo.
    GameStarting {
        game: SessionGame,
        at: DateTime<Utc>,
    },
    /// O jogo foi aberto.
    GameStarted {
        game: SessionGame,
        at: DateTime<Utc>,
    },
    /// O jogo terminou. `exit_code` só quando o Tako abriu o executável direto.
    GameExited {
        game: SessionGame,
        at: DateTime<Utc>,
        played: Duration,
        exit_code: Option<u32>,
    },
}

static SUBSCRIBERS: Mutex<Vec<Sender<SessionEvent>>> = Mutex::new(Vec::new());

/// Um canal novo que recebe os eventos daqui em diante. Soltar o `Receiver` cancela a assinatura.
pub fn subscribe() -> Receiver<SessionEvent> {
    let (tx, rx) = channel();
    SUBSCRIBERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(tx);
    rx
}

/// Entrega a todos os assinantes sem esperar (canal sem limite): a sessão nunca trava por causa de quem ouve.
pub fn publish(event: SessionEvent) {
    tracing::debug!("evento de sessão: {event:?}");
    SUBSCRIBERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .retain(|s| s.send(event.clone()).is_ok());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: &str) -> SessionGame {
        SessionGame {
            game_id: id.into(),
            title: "Hades".into(),
            store: GameStore::Steam,
        }
    }

    #[test]
    fn every_subscriber_gets_the_events_and_a_dropped_one_is_forgotten() {
        let a = subscribe();
        let b = subscribe();
        let at = Utc::now();
        // Outros testes podem publicar ao mesmo tempo: só os deste id contam.
        let mine = |rx: &Receiver<SessionEvent>| {
            rx.try_iter()
                .filter(|e| match e {
                    SessionEvent::GameStarting { game, .. }
                    | SessionEvent::GameStarted { game, .. }
                    | SessionEvent::GameExited { game, .. } => game.game_id == "ev:1",
                })
                .collect::<Vec<_>>()
        };
        publish(SessionEvent::GameStarting {
            game: game("ev:1"),
            at,
        });
        drop(b);
        let exited = SessionEvent::GameExited {
            game: game("ev:1"),
            at,
            played: Duration::from_secs(90),
            exit_code: Some(0),
        };
        publish(exited.clone());
        assert_eq!(
            mine(&a),
            vec![
                SessionEvent::GameStarting {
                    game: game("ev:1"),
                    at
                },
                exited
            ]
        );
    }
}
