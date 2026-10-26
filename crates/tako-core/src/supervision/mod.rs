//! Supervisor e resiliência (MIG-016), sem E/S de processo nem socket: protocolo e linha de comando
//! (port de `Supervision/SupervisorProtocol.cs`), política de reinício ([`policy`]), diário de sessão e
//! orçamento de falhas ([`journal`]) e o enquadramento do IPC ([`ipc`]). O socket e os processos ficam no
//! binário `tako` e em `tako-platform`.

pub mod ipc;
pub mod journal;
pub mod policy;

pub use policy::{
    HeartbeatMonitor, KillReason, SupervisorAction, SupervisorDecision, SupervisorOptions,
    SupervisorPolicy, UiExitKind, UiLaunchMode, UiWatch,
};

use serde::{Deserialize, Serialize};

/// Mensagens entre a UI e o supervisor.
pub mod messages {
    /// Evento UI → supervisor, enviado pela thread da UI: se ela trava, os heartbeats param.
    pub const HEARTBEAT: &str = "ui.heartbeat";
    /// Evento UI → supervisor: a UI vai sair de propósito (atualização, fim da sessão do SO); não reiniciar.
    pub const CLOSING: &str = "ui.closing";
    /// Comando UI → supervisor: estado do supervisor ([`super::SupervisorStatus`]).
    pub const STATUS: &str = "supervisor.status";
    pub const SUPERVISOR_PEER: &str = "tako-supervisor";
    pub const UI_PEER: &str = "tako-ui";
}

/// Por que a UI anunciou a saída.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UiCloseReason {
    /// O atualizador vai trocar o executável: o supervisor também precisa sair.
    Update,
    /// O SO está encerrando a sessão (ou o instalador pediu para fechar).
    SessionEnding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiHeartbeat {
    pub process_id: u32,
    pub sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiClosing {
    pub process_id: u32,
    pub reason: UiCloseReason,
}

/// Resposta de [`messages::STATUS`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorStatus {
    pub process_id: u32,
    pub restarts: u32,
    pub mode: UiLaunchMode,
}

/// Papel deste processo do executável `tako`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessRole {
    /// Processo leve, sem janela, que inicia e vigia a UI.
    Supervisor,
    Ui,
}

/// Linha de comando do executável único. Aberto normalmente, ele vira supervisor e se inicia de novo
/// com `--ui`. Fora das versões publicadas (`cargo run`, testes), roda a UI direto.
pub mod args {
    use super::{ProcessRole, UiLaunchMode};

    /// Este processo é a UI iniciada pelo supervisor.
    pub const UI: &str = "--ui";
    /// Endereço do socket do supervisor (acompanha [`UI`]).
    pub const PIPE: &str = "--supervisor-pipe";
    /// Abre a UI direto, sem supervisor (diagnóstico).
    pub const NO_SUPERVISOR: &str = "--no-supervisor";
    pub const SAFE_MODE: &str = "--safe-mode";
    /// Abre direto na tela cheia, sem perguntar (modo Xbox, atalhos do usuário). ADR-0027.
    pub const BIG_PICTURE: &str = "--big-picture";
    /// Aberto pelo Console Mode (ADR-0025, CMI-004): também vai direto para a tela cheia.
    pub const CONSOLE_SESSION: &str = "--console-session";
    /// Abre no modo desktop sem a pergunta do controle (interface reaberta pelo supervisor). ADR-0027.
    pub const DESKTOP: &str = "--desktop";
    /// `0` desliga e `1` liga o supervisor, em qualquer build.
    pub const ENV_VAR: &str = "TAKO_SUPERVISOR";

    pub fn has(args: &[String], flag: &str) -> bool {
        args.iter().any(|a| a.eq_ignore_ascii_case(flag))
    }

    pub fn resolve_role(args: &[String], env: Option<&str>, published_build: bool) -> ProcessRole {
        if has(args, UI) || has(args, NO_SUPERVISOR) {
            return ProcessRole::Ui;
        }
        match env.map(str::trim) {
            Some("0") => ProcessRole::Ui,
            Some("1") => ProcessRole::Supervisor,
            _ if published_build => ProcessRole::Supervisor,
            _ => ProcessRole::Ui,
        }
    }

    /// Endereço do supervisor passado à UI; `None` se a UI roda sozinha.
    pub fn pipe_name(args: &[String]) -> Option<&str> {
        args.windows(2)
            .find(|w| w[0].eq_ignore_ascii_case(PIPE) && !w[1].is_empty())
            .map(|w| w[1].as_str())
    }

    /// Argumentos do usuário repassados à UI (sem os que o supervisor controla).
    pub fn passthrough(args: &[String]) -> Vec<String> {
        let mut out = Vec::new();
        let mut it = args.iter();
        while let Some(a) = it.next() {
            if a.eq_ignore_ascii_case(PIPE) || a == "--open-url" {
                it.next(); // e o valor
            } else if a
                .get(..7)
                .is_some_and(|s| s.eq_ignore_ascii_case("tako://"))
            {
                // O link é usado uma vez; reiniciar a UI não pode repetir Jogar.
            } else if ![
                UI,
                SAFE_MODE,
                NO_SUPERVISOR,
                // O modo da interface é do supervisor, que acompanha as trocas (ADR-0027).
                BIG_PICTURE,
                CONSOLE_SESSION,
                DESKTOP,
            ]
            .iter()
            .any(|f| a.eq_ignore_ascii_case(f))
            {
                out.push(a.clone());
            }
        }
        out
    }

    /// Linha de comando da UI iniciada pelo supervisor.
    pub fn for_ui(pipe: &str, mode: UiLaunchMode, passthrough: &[String]) -> Vec<String> {
        let mut out = vec![UI.to_owned(), PIPE.to_owned(), pipe.to_owned()];
        if mode == UiLaunchMode::SafeMode {
            out.push(SAFE_MODE.to_owned());
        }
        out.extend_from_slice(passthrough);
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn a_link_is_not_replayed_when_the_ui_restarts() {
        let all = [
            "--open-url",
            "tako://play/steam:570",
            "--data-dir",
            "dados",
            "tako://home",
        ]
        .map(str::to_owned);
        assert_eq!(args::passthrough(&all), ["--data-dir", "dados"]);
    }

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn role_comes_from_arguments_environment_and_build() {
        use ProcessRole::*;
        let cases: [(&[&str], Option<&str>, bool, ProcessRole); 8] = [
            (&[], None, true, Supervisor),
            (&[], None, false, Ui),
            (&["--ui", "--supervisor-pipe", "p"], None, true, Ui),
            (&["--no-supervisor"], None, true, Ui),
            (&[], Some("0"), true, Ui),
            (&[], Some("1"), false, Supervisor),
            (&["--UI"], Some("1"), true, Ui),
            (&["--safe-mode"], None, true, Supervisor),
        ];
        for (a, env, published, expected) in cases {
            assert_eq!(args::resolve_role(&v(a), env, published), expected, "{a:?}");
        }
    }

    #[test]
    fn ui_command_line_carries_pipe_mode_and_user_arguments() {
        let user = v(&["--safe-mode", "--extra", "valor", "--no-supervisor"]);
        let pass = args::passthrough(&user);
        let normal = args::for_ui("tako-sv-1", UiLaunchMode::Normal, &pass);
        let safe = args::for_ui("tako-sv-1", UiLaunchMode::SafeMode, &pass);
        assert_eq!(
            normal,
            v(&["--ui", "--supervisor-pipe", "tako-sv-1", "--extra", "valor"])
        );
        assert_eq!(
            safe,
            v(&[
                "--ui",
                "--supervisor-pipe",
                "tako-sv-1",
                "--safe-mode",
                "--extra",
                "valor"
            ])
        );
        assert_eq!(args::pipe_name(&safe), Some("tako-sv-1"));
        assert_eq!(args::pipe_name(&v(&["--ui"])), None);
        assert_eq!(args::pipe_name(&v(&["--supervisor-pipe", ""])), None);
        let p = args::passthrough(&v(&["--ui", "--supervisor-pipe", "x", "--extra"]));
        assert_eq!(p, v(&["--extra"]));
        // O supervisor repõe o modo em uso; o da linha de comando não volta depois de uma troca.
        let p = args::passthrough(&v(&[
            "--big-picture",
            "--desktop",
            "--console-session",
            "-x",
        ]));
        assert_eq!(p, v(&["-x"]));
    }

    #[test]
    fn payloads_use_the_csharp_json_names() {
        let closing = UiClosing {
            process_id: 7,
            reason: UiCloseReason::SessionEnding,
        };
        let json = serde_json::to_string(&closing).unwrap_or_default();
        assert_eq!(json, r#"{"processId":7,"reason":"sessionEnding"}"#);
        let status = SupervisorStatus {
            process_id: 1,
            restarts: 0,
            mode: UiLaunchMode::SafeMode,
        };
        let json = serde_json::to_string(&status).unwrap_or_default();
        assert_eq!(json, r#"{"processId":1,"restarts":0,"mode":"safeMode"}"#);
    }
}
