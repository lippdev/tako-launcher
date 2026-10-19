//! Servidor HTTP local da API de integração (EXT-002, ADR-0030).
//!
//! Escuta apenas em `127.0.0.1`, exige autenticação `Bearer <token>` e expõe leitura da biblioteca,
//! estado de execução e ações para abrir e jogar jogos.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

pub const API_ENABLED: &str = "LocalApiEnabled";
pub const API_PORT: &str = "LocalApiPort";
pub const API_TOKEN: &str = "LocalApiToken";
pub const DEFAULT_PORT: u16 = 47774;

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
///
/// Ponto de ligação com EXT-001:
/// Quando `session::events` estiver disponível, o provedor poderá atualizar o estado
/// automaticamente através dos eventos `GameStarting`, `GameStarted` e `GameExited`.
pub trait ApiHandler: Send + Sync + 'static {
    fn list_games(&self) -> Vec<ApiGame>;
    fn game_state(&self) -> ApiGameState;
    fn open_game(&self, id: &str) -> Result<(), ApiActionError>;
    fn play_game(&self, id: &str) -> Result<(), ApiActionError>;
}

/// Controle do servidor HTTP em execução.
pub struct ServerHandle {
    addr: SocketAddr,
    running: Arc<AtomicBool>,
    token: Arc<RwLock<String>>,
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

    pub fn set_token(&self, new_token: String) {
        if let Ok(mut t) = self.token.write() {
            *t = new_token;
        }
    }

    pub fn stop(&self) {
        if self.running.swap(false, Ordering::Relaxed) {
            // Conecta localmente para destravar o accept() bloqueante
            let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(100));
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
    let running = Arc::new(AtomicBool::new(true));
    let token_holder = Arc::new(RwLock::new(token));

    let run_flag = running.clone();
    let token_ref = token_holder.clone();

    let thread_handle = std::thread::Builder::new()
        .name("tako-api".into())
        .spawn(move || {
            // Aceita conexões até que running seja falso
            while run_flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if !run_flag.load(Ordering::Relaxed) {
                            break;
                        }
                        let h = handler.clone();
                        let t = token_ref.clone();
                        // Trata a requisição (leitura e resposta)
                        std::thread::spawn(move || {
                            let _ = handle_connection(stream, h, t);
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
        token: token_holder,
        thread: Mutex::new(Some(thread_handle)),
    })
}

fn handle_connection<H: ApiHandler>(
    mut stream: TcpStream,
    handler: Arc<H>,
    token_holder: Arc<RwLock<String>>,
) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        return Ok(());
    }

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

    // Lê cabeçalhos (até \r\n vazio ou limite de 16 KB)
    let mut auth_header: Option<String> = None;
    let mut total_bytes = 0usize;
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        total_bytes += bytes;
        if total_bytes > 16 * 1024 {
            return send_response(
                &mut stream,
                431,
                "Request Header Fields Too Large",
                r#"{"error":"Headers too large"}"#,
            );
        }
        if let Some((name, val)) = line.split_once(':')
            && name.trim().eq_ignore_ascii_case("authorization")
        {
            auth_header = Some(val.trim().to_string());
        }
    }

    // Suporte a OPTIONS (CORS pre-flight local)
    if method.eq_ignore_ascii_case("OPTIONS") {
        return send_options_response(&mut stream);
    }

    // Validação de autenticação: Bearer <token>
    let expected_token = token_holder.read().map(|t| t.clone()).unwrap_or_default();

    let authorized = auth_header.as_deref().is_some_and(|h| {
        if let Some(provided) = h.strip_prefix("Bearer ") {
            !provided.is_empty() && provided == expected_token
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
            let id = decode_path_id(raw_id);
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
                Err(ApiActionError::Other(e)) => {
                    let json = serde_json::json!({ "error": e }).to_string();
                    send_response(&mut stream, 500, "Internal Server Error", &json)
                }
            }
        }
        ("POST", ["api", "v1", "games", raw_id, "play"]) => {
            let id = decode_path_id(raw_id);
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
fn decode_path_id(raw: &str) -> String {
    let mut bytes = raw.bytes();
    let mut decoded = Vec::new();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let Some(hi_byte) = bytes.next() else { break };
            let Some(lo_byte) = bytes.next() else { break };
            let Some(hi) = (hi_byte as char).to_digit(16) else {
                break;
            };
            let Some(lo) = (lo_byte as char).to_digit(16) else {
                break;
            };
            decoded.push((hi * 16 + lo) as u8);
        } else {
            decoded.push(b);
        }
    }
    String::from_utf8(decoded).unwrap_or_else(|_| raw.to_string())
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
         Access-Control-Allow-Origin: *\r\n\
         \r\n\
         {body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

fn send_options_response(stream: &mut TcpStream) -> std::io::Result<()> {
    let response = "HTTP/1.1 204 No Content\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Authorization, Content-Type\r\n\
         Connection: close\r\n\
         \r\n";
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
            if self.games.iter().any(|g| g.id == id) {
                self.opened.lock().unwrap().push(id.to_string());
                Ok(())
            } else {
                Err(ApiActionError::NotFound)
            }
        }

        fn play_game(&self, id: &str) -> Result<(), ApiActionError> {
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
}
