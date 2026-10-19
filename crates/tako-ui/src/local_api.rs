//! Integração do runtime da interface com a API local (EXT-002, ADR-0030).
//!
//! Gerencia o ciclo de vida do servidor HTTP local, mantendo o estado sincronizado com a
//! biblioteca e a execução de jogos, e despachando ações de abrir/jogar para a thread da UI.

use crate::shell::Ctx;
use crate::shell::ctx::keys;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex, RwLock};
use tako_core::api::{
    ApiActionError, ApiGame, ApiGameState, ApiHandler, RunningGameInfo, ServerHandle,
    generate_token, start_server,
};

struct SharedState {
    games: RwLock<Vec<ApiGame>>,
    running: RwLock<Option<RunningGameInfo>>,
}

struct LocalApiHandler {
    state: Arc<SharedState>,
}

impl ApiHandler for LocalApiHandler {
    fn list_games(&self) -> Vec<ApiGame> {
        self.state
            .games
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    fn game_state(&self) -> ApiGameState {
        let running = self.state.running.read().ok().and_then(|r| r.clone());
        ApiGameState {
            is_game_running: running.is_some(),
            running_game: running,
        }
    }

    fn open_game(&self, id: &str) -> Result<(), ApiActionError> {
        let found = self
            .state
            .games
            .read()
            .map(|g| g.iter().any(|item| item.id == id))
            .unwrap_or(false);

        if !found {
            return Err(ApiActionError::NotFound);
        }

        let link = format!("tako://game/{id}");
        let _ = slint::invoke_from_event_loop(move || {
            crate::app::open_current_link(&link);
        });

        Ok(())
    }

    fn play_game(&self, id: &str) -> Result<(), ApiActionError> {
        let found = self
            .state
            .games
            .read()
            .map(|g| g.iter().any(|item| item.id == id))
            .unwrap_or(false);

        if !found {
            return Err(ApiActionError::NotFound);
        }

        let link = format!("tako://play/{id}");
        let _ = slint::invoke_from_event_loop(move || {
            crate::app::open_current_link(&link);
        });

        Ok(())
    }
}

static SERVER: Mutex<Option<ServerHandle>> = Mutex::new(None);
static RUNNING_PORT: Mutex<Option<u16>> = Mutex::new(None);
static SHARED_STATE: std::sync::OnceLock<Arc<SharedState>> = std::sync::OnceLock::new();

fn get_shared_state() -> Arc<SharedState> {
    SHARED_STATE
        .get_or_init(|| {
            // Ponto de ligação com EXT-001 (session::events):
            // O estado de jogos e sessões é sincronizado por sync(&mut ctx).
            // Para notificações push futuras (SSE / WebSocket), conectar
            // aqui a tako_core::session::events::subscribe().
            Arc::new(SharedState {
                games: RwLock::new(Vec::new()),
                running: RwLock::new(None),
            })
        })
        .clone()
}

/// Sincroniza o servidor e o estado compartilhado com o contexto da UI.
pub fn sync(ctx: &mut Ctx) {
    let state = get_shared_state();

    // 1. Atualiza lista de jogos
    let api_games: Vec<ApiGame> = ctx
        .library
        .games()
        .iter()
        .map(|g| ApiGame {
            id: g.id.clone(),
            title: g.title.clone(),
            store: g.store().key().to_string(),
            installed: g.is_installed,
        })
        .collect();

    if let Ok(mut g) = state.games.write() {
        *g = api_games;
    }

    // 2. Atualiza estado de jogo em execução
    let running_info = ctx.running.as_ref().map(|r| RunningGameInfo {
        id: r.id.clone(),
        title: r.title.clone(),
        pid: r.pid,
        started_at: r.started.map(|_| chrono::Utc::now().to_rfc3339()),
    });

    if let Ok(mut r) = state.running.write() {
        *r = running_info;
    }

    // 3. Gerencia o ciclo de vida do servidor
    let enabled = ctx.settings.bool(keys::API_ENABLED, false);
    let mut server_lock = SERVER.lock().unwrap_or_else(|e| e.into_inner());
    let mut port_lock = RUNNING_PORT.lock().unwrap_or_else(|e| e.into_inner());

    if !enabled {
        if let Some(s) = server_lock.take() {
            tracing::info!("desligando a API local do Tako");
            s.stop();
        }
        *port_lock = None;
        return;
    }

    let port = ctx
        .settings
        .get(keys::API_PORT)
        .and_then(|s| s.parse().ok())
        .unwrap_or(tako_core::api::DEFAULT_PORT);

    let mut token = ctx.settings.string(keys::API_TOKEN);
    if token.is_empty() {
        token = generate_token();
        ctx.settings.set(keys::API_TOKEN, &token);
    }

    // Se já está rodando na mesma porta (ou porta 0 efêmera alocada), apenas atualiza o token
    let port_matches = match *port_lock {
        Some(actual) => (port == 0 && actual > 0) || port == actual,
        None => false,
    };
    if let Some(s) = server_lock.as_ref()
        && port_matches
        && s.is_running()
    {
        s.set_token(token);
        return;
    }

    // Caso contrário, (re)inicia na porta configurada
    if let Some(old) = server_lock.take() {
        old.stop();
    }

    let handler = Arc::new(LocalApiHandler {
        state: state.clone(),
    });

    match start_server(Ipv4Addr::LOCALHOST, port, token, handler) {
        Ok(handle) => {
            let actual_port = handle.port();
            tracing::info!("API local do Tako escutando em http://127.0.0.1:{actual_port}");
            *server_lock = Some(handle);
            *port_lock = Some(actual_port);
        }
        Err(e) => {
            tracing::error!("não foi possível iniciar a API local na porta {port}: {e}");
            ctx.toast(format!("Falha na API local: {e}"));
        }
    }
}

/// Diz se o servidor da API local está rodando.
pub fn is_running() -> bool {
    SERVER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .is_some_and(|s| s.is_running())
}

/// Encerra o servidor caso esteja ativo.
pub fn shutdown() {
    let mut server = SERVER.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = server.take() {
        s.stop();
    }
    let mut port = RUNNING_PORT.lock().unwrap_or_else(|e| e.into_inner());
    *port = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    #[test]
    fn local_api_starts_when_enabled_and_stops_when_disabled() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);

        // Desligado por padrão
        assert!(!ctx.settings.bool(keys::API_ENABLED, false));
        sync(&mut ctx);
        assert!(!is_running());

        // Ligar com porta 0 (efêmera para o teste)
        ctx.settings.set_bool(keys::API_ENABLED, true);
        ctx.settings.set(keys::API_PORT, "0");
        sync(&mut ctx);
        assert!(is_running());

        let token = ctx.settings.string(keys::API_TOKEN);
        assert!(!token.is_empty());

        let port = RUNNING_PORT.lock().unwrap().unwrap();
        assert!(port > 0);

        // Chamada real HTTP ao servidor sincronizado
        let res = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/games"))
            .header("Authorization", &format!("Bearer {token}"))
            .call()
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);

        // Estado inicial
        let state_res: ApiGameState = ureq::get(&format!("http://127.0.0.1:{port}/api/v1/state"))
            .header("Authorization", &format!("Bearer {token}"))
            .call()
            .unwrap()
            .body_mut()
            .read_json()
            .unwrap();
        assert!(!state_res.is_game_running);

        // Ações em jogo existente (a demo tem jogos)
        let game_id = ctx.library.games()[0].id.clone();
        let open_res = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/{game_id}/open"
        ))
        .header("Authorization", &format!("Bearer {token}"))
        .send_empty()
        .unwrap();
        assert_eq!(open_res.status().as_u16(), 200);

        let play_res = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/{game_id}/play"
        ))
        .header("Authorization", &format!("Bearer {token}"))
        .send_empty()
        .unwrap();
        assert_eq!(play_res.status().as_u16(), 200);

        // Ação em jogo inexistente -> 404
        let not_found_res = ureq::post(&format!(
            "http://127.0.0.1:{port}/api/v1/games/inexistente/play"
        ))
        .header("Authorization", &format!("Bearer {token}"))
        .send_empty()
        .unwrap_err();
        if let ureq::Error::StatusCode(code) = not_found_res {
            assert_eq!(code, 404);
        } else {
            panic!("Esperado 404");
        }

        // Atualização de estado com jogo em execução
        ctx.running = Some(crate::shell::ctx::RunningGame {
            id: game_id.clone(),
            title: "Demo Game".into(),
            pid: Some(999),
            started: Some(std::time::Instant::now()),
        });
        sync(&mut ctx);

        let running_state: ApiGameState =
            ureq::get(&format!("http://127.0.0.1:{port}/api/v1/state"))
                .header("Authorization", &format!("Bearer {token}"))
                .call()
                .unwrap()
                .body_mut()
                .read_json()
                .unwrap();
        assert!(running_state.is_game_running);
        assert_eq!(running_state.running_game.unwrap().id, game_id);

        // Desligar
        ctx.settings.set_bool(keys::API_ENABLED, false);
        sync(&mut ctx);
        assert!(!is_running());

        shutdown();
    }
}
