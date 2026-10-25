//! Links `tako://` na versão web (EXT-005, WEB-34): do navegador, de outro Tako que abriu em seguida ou da API local.
//!
//! A validação é a do Slint (`tako_core::links::parse`: só telas conhecidas e IDs da biblioteca, nunca caminho,
//! comando ou outra URL). Aqui um link já válido vira o que a interface mostra: o estado de tela (`ui.state`, D-10)
//! passa a apontar o destino e o hospedeiro abre ou reabre a interface nele. Com `play`, o jogo só abre direto quando
//! o Slint também abriria sem perguntar; do contrário a página do jogo aparece e a confirmação é do usuário.

use crate::AppService;
use crate::protocol::{GameRequest, Store};
use serde_json::{Map, Value, json};
use tako_core::library::Game;
use tako_core::links::Link;

/// Versão do formato do estado de tela, de `web/src/app/uiState.ts` (`SCREEN_STATE_VERSION`).
const SCREEN_STATE_VERSION: u64 = 1;

/// O que o hospedeiro faz depois de [`AppService::open_link`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOutcome {
    /// Jogo que a biblioteca não conhece: nada acontece.
    Missing,
    /// A leitura das lojas ainda não terminou e o jogo não está no que já se sabe: o link espera e volta por
    /// [`AppService::resume_pending_link`].
    Waiting,
    /// O estado de tela aponta o destino: a interface precisa mostrá-lo (abrir, se estiver fechada; reabrir, se
    /// estiver aberta noutro lugar).
    Show,
    /// Feito sem mexer na janela: o jogo abriu (ou voltou para a frente) e a interface reabre nele quando terminar.
    Handled,
}

/// Se um `tako://play` abre o jogo na hora ou deixa a decisão para o usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayNow {
    Launch,
    /// O próprio jogo já está aberto: volta para ele.
    Resume,
    /// Pede confirmação ou ação do usuário (outro jogo aberto, jogo quebrado, cliente ausente, comandos sem
    /// aprovação, jogo não instalado): mostra a página do jogo.
    Ask,
}

impl AppService {
    /// Executa um link válido. Chamado na thread do núcleo, com o link vindo da linha de comando, do arquivo de
    /// ativação do segundo Tako ou da API local.
    pub fn open_link(&mut self, link: Link) -> LinkOutcome {
        if self.core.is_none() {
            return LinkOutcome::Missing;
        }
        match link {
            Link::Screen(route) => {
                self.show_screen(route);
                LinkOutcome::Show
            }
            Link::Game { id, play } => self.open_game_link(id, play),
        }
    }

    /// Um link que esperava a leitura das lojas e já pode valer (a leitura terminou).
    pub fn resume_pending_link(&mut self) -> Option<LinkOutcome> {
        if self.pending_link.is_none() || self.scanning() {
            return None;
        }
        let link = self.pending_link.take()?;
        Some(self.open_link(link))
    }

    fn scanning(&self) -> bool {
        self.core.as_ref().is_some_and(|c| c.sources.state.scanning)
    }

    fn open_game_link(&mut self, id: String, play: bool) -> LinkOutcome {
        let known = self
            .core
            .as_ref()
            .is_some_and(|c| c.library.find(&id).is_some());
        if !known {
            if self.scanning() {
                tracing::info!("link para jogo à espera da leitura das lojas");
                self.pending_link = Some(Link::Game { id, play });
                return LinkOutcome::Waiting;
            }
            tracing::info!("link para um jogo que a biblioteca não conhece");
            return LinkOutcome::Missing;
        }
        self.show_game(&id);
        if !play {
            return LinkOutcome::Show;
        }
        match self.play_now(&id) {
            PlayNow::Launch => match self.play(GameRequest { game_id: id }) {
                Ok(_) => LinkOutcome::Handled,
                Err(e) => {
                    tracing::info!("link para jogar: {}", e.message);
                    LinkOutcome::Show
                }
            },
            PlayNow::Resume => match self.resume() {
                Ok(_) => LinkOutcome::Handled,
                Err(e) => {
                    tracing::info!("link para voltar ao jogo: {}", e.message);
                    LinkOutcome::Show
                }
            },
            PlayNow::Ask => LinkOutcome::Show,
        }
    }

    /// As mesmas condições de `details::primary_action` do Slint para abrir sem diálogo.
    fn play_now(&self, id: &str) -> PlayNow {
        let Some(core) = self.core.as_ref() else {
            return PlayNow::Ask;
        };
        let Some(game) = core.library.find(id) else {
            return PlayNow::Ask;
        };
        match core.session.active_game() {
            Some(active) if active == id => return PlayNow::Resume,
            Some(_) => return PlayNow::Ask,
            None => {}
        }
        if game.is_broken
            || !game.can_play()
            || client_missing(core.sources.state.sources.iter(), game)
            || !core.session.commands_approved(&core.settings, id)
        {
            return PlayNow::Ask;
        }
        PlayNow::Launch
    }

    fn show_game(&mut self, id: &str) {
        let id = id.to_owned();
        self.edit_screen(move |state| {
            state.insert("view".into(), json!("biblioteca"));
            state.insert("activeGame".into(), json!(id));
        });
    }

    fn show_screen(&mut self, route: &str) {
        let route = route.to_owned();
        self.edit_screen(move |state| match route.as_str() {
            "settings" => {
                state.insert("view".into(), json!("configuracoes"));
            }
            "home" | "library:all" | "library:installed" | "library:favorites" => {
                state.insert("view".into(), json!("biblioteca"));
                state.insert("activeGame".into(), Value::Null);
                if let Some(filter) = route.strip_prefix("library:") {
                    let library = state
                        .entry("library")
                        .or_insert_with(|| json!({}))
                        .as_object_mut();
                    if let Some(library) = library {
                        library.insert("filter".into(), json!(filter));
                    }
                }
            }
            other => tracing::warn!(route = other, "tela de link desconhecida"),
        });
    }

    /// Muda só o que o link decide no estado de tela; o resto (filtros, categoria, lista recolhida) fica como o
    /// usuário deixou. Estado ausente, quebrado ou de outro formato recomeça do padrão.
    fn edit_screen(&mut self, edit: impl FnOnce(&mut Map<String, Value>)) {
        let current = self
            .ui_state
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok());
        let mut state = match current {
            Some(Value::Object(map))
                if map.get("v").and_then(Value::as_u64) == Some(SCREEN_STATE_VERSION) =>
            {
                map
            }
            _ => Map::from_iter([("v".to_owned(), json!(SCREEN_STATE_VERSION))]),
        };
        edit(&mut state);
        self.ui_state = Some(Value::Object(state).to_string());
    }
}

/// O cliente da loja do jogo não foi achado na última leitura (SRC-003): abrir o jogo só falharia, e o aviso é da
/// página do jogo.
fn client_missing<'a>(
    mut reports: impl Iterator<Item = &'a crate::protocol::LibrarySourceReport>,
    game: &Game,
) -> bool {
    let store = Store::from(game.store());
    reports.any(|r| r.store == store && r.enabled && !r.client_installed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Call, Reply, UiState};

    fn state_of(app: &mut AppService) -> Value {
        let Ok(Reply::UiState(UiState { json: Some(json) })) = app.handle(Call::UiState) else {
            panic!("sem estado de tela");
        };
        serde_json::from_str(&json).unwrap()
    }

    fn set_state(app: &mut AppService, json: &str) {
        app.handle(Call::UiStateSet(UiState {
            json: Some(json.into()),
        }))
        .unwrap();
    }

    /// Sem núcleo aberto, nenhum link faz nada (e não derruba nada).
    #[test]
    fn link_without_a_core_is_ignored() {
        let mut app = AppService::new("t");
        assert_eq!(
            app.open_link(Link::Screen("settings")),
            LinkOutcome::Missing
        );
        assert_eq!(app.ui_state, None);
    }

    #[test]
    fn editing_the_screen_keeps_what_the_user_had_and_resets_what_is_broken() {
        let mut app = AppService::new("t");
        set_state(
            &mut app,
            r#"{"v":1,"view":"configuracoes","listCollapsed":true,"activeGame":null,"settingsCategory":"theme","library":{"filter":"all","search":"zelda"}}"#,
        );
        app.show_screen("library:favorites");
        let state = state_of(&mut app);
        assert_eq!(state["view"], "biblioteca");
        assert_eq!(state["activeGame"], Value::Null);
        assert_eq!(state["library"]["filter"], "favorites");
        assert_eq!(state["library"]["search"], "zelda", "a busca continua");
        assert_eq!(state["listCollapsed"], true);
        assert_eq!(state["settingsCategory"], "theme");

        app.show_game("steam:570");
        let state = state_of(&mut app);
        assert_eq!(state["activeGame"], "steam:570");
        assert_eq!(state["library"]["filter"], "favorites");

        app.show_screen("settings");
        assert_eq!(state_of(&mut app)["view"], "configuracoes");
        assert_eq!(state_of(&mut app)["activeGame"], "steam:570");

        // Formato de outra versão ou texto quebrado: recomeça do padrão, sem erro.
        for broken in [r#"{"v":99,"view":"downloads"}"#, "não é json", "[1]"] {
            set_state(&mut app, broken);
            app.show_game("manual:abc");
            let state = state_of(&mut app);
            assert_eq!(state["v"], 1);
            assert_eq!(state["activeGame"], "manual:abc");
            assert!(state.get("view").is_some_and(|v| v == "biblioteca"));
        }
    }

    #[test]
    fn unknown_screen_changes_nothing() {
        let mut app = AppService::new("t");
        set_state(&mut app, r#"{"v":1,"view":"downloads"}"#);
        app.show_screen("nao-existe");
        assert_eq!(state_of(&mut app)["view"], "downloads");
    }
}
