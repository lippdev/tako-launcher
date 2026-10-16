//! Abrir jogos e observar processos (MIG-024).

use super::DeviceResult;
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

pub trait ProcessLauncher: Send + Sync {
    /// Abre um caminho (executável, script, `.desktop`, `.app`, atalho) ou uma URI pelo jeito do SO.
    /// Devolve o PID quando o SO informa um processo (URIs normalmente não informam).
    fn launch(
        &self,
        target: &str,
        arguments: Option<&str>,
        working_dir: Option<&str>,
    ) -> DeviceResult<Option<u32>>;
    fn snapshot(&self) -> Vec<ProcessEntry>;
    /// Pede para fechar (como o X da janela); não mata à força.
    fn request_close(&self, pids: &[u32]) -> DeviceResult<()>;
    /// `RunningAppID` da Steam (registro no Windows, `registry.vdf` no Linux e no macOS). 0 = nenhum.
    fn steam_running_app_id(&self) -> u32;
    /// Abre uma pasta no gerenciador de arquivos.
    fn open_folder(&self, path: &str) -> DeviceResult<()>;
}
