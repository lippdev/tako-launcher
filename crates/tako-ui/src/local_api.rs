//! Integração do runtime da interface com a API local (EXT-002, ADR-0033).
//!
//! Gerencia o ciclo de vida do servidor HTTP local, mantendo o estado sincronizado com a
//! biblioteca e a execução de jogos, e despachando ações de abrir/jogar para a thread da UI.

use crate::shell::Ctx;
use crate::shell::ctx::keys;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex, RwLock};
use tako_core::api::{
    ApiActionError, ApiGame, ApiGameState, ApiHandler, RunningGameInfo, ServerHandle,
    generate_token, is_valid_token, start_server,
};
use tako_core::session::events::{self, SessionEvent};

struct SharedState {
    games: RwLock<Vec<ApiGame>>,
    running: RwLock<Option<RunningGameInfo>>,
}

pub struct LocalApiHandler {
    state: Arc<SharedState>,
}

impl LocalApiHandler {
    fn dispatch_game_action(&self, id: &str, play: bool) -> Result<(), ApiActionError> {
        let found = self
            .state
            .games
            .read()
            .map(|g| g.iter().any(|item| item.id == id))
            .unwrap_or(false);

        if !found {
            return Err(ApiActionError::NotFound);
        }

        let link = tako_core::links::Link::Game {
            id: id.to_string(),
            play,
        };

        slint::invoke_from_event_loop(move || {
            crate::app::dispatch_link(link);
        })
        .map_err(|_| ApiActionError::Unavailable)?;

        Ok(())
    }
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
        self.dispatch_game_action(id, false)
    }

    fn play_game(&self, id: &str) -> Result<(), ApiActionError> {
        self.dispatch_game_action(id, true)
    }
}

static SERVER: Mutex<Option<ServerHandle>> = Mutex::new(None);
static RUNNING_PORT: Mutex<Option<u16>> = Mutex::new(None);
static LAST_WARNED_ERROR_PORT: Mutex<Option<u16>> = Mutex::new(None);
static SHARED_STATE: std::sync::OnceLock<Arc<SharedState>> = std::sync::OnceLock::new();

fn get_shared_state() -> Arc<SharedState> {
    SHARED_STATE
        .get_or_init(|| {
            let state = Arc::new(SharedState {
                games: RwLock::new(Vec::new()),
                running: RwLock::new(None),
            });

            // Cria a assinatura de eventos de forma síncrona antes de iniciar a thread,
            // garantindo que nenhum evento publicado seja perdido logo após a inicialização.
            let rx = events::subscribe();
            let state_clone = state.clone();
            std::thread::Builder::new()
                .name("tako-api-events".into())
                .spawn(move || {
                    while let Ok(event) = rx.recv() {
                        match event {
                            SessionEvent::GameStarting { game, at }
                            | SessionEvent::GameStarted { game, at } => {
                                if let Ok(mut r) = state_clone.running.write() {
                                    let pid = r
                                        .as_ref()
                                        .filter(|info| info.id == game.game_id)
                                        .and_then(|info| info.pid);
                                    *r = Some(RunningGameInfo {
                                        id: game.game_id,
                                        title: game.title,
                                        pid,
                                        started_at: Some(at.to_rfc3339()),
                                    });
                                }
                            }
                            SessionEvent::GameExited { game, .. } => {
                                if let Ok(mut r) = state_clone.running.write()
                                    && r.as_ref().is_some_and(|info| info.id == game.game_id)
                                {
                                    *r = None;
                                }
                            }
                        }
                    }
                })
                .ok();

            state
        })
        .clone()
}

#[cfg(unix)]
fn protect_db_file(path: &std::path::Path) {
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
fn protect_db_file(_path: &std::path::Path) {}

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

    // 2. Atualiza estado de jogo em execução usando o timestamp real de início (ou None)
    let running_info = ctx.running.as_ref().map(|r| {
        let started_at = r.started.and_then(|inst| {
            let elapsed = inst.elapsed();
            chrono::Duration::from_std(elapsed)
                .ok()
                .map(|dur| (chrono::Utc::now() - dur).to_rfc3339())
        });
        RunningGameInfo {
            id: r.id.clone(),
            title: r.title.clone(),
            pid: r.pid,
            started_at,
        }
    });

    if let Ok(mut r) = state.running.write() {
        if let (Some(mut current), Some(existing)) = (running_info.clone(), r.as_ref()) {
            if current.id == existing.id && existing.started_at.is_some() {
                current.started_at = existing.started_at.clone();
            }
            *r = Some(current);
        } else if ctx.running.is_none() {
            *r = None;
        } else {
            *r = running_info;
        }
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
        let mut warned = LAST_WARNED_ERROR_PORT.lock().unwrap_or_else(|e| e.into_inner());
        *warned = None;
        return;
    }

    protect_db_file(&ctx.paths.database());

    let port = ctx
        .settings
        .get(keys::API_PORT)
        .and_then(|s| s.parse().ok())
        .unwrap_or(tako_core::api::DEFAULT_PORT);

    let mut token = ctx.settings.string(keys::API_TOKEN);
    if token.is_empty() || !is_valid_token(&token) {
        let had_invalid = !token.is_empty();
        token = generate_token();
        ctx.settings.set(keys::API_TOKEN, &token);
        if had_invalid {
            ctx.toast(ctx.t("settings.system.api_invalid_token"));
        }
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
            let mut warned = LAST_WARNED_ERROR_PORT.lock().unwrap_or_else(|e| e.into_inner());
            *warned = None;
        }
        Err(e) => {
            tracing::error!("não foi possível iniciar a API local na porta {port}: {e}");
            let mut warned = LAST_WARNED_ERROR_PORT.lock().unwrap_or_else(|e| e.into_inner());
            if *warned != Some(port) {
                *warned = Some(port);
                ctx.toast(ctx.tf(
                    "settings.system.api_start_failed",
                    &[("error", &e.to_string())],
                ));
            }
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
    let mut warned = LAST_WARNED_ERROR_PORT.lock().unwrap_or_else(|e| e.into_inner());
    *warned = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::library::GameStore;
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    #[test]
    fn sync_does_not_repeat_toast_on_busy_port() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        shutdown();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let occupied_port = listener.local_addr().unwrap().port();

        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        ctx.settings.set_bool(keys::API_ENABLED, true);
        ctx.settings.set(keys::API_PORT, &occupied_port.to_string());

        // 1ª tentativa com a porta ocupada: deve emitir toast de erro
        sync(&mut ctx);
        assert!(!is_running());
        let first_toast = ctx.toast.take();
        assert!(first_toast.is_some(), "deve emitir toast na primeira falha");

        // 2ª tentativa (ex: edição subsequente na biblioteca): não deve emitir novo toast
        sync(&mut ctx);
        assert!(ctx.toast.is_none(), "não deve repetir o toast com a mesma porta ocupada");

        drop(listener);
        shutdown();
    }

    #[test]
    fn local_api_starts_when_enabled_and_stops_when_disabled() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(mut r) = get_shared_state().running.write() {
            *r = None;
        }
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

        // Ações em jogo inexistente -> 404
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

        // Desligar
        ctx.settings.set_bool(keys::API_ENABLED, false);
        sync(&mut ctx);
        assert!(!is_running());

        shutdown();
    }

    #[test]
    fn session_events_update_shared_state_reactively() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let state = get_shared_state();
        if let Ok(mut r) = state.running.write() {
            *r = None;
        }
        let at = chrono::Utc::now();
        let game = events::SessionGame {
            game_id: "test:portal".into(),
            title: "Portal".into(),
            store: GameStore::Steam,
        };

        // Evento GameStarting
        events::publish(SessionEvent::GameStarting {
            game: game.clone(),
            at,
        });

        // Espera breve para a thread do evento processar
        let mut running = None;
        for _ in 0..100 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            if let Ok(r) = state.running.read()
                && let Some(info) = r.as_ref()
                && info.id == "test:portal"
            {
                running = Some(info.clone());
                break;
            }
        }
        let running = running.expect("deve refletir o jogo que iniciou via evento");
        assert_eq!(running.id, "test:portal");
        assert_eq!(running.title, "Portal");
        assert_eq!(running.started_at, Some(at.to_rfc3339()));

        // Evento GameExited
        events::publish(SessionEvent::GameExited {
            game,
            at: chrono::Utc::now(),
            played: Some(std::time::Duration::from_secs(120)),
            exit_code: Some(0),
        });

        let mut exited = false;
        for _ in 0..100 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            if let Ok(r) = state.running.read()
                && r.is_none()
            {
                exited = true;
                break;
            }
        }
        assert!(
            exited,
            "deve limpar o jogo em execução ao receber GameExited"
        );
    }

    #[test]
    fn invoke_from_event_loop_failure_returns_unavailable() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let state = get_shared_state();
        if let Ok(mut g) = state.games.write() {
            g.push(ApiGame {
                id: "test:hades".into(),
                title: "Hades".into(),
                store: "Steam".into(),
                installed: true,
            });
        }
        let handler = LocalApiHandler { state };

        // Fora do loop de eventos do Slint, invoke_from_event_loop falha com erro
        let err_open = handler.open_game("test:hades");
        assert_eq!(err_open, Err(ApiActionError::Unavailable));

        let err_play = handler.play_game("test:hades");
        assert_eq!(err_play, Err(ApiActionError::Unavailable));
    }

    #[test]
    fn play_action_routes_through_open_link_and_requests_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, mut ctx) = crate::metadata::test_ctx(dir.path(), false);
        let exe = dir.path().join("jogo.exe");
        std::fs::write(&exe, "").unwrap();
        let record = tako_core::library::ManualGameRecord::create(
            "Jogo",
            &exe.to_string_lossy(),
            chrono::Utc::now(),
        );
        let id = record.id.clone();
        ctx.library.add_manual(record).unwrap();

        // Configura comando não aprovado no jogo para exigir confirmação
        let mut shell = crate::shell::Shell::new(ctx);
        if let Some(game_settings) = &shell.ctx.game_settings {
            let over = tako_core::session::settings::GameSettingsOverride {
                before_command: Some("montar.bat".into()),
                approved_commands: None,
                ..Default::default()
            };
            game_settings.save(&id, &over).unwrap();
        }

        // 1. Executar tako://play/id (mesma rota que POST /games/:id/play despacha)
        let link_play = tako_core::links::Link::Game {
            id: id.clone(),
            play: true,
        };
        shell.open_link(link_play);

        // Deve abrir details e abrir a camada de diálogo para aprovação de comandos
        assert_eq!(shell.page().kind(), "details");
        assert!(
            shell.has_layers(),
            "Deve pedir confirmação exibindo camada de diálogo"
        );

        // 2. Executar tako://game/id (mesma rota que POST /games/:id/open despacha)
        shell.close_game_layers();
        let link_open = tako_core::links::Link::Game {
            id: id.clone(),
            play: false,
        };
        shell.open_link(link_open);
        assert_eq!(shell.page().kind(), "details");
        assert!(
            !shell.has_layers(),
            "Abrir tela de jogo não deve exibir diálogo de confirmação"
        );
    }
}
