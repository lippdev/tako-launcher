//! API local de integração no núcleo da versão web (EXT-002, ADR-0033, WEB-34).
//!
//! É a mesma do Slint: o servidor, o contrato, a porta, o token, a autenticação e os limites são de
//! `tako_core::api` (`runtime`). Aqui só se liga às chaves do núcleo (`LocalApiEnabled`, `LocalApiPort`,
//! `LocalApiToken`), à lista de jogos e à entrega das ações. **Não é a ponte da interface** (D-09): quem fala com a
//! interface continua sendo o pipe entre os dois processos; esta porta (só em `127.0.0.1`, com token) é para
//! integrações externas e fica fechada com a chave desligada, que vem desligada.

use crate::AppService;
use crate::settings;
use tako_core::api::runtime::{ApiSettings, Dispatch, LocalApi, protect_database_file};
use tako_core::api::{API_ENABLED, API_PORT, API_TOKEN, ApiActionError, ApiGame};

/// O que já foi entregue ao servidor, para só refazer quando algo mudar.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Synced {
    enabled: bool,
    port: Option<String>,
    token: String,
    library: u64,
}

#[derive(Default)]
pub(crate) struct WebApi {
    /// Só existe depois da primeira vez que a chave está ligada: desligada, nada assina evento nem abre porta.
    api: Option<LocalApi>,
    dispatch: Option<Dispatch>,
    synced: Option<Synced>,
}

fn unavailable() -> Dispatch {
    std::sync::Arc::new(|_| Err(ApiActionError::Unavailable))
}

impl AppService {
    /// Por onde a API local entrega abrir e jogar (o hospedeiro leva o link à thread do núcleo). Sem isto, as ações
    /// respondem `503`.
    pub fn set_link_dispatch(&mut self, dispatch: Dispatch) {
        self.local_api.dispatch = Some(dispatch);
        self.local_api.synced = None;
    }

    /// Liga, atualiza ou desliga a API local conforme as chaves de configuração e mantém a lista de jogos que ela
    /// responde. Barata quando nada mudou: o hospedeiro chama a cada volta do laço.
    pub fn sync_local_api(&mut self) {
        let Some(core) = self.core.as_ref() else {
            return;
        };
        let enabled = !core.simulate && core.settings.get(API_ENABLED).is_some_and(|v| v == "true");
        let wanted = Synced {
            enabled,
            port: core.settings.get(API_PORT).cloned(),
            token: core.settings.get(API_TOKEN).cloned().unwrap_or_default(),
            library: core.library.version(),
        };
        if self.local_api.synced.as_ref() == Some(&wanted) {
            return;
        }
        if !enabled && self.local_api.api.is_none() {
            // Nunca ligou: nada a desligar.
            self.local_api.synced = Some(wanted);
            return;
        }
        let api = self.local_api.api.get_or_insert_with(LocalApi::new);
        api.state().set_games(
            core.library
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
        if enabled {
            protect_database_file(&core.session.paths().database());
        }
        let dispatch = self.local_api.dispatch.clone().unwrap_or_else(unavailable);
        let report = api.sync(
            ApiSettings {
                enabled,
                port: wanted.port.as_deref(),
                token: &wanted.token,
            },
            &dispatch,
        );
        if report.invalid_token_replaced {
            tracing::warn!("o token da API local era inválido e foi trocado");
        }
        if let Some(error) = report.start_error {
            tracing::warn!("API local não iniciou: {error}");
        }
        let mut wanted = wanted;
        if let Some(token) = report.new_token
            && let Some(core) = self.core.as_mut()
        {
            // O token é um segredo local da máquina (ADR-0033): nunca entra em log.
            match settings::save(core, API_TOKEN, &token) {
                Ok(()) => wanted.token = token,
                Err(e) => tracing::warn!(
                    "não foi possível guardar o token da API local: {}",
                    e.message
                ),
            }
        }
        self.local_api.synced = Some(wanted);
    }

    /// A porta em que a API local escuta, se estiver ligada.
    pub fn local_api_port(&self) -> Option<u16> {
        self.local_api.api.as_ref().and_then(LocalApi::port)
    }
}
