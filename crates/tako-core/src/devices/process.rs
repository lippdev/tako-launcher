//! Abrir jogos e observar processos (MIG-024).

use super::{DeviceError, DeviceResult};
use chrono::{DateTime, Utc};

/// Uma linha do retrato de processos (port de `Services/ProcessSnapshot.cs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessEntry {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    /// Caminho completo do executável, quando o SO informa.
    pub exe_path: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
}

/// Janela do jogo na tela.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWindow {
    None,
    /// Visível, mas sem cobrir o monitor (jogo em janela, ou a janela inicial antes de ir para tela cheia).
    Windowed,
    /// Cobre o monitor inteiro (tela cheia ou sem borda).
    Fullscreen,
}

pub trait ProcessLauncher: Send + Sync {
    /// Abre um caminho (executável, script, `.desktop`, `.app`, atalho) ou uma URI pelo jeito do SO.
    /// Devolve o PID quando o SO informa um processo (URIs normalmente não informam).
    fn launch(
        &self,
        target: &str,
        arguments: Option<&str>,
        working_dir: Option<&str>,
    ) -> DeviceResult<Option<u32>>;
    /// Como [`Self::launch`], com variáveis de ambiente a mais (SESSION-004). Onde o SO não implementa, abre sem
    /// elas (e avisa no log).
    fn launch_with_env(
        &self,
        target: &str,
        arguments: Option<&str>,
        working_dir: Option<&str>,
        env: &[(String, String)],
    ) -> DeviceResult<Option<u32>> {
        if !env.is_empty() {
            tracing::warn!("variáveis de ambiente não aplicadas neste SO ({target})");
        }
        self.launch(target, arguments, working_dir)
    }
    /// Roda um comando do usuário pelo shell do SO (SESSION-004). `wait`: esperar terminar até esse tempo; erro se
    /// sair com falha ou passar do tempo (o comando continua rodando).
    fn run_command(&self, command: &str, wait: Option<std::time::Duration>) -> DeviceResult<()> {
        let _ = (command, wait);
        Err(DeviceError::Unsupported(
            "comandos antes e depois do jogo".into(),
        ))
    }
    fn snapshot(&self) -> Vec<ProcessEntry>;
    /// Pede para fechar (como o X da janela); não mata à força.
    fn request_close(&self, pids: &[u32]) -> DeviceResult<()>;
    /// Encerra à força (o jogo não respondeu ao pedido de fechar).
    fn force_close(&self, pids: &[u32]) -> DeviceResult<()>;
    /// A maior janela principal destes processos (a interface sai da tela "Abrindo…" quando o jogo cobre a tela).
    /// Sem como saber, basta haver processo.
    fn game_window(&self, pids: &[u32]) -> GameWindow {
        if pids.is_empty() {
            GameWindow::None
        } else {
            GameWindow::Fullscreen
        }
    }
    /// `RunningAppID` da Steam (registro no Windows, `registry.vdf` no Linux e no macOS). 0 = nenhum.
    fn steam_running_app_id(&self) -> u32;
    /// Abre uma pasta no gerenciador de arquivos.
    fn open_folder(&self, path: &str) -> DeviceResult<()>;
}
