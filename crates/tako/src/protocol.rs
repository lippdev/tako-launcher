//! Conversa entre o supervisor e a interface (ADR-0026): uma linha JSON por mensagem na saída e na entrada
//! padrão do processo da interface. Os canais anônimos são só dos dois processos; nada fica exposto a
//! outros usuários.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::time::Duration;
use tako_ui::session::TrackRequest;

/// Interface → supervisor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiMsg {
    Heartbeat {
        seq: u64,
    },
    /// O jogo abriu; o supervisor acompanha. A interface desktop permanece minimizada (ADR-0027).
    Track {
        req: TrackRequest,
    },
    /// Voltar ao jogo (a interface minimiza no desktop ou fecha na tela cheia).
    Resume,
    /// Central › Encerrar jogo.
    EndGame,
    /// "Fechando o jogo…" › Forçar encerramento.
    ForceEndGame,
    /// Central › Ponteiro.
    Pointer {
        hide: bool,
        mouse: bool,
    },
    /// A interface vai sair de propósito: `update` = o supervisor também sai.
    Closing {
        reason: String,
    },
    /// A interface abriu ou trocou de modo (ADR-0027): o supervisor reabre nele e, no desktop, não esconde o
    /// controle do jogo.
    Mode {
        big_picture: bool,
    },
}

/// Supervisor → interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SupMsg {
    /// O jogo em segundo plano terminou (a interface estava aberta).
    GameEnded {
        game_id: String,
        played_secs: u64,
        /// [`tako_core::session::GameEnd::key`]; ausente (supervisor antigo) = normal.
        #[serde(default)]
        end: String,
        /// Código de saída do jogo, quando o Tako abriu o executável direto; ausente (supervisor antigo) = sem.
        #[serde(default)]
        exit_code: Option<u32>,
    },
    /// Alguém abriu o Tako de novo: trazer a janela para a frente.
    Activate,
    OpenLink {
        url: String,
    },
    /// A janela do jogo apareceu: sai de "Abrindo…" e minimiza no desktop ou fecha na tela cheia.
    GameShown,
}

pub fn write<T: Serialize>(out: &mut impl Write, msg: &T) -> std::io::Result<()> {
    let mut line = serde_json::to_string(msg).map_err(std::io::Error::other)?;
    line.push('\n');
    out.write_all(line.as_bytes())?;
    out.flush()
}

/// Lê mensagens até o fim do canal; linhas inválidas são ignoradas (dado de outro processo).
pub fn read_lines<T: for<'de> Deserialize<'de>>(input: impl BufRead, mut on: impl FnMut(T)) {
    for line in input.lines() {
        let Ok(line) = line else { return };
        if line.len() > 64 * 1024 {
            continue;
        }
        match serde_json::from_str::<T>(&line) {
            Ok(m) => on(m),
            Err(e) => tracing::debug!("mensagem ignorada: {e}"),
        }
    }
}

pub const HEARTBEAT: Duration = Duration::from_secs(2);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut buf = Vec::new();
        write(
            &mut buf,
            &UiMsg::Pointer {
                hide: true,
                mouse: false,
            },
        )
        .unwrap();
        write(&mut buf, &UiMsg::Resume).unwrap();
        buf.extend_from_slice(b"lixo\n");
        let mut got = Vec::new();
        read_lines::<UiMsg>(buf.as_slice(), |m| got.push(m));
        assert_eq!(
            got,
            vec![
                UiMsg::Pointer {
                    hide: true,
                    mouse: false
                },
                UiMsg::Resume
            ]
        );
    }

    #[test]
    fn desktop_lifecycle_and_central_messages_round_trip() {
        let messages = [
            UiMsg::Mode { big_picture: false },
            UiMsg::Mode { big_picture: true },
            UiMsg::Resume,
            UiMsg::EndGame,
            UiMsg::ForceEndGame,
        ];
        let mut buf = Vec::new();
        for msg in &messages {
            write(&mut buf, msg).unwrap();
        }
        let mut got = Vec::new();
        read_lines::<UiMsg>(buf.as_slice(), |m| got.push(m));
        assert_eq!(got, messages);
        let messages = [
            SupMsg::GameShown,
            SupMsg::Activate,
            SupMsg::GameEnded {
                game_id: "manual:teste".into(),
                played_secs: 120,
                end: "normal".into(),
                exit_code: Some(0),
            },
        ];
        let mut buf = Vec::new();
        for msg in &messages {
            write(&mut buf, msg).unwrap();
        }
        let mut got = Vec::new();
        read_lines::<SupMsg>(buf.as_slice(), |m| got.push(m));
        assert_eq!(got, messages);
    }

    #[test]
    fn game_ended_carries_the_exit_code_and_old_supervisors_still_work() {
        let msg = SupMsg::GameEnded {
            game_id: "g".into(),
            played_secs: 3,
            end: "quick".into(),
            exit_code: Some(0xC000_0135),
        };
        let mut buf = Vec::new();
        write(&mut buf, &msg).unwrap();
        // Supervisor antigo: sem `exit_code` (nem `end`).
        buf.extend_from_slice(b"{\"type\":\"game_ended\",\"game_id\":\"g\",\"played_secs\":3}\n");
        let mut got = Vec::new();
        read_lines::<SupMsg>(buf.as_slice(), |m| got.push(m));
        assert_eq!(
            got,
            vec![
                msg,
                SupMsg::GameEnded {
                    game_id: "g".into(),
                    played_secs: 3,
                    end: String::new(),
                    exit_code: None,
                }
            ]
        );
    }
}
