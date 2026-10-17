//! Ferramentas de linha de comando, processos destacados, sinais e arquivos (Linux e macOS).

use crate::Autostart;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tako_core::devices::{DeviceError, DeviceResult};

fn io_error(what: &str, e: io::Error) -> DeviceError {
    match e.kind() {
        io::ErrorKind::NotFound => DeviceError::NotFound(format!("{what}: {e}")),
        io::ErrorKind::PermissionDenied => DeviceError::PermissionDenied(format!("{what}: {e}")),
        _ => DeviceError::Failed(format!("{what}: {e}")),
    }
}

/// Roda uma ferramenta e devolve a saída padrão. Ferramenta ausente = `Unsupported`.
pub fn tool(program: &str, args: &[String]) -> DeviceResult<String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => {
                DeviceError::Unsupported(format!("{program} não está instalado"))
            }
            _ => io_error(program, e),
        })?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let err: String = err.trim().chars().take(500).collect();
        return Err(DeviceError::Failed(format!("{program}: {err}")));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn has_tool(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| {
        std::env::split_paths(&p).any(|d| {
            std::fs::metadata(d.join(program)).is_ok_and(|m| {
                use std::os::unix::fs::PermissionsExt;
                m.is_file() && m.permissions().mode() & 0o111 != 0
            })
        })
    })
}

/// Abre sem prender o Tako: grupo de processos próprio, sem terminal, e uma thread espera o fim
/// (senão o processo vira zumbi e parece rodando no retrato).
pub fn spawn(cmd: &mut Command) -> DeviceResult<u32> {
    use std::os::unix::process::CommandExt;
    let what = cmd.get_program().to_string_lossy().into_owned();
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| io_error(&what, e))?;
    let pid = child.id();
    std::thread::spawn(move || child.wait());
    Ok(pid)
}

/// SIGTERM: pede para fechar, como o X da janela.
pub fn terminate(pids: &[u32]) -> DeviceResult<()> {
    signal(pids, libc::SIGTERM)
}

/// SIGKILL: encerra à força (o jogo não respondeu ao SIGTERM).
pub fn kill(pids: &[u32]) -> DeviceResult<()> {
    signal(pids, libc::SIGKILL)
}

fn signal(pids: &[u32], sig: libc::c_int) -> DeviceResult<()> {
    let mut failed = Vec::new();
    for &pid in pids {
        let Ok(p) = libc::pid_t::try_from(pid) else {
            continue;
        };
        if p <= 1 {
            continue;
        }
        // SAFETY: `kill` só recebe números; pid > 1 evita sinalizar grupos ou o init.
        if unsafe { libc::kill(p, sig) } != 0 {
            let e = io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::ESRCH) {
                failed.push(format!("{pid}: {e}"));
            }
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(DeviceError::Failed(failed.join("; ")))
    }
}

/// Lê um arquivo pequeno (texto de terceiros: com limite de tamanho).
pub fn read_limited(path: &Path, max: u64) -> Option<String> {
    use std::io::Read;
    let mut s = String::new();
    std::fs::File::open(path)
        .ok()?
        .take(max)
        .read_to_string(&mut s)
        .ok()?;
    Some(s)
}

pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

/// `RunningAppID` do primeiro `registry.vdf` que tiver um jogo rodando.
pub fn steam_app_id(candidates: &[PathBuf]) -> u32 {
    candidates
        .iter()
        .filter_map(|p| read_limited(p, 4 << 20))
        .map(|t| super::text::steam_running_app_id(&t))
        .find(|&id| id != 0)
        .unwrap_or(0)
}

/// Marcador de cursor escondido (`<dados>/cursor-hidden`) para recuperar depois de uma queda.
pub fn set_marker(path: &Path, on: bool) {
    let r = if on {
        std::fs::write(path, b"1")
    } else {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            r => r,
        }
    };
    if let Err(e) = r {
        tracing::warn!("marcador do cursor {}: {e}", path.display());
    }
}

/// Comando do usuário antes ou depois do jogo (SESSION-004), por `sh -c`, com a espera e os erros do Windows:
/// `wait` é o limite para terminar; falha ou fim do prazo é erro, e o comando continua rodando.
pub fn user_command(command: &str, wait: Option<Duration>) -> DeviceResult<()> {
    use std::os::unix::process::CommandExt;
    let failed = |e: io::Error| DeviceError::Failed(format!("comando \"{command}\": {e}"));
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(failed)?;
    // Quem fica rodando é esperado numa thread, senão vira zumbi.
    let reap = |mut child: std::process::Child| {
        std::thread::spawn(move || child.wait());
    };
    let Some(limit) = wait else {
        reap(child);
        return Ok(());
    };
    let started = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(DeviceError::Failed(format!(
                    "comando \"{command}\" saiu com {status}"
                )));
            }
            Ok(None) if started.elapsed() >= limit => {
                reap(child);
                return Err(DeviceError::Failed(format!(
                    "comando \"{command}\" não terminou em {} s",
                    limit.as_secs()
                )));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(e) => return Err(failed(e)),
        }
    }
}

/// Abrir com o sistema = um arquivo numa pasta conhecida (`.desktop` ou LaunchAgent).
pub struct FileAutostart {
    pub path: Option<PathBuf>,
    pub content: String,
}

impl Autostart for FileAutostart {
    fn is_enabled(&self) -> bool {
        self.path.as_ref().is_some_and(|p| p.is_file())
    }

    fn set_enabled(&self, enabled: bool) -> DeviceResult<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| DeviceError::NotFound("pasta do usuário".into()))?;
        if enabled {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).map_err(|e| io_error("autostart", e))?;
            }
            std::fs::write(path, &self.content).map_err(|e| io_error("autostart", e))
        } else {
            match std::fs::remove_file(path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(io_error("autostart", e)),
                _ => Ok(()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("tako-autostart-{}", std::process::id()));
        let a = FileAutostart {
            path: Some(dir.join("sub/tako.desktop")),
            content: "x".into(),
        };
        assert!(!a.is_enabled());
        a.set_enabled(true).unwrap();
        assert!(a.is_enabled());
        a.set_enabled(false).unwrap();
        a.set_enabled(false).unwrap();
        assert!(!a.is_enabled());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tools_and_signals() {
        assert!(matches!(
            tool("tako-no-such-tool", &[]),
            Err(DeviceError::Unsupported(_))
        ));
        assert!(has_tool("sh"));
        assert!(!has_tool("tako-no-such-tool"));
        let pid = spawn(Command::new("sh").args(["-c", "sleep 30"])).unwrap();
        terminate(&[pid, 0, 1, u32::MAX]).unwrap();
    }

    #[test]
    fn user_commands_report_failure_and_timeouts() {
        let wait = Some(Duration::from_secs(10));
        assert!(user_command("exit 0", wait).is_ok());
        let failed = user_command("exit 3", wait).unwrap_err().to_string();
        assert!(failed.contains("saiu com"), "{failed}");
        let slow = user_command("sleep 5", Some(Duration::from_millis(300)));
        assert!(slow.unwrap_err().to_string().contains("não terminou"));
        assert!(user_command("exit 3", None).is_ok());
        // `sh -c`: redirecionamento e variáveis do shell funcionam como no terminal.
        let out = std::env::temp_dir().join(format!("tako-command-{}.txt", std::process::id()));
        let cmd = format!("X=ok; echo \"$X\" > '{}'", out.display());
        user_command(&cmd, wait).unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap().trim(), "ok");
        let _ = std::fs::remove_file(out);
    }
}
