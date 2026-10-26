//! Abrir jogos e observar processos no macOS (`open`, `libproc`).

use crate::unix::{run, text};
use chrono::DateTime;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tako_core::devices::{DeviceError, DeviceResult, ProcessEntry, ProcessLauncher};

pub struct MacProcesses;

fn open(args: &[&str], extra: Vec<String>, env: &[(String, String)]) -> DeviceResult<Option<u32>> {
    let mut c = Command::new("open");
    // `--env NOME=VALOR` (open(1)): variável para o app aberto, se ele ainda não estiver rodando.
    for (k, v) in env {
        c.arg("--env").arg(format!("{k}={v}"));
    }
    c.args(args);
    if !extra.is_empty() {
        c.arg("--args").args(extra);
    }
    // O PID seria o do `open`, não o do app.
    run::spawn(&mut c).map(|_| None)
}

fn pid_info(pid: i32) -> Option<ProcessEntry> {
    // SAFETY: `proc_bsdinfo` é uma struct C simples; zerada é um valor válido.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = size_of::<libc::proc_bsdinfo>() as i32;
    // SAFETY: `info` tem `size` bytes.
    let n = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    if n != size {
        return None;
    }
    let mut buf = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: `buf` tem o tamanho passado.
    let len = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
    let exe_path = (len > 0).then(|| String::from_utf8_lossy(&buf[..len as usize]).into_owned());
    Some(ProcessEntry {
        pid: pid as u32,
        parent_pid: (info.pbi_ppid != 0).then_some(info.pbi_ppid),
        exe_path,
        started_at: i64::try_from(info.pbi_start_tvsec)
            .ok()
            .and_then(|s| DateTime::from_timestamp(s, 0)),
    })
}

/// Abre um jogo; `env` vai para o processo do jogo (executável ou `.app`). URIs passam pelo app da loja, que já está
/// aberto ou abre do jeito dele: ali as variáveis não chegam (como no Windows).
fn start(
    target: &str,
    arguments: Option<&str>,
    working_dir: Option<&str>,
    env: &[(String, String)],
) -> DeviceResult<Option<u32>> {
    let target = target.trim();
    if target.is_empty() {
        return Err(DeviceError::NotFound("alvo vazio".into()));
    }
    let args = arguments.map(text::split_args).unwrap_or_default();
    if text::is_uri(target) {
        if !env.is_empty() {
            tracing::warn!("variáveis de ambiente só valem para executáveis: {target}");
        }
        return open(&[target], Vec::new(), &[]);
    }
    let path = Path::new(target);
    if !path.exists() {
        return Err(DeviceError::NotFound(target.into()));
    }
    if path.extension().is_some_and(|e| e == "app") {
        return open(&["-a", target], args, env);
    }
    use std::os::unix::fs::PermissionsExt;
    let executable = path.is_file()
        && std::fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0);
    if !executable {
        return open(&[target], args, env);
    }
    let mut cmd = Command::new(path);
    cmd.args(args);
    if let Some(dir) = working_dir
        .filter(|d| !d.trim().is_empty())
        .map(Path::new)
        .or_else(|| path.parent())
        .filter(|d| d.is_dir())
    {
        cmd.current_dir(dir);
    }
    cmd.envs(env.iter().map(|(k, v)| (k, v)));
    run::spawn(&mut cmd).map(Some)
}

impl ProcessLauncher for MacProcesses {
    fn launch(
        &self,
        target: &str,
        arguments: Option<&str>,
        working_dir: Option<&str>,
    ) -> DeviceResult<Option<u32>> {
        start(target, arguments, working_dir, &[])
    }

    fn launch_with_env(
        &self,
        target: &str,
        arguments: Option<&str>,
        working_dir: Option<&str>,
        env: &[(String, String)],
    ) -> DeviceResult<Option<u32>> {
        start(target, arguments, working_dir, env)
    }

    fn run_command(&self, command: &str, wait: Option<Duration>) -> DeviceResult<()> {
        run::user_command(command, wait)
    }

    fn exit_code(&self, pid: u32) -> Option<u32> {
        run::exit_code(pid)
    }

    fn snapshot(&self) -> Vec<ProcessEntry> {
        // SAFETY: com buffer nulo, devolve só quantos PIDs existem.
        let n = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
        if n <= 0 {
            return Vec::new();
        }
        // Folga para processos que nascem entre as duas chamadas.
        let mut pids = vec![0i32; n as usize + 64];
        let bytes = (pids.len() * size_of::<i32>()) as i32;
        // SAFETY: `pids` tem `bytes` bytes.
        let n = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), bytes) };
        pids.truncate(n.max(0) as usize);
        pids.into_iter()
            .filter(|&p| p > 0)
            .filter_map(pid_info)
            .collect()
    }

    fn request_close(&self, pids: &[u32]) -> DeviceResult<()> {
        run::terminate(pids)
    }

    fn force_close(&self, pids: &[u32]) -> DeviceResult<()> {
        run::kill(pids)
    }

    fn steam_running_app_id(&self) -> u32 {
        run::home().map_or(0, |h| {
            run::steam_app_id(&[h.join("Library/Application Support/Steam/registry.vdf")])
        })
    }

    fn open_folder(&self, path: &str) -> DeviceResult<()> {
        if !Path::new(path).is_dir() {
            return Err(DeviceError::NotFound(path.into()));
        }
        open(&[path], Vec::new(), &[]).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_reaches_the_game_and_commands_run_through_sh() {
        let p = MacProcesses;
        let out = std::env::temp_dir().join(format!("tako-env-{}.txt", std::process::id()));
        let env = [("TAKO_TESTE".to_owned(), "valor com espaço".to_owned())];
        let script = format!("-c 'echo \"$TAKO_TESTE\" > {}'", out.display());
        p.launch_with_env("/bin/sh", Some(&script), None, &env)
            .unwrap()
            .unwrap();
        let mut text = String::new();
        for _ in 0..50 {
            text = std::fs::read_to_string(&out).unwrap_or_default();
            if !text.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = std::fs::remove_file(&out);
        assert_eq!(text.trim(), "valor com espaço");
        let failed = p.run_command("exit 3", Some(Duration::from_secs(10)));
        assert!(failed.unwrap_err().to_string().contains("saiu com"));
        assert!(p.run_command("true", Some(Duration::from_secs(10))).is_ok());
    }
}
