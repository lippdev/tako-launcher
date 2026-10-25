//! Integração do runtime da interface com a API local (EXT-002, ADR-0033).
//!
//! O ciclo de vida do servidor, o estado e o token são de `tako_core::api::runtime` (os mesmos do núcleo da versão
//! web). Aqui ficam o que é da interface Slint: a lista de jogos da biblioteca, o jogo aberto, os avisos na tela e o
//! despacho das ações de abrir e jogar para a thread da UI.

use crate::shell::Ctx;
use crate::shell::ctx::keys;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use tako_core::api::runtime::{ApiSettings, Dispatch, LocalApi, protect_database_file};
use tako_core::api::{ApiActionError, ApiGame, RunningGameInfo};

static API: LazyLock<Mutex<LocalApi>> = LazyLock::new(|| Mutex::new(LocalApi::new()));

fn api() -> MutexGuard<'static, LocalApi> {
    API.lock().unwrap_or_else(|e| e.into_inner())
}

/// Entrega o link do jogo à thread da interface; fora do loop de eventos (interface fechada) é `Unavailable`.
fn dispatch() -> Dispatch {
    Arc::new(|link| {
        slint::invoke_from_event_loop(move || {
            crate::app::dispatch_link(link);
        })
        .map_err(|_| ApiActionError::Unavailable)
    })
}

/// Sincroniza o servidor e o estado compartilhado com o contexto da UI.
pub fn sync(ctx: &mut Ctx) {
    let mut api = api();

    // 1. Lista de jogos
    api.state().set_games(
        ctx.library
            .games()
            .iter()
            .map(|g| ApiGame {
                id: g.id.clone(),
                title: g.title.clone(),
                store: g.store().key().to_string(),
                installed: g.is_installed,
            })
            .collect(),
    );

    // 2. Jogo em execução, com o timestamp real de início (ou nenhum)
    api.state()
        .reconcile_running(ctx.running.as_ref().map(|r| RunningGameInfo {
            id: r.id.clone(),
            title: r.title.clone(),
            pid: r.pid,
            started_at: r.started.and_then(|inst| {
                chrono::Duration::from_std(inst.elapsed())
                    .ok()
                    .map(|dur| (chrono::Utc::now() - dur).to_rfc3339())
            }),
        }));

    // 3. Ciclo de vida do servidor
    let enabled = ctx.settings.bool(keys::API_ENABLED, false);
    if enabled {
        protect_database_file(&ctx.paths.database());
    }
    let token = ctx.settings.string(keys::API_TOKEN);
    let report = api.sync(
        ApiSettings {
            enabled,
            port: ctx.settings.get(keys::API_PORT),
            token: &token,
        },
        &dispatch(),
    );
    if let Some(token) = report.new_token {
        ctx.settings.set(keys::API_TOKEN, &token);
    }
    if report.invalid_token_replaced {
        ctx.toast_error(ctx.t("settings.system.api_invalid_token"));
    }
    if let Some(error) = report.start_error {
        ctx.toast_error(ctx.tf("settings.system.api_start_failed", &[("error", &error)]));
    }
}

/// Diz se o servidor da API local está rodando.
pub fn is_running() -> bool {
    api().is_running()
}

/// Encerra o servidor caso esteja ativo.
pub fn shutdown() {
    api().shutdown();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::api::ApiGameState;
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
        assert!(
            ctx.toast.is_none(),
            "não deve repetir o toast com a mesma porta ocupada"
        );

        drop(listener);
        shutdown();
    }

    #[test]
    fn local_api_starts_when_enabled_and_stops_when_disabled() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        api().state().reconcile_running(None);
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

        let port = api().port().unwrap();
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
    fn dispatch_outside_the_event_loop_is_unavailable() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // Fora do loop de eventos do Slint, invoke_from_event_loop falha: a API responde 503.
        for play in [false, true] {
            let link = tako_core::links::Link::Game {
                id: "test:hades".into(),
                play,
            };
            assert_eq!(dispatch()(link), Err(ApiActionError::Unavailable));
        }
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
