//! Protocolo e tipos de integração com o Console Mode (ADR-0025, épico CMI).
//!
//! Mensagens JSON com prefixo de tamanho de 4 bytes little-endian em socket
//! ou named pipe local restrito ao usuário.

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

/// Versão canônica da API do Console Mode suportada pelo Tako.
pub const CURRENT_API_VERSION: u32 = 1;

/// Tamanho máximo permitido para um quadro de aperto de mão (64 KiB).
pub const MAX_HANDSHAKE_FRAME_BYTES: usize = 64 * 1024;


/// Resultado tipado da detecção do Console Mode (CMI-002).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "estado", rename_all = "snake_case")]
pub enum ConsoleModeStatus {
    NaoInstalado,
    Instalado {
        versao_api: u32,
        capacidades: Vec<String>,
    },
    Incompativel {
        versao: String,
    },
    Erro,
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("tamanho do quadro excede o limite permitido: {0} bytes")]
    FrameTooLarge(usize),
    #[error("cabeçalho de tamanho do quadro incompleto")]
    IncompleteHeader,
    #[error("corpo da mensagem incompleto")]
    IncompleteBody,
    #[error("mensagem JSON malformada ou inesperada: {0}")]
    Malformed(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

/// Mensagem enviada pelo Tako no aperto de mão (`hello`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloClient {
    pub r#type: String,
    pub client: String,
    pub api: u32,
}

/// Resposta esperada do Console Mode no aperto de mão (`hello`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelloServer {
    pub r#type: String,
    pub server: String,
    pub api: u32,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}


/// Codifica a mensagem `hello` do cliente com prefixo de 4 bytes little-endian.
pub fn encode_hello_frame(api: u32) -> Result<Vec<u8>, ProtocolError> {
    let msg = HelloClient {
        r#type: "hello".into(),
        client: "tako".into(),
        api,
    };
    let json = serde_json::to_vec(&msg).map_err(|e| ProtocolError::Malformed(e.to_string()))?;
    let len = u32::try_from(json.len()).map_err(|_| ProtocolError::FrameTooLarge(json.len()))?;
    let mut frame = Vec::with_capacity(4 + json.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&json);
    Ok(frame)
}

/// Decodifica e valida um quadro de aperto de mão recebido do Console Mode.
pub fn parse_hello_response(bytes: &[u8]) -> Result<ConsoleModeStatus, ProtocolError> {
    if bytes.len() < 4 {
        return Err(ProtocolError::IncompleteHeader);
    }
    let Ok(arr) = bytes[0..4].try_into() else {
        return Err(ProtocolError::IncompleteHeader);
    };
    let len = u32::from_le_bytes(arr) as usize;
    if len > MAX_HANDSHAKE_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge(len));
    }
    if bytes.len() < 4 + len {
        return Err(ProtocolError::IncompleteBody);
    }
    let body = &bytes[4..4 + len];
    let resp: HelloServer =
        serde_json::from_slice(body).map_err(|e| ProtocolError::Malformed(e.to_string()))?;

    if resp.r#type != "hello" || resp.server != "consolemode" {
        return Err(ProtocolError::Malformed(
            "tipo ou servidor inválido no aperto de mão".into(),
        ));
    }

    if resp.api == CURRENT_API_VERSION {
        Ok(ConsoleModeStatus::Instalado {
            versao_api: resp.api,
            capacidades: resp.capabilities,
        })
    } else {
        let versao = if resp.version.is_empty() {
            format!("API v{}", resp.api)
        } else {
            resp.version
        };
        Ok(ConsoleModeStatus::Incompativel { versao })
    }
}


/// Executa o aperto de mão com o Console Mode através de um fluxo `Read + Write`.
pub fn perform_handshake<S: Read + Write>(stream: &mut S) -> ConsoleModeStatus {
    let frame = match encode_hello_frame(CURRENT_API_VERSION) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("erro ao codificar quadro de aperto de mão: {e}");
            return ConsoleModeStatus::Erro;
        }
    };

    if let Err(e) = stream.write_all(&frame) {
        tracing::warn!("erro ao enviar aperto de mão para o Console Mode: {e}");
        return ConsoleModeStatus::Erro;
    }
    if let Err(e) = stream.flush() {
        tracing::warn!("erro ao descarregar envio do aperto de mão: {e}");
        return ConsoleModeStatus::Erro;
    }

    let mut header = [0u8; 4];
    if let Err(e) = stream.read_exact(&mut header) {
        tracing::warn!("erro ao ler cabeçalho de resposta do Console Mode: {e}");
        return ConsoleModeStatus::Erro;
    }
    let len = u32::from_le_bytes(header) as usize;
    if len > MAX_HANDSHAKE_FRAME_BYTES {
        tracing::warn!("resposta do Console Mode excede o limite: {len} bytes");
        return ConsoleModeStatus::Erro;
    }

    let mut buffer = vec![0u8; 4 + len];
    buffer[0..4].copy_from_slice(&header);
    if let Err(e) = stream.read_exact(&mut buffer[4..]) {
        tracing::warn!("erro ao ler corpo da resposta do Console Mode: {e}");
        return ConsoleModeStatus::Erro;
    }

    match parse_hello_response(&buffer) {
        Ok(status) => status,
        Err(e) => {
            tracing::warn!("resposta do Console Mode rejeitada: {e}");
            ConsoleModeStatus::Erro
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct TimeoutMockStream;

    impl Read for TimeoutMockStream {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "tempo limite esgotado para leitura",
            ))
        }
    }

    impl Write for TimeoutMockStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct MockSocketStream {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl MockSocketStream {
        fn new(response_payload: &[u8]) -> Self {
            let len = u32::try_from(response_payload.len()).unwrap();
            let mut data = Vec::with_capacity(4 + response_payload.len());
            data.extend_from_slice(&len.to_le_bytes());
            data.extend_from_slice(response_payload);
            Self {
                input: Cursor::new(data),
                output: Vec::new(),
            }
        }
    }

    impl Read for MockSocketStream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl Write for MockSocketStream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn encode_hello_frame_has_valid_length_prefix_and_json() {
        let frame = encode_hello_frame(1).unwrap();
        assert!(frame.len() >= 4);
        let len = u32::from_le_bytes(frame[0..4].try_into().unwrap()) as usize;
        assert_eq!(len, frame.len() - 4);
        let parsed: HelloClient = serde_json::from_slice(&frame[4..]).unwrap();
        assert_eq!(parsed.r#type, "hello");
        assert_eq!(parsed.client, "tako");
        assert_eq!(parsed.api, 1);
    }

    #[test]
    fn parse_hello_response_installed_success() {
        let json = br#"{"type":"hello","server":"consolemode","api":1,"version":"1.2.0","capabilities":["tv","fps","windows","console-session"]}"#;
        let mut frame = Vec::new();
        frame.extend_from_slice(&(json.len() as u32).to_le_bytes());
        frame.extend_from_slice(json);

        let status = parse_hello_response(&frame).unwrap();
        assert_eq!(
            status,
            ConsoleModeStatus::Instalado {
                versao_api: 1,
                capacidades: vec![
                    "tv".into(),
                    "fps".into(),
                    "windows".into(),
                    "console-session".into()
                ],
            }
        );
    }

    #[test]
    fn parse_hello_response_incompatible_api_version() {
        let json = br#"{"type":"hello","server":"consolemode","api":2,"version":"2.0.0","capabilities":["tv"]}"#;
        let mut frame = Vec::new();
        frame.extend_from_slice(&(json.len() as u32).to_le_bytes());
        frame.extend_from_slice(json);

        let status = parse_hello_response(&frame).unwrap();
        assert_eq!(
            status,
            ConsoleModeStatus::Incompativel {
                versao: "2.0.0".into(),
            }
        );
    }

    #[test]
    fn parse_hello_response_rejects_wrong_server_or_type() {
        let json = br#"{"type":"welcome","server":"other","api":1,"version":"1.0.0"}"#;
        let mut frame = Vec::new();
        frame.extend_from_slice(&(json.len() as u32).to_le_bytes());
        frame.extend_from_slice(json);

        assert!(parse_hello_response(&frame).is_err());
    }

    #[test]
    fn perform_handshake_with_mock_stream_installed() {
        let response = br#"{"type":"hello","server":"consolemode","api":1,"version":"1.0.0","capabilities":["tv","fps"]}"#;
        let mut stream = MockSocketStream::new(response);
        let status = perform_handshake(&mut stream);

        assert_eq!(
            status,
            ConsoleModeStatus::Instalado {
                versao_api: 1,
                capacidades: vec!["tv".into(), "fps".into()],
            }
        );
        // Garante que o Tako enviou a saudação esperada
        assert!(stream.output.len() >= 4);
        let client_hello: HelloClient = serde_json::from_slice(&stream.output[4..]).unwrap();
        assert_eq!(client_hello.client, "tako");
        assert_eq!(client_hello.api, 1);
    }

    #[test]
    fn perform_handshake_with_mock_stream_incompatible() {
        let response = br#"{"type":"hello","server":"consolemode","api":3,"version":"3.1.0","capabilities":[]}"#;
        let mut stream = MockSocketStream::new(response);
        let status = perform_handshake(&mut stream);

        assert_eq!(
            status,
            ConsoleModeStatus::Incompativel {
                versao: "3.1.0".into(),
            }
        );
    }

    #[test]
    fn perform_handshake_timeout_returns_error() {
        let mut stream = TimeoutMockStream;
        let status = perform_handshake(&mut stream);
        assert_eq!(status, ConsoleModeStatus::Erro);
    }


}
