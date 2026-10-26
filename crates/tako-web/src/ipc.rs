//! Transporte entre o processo do núcleo (`tako --web`) e o da interface (`tako --web-ui`), WEB-02b.
//!
//! Uma linha JSON por mensagem: o núcleo escreve na entrada padrão do filho e lê a saída padrão dele. Os pipes
//! anônimos só existem entre os dois processos (como o supervisor do Slint, ADR-0026); nenhuma porta de rede
//! nem pipe nomeado fica exposto a outro processo. O texto da página viaja dentro de uma string JSON, então uma
//! quebra de linha escrita pela página não quebra o enquadramento.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Read, Write};
use tako_app::protocol::{MAX_MESSAGE_BYTES, WindowAction};

/// Argumento do processo da interface. Só o núcleo o abre, com [`CORE_PID_ENV`] definida. A variável só evita
/// rodar a interface à mão por engano; quem garante que só o núcleo fala com ela são os pipes herdados.
pub const UI_ARG: &str = "--web-ui";

/// PID do núcleo, posto pelo núcleo no ambiente do filho.
pub const CORE_PID_ENV: &str = "TAKO_WEB_CORE_PID";

/// Maior linha aceita da interface: um pedido de até 1 MiB com escapes JSON (até 6 bytes por caractere).
pub const MAX_UI_LINE: usize = 6 * MAX_MESSAGE_BYTES + 64 * 1024;

/// Maior linha aceita do núcleo (respostas e eventos em lote). O núcleo é o pai e é confiável; o limite só
/// impede que um defeito esgote a memória da interface.
pub const MAX_CORE_LINE: usize = 64 * 1024 * 1024;

/// Uma página da interface: navegador, frame principal e geração do contexto JavaScript. O núcleo só devolve
/// o que recebeu; quem confere é o processo da interface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub browser: i32,
    pub frame: String,
    pub generation: String,
}

/// Interface → núcleo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiMsg {
    /// Texto de `takoHost.post`, já validado pela interface (frame principal, origem, até 1 MiB).
    Request { page: Page, text: String },
    /// Contexto JavaScript novo (página carregada ou recarregada).
    Page { page: Page },
    /// Navegador fechado (inclui DevTools).
    Closed { browser: i32 },
    /// A última janela fechou sem pedido do núcleo (X, `window.action` close): o app termina.
    UserClosed,
}

/// Núcleo → interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreMsg {
    /// Resposta e eventos, nessa ordem, e ações de janela para uma página.
    Deliver {
        page: Page,
        messages: Vec<String>,
        actions: Vec<WindowAction>,
    },
    /// Fechar a interface (jogo, bandeja, teste). Sem resposta: o núcleo vê o processo terminar.
    Close,
}

pub fn write_message<T: Serialize>(out: &mut impl Write, message: &T) -> io::Result<()> {
    let mut line = serde_json::to_vec(message).map_err(io::Error::other)?;
    line.push(b'\n');
    out.write_all(&line)?;
    out.flush()
}

/// Lê mensagens até o fim do canal ou até `on` devolver `false`. Linha acima de `limit` é descartada sem
/// guardar o excesso; linha inválida é ignorada (veio de outro processo).
pub fn read_messages<T: DeserializeOwned>(
    mut input: impl BufRead,
    limit: usize,
    mut on: impl FnMut(T) -> bool,
) -> io::Result<()> {
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = (&mut input)
            .take(limit as u64 + 1)
            .read_until(b'\n', &mut line)?;
        if read == 0 {
            return Ok(());
        }
        if line.last() != Some(&b'\n') {
            if line.len() <= limit {
                // Fim do canal no meio de uma linha: o outro lado caiu.
                return Ok(());
            }
            tracing::warn!(limit, "transporte: linha grande demais descartada");
            skip_line(&mut input)?;
            continue;
        }
        match serde_json::from_slice::<T>(&line) {
            Ok(message) => {
                if !on(message) {
                    return Ok(());
                }
            }
            Err(error) => tracing::debug!("transporte: mensagem ignorada: {error}"),
        }
    }
}

fn skip_line(input: &mut impl BufRead) -> io::Result<()> {
    loop {
        let buffer = input.fill_buf()?;
        if buffer.is_empty() {
            return Ok(());
        }
        if let Some(end) = buffer.iter().position(|&b| b == b'\n') {
            input.consume(end + 1);
            return Ok(());
        }
        let length = buffer.len();
        input.consume(length);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Page {
        Page {
            browser: 1,
            frame: "F".into(),
            generation: "10:0".into(),
        }
    }

    #[test]
    fn page_text_with_line_breaks_keeps_the_framing() {
        let messages = [
            UiMsg::Request {
                page: page(),
                text: "{\n\"v\":1,\r\n\"id\":2,\"name\":\"app.info\"}\u{2028}".into(),
            },
            UiMsg::Page { page: page() },
            UiMsg::Closed { browser: 3 },
            UiMsg::UserClosed,
        ];
        let mut wire = Vec::new();
        for message in &messages {
            write_message(&mut wire, message).unwrap();
        }
        assert_eq!(wire.iter().filter(|&&b| b == b'\n').count(), messages.len());
        let mut got = Vec::new();
        read_messages::<UiMsg>(wire.as_slice(), MAX_UI_LINE, |m| {
            got.push(m);
            true
        })
        .unwrap();
        assert_eq!(got, messages);
    }

    #[test]
    fn core_messages_round_trip_in_order() {
        let messages = [
            CoreMsg::Deliver {
                page: page(),
                messages: vec![r#"{"v":1,"id":1,"ok":true}"#.into(), "evento\n".into()],
                actions: vec![WindowAction::ToggleMaximize],
            },
            CoreMsg::Close,
        ];
        let mut wire = Vec::new();
        for message in &messages {
            write_message(&mut wire, message).unwrap();
        }
        let mut got = Vec::new();
        read_messages::<CoreMsg>(wire.as_slice(), MAX_CORE_LINE, |m| {
            got.push(m);
            true
        })
        .unwrap();
        assert_eq!(got, messages);
    }

    #[test]
    fn oversized_broken_and_truncated_lines_are_skipped() {
        let mut wire = Vec::new();
        wire.extend_from_slice(b"lixo\n");
        wire.extend_from_slice(&[b'x'; 300]);
        wire.push(b'\n');
        write_message(&mut wire, &UiMsg::UserClosed).unwrap();
        wire.extend_from_slice(br#"{"type":"closed","#);
        let mut got = Vec::new();
        read_messages::<UiMsg>(wire.as_slice(), 64, |m| {
            got.push(m);
            true
        })
        .unwrap();
        assert_eq!(got, [UiMsg::UserClosed]);
    }

    #[test]
    fn largest_page_message_fits_the_line_limit() {
        let text = "\u{1}".repeat(MAX_MESSAGE_BYTES);
        let line = serde_json::to_vec(&UiMsg::Request { page: page(), text }).unwrap();
        assert!(line.len() <= MAX_UI_LINE, "{}", line.len());
    }
}
