//! Ciclo de vida da API local (EXT-002, ADR-0033), o mesmo para as duas interfaces: a Slint (`tako_ui::local_api`) e
//! o núcleo da versão web (`tako_app::local_api`).
//!
//! Quem usa entrega as chaves de configuração e a lista de jogos; aqui ficam o estado compartilhado com a sessão
//! (pelos eventos do EXT-001), o servidor ligado ou desligado pela chave, o token e o despacho das ações. A forma de
//! abrir ou jogar um jogo (a interface Slint, a janela web) é de quem usa: chega como [`Dispatch`].

use super::{
    ApiActionError, ApiGame, ApiGameState, ApiHandler, DEFAULT_PORT, RunningGameInfo, ServerHandle,
    generate_token, is_valid_token, start_server,
};
use crate::links::Link;
use crate::session::events::{self, SessionEvent};
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock, Weak};
use std::time::Duration;

/// Entrega a ação de um jogo (`tako://game/<id>` ou `tako://play/<id>`) à interface. `Unavailable` quando ela não
/// pode atender agora.
pub type Dispatch = Arc<dyn Fn(Link) -> Result<(), ApiActionError> + Send + Sync>;

/// O que a API local responde: a lista de jogos e o jogo aberto.
#[derive(Default)]
pub struct ApiState {
    games: RwLock<Vec<ApiGame>>,
    running: RwLock<Option<RunningGameInfo>>,
}

impl ApiState {
    /// Estado vazio, que ainda não acompanha a sessão (veja [`ApiState::follow_session_events`]).
    pub fn new() -> Arc<ApiState> {
        Arc::new(ApiState::default())
    }

    pub fn set_games(&self, games: Vec<ApiGame>) {
        if let Ok(mut current) = self.games.write() {
            *current = games;
        }
    }

    pub fn has_game(&self, id: &str) -> bool {
        self.games
            .read()
            .map(|games| games.iter().any(|game| game.id == id))
            .unwrap_or(false)
    }

    pub fn running(&self) -> Option<RunningGameInfo> {
        self.running.read().ok().and_then(|running| running.clone())
    }

    /// Atualiza o jogo aberto a partir do que a interface sabe agora. O início que a sessão já informou (real, vindo
    /// do evento) vale mais do que o recalculado, e não volta a andar a cada consulta.
    pub fn reconcile_running(&self, current: Option<RunningGameInfo>) {
        let Ok(mut running) = self.running.write() else {
            return;
        };
        *running = current.map(|mut info| {
            if let Some(known) = running
                .as_ref()
                .filter(|known| known.id == info.id && known.started_at.is_some())
            {
                info.started_at = known.started_at.clone();
            }
            info
        });
    }

    /// Passa a acompanhar os eventos de sessão (`GameStarting`, `GameStarted`, `GameExited`). A assinatura nasce
    /// aqui, antes da thread, para nenhum evento publicado depois se perder. A thread termina quando o estado é solto.
    pub fn follow_session_events(self: &Arc<Self>) {
        let rx = events::subscribe();
        let state: Weak<ApiState> = Arc::downgrade(self);
        let spawned = std::thread::Builder::new()
            .name("tako-api-events".into())
            .spawn(move || {
                loop {
                    let event = match rx.recv_timeout(Duration::from_secs(30)) {
                        Ok(event) => event,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            if state.strong_count() == 0 {
                                return;
                            }
                            continue;
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                    };
                    let Some(state) = state.upgrade() else {
                        return;
                    };
                    state.apply(event);
                }
            });
        if let Err(e) = spawned {
            tracing::warn!("API local sem eventos de sessão: {e}");
        }
    }

    fn apply(&self, event: SessionEvent) {
        let Ok(mut running) = self.running.write() else {
            return;
        };
        match event {
            SessionEvent::GameStarting { game, at } | SessionEvent::GameStarted { game, at } => {
                let pid = running
                    .as_ref()
                    .filter(|info| info.id == game.game_id)
                    .and_then(|info| info.pid);
                *running = Some(RunningGameInfo {
                    id: game.game_id,
                    title: game.title,
                    pid,
                    started_at: Some(at.to_rfc3339()),
                });
            }
            SessionEvent::GameExited { game, .. } => {
                if running.as_ref().is_some_and(|info| info.id == game.game_id) {
                    *running = None;
                }
            }
        }
    }
}

struct StateHandler {
    state: Arc<ApiState>,
    dispatch: Dispatch,
}

impl StateHandler {
    /// Mesmas checagens do link `tako://`: só ID conhecido da biblioteca, e a interface decide o resto (confirmações
    /// incluídas), exatamente como na navegação interativa.
    fn game_action(&self, id: &str, play: bool) -> Result<(), ApiActionError> {
        if !self.state.has_game(id) {
            return Err(ApiActionError::NotFound);
        }
        (self.dispatch)(Link::Game {
            id: id.to_owned(),
            play,
        })
    }
}

impl ApiHandler for StateHandler {
    fn list_games(&self) -> Vec<ApiGame> {
        self.state
            .games
            .read()
            .map(|games| games.clone())
            .unwrap_or_default()
    }

    fn game_state(&self) -> ApiGameState {
        let running = self.state.running();
        ApiGameState {
            is_game_running: running.is_some(),
            running_game: running,
        }
    }

    fn open_game(&self, id: &str) -> Result<(), ApiActionError> {
        self.game_action(id, false)
    }

    fn play_game(&self, id: &str) -> Result<(), ApiActionError> {
        self.game_action(id, true)
    }
}

/// As três chaves da API nas configurações (`LocalApiEnabled`, `LocalApiPort`, `LocalApiToken`).
#[derive(Debug, Clone, Copy)]
pub struct ApiSettings<'a> {
    pub enabled: bool,
    /// O texto guardado; vazio ou inválido usa a porta padrão.
    pub port: Option<&'a str>,
    pub token: &'a str,
}

/// O que quem usa deve fazer depois de [`LocalApi::sync`].
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Token novo para gravar na configuração (não havia ou o guardado era inválido).
    pub new_token: Option<String>,
    /// O token guardado era inválido e foi trocado: vale avisar o usuário.
    pub invalid_token_replaced: bool,
    /// A porta não abriu; uma vez por porta, para a mesma falha não repetir o aviso a cada sincronização.
    pub start_error: Option<String>,
}

/// O servidor da API local e o estado que ele responde.
pub struct LocalApi {
    state: Arc<ApiState>,
    server: Option<ServerHandle>,
    port: Option<u16>,
    warned_port: Option<u16>,
}

impl Default for LocalApi {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalApi {
    /// Desligada, com o estado já acompanhando os eventos de sessão.
    pub fn new() -> LocalApi {
        let state = ApiState::new();
        state.follow_session_events();
        LocalApi {
            state,
            server: None,
            port: None,
            warned_port: None,
        }
    }

    pub fn state(&self) -> &Arc<ApiState> {
        &self.state
    }

    /// A porta em que o servidor escuta (a efêmera, com a porta 0 configurada).
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn is_running(&self) -> bool {
        self.server.as_ref().is_some_and(ServerHandle::is_running)
    }

    /// Liga, troca de porta, atualiza o token ou desliga conforme as chaves. Desligada pela chave, nenhuma porta
    /// fica aberta.
    pub fn sync(&mut self, settings: ApiSettings<'_>, dispatch: &Dispatch) -> SyncReport {
        let mut report = SyncReport::default();
        if !settings.enabled {
            if self.server.is_some() {
                tracing::info!("desligando a API local do Tako");
            }
            self.shutdown();
            return report;
        }
        let port = settings
            .port
            .and_then(|text| text.parse().ok())
            .unwrap_or(DEFAULT_PORT);
        let token = if is_valid_token(settings.token) {
            settings.token.to_owned()
        } else {
            let fresh = generate_token();
            report.invalid_token_replaced = !settings.token.is_empty();
            report.new_token = Some(fresh.clone());
            fresh
        };

        // Já escutando na mesma porta (ou a efêmera, com a porta 0): só atualiza o token.
        let same_port = self
            .port
            .is_some_and(|actual| port == actual || (port == 0 && actual > 0));
        if let Some(server) = &self.server
            && same_port
            && server.is_running()
        {
            server.set_token(token);
            return report;
        }

        if let Some(old) = self.server.take() {
            old.stop();
        }
        let handler = Arc::new(StateHandler {
            state: self.state.clone(),
            dispatch: dispatch.clone(),
        });
        match start_server(Ipv4Addr::LOCALHOST, port, token, handler) {
            Ok(handle) => {
                let actual = handle.port();
                tracing::info!("API local do Tako escutando em http://127.0.0.1:{actual}");
                self.server = Some(handle);
                self.port = Some(actual);
                self.warned_port = None;
            }
            Err(e) => {
                tracing::error!("não foi possível iniciar a API local na porta {port}: {e}");
                self.port = None;
                if self.warned_port != Some(port) {
                    self.warned_port = Some(port);
                    report.start_error = Some(e.to_string());
                }
            }
        }
        report
    }

    /// Encerra o servidor, se estiver ativo.
    pub fn shutdown(&mut self) {
        if let Some(server) = self.server.take() {
            server.stop();
        }
        self.port = None;
        self.warned_port = None;
    }
}

impl Drop for LocalApi {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Com a API ligada, só o dono lê e grava `tako.db` (ADR-0033): no Unix o arquivo fica `0600`. No Windows, quem
/// protege é a pasta do perfil do usuário (risco residual registrado no ADR).
#[cfg(unix)]
pub fn protect_database_file(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if path.exists()
        && let Ok(metadata) = std::fs::metadata(path)
    {
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        if let Err(e) = std::fs::set_permissions(path, perms) {
            tracing::warn!(path = %path.display(), error = %e, "não foi possível aplicar 0600 ao tako.db");
        }
    }
}

#[cfg(not(unix))]
pub fn protect_database_file(_path: &std::path::Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::GameStore;
    use crate::session::events::SessionGame;
    use std::sync::Mutex;

    const NO_TOKEN: &str = "";

    fn dispatch_into(sink: Arc<Mutex<Vec<Link>>>) -> Dispatch {
        Arc::new(move |link| {
            sink.lock().unwrap().push(link);
            Ok(())
        })
    }

    fn nothing() -> Dispatch {
        dispatch_into(Arc::default())
    }

    fn on<'a>(port: &'a str, token: &'a str) -> ApiSettings<'a> {
        ApiSettings {
            enabled: true,
            port: Some(port),
            token,
        }
    }

    fn game(id: &str) -> ApiGame {
        ApiGame {
            id: id.into(),
            title: "Hades".into(),
            store: "Steam".into(),
            installed: true,
        }
    }

    fn get(port: u16, path: &str, token: Option<&str>) -> Result<u16, u16> {
        let mut request = ureq::get(format!("http://127.0.0.1:{port}{path}"));
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        match request.call() {
            Ok(response) => Ok(response.status().as_u16()),
            Err(ureq::Error::StatusCode(code)) => Err(code),
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn stays_off_until_the_key_is_on_and_stops_when_it_goes_off() {
        let mut api = LocalApi::new();
        let off = ApiSettings {
            enabled: false,
            port: Some("0"),
            token: NO_TOKEN,
        };
        let report = api.sync(off, &nothing());
        assert_eq!(report, SyncReport::default());
        assert!(!api.is_running() && api.port().is_none());

        let report = api.sync(on("0", NO_TOKEN), &nothing());
        let token = report.new_token.expect("gera o token na primeira vez");
        assert!(is_valid_token(&token));
        assert!(!report.invalid_token_replaced);
        assert!(api.is_running());
        let port = api.port().unwrap();
        assert!(port > 0);
        assert_eq!(get(port, "/api/v1/games", Some(&token)), Ok(200));
        assert_eq!(get(port, "/api/v1/games", None), Err(401));
        assert_eq!(get(port, "/api/v1/games", Some(&"0".repeat(32))), Err(401));

        // Mesma porta e o token que a configuração já tem: nada muda e nada novo para gravar.
        let report = api.sync(on("0", &token), &nothing());
        assert_eq!(report, SyncReport::default());
        assert_eq!(api.port(), Some(port));

        api.sync(off, &nothing());
        assert!(!api.is_running());
        assert!(
            std::net::TcpStream::connect(("127.0.0.1", port)).is_err(),
            "chave desligada: a porta não fica aberta"
        );
    }

    #[test]
    fn invalid_stored_token_is_replaced_and_reported() {
        let mut api = LocalApi::new();
        let report = api.sync(on("0", "curto"), &nothing());
        assert!(report.invalid_token_replaced);
        assert!(is_valid_token(&report.new_token.unwrap()));
    }

    #[test]
    fn rotating_the_token_closes_the_old_one() {
        let mut api = LocalApi::new();
        let old = generate_token();
        api.sync(on("0", &old), &nothing());
        let port = api.port().unwrap();
        assert_eq!(get(port, "/api/v1/state", Some(&old)), Ok(200));
        let fresh = generate_token();
        api.sync(on("0", &fresh), &nothing());
        assert_eq!(get(port, "/api/v1/state", Some(&old)), Err(401));
        assert_eq!(get(port, "/api/v1/state", Some(&fresh)), Ok(200));
    }

    #[test]
    fn a_busy_port_warns_once() {
        let busy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = busy.local_addr().unwrap().port().to_string();
        let mut api = LocalApi::new();
        let first = api.sync(on(&port, &generate_token()), &nothing());
        assert!(first.start_error.is_some());
        assert!(!api.is_running());
        let again = api.sync(on(&port, &generate_token()), &nothing());
        assert!(again.start_error.is_none(), "o mesmo aviso não se repete");
        drop(busy);
    }

    #[test]
    fn actions_go_through_the_same_link_as_tako_urls() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let mut api = LocalApi::new();
        api.state().set_games(vec![game("steam:570")]);
        let token = generate_token();
        api.sync(on("0", &token), &dispatch_into(seen.clone()));
        let port = api.port().unwrap();
        let post = |path: &str| match ureq::post(format!("http://127.0.0.1:{port}{path}"))
            .header("Authorization", format!("Bearer {token}"))
            .send_empty()
        {
            Ok(response) => response.status().as_u16(),
            Err(ureq::Error::StatusCode(code)) => code,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(post("/api/v1/games/steam:570/open"), 200);
        assert_eq!(post("/api/v1/games/steam:570/play"), 200);
        assert_eq!(post("/api/v1/games/inexistente/play"), 404);
        assert_eq!(
            *seen.lock().unwrap(),
            vec![
                Link::Game {
                    id: "steam:570".into(),
                    play: false
                },
                Link::Game {
                    id: "steam:570".into(),
                    play: true
                },
            ],
            "o id desconhecido nem chega à interface"
        );
    }

    #[test]
    fn an_unavailable_interface_is_a_503() {
        let unavailable: Dispatch = Arc::new(|_| Err(ApiActionError::Unavailable));
        let mut api = LocalApi::new();
        api.state().set_games(vec![game("steam:570")]);
        let token = generate_token();
        api.sync(on("0", &token), &unavailable);
        let code = match ureq::post(format!(
            "http://127.0.0.1:{}/api/v1/games/steam:570/open",
            api.port().unwrap()
        ))
        .header("Authorization", format!("Bearer {token}"))
        .send_empty()
        {
            Err(ureq::Error::StatusCode(code)) => code,
            other => panic!("{other:?}"),
        };
        assert_eq!(code, 503);
    }

    #[test]
    fn session_events_update_the_running_game() {
        let state = ApiState::new();
        state.follow_session_events();
        let at = chrono::Utc::now();
        let session_game = SessionGame {
            game_id: "test:portal-eventos".into(),
            title: "Portal".into(),
            store: GameStore::Steam,
        };
        events::publish(SessionEvent::GameStarting {
            game: session_game.clone(),
            at,
        });
        let wait = |done: &dyn Fn() -> bool| {
            for _ in 0..200 {
                if done() {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            false
        };
        assert!(wait(&|| state.running().is_some()));
        let running = state.running().unwrap();
        assert_eq!(
            (running.id.as_str(), running.title.as_str()),
            ("test:portal-eventos", "Portal")
        );
        assert_eq!(running.started_at, Some(at.to_rfc3339()));

        events::publish(SessionEvent::GameExited {
            game: session_game,
            at: chrono::Utc::now(),
            played: Some(Duration::from_secs(120)),
            exit_code: Some(0),
        });
        assert!(wait(&|| state.running().is_none()));
    }

    #[test]
    fn reconcile_keeps_the_real_start_of_the_same_game() {
        let state = ApiState::new();
        let info = |id: &str, started: Option<&str>| RunningGameInfo {
            id: id.into(),
            title: "Jogo".into(),
            pid: Some(7),
            started_at: started.map(str::to_owned),
        };
        state.reconcile_running(Some(info("a", Some("2026-10-24T10:00:00+00:00"))));
        // A interface recalcula o início (relógio andou): o real, já informado, fica.
        state.reconcile_running(Some(info("a", Some("2026-10-24T10:05:00+00:00"))));
        assert_eq!(
            state.running().unwrap().started_at.as_deref(),
            Some("2026-10-24T10:00:00+00:00")
        );
        // Outro jogo: vale o dele.
        state.reconcile_running(Some(info("b", Some("2026-10-24T11:00:00+00:00"))));
        assert_eq!(
            state.running().unwrap().started_at.as_deref(),
            Some("2026-10-24T11:00:00+00:00")
        );
        state.reconcile_running(None);
        assert!(state.running().is_none());
    }
}
