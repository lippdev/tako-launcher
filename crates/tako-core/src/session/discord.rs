//! Discord Rich Presence (EXT-004): mostra no Discord o jogo aberto pelo Tako. Fala direto com o IPC local do
//! Discord (handshake e `SET_ACTIVITY`; cada quadro é op u32 LE, tamanho u32 LE e JSON), sem dependência nova.
//! Discord fechado não é erro: só um log de depuração. Serve às duas interfaces: a Slint (`tako_ui::session`) e o
//! núcleo da versão web (`tako_app`, WEB-34).

use chrono::{DateTime, Utc};
use std::io::{self, Read, Write};
use std::sync::mpsc::{Receiver, Sender, channel};

/// Id do aplicativo no portal de desenvolvedores do Discord (do mantenedor). Vazio = recurso escondido.
pub const DISCORD_CLIENT_ID: &str = "";

const HANDSHAKE: u32 = 0;
const FRAME: u32 = 1;
/// Resposta maior que isto não é do Discord.
const MAX_REPLY: u32 = 64 * 1024;

/// O recurso aparece nas configurações (e roda) só com o id do aplicativo definido.
pub fn available() -> bool {
    !DISCORD_CLIENT_ID.is_empty()
}

/// Presença enquanto o jogo roda: soltar o valor limpa a atividade e fecha a conexão.
pub struct Presence {
    _stop: Sender<()>,
}

impl Presence {
    /// Conecta numa thread própria: nunca segura a interface nem a abertura do jogo.
    pub fn start(title: &str, started_at: DateTime<Utc>) -> Option<Presence> {
        if !available() {
            return None;
        }
        Self::start_with(DISCORD_CLIENT_ID, connect, title, started_at)
    }

    /// Como [`Presence::start`], com o id do aplicativo e a conexão fornecidos (o IPC falso, nos testes).
    pub fn start_with<T: Read + Write>(
        client_id: &str,
        connect: impl FnOnce() -> io::Result<T> + Send + 'static,
        title: &str,
        started_at: DateTime<Utc>,
    ) -> Option<Presence> {
        let (stop, rx) = channel();
        let client_id = client_id.to_owned();
        let title = title.to_owned();
        std::thread::Builder::new()
            .name("discord".into())
            .spawn(move || {
                let result =
                    connect().and_then(|mut io| run(&mut io, &client_id, &title, started_at, &rx));
                if let Err(e) = result {
                    tracing::debug!("discord: {e}");
                }
            })
            .ok()?;
        Some(Presence { _stop: stop })
    }
}

/// Quadro do IPC: op, tamanho e o JSON.
fn frame(op: u32, json: &serde_json::Value) -> Vec<u8> {
    let body = json.to_string();
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body.as_bytes());
    out
}

fn read_frame(io: &mut impl Read) -> io::Result<(u32, Vec<u8>)> {
    let mut head = [0u8; 8];
    io.read_exact(&mut head)?;
    let op = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let len = u32::from_le_bytes([head[4], head[5], head[6], head[7]]);
    if len > MAX_REPLY {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "quadro grande demais",
        ));
    }
    let mut body = vec![0; len as usize];
    io.read_exact(&mut body)?;
    Ok((op, body))
}

fn set_activity(activity: serde_json::Value, nonce: &str) -> Vec<u8> {
    frame(
        FRAME,
        &serde_json::json!({
            "cmd": "SET_ACTIVITY",
            "args": { "pid": std::process::id(), "activity": activity },
            "nonce": nonce,
        }),
    )
}

/// Handshake, mostra o jogo, espera o fim da sessão (o `Sender` ser solto) e limpa.
fn run(
    io: &mut (impl Read + Write),
    client_id: &str,
    title: &str,
    started_at: DateTime<Utc>,
    stop: &Receiver<()>,
) -> io::Result<()> {
    io.write_all(&frame(
        HANDSHAKE,
        &serde_json::json!({ "v": 1, "client_id": client_id }),
    ))?;
    let (op, body) = read_frame(io)?;
    if op != FRAME {
        // Handshake recusado (id inválido etc.): o Discord manda CLOSE com o motivo.
        return Err(io::Error::other(
            String::from_utf8_lossy(&body).into_owned(),
        ));
    }
    let activity = serde_json::json!({
        "details": title,
        "timestamps": { "start": started_at.timestamp() },
    });
    io.write_all(&set_activity(activity, "1"))?;
    let _ = stop.recv();
    io.write_all(&set_activity(serde_json::Value::Null, "2"))
}

#[cfg(unix)]
fn connect() -> io::Result<std::os::unix::net::UnixStream> {
    let dirs = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"]
        .iter()
        .filter_map(std::env::var_os)
        .map(std::path::PathBuf::from)
        .chain([std::path::PathBuf::from("/tmp")]);
    let mut last = io::Error::from(io::ErrorKind::NotFound);
    for dir in dirs {
        match std::os::unix::net::UnixStream::connect(dir.join("discord-ipc-0")) {
            Ok(s) => {
                s.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
                return Ok(s);
            }
            Err(e) => last = e,
        }
    }
    Err(last)
}

#[cfg(windows)]
fn connect() -> io::Result<std::fs::File> {
    // shortcut: sem timeout de leitura no pipe; se o Discord não responder, só esta thread fica parada.
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(r"\\.\pipe\discord-ipc-0")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Transporte falso: lê respostas prontas e guarda o que foi escrito.
    struct Fake {
        input: io::Cursor<Vec<u8>>,
        output: Vec<u8>,
    }
    impl Read for Fake {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.input.read(buf)
        }
    }
    impl Write for Fake {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn frames(mut bytes: &[u8]) -> Vec<(u32, serde_json::Value)> {
        let mut out = Vec::new();
        while !bytes.is_empty() {
            let (op, body) = read_frame(&mut bytes).unwrap();
            out.push((op, serde_json::from_slice(&body).unwrap()));
        }
        out
    }

    #[test]
    fn shows_the_game_then_clears_it_when_the_session_ends() {
        let ready = frame(FRAME, &serde_json::json!({ "evt": "READY" }));
        assert_eq!(&ready[..4], &1u32.to_le_bytes());
        assert_eq!(ready[4..8], ((ready.len() - 8) as u32).to_le_bytes());
        let mut io = Fake {
            input: io::Cursor::new(ready),
            output: Vec::new(),
        };
        let (stop, rx) = channel::<()>();
        drop(stop); // a sessão já terminou: mostra e limpa em seguida
        let at = DateTime::from_timestamp(1_700_000_000, 0).unwrap();

        run(&mut io, "123", "Hades", at, &rx).unwrap();

        let sent = frames(&io.output);
        assert_eq!(sent.len(), 3);
        assert_eq!(
            sent[0],
            (0, serde_json::json!({ "v": 1, "client_id": "123" }))
        );
        let shown = &sent[1].1["args"]["activity"];
        assert_eq!(
            (sent[1].0, &shown["details"]),
            (1, &serde_json::json!("Hades"))
        );
        assert_eq!(shown["timestamps"]["start"], 1_700_000_000);
        assert!(sent[2].1["args"]["activity"].is_null());

        // Handshake recusado: erro (só vira log) e nada de atividade.
        let mut io = Fake {
            input: io::Cursor::new(frame(2, &serde_json::json!({ "code": 4000 }))),
            output: Vec::new(),
        };
        assert!(run(&mut io, "x", "Hades", at, &rx).is_err());
        assert_eq!(frames(&io.output).len(), 1);
        // Sem id do aplicativo, o recurso fica desligado.
        assert!(available() || Presence::start("Hades", at).is_none());
    }

    /// Uma ponta de pipes em memória (um em cada sentido), para a thread da presença falar com o "Discord".
    struct Duplex(io::PipeReader, io::PipeWriter);
    impl Read for Duplex {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl Write for Duplex {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.1.write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.1.flush()
        }
    }

    #[test]
    fn presence_shows_while_alive_and_clears_when_dropped() {
        let (mut from_app, app_out) = io::pipe().unwrap();
        let (app_in, mut to_app) = io::pipe().unwrap();
        let at = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let presence =
            Presence::start_with("42", move || Ok(Duplex(app_in, app_out)), "Hades", at).unwrap();

        let (op, body) = read_frame(&mut from_app).unwrap();
        let hello: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!((op, &hello["client_id"]), (HANDSHAKE, &"42".into()));
        to_app
            .write_all(&frame(FRAME, &serde_json::json!({ "evt": "READY" })))
            .unwrap();

        let (_, body) = read_frame(&mut from_app).unwrap();
        let shown: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(shown["args"]["activity"]["details"], "Hades");

        // O jogo fechou: soltar a presença limpa a atividade e encerra a conexão.
        drop(presence);
        let (_, body) = read_frame(&mut from_app).unwrap();
        let cleared: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(cleared["args"]["activity"].is_null());
        assert!(read_frame(&mut from_app).is_err());
    }

    #[test]
    fn discord_closed_is_not_an_error() {
        let at = DateTime::from_timestamp(1_700_000_000, 0).unwrap();
        let presence = Presence::start_with::<Duplex>(
            "42",
            || Err(io::Error::from(io::ErrorKind::NotFound)),
            "Hades",
            at,
        );
        // A conexão falha na thread própria: quem chamou só recebe a presença (inerte) e segue.
        drop(presence);
    }
}
