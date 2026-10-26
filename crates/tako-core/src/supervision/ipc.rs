//! Protocolo do IPC local entre a UI e o supervisor (port de `ConsoleMode.Runtime/Ipc/{IpcFraming,
//! IpcProtocol,IpcEnvelope}.cs` e do handshake do `IpcConnection.cs`): quadros
//! `[tamanho i32 little-endian][JSON UTF-8]` com cabeçalho de versão, sobre qualquer `Read + Write`.
//! O socket restrito ao usuário (named pipe, socket Unix) e os prazos de leitura ficam com quem chama.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

/// Versão mais nova que este código fala. Mudou o contrato de forma incompatível: incremente.
pub const CURRENT_VERSION: u32 = 1;
/// Versão mais antiga que este código ainda aceita.
pub const MINIMUM_VERSION: u32 = 1;
/// Limite de um quadro: o outro lado é dado não confiável.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// Códigos de erro estáveis (o texto é só para diagnóstico).
pub mod codes {
    pub const VERSION_MISMATCH: &str = "version_mismatch";
    pub const INVALID_MESSAGE: &str = "invalid_message";
    pub const METHOD_NOT_FOUND: &str = "method_not_found";
    pub const HANDLER_FAILED: &str = "handler_failed";
}

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    /// O outro lado fechou no meio de um quadro ou do handshake.
    #[error("{0}")]
    Disconnected(String),
    /// Quadro fora do formato, grande demais ou fora de ordem.
    #[error("{0}")]
    Protocol(String),
    #[error("{0}")]
    VersionMismatch(String),
    /// O outro lado recusou ou respondeu com erro.
    #[error("{code}: {message}")]
    Remote { code: String, message: String },
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageKind {
    /// Cliente → servidor: faixa de versões e identificação.
    Hello,
    /// Servidor → cliente: versão escolhida.
    Welcome,
    /// Servidor → cliente: conexão recusada; o servidor fecha em seguida.
    Reject,
    /// Comando que espera resposta (`id` obrigatório).
    Request,
    /// Resposta a um comando, com o mesmo `id`.
    Response,
    /// Notificação sem resposta.
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

/// Envelope de toda mensagem. `v` é o cabeçalho de versão: no handshake, a maior versão de quem envia;
/// depois, sempre a negociada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    pub kind: MessageKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

impl Envelope {
    pub fn new(v: u32, kind: MessageKind) -> Self {
        Envelope {
            v,
            kind,
            id: None,
            method: None,
            payload: None,
            error: None,
        }
    }

    pub fn event<T: Serialize>(v: u32, method: &str, payload: &T) -> Result<Self, IpcError> {
        Ok(Envelope {
            method: Some(method.to_owned()),
            payload: Some(to_value(payload)?),
            ..Self::new(v, MessageKind::Event)
        })
    }

    pub fn request<T: Serialize>(
        v: u32,
        id: i64,
        method: &str,
        payload: &T,
    ) -> Result<Self, IpcError> {
        Ok(Envelope {
            id: Some(id),
            method: Some(method.to_owned()),
            payload: Some(to_value(payload)?),
            ..Self::new(v, MessageKind::Request)
        })
    }

    pub fn response<T: Serialize>(v: u32, id: i64, payload: &T) -> Result<Self, IpcError> {
        Ok(Envelope {
            id: Some(id),
            payload: Some(to_value(payload)?),
            ..Self::new(v, MessageKind::Response)
        })
    }

    pub fn error_response(v: u32, id: i64, code: &str, message: &str) -> Self {
        Envelope {
            id: Some(id),
            error: Some(ErrorInfo {
                code: code.to_owned(),
                message: message.to_owned(),
            }),
            ..Self::new(v, MessageKind::Response)
        }
    }

    /// Converte a carga; ausente ou nula = `None`; formato errado é erro de protocolo.
    pub fn payload_as<T: DeserializeOwned>(&self) -> Result<Option<T>, IpcError> {
        match &self.payload {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(v) => serde_json::from_value(v.clone())
                .map(Some)
                .map_err(|e| IpcError::Protocol(format!("carga IPC não confere: {e}"))),
        }
    }
}

fn to_value<T: Serialize>(payload: &T) -> Result<serde_json::Value, IpcError> {
    serde_json::to_value(payload).map_err(|e| IpcError::Protocol(e.to_string()))
}

/// Maior versão em comum entre as faixas, ou `None` quando não há.
pub fn negotiate(local_min: u32, local_max: u32, remote_min: u32, remote_max: u32) -> Option<u32> {
    if local_min > local_max || remote_min > remote_max {
        return None;
    }
    let best = local_max.min(remote_max);
    (best >= local_min.max(remote_min)).then_some(best)
}

/// Quadro pronto para enviar (cabeçalho + JSON).
pub fn encode(envelope: &Envelope) -> Result<Vec<u8>, IpcError> {
    let json = serde_json::to_vec(envelope).map_err(|e| IpcError::Protocol(e.to_string()))?;
    let len = i32::try_from(json.len())
        .map_err(|_| IpcError::Protocol("quadro IPC grande demais".into()))?;
    let mut frame = Vec::with_capacity(4 + json.len());
    frame.extend_from_slice(&len.to_le_bytes());
    frame.extend_from_slice(&json);
    Ok(frame)
}

/// JSON inválido ou sem os campos obrigatórios vira erro de protocolo.
pub fn parse(json: &[u8]) -> Result<Envelope, IpcError> {
    serde_json::from_slice(json)
        .map_err(|e| IpcError::Protocol(format!("quadro IPC inválido: {e}")))
}

pub fn write_frame(w: &mut impl Write, envelope: &Envelope) -> Result<(), IpcError> {
    w.write_all(&encode(envelope)?)?;
    w.flush()?;
    Ok(())
}

/// Lê um quadro. `None` = o outro lado fechou entre quadros (fim limpo).
pub fn read_frame(r: &mut impl Read, max_frame_bytes: usize) -> Result<Option<Envelope>, IpcError> {
    let mut header = [0u8; 4];
    let mut read = 0;
    while read < header.len() {
        match r.read(&mut header[read..]) {
            Ok(0) if read == 0 => return Ok(None),
            Ok(0) => return Err(cut()),
            Ok(n) => read += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
    let len = i32::from_le_bytes(header);
    let size = usize::try_from(len)
        .ok()
        .filter(|n| *n > 0 && *n <= max_frame_bytes);
    let Some(size) = size else {
        return Err(IpcError::Protocol(format!(
            "quadro IPC com tamanho inválido: {len} bytes (limite {max_frame_bytes})"
        )));
    };
    let mut body = vec![0u8; size];
    r.read_exact(&mut body).map_err(|e| match e.kind() {
        io::ErrorKind::UnexpectedEof => cut(),
        _ => e.into(),
    })?;
    parse(&body).map(Some)
}

fn cut() -> IpcError {
    IpcError::Disconnected("conexão IPC encerrada no meio de um quadro".into())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub min_version: u32,
    pub max_version: u32,
    pub peer: String,
    pub process_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Welcome {
    pub version: u32,
    pub peer: String,
    pub process_id: u32,
}

/// Quem sou eu no handshake.
#[derive(Debug, Clone)]
pub struct Peer {
    pub name: String,
    pub min_version: u32,
    pub max_version: u32,
    pub process_id: u32,
}

impl Peer {
    pub fn new(name: &str) -> Self {
        Peer {
            name: name.to_owned(),
            min_version: MINIMUM_VERSION,
            max_version: CURRENT_VERSION,
            process_id: std::process::id(),
        }
    }
}

/// Conexão negociada: todo quadro depois do handshake deve ter `version` no cabeçalho.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Negotiated {
    pub version: u32,
    pub remote_peer: String,
    pub remote_process_id: u32,
}

impl Negotiated {
    /// Quadro com outra versão fecha a conexão.
    pub fn check(&self, envelope: &Envelope) -> Result<(), IpcError> {
        if envelope.v == self.version {
            Ok(())
        } else {
            Err(IpcError::Protocol(format!(
                "quadro IPC com versão {} (negociada {})",
                envelope.v, self.version
            )))
        }
    }
}

/// Lado cliente: envia `hello` e espera `welcome`.
pub fn connect<S: Read + Write>(stream: &mut S, me: &Peer) -> Result<Negotiated, IpcError> {
    let hello = Hello {
        min_version: me.min_version,
        max_version: me.max_version,
        peer: me.name.clone(),
        process_id: me.process_id,
    };
    let first = Envelope {
        payload: Some(to_value(&hello)?),
        ..Envelope::new(me.max_version, MessageKind::Hello)
    };
    write_frame(stream, &first)?;
    let reply = read_frame(stream, MAX_FRAME_BYTES)?.ok_or_else(|| {
        IpcError::Disconnected("o servidor IPC fechou a conexão durante o handshake".into())
    })?;
    match reply.kind {
        MessageKind::Welcome => {
            let w: Welcome = reply
                .payload_as()?
                .ok_or_else(|| IpcError::Protocol("welcome IPC sem carga".into()))?;
            if w.version < me.min_version || w.version > me.max_version {
                return Err(IpcError::VersionMismatch(format!(
                    "o servidor IPC escolheu a versão {}, fora da faixa {}–{} deste lado",
                    w.version, me.min_version, me.max_version
                )));
            }
            Ok(Negotiated {
                version: w.version,
                remote_peer: w.peer,
                remote_process_id: w.process_id,
            })
        }
        MessageKind::Reject => {
            let e = reply.error.unwrap_or(ErrorInfo {
                code: codes::INVALID_MESSAGE.into(),
                message: "conexão IPC recusada".into(),
            });
            if e.code == codes::VERSION_MISMATCH {
                Err(IpcError::VersionMismatch(e.message))
            } else {
                Err(IpcError::Remote {
                    code: e.code,
                    message: e.message,
                })
            }
        }
        other => Err(IpcError::Protocol(format!(
            "esperava 'welcome' no handshake IPC e chegou '{other:?}'"
        ))),
    }
}

/// Lado servidor: espera `hello`, escolhe a maior versão em comum e responde `welcome`; sem versão
/// em comum, responde `reject` e devolve [`IpcError::VersionMismatch`].
pub fn accept<S: Read + Write>(stream: &mut S, me: &Peer) -> Result<Negotiated, IpcError> {
    let first = read_frame(stream, MAX_FRAME_BYTES)?.ok_or_else(|| {
        IpcError::Disconnected("o cliente IPC fechou a conexão antes do handshake".into())
    })?;
    let hello: Option<Hello> = match first.kind {
        MessageKind::Hello => first.payload_as().unwrap_or(None),
        _ => None,
    };
    let Some(hello) = hello else {
        reject(
            stream,
            me,
            codes::INVALID_MESSAGE,
            "a primeira mensagem IPC deve ser 'hello'",
        )?;
        return Err(IpcError::Protocol(format!(
            "esperava 'hello' no handshake IPC e chegou '{:?}'",
            first.kind
        )));
    };
    let Some(version) = negotiate(
        me.min_version,
        me.max_version,
        hello.min_version,
        hello.max_version,
    ) else {
        let message = format!(
            "Versão do protocolo IPC incompatível: '{}' aceita {}–{} e '{}' aceita {}–{}. \
             Os dois processos precisam ser da mesma versão do Tako.",
            me.name,
            me.min_version,
            me.max_version,
            hello.peer,
            hello.min_version,
            hello.max_version
        );
        reject(stream, me, codes::VERSION_MISMATCH, &message)?;
        return Err(IpcError::VersionMismatch(message));
    };
    let welcome = Welcome {
        version,
        peer: me.name.clone(),
        process_id: me.process_id,
    };
    let reply = Envelope {
        payload: Some(to_value(&welcome)?),
        ..Envelope::new(version, MessageKind::Welcome)
    };
    write_frame(stream, &reply)?;
    Ok(Negotiated {
        version,
        remote_peer: hello.peer,
        remote_process_id: hello.process_id,
    })
}

fn reject<W: Write>(w: &mut W, me: &Peer, code: &str, message: &str) -> Result<(), IpcError> {
    let env = Envelope {
        error: Some(ErrorInfo {
            code: code.to_owned(),
            message: message.to_owned(),
        }),
        ..Envelope::new(me.max_version, MessageKind::Reject)
    };
    write_frame(w, &env)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::{PipeReader, PipeWriter};

    #[test]
    fn negotiation_picks_the_highest_common_version() {
        for (lmin, lmax, rmin, rmax, expected) in [
            (1, 1, 1, 1, Some(1)),
            (1, 3, 2, 5, Some(3)),
            (2, 5, 1, 3, Some(3)),
            (1, 1, 2, 3, None),
            (3, 4, 1, 2, None),
            (2, 1, 1, 2, None),
        ] {
            assert_eq!(negotiate(lmin, lmax, rmin, rmax), expected);
        }
    }

    #[test]
    fn frame_round_trips_with_version_header() {
        let env = Envelope {
            id: Some(7),
            method: Some("ping".into()),
            ..Envelope::new(1, MessageKind::Request)
        };
        let frame = encode(&env).unwrap();
        let len = i32::from_le_bytes(frame[..4].try_into().unwrap());
        assert_eq!(len as usize, frame.len() - 4);
        assert_eq!(parse(&frame[4..]).unwrap(), env);
        let json = String::from_utf8(frame[4..].to_vec()).unwrap();
        assert!(
            json.contains(r#""v":1"#) && json.contains(r#""kind":"request""#),
            "{json}"
        );
        assert!(!json.contains("payload"));
        assert_eq!(
            read_frame(&mut frame.as_slice(), MAX_FRAME_BYTES).unwrap(),
            Some(env)
        );
    }

    #[test]
    fn malformed_frames_are_protocol_errors() {
        for json in [
            "{not json",
            r#"{"v":1}"#,
            r#"{"kind":"event"}"#,
            r#"{"v":1,"kind":"shout"}"#,
            r#"{"v":-1,"kind":"event"}"#,
        ] {
            assert!(
                matches!(parse(json.as_bytes()), Err(IpcError::Protocol(_))),
                "{json}"
            );
        }
    }

    #[test]
    fn frame_length_outside_the_limit_is_rejected() {
        for len in [0i32, -1, MAX_FRAME_BYTES as i32 + 1] {
            let header = len.to_le_bytes();
            let r = read_frame(&mut header.as_slice(), MAX_FRAME_BYTES);
            assert!(matches!(r, Err(IpcError::Protocol(_))), "{len}");
        }
    }

    #[test]
    fn clean_end_of_stream_reads_as_none_and_a_cut_frame_as_disconnected() {
        assert!(
            read_frame(&mut [].as_slice(), MAX_FRAME_BYTES)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            read_frame(&mut [1u8, 0].as_slice(), MAX_FRAME_BYTES),
            Err(IpcError::Disconnected(_))
        ));
        let mut frame = encode(&Envelope::new(1, MessageKind::Event)).unwrap();
        frame.truncate(frame.len() - 2);
        assert!(matches!(
            read_frame(&mut frame.as_slice(), MAX_FRAME_BYTES),
            Err(IpcError::Disconnected(_))
        ));
    }

    #[test]
    fn payloads_round_trip_and_wrong_shapes_are_protocol_errors() {
        let beat = super::super::UiHeartbeat {
            process_id: 9,
            sequence: 3,
        };
        let env = Envelope::event(1, super::super::messages::HEARTBEAT, &beat).unwrap();
        assert_eq!(
            env.payload_as::<super::super::UiHeartbeat>().unwrap(),
            Some(beat)
        );
        let wrong = Envelope {
            payload: Some(serde_json::json!("x")),
            ..env
        };
        assert!(wrong.payload_as::<super::super::UiHeartbeat>().is_err());
        let n = Negotiated {
            version: 1,
            remote_peer: "x".into(),
            remote_process_id: 1,
        };
        assert!(n.check(&Envelope::new(2, MessageKind::Event)).is_err());
    }

    /// Ponta de um par de pipes em memória (um em cada sentido).
    struct Duplex(PipeReader, PipeWriter);
    impl Read for Duplex {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            self.0.read(b)
        }
    }
    impl Write for Duplex {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.1.write(b)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.1.flush()
        }
    }

    fn pair() -> (Duplex, Duplex) {
        let (r1, w1) = io::pipe().unwrap();
        let (r2, w2) = io::pipe().unwrap();
        (Duplex(r1, w2), Duplex(r2, w1))
    }

    fn peer(name: &str, min: u32, max: u32) -> Peer {
        Peer {
            min_version: min,
            max_version: max,
            ..Peer::new(name)
        }
    }

    #[test]
    fn handshake_agrees_on_version_and_identifies_both_sides() {
        let (mut c, mut s) = pair();
        let server = std::thread::spawn(move || accept(&mut s, &Peer::new("servidor")).unwrap());
        let client = connect(&mut c, &Peer::new("cliente")).unwrap();
        let server = server.join().unwrap();
        assert_eq!(client.version, CURRENT_VERSION);
        assert_eq!(server.version, CURRENT_VERSION);
        assert_eq!(client.remote_peer, "servidor");
        assert_eq!(server.remote_peer, "cliente");
        assert_eq!(client.remote_process_id, std::process::id());
    }

    #[test]
    fn newer_client_falls_back_to_the_servers_version() {
        let (mut c, mut s) = pair();
        let server = std::thread::spawn(move || accept(&mut s, &peer("s", 1, 2)).unwrap());
        assert_eq!(connect(&mut c, &peer("c", 2, 3)).unwrap().version, 2);
        assert_eq!(server.join().unwrap().version, 2);
    }

    #[test]
    fn incompatible_version_is_refused_with_a_clear_error_on_both_sides() {
        let (mut c, mut s) = pair();
        let server = std::thread::spawn(move || accept(&mut s, &peer("tako-supervisor", 1, 1)));
        let client = connect(&mut c, &peer("tako-ui", 2, 3));
        let server = server.join().unwrap();
        let (Err(IpcError::VersionMismatch(cm)), Err(IpcError::VersionMismatch(sm))) =
            (client, server)
        else {
            panic!("as duas pontas deveriam recusar");
        };
        assert!(cm.contains("incompatível"));
        assert!(cm.contains("'tako-supervisor' aceita 1–1"));
        assert!(cm.contains("'tako-ui' aceita 2–3"));
        assert_eq!(cm, sm);
    }

    #[test]
    fn server_refuses_a_client_that_skips_hello() {
        let (mut c, mut s) = pair();
        let server = std::thread::spawn(move || accept(&mut s, &Peer::new("s")));
        let req = Envelope {
            id: Some(1),
            method: Some("x".into()),
            ..Envelope::new(1, MessageKind::Request)
        };
        write_frame(&mut c, &req).unwrap();
        let reply = read_frame(&mut c, MAX_FRAME_BYTES).unwrap().unwrap();
        assert!(matches!(server.join().unwrap(), Err(IpcError::Protocol(_))));
        assert_eq!(reply.kind, MessageKind::Reject);
        assert_eq!(reply.error.unwrap().code, codes::INVALID_MESSAGE);
    }
}
