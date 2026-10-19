//! Servidor HTTP local da API de integração (EXT-002, ADR-0033).
//!
//! Escuta apenas em `127.0.0.1`, exige autenticação `Bearer <token>` e expõe leitura da biblioteca,
//! estado de execução e ações para abrir e jogar jogos.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

pub const API_ENABLED: &str = "LocalApiEnabled";
pub const API_PORT: &str = "LocalApiPort";
pub const API_TOKEN: &str = "LocalApiToken";
pub const DEFAULT_PORT: u16 = 47774;

pub const MAX_CONCURRENT_CONNECTIONS: usize = 16;
pub const MAX_REQUEST_LINE_BYTES: usize = 8192;
pub const MAX_HEADER_LINE_BYTES: usize = 4096;
pub const MAX_HEADERS_TOTAL_BYTES: usize = 16384;
pub const READ_TIMEOUT: Duration = Duration::from_secs(5);
pub const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// Chaves de configuração que NÃO podem entrar em backup ou exportação (segredos locais do dispositivo).
pub const BACKUP_EXCLUDED_SETTINGS: &[&str] = &[API_TOKEN];

/// Chaves de configuração que NÃO podem ser importadas como consentimento do usuário (opt-in estrito).
pub const IMPORT_EXCLUDED_SETTINGS: &[&str] = &[API_TOKEN, API_ENABLED];

/// Diz se uma chave de configuração da API deve ser excluída de backups/exportações (CLOUD-001 / PR #137).
pub fn is_backup_excluded(key: &str) -> bool {
    BACKUP_EXCLUDED_SETTINGS.contains(&key)
}

/// Diz se uma chave de configuração não pode ser restaurada/importada como consentimento (CLOUD-001 / PR #137).
pub fn is_import_excluded(key: &str) -> bool {
    IMPORT_EXCLUDED_SETTINGS.contains(&key)
}

/// Valida se uma string é um token de API válido (32 caracteres hexadecimais).
pub fn is_valid_token(token: &str) -> bool {
    token.len() == 32 && token.chars().all(|c| c.is_ascii_hexdigit())
}

/// Um jogo exposto pela API de leitura.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiGame {
    pub id: String,
    pub title: String,
    pub store: String,
    pub installed: bool,
}

/// Detalhes do jogo rodando no momento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunningGameInfo {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
}

/// Estado do Tako: se há jogo aberto e qual é.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiGameState {
    pub is_game_running: bool,
    pub running_game: Option<RunningGameInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApiActionError {
    #[error("Game not found")]
    NotFound,
    #[error("Interface unavailable")]
    Unavailable,
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error)]
pub enum ApiServerError {
    #[error("API local só pode escutar em endereço de loopback (127.0.0.1)")]
    NonLoopbackAddress,
    #[error("Falha ao vincular porta: {0}")]
    Io(#[from] std::io::Error),
}

/// Configurações da API local.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiConfig {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            token: String::new(),
        }
    }
}

/// Gera um token de autenticação seguro (32 caracteres hexadecimais aleatórios).
pub fn generate_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Contrato para quem provê os dados e executa as ações da API.
///
/// Mantém `tako-core` desacoplado da UI e do SO.
pub trait ApiHandler: Send + Sync + 'static {
    fn list_games(&self) -> Vec<ApiGame>;
    fn game_state(&self) -> ApiGameState;
    fn open_game(&self, id: &str) -> Result<(), ApiActionError>;
    fn play_game(&self, id: &str) -> Result<(), ApiActionError>;
}

struct ActiveStreams {
    next_id: u64,
    map: HashMap<u64, TcpStream>,
}

/// Controle do servidor HTTP em execução.
pub struct ServerHandle {
    addr: SocketAddr,
    running: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    token: Arc<RwLock<String>>,
    active_streams: Arc<Mutex<ActiveStreams>>,
    active_conns: Arc<AtomicUsize>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl ServerHandle {
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn active_connections(&self) -> usize {
        self.active_conns.load(Ordering::Relaxed)
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub fn set_token(&self, new_token: String) {
        if let Ok(mut t) = self.token.write()
            && *t != new_token
        {
            *t = new_token;
            self.generation.fetch_add(1, Ordering::SeqCst);
            if let Ok(mut streams) = self.active_streams.lock() {
                for (_, s) in streams.map.drain() {
                    let _ = s.shutdown(std::net::Shutdown::Both);
                }
            }
        }
    }

    pub fn stop(&self) {
        if self.running.swap(false, Ordering::SeqCst) {
            self.generation.fetch_add(1, Ordering::SeqCst);
            // Encerra imediatamente conexões ativas para destravar workers sem travar a UI
            if let Ok(mut streams) = self.active_streams.lock() {
                for (_, s) in streams.map.drain() {
                    let _ = s.shutdown(std::net::Shutdown::Both);
                }
            }
            // Conecta localmente para destravar o accept() bloqueante
            let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(50));
            if let Ok(mut lock) = self.thread.lock()
                && let Some(handle) = lock.take()
            {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Inicia o servidor HTTP local em segundo plano.
///
/// Falha se o IP não for loopback (`127.0.0.1`), garantindo que nenhuma interface externa seja exposta.
pub fn start_server<H: ApiHandler>(
    ip: Ipv4Addr,
    port: u16,
    token: String,
    handler: Arc<H>,
) -> Result<ServerHandle, ApiServerError> {
    if !ip.is_loopback() {
        return Err(ApiServerError::NonLoopbackAddress);
    }

    let socket_addr = SocketAddr::new(IpAddr::V4(ip), port);
    let listener = TcpListener::bind(socket_addr)?;
    let actual_addr = listener.local_addr()?;
    let actual_port = actual_addr.port();
    let running = Arc::new(AtomicBool::new(true));
    let generation = Arc::new(AtomicU64::new(1));
    let token_holder = Arc::new(RwLock::new(token));
    let active_streams = Arc::new(Mutex::new(ActiveStreams {
        next_id: 1,
        map: HashMap::new(),
    }));
    let active_conns = Arc::new(AtomicUsize::new(0));

    let run_flag = running.clone();
    let gen_flag = generation.clone();
    let token_ref = token_holder.clone();
    let streams_ref = active_streams.clone();
    let conns_ref = active_conns.clone();

    let thread_handle = std::thread::Builder::new()
        .name("tako-api".into())
        .spawn(move || {
            while run_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        if !run_flag.load(Ordering::Relaxed) {
                            break;
                        }
                        let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
                        let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));

                        if conns_ref.load(Ordering::Relaxed) >= MAX_CONCURRENT_CONNECTIONS {
                            let _ = send_response(
                                &mut stream,
                                429,
                                "Too Many Requests",
                                r#"{"error":"Too many concurrent connections"}"#,
                            );
                            continue;
                        }

                        conns_ref.fetch_add(1, Ordering::Relaxed);

                        let stream_id = if let Ok(mut streams) = streams_ref.lock() {
                            let id = streams.next_id;
                            streams.next_id = streams.next_id.wrapping_add(1);
                            if let Ok(cloned) = stream.try_clone() {
                                streams.map.insert(id, cloned);
                                Some(id)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let h = handler.clone();
                        let t = token_ref.clone();
                        let r = run_flag.clone();
                        let g = gen_flag.clone();
                        let conn_gen = gen_flag.load(Ordering::SeqCst);
                        let s_map = streams_ref.clone();
                        let c_counter = conns_ref.clone();
                        let s_port = actual_port;

                        std::thread::spawn(move || {
                            struct ConnGuard {
                                id: Option<u64>,
                                streams: Arc<Mutex<ActiveStreams>>,
                                conns: Arc<AtomicUsize>,
                            }
                            impl Drop for ConnGuard {
                                fn drop(&mut self) {
                                    self.conns.fetch_sub(1, Ordering::Relaxed);
                                    if let Some(id) = self.id
                                        && let Ok(mut s) = self.streams.lock()
                                    {
                                        s.map.remove(&id);
                                    }
                                }
                            }
                            let _guard = ConnGuard {
                                id: stream_id,
                                streams: s_map,
                                conns: c_counter,
                            };
                            let _ = handle_connection(stream, h, t, r, g, conn_gen, s_port);
                        });
                    }
                    Err(_) => {
                        if !run_flag.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
            }
        })
        .map_err(ApiServerError::Io)?;

    Ok(ServerHandle {
        addr: actual_addr,
        running,
        generation,
        token: token_holder,
        active_streams,
        active_conns,
        thread: Mutex::new(Some(thread_handle)),
    })
}

#[derive(Debug)]
enum ReadLineError {
    Io(std::io::Error),
    TooLarge,
}

fn read_line_limited<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> Result<Option<String>, ReadLineError> {
    let mut buf = Vec::new();
    loop {
        let (done, used) = {
            let available = match reader.fill_buf() {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(ReadLineError::Io(e)),
            };
            if available.is_empty() {
                if buf.is_empty() {
                    return Ok(None);
                }
                break;
            }
            match available.iter().position(|&b| b == b'\n') {
                Some(pos) => (true, pos + 1),
                None => (false, available.len()),
            }
        };
        if buf.len() + used > max_bytes {
            return Err(ReadLineError::TooLarge);
        }
        let available = reader.fill_buf().map_err(ReadLineError::Io)?;
        buf.extend_from_slice(&available[..used]);
        reader.consume(used);
        if done {
            break;
        }
    }
    let s = String::from_utf8(buf)
        .map_err(|e| ReadLineError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
    Ok(Some(s))
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn handle_connection<H: ApiHandler>(
    mut stream: TcpStream,
    handler: Arc<H>,
    token_holder: Arc<RwLock<String>>,
    run_flag: Arc<AtomicBool>,
    gen_flag: Arc<AtomicU64>,
    conn_gen: u64,
    server_port: u16,
) -> std::io::Result<()> {
    let cloned_stream = stream.try_clone()?;
    let mut reader = BufReader::new(cloned_stream);

    let request_line = match read_line_limited(&mut reader, MAX_REQUEST_LINE_BYTES) {
        Ok(Some(line)) => line,
        Ok(None) => return Ok(()),
        Err(ReadLineError::TooLarge) => {
            return send_response(
                &mut stream,
                414,
                "URI Too Long",
                r#"{"error":"Request line too long"}"#,
            );
        }
        Err(ReadLineError::Io(e)) => return Err(e),
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return send_response(
            &mut stream,
            400,
            "Bad Request",
            r#"{"error":"Bad request"}"#,
        );
    }
    let method = parts[0];
    let path = parts[1];

    // Lê cabeçalhos com limites estritos
    let mut auth_header: Option<String> = None;
    let mut host_header: Option<String> = None;
    let mut total_bytes = 0usize;
    loop {
        let line = match read_line_limited(&mut reader, MAX_HEADER_LINE_BYTES) {
            Ok(Some(l)) => l,
            Ok(None) => break,
            Err(ReadLineError::TooLarge) => {
                return send_response(
                    &mut stream,
                    431,
                    "Request Header Fields Too Large",
                    r#"{"error":"Header line too large"}"#,
                );
            }
            Err(ReadLineError::Io(e)) => return Err(e),
        };

        if line == "\r\n" || line == "\n" {
            break;
        }
        total_bytes += line.len();
        if total_bytes > MAX_HEADERS_TOTAL_BYTES {
            return send_response(
                &mut stream,
                431,
                "Request Header Fields Too Large",
                r#"{"error":"Headers too large"}"#,
            );
        }
        if let Some((name, val)) = line.split_once(':') {
            let name_trim = name.trim();
            if name_trim.eq_ignore_ascii_case("authorization") {
                auth_header = Some(val.trim().to_string());
            } else if name_trim.eq_ignore_ascii_case("host") {
                host_header = Some(val.trim().to_string());
            }
        }
    }

    // Proteção contra DNS rebinding: rejeita com 421 se Host não for 127.0.0.1:<porta> ou localhost:<porta>
    let expected_host1 = format!("127.0.0.1:{server_port}");
    let expected_host2 = format!("localhost:{server_port}");
    let host_valid = host_header.as_deref().is_some_and(|h| {
        h.eq_ignore_ascii_case(&expected_host1) || h.eq_ignore_ascii_case(&expected_host2)
    });
    if !host_valid {
        return send_response(
            &mut stream,
            421,
            "Misdirected Request",
            r#"{"error":"Misdirected Request"}"#,
        );
    }

    // CORS pre-flight removido: responde 405 Method Not Allowed a OPTIONS
    if method.eq_ignore_ascii_case("OPTIONS") {
        return send_response(
            &mut stream,
            405,
            "Method Not Allowed",
            r#"{"error":"Method not allowed"}"#,
        );
    }

    // Validação de autenticação: Bearer <token> em tempo constante
    let expected_token = token_holder.read().map(|t| t.clone()).unwrap_or_default();

    let authorized = auth_header.as_deref().is_some_and(|h| {
        if let Some(provided) = h.strip_prefix("Bearer ") {
            !provided.is_empty()
                && !expected_token.is_empty()
                && constant_time_eq(provided.as_bytes(), expected_token.as_bytes())
        } else {
            false
        }
    });

    if !authorized {
        return send_response(
            &mut stream,
            401,
            "Unauthorized",
            r#"{"error":"Unauthorized"}"#,
        );
    }

    // Roteamento
    let clean_path = path.split('?').next().unwrap_or(path);
    let segments: Vec<&str> = clean_path
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    match (method, segments.as_slice()) {
        ("GET", ["api", "v1", "games"]) | ("GET", ["api", "v1", "library"]) => {
            let games = handler.list_games();
            let json = serde_json::to_string(&games).unwrap_or_else(|_| "[]".into());
            send_response(&mut stream, 200, "OK", &json)
        }
        ("GET", ["api", "v1", "state"]) | ("GET", ["api", "v1", "status"]) => {
            let state = handler.game_state();
            let json = serde_json::to_string(&state).unwrap_or_else(|_| "{}".into());
            send_response(&mut stream, 200, "OK", &json)
        }
        ("POST", ["api", "v1", "games", raw_id, "open"]) => {
            let Ok(id) = decode_path_id(raw_id) else {
                return send_response(
                    &mut stream,
                    400,
                    "Bad Request",
                    r#"{"error":"Invalid percent-encoding"}"#,
                );
            };

            // Revalidação de sessão e geração ativa antes do despacho
            if !run_flag.load(Ordering::SeqCst) {
                return send_response(
                    &mut stream,
                    503,
                    "Service Unavailable",
                    r#"{"error":"Interface unavailable"}"#,
                );
            }
            if gen_flag.load(Ordering::SeqCst) != conn_gen {
                return send_response(
                    &mut stream,
                    401,
                    "Unauthorized",
                    r#"{"error":"Unauthorized"}"#,
                );
            }

            match handler.open_game(&id) {
                Ok(()) => {
                    let json = serde_json::json!({ "success": true, "id": id }).to_string();
                    send_response(&mut stream, 200, "OK", &json)
                }
                Err(ApiActionError::NotFound) => send_response(
                    &mut stream,
                    404,
                    "Not Found",
                    r#"{"error":"Game not found"}"#,
                ),
                Err(ApiActionError::Unavailable) => send_response(
                    &mut stream,
                    503,
                    "Service Unavailable",
                    r#"{"error":"Interface unavailable"}"#,
                ),
                Err(ApiActionError::Other(e)) => {
                    let json = serde_json::json!({ "error": e }).to_string();
                    send_response(&mut stream, 500, "Internal Server Error", &json)
                }
            }
        }
        ("POST", ["api", "v1", "games", raw_id, "play"]) => {
            let Ok(id) = decode_path_id(raw_id) else {
                return send_response(
                    &mut stream,
                    400,
                    "Bad Request",
                    r#"{"error":"Invalid percent-encoding"}"#,
                );
            };

            // Revalidação de sessão e geração ativa antes do despacho
            if !run_flag.load(Ordering::SeqCst) {
                return send_response(
                    &mut stream,
                    503,
                    "Service Unavailable",
                    r#"{"error":"Interface unavailable"}"#,
                );
            }
            if gen_flag.load(Ordering::SeqCst) != conn_gen {
                return send_response(
                    &mut stream,
                    401,
                    "Unauthorized",
                    r#"{"error":"Unauthorized"}"#,
                );
            }

            match handler.play_game(&id) {
                Ok(()) => {
                    let json = serde_json::json!({ "success": true, "id": id }).to_string();
                    send_response(&mut stream, 200, "OK", &json)
                }
                Err(ApiActionError::NotFound) => send_response(
                    &mut stream,
                    404,
                    "Not Found",
                    r#"{"error":"Game not found"}"#,
                ),
                Err(ApiActionError::Unavailable) => send_response(
                    &mut stream,
                    503,
                    "Service Unavailable",
                    r#"{"error":"Interface unavailable"}"#,
                ),
                Err(ApiActionError::Other(e)) => {
                    let json = serde_json::json!({ "error": e }).to_string();
                    send_response(&mut stream, 500, "Internal Server Error", &json)
                }
            }
        }
        // Método não permitido para rotas conhecidas
        (_, ["api", "v1", "games"])
        | (_, ["api", "v1", "library"])
        | (_, ["api", "v1", "state"])
        | (_, ["api", "v1", "status"])
        | (_, ["api", "v1", "games", _, "open"])
        | (_, ["api", "v1", "games", _, "play"]) => send_response(
            &mut stream,
            405,
            "Method Not Allowed",
            r#"{"error":"Method not allowed"}"#,
        ),
        _ => send_response(&mut stream, 404, "Not Found", r#"{"error":"Not found"}"#),
    }
}

/// Decodifica percent-encoding de um identificador na URL (ex: `steam%3A570` -> `steam:570`).
/// Retorna `Err(())` se houver sequência inválida ou se o resultado não for UTF-8 válido.
fn decode_path_id(raw: &str) -> Result<String, ()> {
    let mut bytes = raw.bytes();
    let mut decoded = Vec::new();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi_byte = bytes.next().ok_or(())?;
            let lo_byte = bytes.next().ok_or(())?;
            let hi = (hi_byte as char).to_digit(16).ok_or(())?;
            let lo = (lo_byte as char).to_digit(16).ok_or(())?;
            decoded.push((hi * 16 + lo) as u8);
        } else {
            decoded.push(b);
        }
    }
    String::from_utf8(decoded).map_err(|_| ())
}

fn send_response(
    stream: &mut TcpStream,
    status_code: u16,
    reason: &str,
    body: &str,
) -> std::io::Result<()> {
    let response = format!(
        "HTTP/1.1 {status_code} {reason}\r\n\
         Content-Type: application/json; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHandler {
        games: Vec<ApiGame>,
        opened: Mutex<Vec<String>>,
        played: Mutex<Vec<String>>,
        fail_unavailable: AtomicBool,
    }

    impl MockHandler {
        fn new() -> Self {
            Self {
                games: vec![
                    ApiGame {
                        id: "steam:570".into(),
                        title: "Dota 2".into(),
                        store: "Steam".into(),
                        installed: true,
                    },
                    ApiGame {
                        id: "manual:hades".into(),
                        title: "Hades".into(),
                        store: "Manual".into(),
                        installed: true,
                    },
                ],
                opened: Mutex::new(Vec::new()),
                played: Mutex::new(Vec::new()),
                fail_unavailable: AtomicBool::new(false),
            }
        }
    }

    impl ApiHandler for MockHandler {
        fn list_games(&self) -> Vec<ApiGame> {
            self.games.clone()
        }

        fn game_state(&self) -> ApiGameState {
            let last_played = self.played.lock().unwrap().last().cloned();
            match last_played {
                Some(id) => ApiGameState {
                    is_game_running: true,
                    running_game: Some(RunningGameInfo {
                        id,
                        title: "Dota 2".into(),
                        pid: Some(1234),
                        started_at: Some("2026-10-18T20:00:00Z".into()),
                    }),
                },
                None => ApiGameState {
                    is_game_running: false,
                    running_game: None,
                },
            }
        }

        fn open_game(&self, id: &str) -> Result<(), ApiActionError> {
            if self.fail_unavailable.load(Ordering::Relaxed) {
                return Err(ApiActionError::Unavailable);
            }
            if self.games.iter().any(|g| g.id == id) {
                self.opened.lock().unwrap().push(id.to_string());
                Ok(())
            } else {
                Err(ApiActionError::NotFound)
            }
        }

        fn play_game(&self, id: &str) -> Result<(), ApiActionError> {
            if self.fail_unavailable.load(Ordering::Relaxed) {
                return Err(ApiActionError::Unavailable);
            }
            if self.games.iter().any(|g| g.id == id) {
                self.played.lock().unwrap().push(id.to_string());
                Ok(())
            } else {
                Err(ApiActionError::NotFound)
            }
        }
    }

    #[test]
    fn bind_rejects_non_loopback() {
        let handler = Arc::new(MockHandler::new());
        let res = start_server(
            Ipv4Addr::new(192, 168, 1, 100),
            0,
            "secret".into(),
            handler.clone(),
        );
        assert!(matches!(res, Err(ApiServerError::NonLoopbackAddress)));

        let res0 = start_server(
            Ipv4Addr::new(0, 0, 0, 0),
            0,
            "secret".into(),
            handler.clone(),
        );
        assert!(matches!(res0, Err(ApiServerError::NonLoopbackAddress)));
    }

    #[test]
    fn authentication_verifications() {
        let handler = Arc::new(MockHandler::new());
        let server = start_server(Ipv4Addr::LOCALHOST, 0, "token123".into(), handler).unwrap();
        let port = server.port();

        // 1. Sem token -> 401
        let res = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/games")).call();
        assert!(res.is_err());
        let err = res.unwrap_err();
        if let ureq::Error::StatusCode(code) = err {
            assert_eq!(code, 401);
        } else {
            panic!("Esperado status 401");
        }

        // 2. Token errado -> 401
        let res_wrong = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/games"))
            .header("Authorization", "Bearer errado")
            .call();
        assert!(res_wrong.is_err());
        let err_wrong = res_wrong.unwrap_err();
        if let ureq::Error::StatusCode(code) = err_wrong {
            assert_eq!(code, 401);
        } else {
            panic!("Esperado status 401");
        }

        // 3. Token correto -> 200
        let res_ok = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/games"))
            .header("Authorization", "Bearer token123")
            .call()
            .unwrap();
        assert_eq!(res_ok.status().as_u16(), 200);

        // 4. Regenerar token dinamicamente
        server.set_token("token_novo".into());

        // Token antigo agora falha
        let res_old = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/games"))
            .header("Authorization", "Bearer token123")
            .call();
        assert!(res_old.is_err());

        // Token novo passa
        let res_new = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/games"))
            .header("Authorization", "Bearer token_novo")
            .call()
            .unwrap();
        assert_eq!(res_new.status().as_u16(), 200);
    }

    #[test]
    fn read_endpoints_and_actions() {
        let handler = Arc::new(MockHandler::new());
        let server =
            start_server(Ipv4Addr::LOCALHOST, 0, "secret".into(), handler.clone()).unwrap();
        let port = server.port();

        // GET /api/v1/games
        let games: Vec<ApiGame> = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/games"))
            .header("Authorization", "Bearer secret")
            .call()
            .unwrap()
            .body_mut()
            .read_json()
            .unwrap();
        assert_eq!(games.len(), 2);
        assert_eq!(games[0].id, "steam:570");

        // GET /api/v1/state (inicialmente sem jogo)
        let state1: ApiGameState = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/state"))
            .header("Authorization", "Bearer secret")
            .call()
            .unwrap()
            .body_mut()
            .read_json()
            .unwrap();
        assert!(!state1.is_game_running);
        assert!(state1.running_game.is_none());

        // POST /api/v1/games/steam%3A570/open
        let open_res = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/steam%3A570/open"
        ))
        .header("Authorization", "Bearer secret")
        .send_empty()
        .unwrap();
        assert_eq!(open_res.status().as_u16(), 200);
        assert_eq!(handler.opened.lock().unwrap().as_slice(), &["steam:570"]);

        // POST /api/v1/games/steam:570/play
        let play_res = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/steam:570/play"
        ))
        .header("Authorization", "Bearer secret")
        .send_empty()
        .unwrap();
        assert_eq!(play_res.status().as_u16(), 200);
        assert_eq!(handler.played.lock().unwrap().as_slice(), &["steam:570"]);

        // GET /api/v1/state (agora com jogo aberto)
        let state2: ApiGameState = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/state"))
            .header("Authorization", "Bearer secret")
            .call()
            .unwrap()
            .body_mut()
            .read_json()
            .unwrap();
        assert!(state2.is_game_running);
        assert_eq!(state2.running_game.unwrap().id, "steam:570");

        // POST em jogo inexistente -> 404
        let err_open = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/inexistente/open"
        ))
        .header("Authorization", "Bearer secret")
        .send_empty()
        .unwrap_err();
        if let ureq::Error::StatusCode(code) = err_open {
            assert_eq!(code, 404);
        } else {
            panic!("Esperado 404");
        }

        // Método não permitido -> 405
        let err_method = ureq::post(&format!("http://127.0.0.1:{port}/api/v1/games"))
            .header("Authorization", "Bearer secret")
            .send_empty()
            .unwrap_err();
        if let ureq::Error::StatusCode(code) = err_method {
            assert_eq!(code, 405);
        } else {
            panic!("Esperado 405");
        }

        // Rota desconhecida -> 404
        let err_route = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/desconhecido"))
            .header("Authorization", "Bearer secret")
            .call()
            .unwrap_err();
        if let ureq::Error::StatusCode(code) = err_route {
            assert_eq!(code, 404);
        } else {
            panic!("Esperado 404");
        }
    }

    #[test]
    fn invalid_percent_encoding_returns_400() {
        let handler = Arc::new(MockHandler::new());
        let server =
            start_server(Ipv4Addr::LOCALHOST, 0, "secret".into(), handler.clone()).unwrap();
        let port = server.port();

        // 1. Percent sem dígitos seguintes (% truncado)
        let err1 = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/steam%3/open"
        ))
        .header("Authorization", "Bearer secret")
        .send_empty()
        .unwrap_err();
        if let ureq::Error::StatusCode(code) = err1 {
            assert_eq!(code, 400);
        } else {
            panic!("Esperado 400 para encoding truncado");
        }

        // 2. Percent com caracteres hexadecimais inválidos
        let err2 = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/steam%GG/play"
        ))
        .header("Authorization", "Bearer secret")
        .send_empty()
        .unwrap_err();
        if let ureq::Error::StatusCode(code) = err2 {
            assert_eq!(code, 400);
        } else {
            panic!("Esperado 400 para hex inválido");
        }

        // 3. Percent incompleto no final
        let err3 = ureq::post(&format!("http://127.0.0.1:{port}/api/v1/games/steam%/open"))
            .header("Authorization", "Bearer secret")
            .send_empty()
            .unwrap_err();
        if let ureq::Error::StatusCode(code) = err3 {
            assert_eq!(code, 400);
        } else {
            panic!("Esperado 400 para % solitário");
        }

        // Nenhuma ação foi registrada
        assert!(handler.opened.lock().unwrap().is_empty());
        assert!(handler.played.lock().unwrap().is_empty());
    }

    #[test]
    fn unavailable_handler_returns_503() {
        let handler = Arc::new(MockHandler::new());
        handler.fail_unavailable.store(true, Ordering::Relaxed);
        let server =
            start_server(Ipv4Addr::LOCALHOST, 0, "secret".into(), handler.clone()).unwrap();
        let port = server.port();

        let err = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/steam:570/open"
        ))
        .header("Authorization", "Bearer secret")
        .send_empty()
        .unwrap_err();
        if let ureq::Error::StatusCode(code) = err {
            assert_eq!(code, 503);
        } else {
            panic!("Esperado 503");
        }
    }

    #[test]
    fn token_validation_and_backup_exclusion() {
        assert!(is_valid_token("0123456789abcdef0123456789abcdef"));
        assert!(!is_valid_token("curto"));
        assert!(!is_valid_token("0123456789abcdef0123456789abcdeg")); // 'g' não é hex
        assert!(!is_valid_token("0123456789abcdef0123456789abcdef0")); // 33 chars

        assert!(is_backup_excluded(API_TOKEN));
        assert!(!is_backup_excluded(API_ENABLED));

        assert!(is_import_excluded(API_TOKEN));
        assert!(is_import_excluded(API_ENABLED));
        assert!(!is_import_excluded(API_PORT));
    }

    #[test]
    fn concurrent_connections_are_bounded_at_16() {
        let handler = Arc::new(MockHandler::new());
        let server = start_server(Ipv4Addr::LOCALHOST, 0, "secret".into(), handler).unwrap();
        let port = server.port();

        // Abre 16 conexões simultâneas TCP sem enviar nada (fica no read_line)
        let mut streams = Vec::new();
        for _ in 0..MAX_CONCURRENT_CONNECTIONS {
            let s = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
            streams.push(s);
        }

        // Aguarda workers registrarem
        std::thread::sleep(Duration::from_millis(50));

        // A 17ª conexão deve ser rejeitada com 429 Too Many Requests
        let mut rejected = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        let mut response = String::new();
        let _ = BufReader::new(&mut rejected).read_line(&mut response);
        assert!(response.starts_with("HTTP/1.1 429 Too Many Requests"));

        drop(streams);
    }

    #[test]
    fn limits_request_line_and_header_sizes() {
        let handler = Arc::new(MockHandler::new());
        let server = start_server(Ipv4Addr::LOCALHOST, 0, "secret".into(), handler).unwrap();
        let port = server.port();

        // 1. Linha de requisição acima de 8 KiB -> 414 URI Too Long
        let mut stream1 = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        let long_uri = "a".repeat(MAX_REQUEST_LINE_BYTES + 100);
        let _ = write!(stream1, "GET /{long_uri} HTTP/1.1\r\n\r\n");
        let mut res1 = String::new();
        let _ = BufReader::new(&mut stream1).read_line(&mut res1);
        assert!(res1.starts_with("HTTP/1.1 414 URI Too Long"));

        // 2. Cabeçalhos acima de 16 KiB -> 431 Request Header Fields Too Large
        let mut stream2 = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        let long_header =
            "X-Padding: ".to_string() + &"x".repeat(MAX_HEADER_LINE_BYTES + 100) + "\r\n";
        let _ = write!(stream2, "GET /api/v1/games HTTP/1.1\r\n{long_header}\r\n");
        let mut res2 = String::new();
        let _ = BufReader::new(&mut stream2).read_line(&mut res2);
        assert!(res2.starts_with("HTTP/1.1 431 Request Header Fields Too Large"));
    }

    #[test]
    fn server_stop_terminates_active_connections_and_revalidates_dispatch() {
        let handler = Arc::new(MockHandler::new());
        let server =
            start_server(Ipv4Addr::LOCALHOST, 0, "secret".into(), handler.clone()).unwrap();
        let port = server.port();

        // Conecta e para o servidor
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        server.stop();
        assert!(!server.is_running());

        // Escrever na stream conectada antes do stop
        use std::io::Read;
        let _ = write!(
            stream,
            "POST /api/v1/games/steam:570/play HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n"
        );
        let mut res = String::new();
        let _ = BufReader::new(&mut stream).read_to_string(&mut res);
        assert!(res.is_empty() || res.starts_with("HTTP/1.1 503"));
        assert!(
            handler.played.lock().unwrap().is_empty(),
            "nenhuma ação deve ter sido executada"
        );
    }
    #[test]
    fn options_method_returns_405_and_no_cors_wildcard() {
        let handler = Arc::new(MockHandler::new());
        let server = start_server(Ipv4Addr::LOCALHOST, 0, "secret".into(), handler).unwrap();
        let port = server.port();

        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        use std::io::Read;
        let _ = write!(
            stream,
            "OPTIONS /api/v1/games HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"
        );
        let mut response = String::new();
        let _ = BufReader::new(&mut stream).read_to_string(&mut response);
        assert!(response.starts_with("HTTP/1.1 405 Method Not Allowed"));
        assert!(!response.contains("Access-Control-Allow-Origin"));
    }

    #[test]
    fn dns_rebinding_rejected_with_421() {
        let handler = Arc::new(MockHandler::new());
        let server = start_server(Ipv4Addr::LOCALHOST, 0, "secret".into(), handler).unwrap();
        let port = server.port();

        use std::io::Read;
        // 1. Host externo (ex: attacker.com) -> 421 Misdirected Request
        let mut stream1 = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        let _ = write!(
            stream1,
            "GET /api/v1/games HTTP/1.1\r\nHost: attacker.com\r\nAuthorization: Bearer secret\r\n\r\n"
        );
        let mut res1 = String::new();
        let _ = BufReader::new(&mut stream1).read_to_string(&mut res1);
        assert!(res1.starts_with("HTTP/1.1 421 Misdirected Request"));

        // 2. Sem cabeçalho Host -> 421 Misdirected Request
        let mut stream2 = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        let _ = write!(
            stream2,
            "GET /api/v1/games HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n"
        );
        let mut res2 = String::new();
        let _ = BufReader::new(&mut stream2).read_to_string(&mut res2);
        assert!(res2.starts_with("HTTP/1.1 421 Misdirected Request"));

        // 3. Host localhost com porta correta -> 200 OK
        let mut stream3 = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        let _ = write!(
            stream3,
            "GET /api/v1/games HTTP/1.1\r\nHost: localhost:{port}\r\nAuthorization: Bearer secret\r\n\r\n"
        );
        let mut res3 = String::new();
        let _ = BufReader::new(&mut stream3).read_to_string(&mut res3);
        assert!(res3.starts_with("HTTP/1.1 200 OK"));
    }

    #[test]
    fn set_token_only_increments_generation_when_changed_and_closes_connections() {
        let handler = Arc::new(MockHandler::new());
        let server = start_server(Ipv4Addr::LOCALHOST, 0, "token_inicial".into(), handler).unwrap();
        let port = server.port();

        let gen0 = server.generation();

        // Mesma string: não deve incrementar geração
        server.set_token("token_inicial".into());
        assert_eq!(server.generation(), gen0);

        // Conecta uma stream
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        // Token novo: deve incrementar geração e encerrar conexões aceitas
        server.set_token("token_diferente".into());
        assert_eq!(server.generation(), gen0 + 1);

        // A stream antiga deve estar encerrada / desconectada
        use std::io::Read;
        let _ = write!(
            stream,
            "GET /api/v1/games HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer token_diferente\r\n\r\n"
        );
        let mut res = String::new();
        let _ = BufReader::new(&mut stream).read_to_string(&mut res);
        assert!(res.is_empty());
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"abcdef", b"abcdef"));
        assert!(!constant_time_eq(b"abcdef", b"abcdeg"));
        assert!(!constant_time_eq(b"abcdef", b"abcde"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(constant_time_eq(b"", b""));
    }
}
