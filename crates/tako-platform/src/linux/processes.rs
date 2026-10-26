//! Abrir jogos e observar processos no Linux (`/proc`, `xdg-open`, `gio launch`).

use crate::unix::{run, text};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tako_core::devices::{DeviceError, DeviceResult, ProcessEntry, ProcessLauncher};

pub struct LinuxProcesses;

/// Abre pelo jeito do sistema (URI, `.desktop`). O PID devolvido seria o do `xdg-open`, então nenhum.
fn open_with_system(target: &str) -> DeviceResult<Option<u32>> {
    run::spawn(Command::new("xdg-open").arg(target)).map(|_| None)
}

fn command_for(target: &Path, args: Vec<String>) -> Command {
    use std::os::unix::fs::PermissionsExt;
    let executable = std::fs::metadata(target).is_ok_and(|m| m.permissions().mode() & 0o111 != 0);
    let is_script = target.extension().is_some_and(|e| e == "sh");
    // Script sem permissão de execução ainda abre pelo `sh`.
    let mut cmd = if !executable && is_script {
        let mut c = Command::new("sh");
        c.arg(target);
        c
    } else {
        Command::new(target)
    };
    cmd.args(args);
    cmd
}

/// Variáveis de ambiente só chegam a quem o Tako abre direto; URI e `.desktop` passam por outro programa (como no
/// Windows, onde atalhos e URIs abrem sem elas).
fn warn_env_ignored(env: &[(String, String)], target: &str) {
    if !env.is_empty() {
        tracing::warn!("variáveis de ambiente só valem para executáveis: {target}");
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LaunchTarget<'a> {
    Empty,
    Uri(&'a str),
    MissingPath(&'a str),
    DesktopFile(&'a Path),
    Command(&'a Path),
}

/// Classifica o alvo de lançamento sem disparar comandos reais.
/// Alvos contendo barra são caminhos relativos ou absolutos que precisam existir no disco.
/// Alvos sem barra (ex.: `bottles-cli`, `flatpak`) são resolvidos pelo PATH e não falham na checagem local.
pub(crate) fn classify_target_with(
    target: &str,
    exists: impl Fn(&Path) -> bool,
) -> LaunchTarget<'_> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return LaunchTarget::Empty;
    }
    if text::is_uri(trimmed) {
        return LaunchTarget::Uri(trimmed);
    }
    let path = Path::new(trimmed);
    if trimmed.contains('/') && !exists(path) {
        return LaunchTarget::MissingPath(trimmed);
    }
    if path.extension().is_some_and(|e| e == "desktop") {
        return LaunchTarget::DesktopFile(path);
    }
    LaunchTarget::Command(path)
}

fn classify_target(target: &str) -> LaunchTarget<'_> {
    classify_target_with(target, |p| p.exists())
}

fn start(
    target: &str,
    arguments: Option<&str>,
    working_dir: Option<&str>,
    env: &[(String, String)],
) -> DeviceResult<Option<u32>> {
    let args = arguments.map(text::split_args).unwrap_or_default();
    start_with_args(target, &args, working_dir, env)
}

fn start_with_args(
    target: &str,
    args: &[String],
    working_dir: Option<&str>,
    env: &[(String, String)],
) -> DeviceResult<Option<u32>> {
    match classify_target(target) {
        LaunchTarget::Empty => Err(DeviceError::NotFound("alvo vazio".into())),
        LaunchTarget::Uri(uri) => {
            warn_env_ignored(env, uri);
            open_with_system(uri)
        }
        LaunchTarget::MissingPath(missing) => Err(DeviceError::NotFound(missing.into())),
        LaunchTarget::DesktopFile(path) => {
            let target_str = target.trim();
            warn_env_ignored(env, target_str);
            if run::has_tool("gio") {
                let mut c = Command::new("gio");
                c.arg("launch").arg(path).args(args);
                return run::spawn(&mut c).map(|_| None);
            }
            open_with_system(target_str)
        }
        LaunchTarget::Command(path) => {
            let mut cmd = command_for(path, args.to_vec());
            if let Some(dir) = working_dir
                .filter(|d| !d.trim().is_empty())
                .map(PathBuf::from)
                .or_else(|| path.parent().map(Path::to_path_buf))
                .filter(|d| d.is_dir())
            {
                cmd.current_dir(dir);
            }
            cmd.envs(env.iter().map(|(k, v)| (k, v)));
            run::spawn(&mut cmd).map(Some)
        }
    }
}

impl ProcessLauncher for LinuxProcesses {
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

    fn launch_args_with_env(
        &self,
        target: &str,
        arguments: &[String],
        working_dir: Option<&str>,
        env: &[(String, String)],
    ) -> DeviceResult<Option<u32>> {
        start_with_args(target, arguments, working_dir, env)
    }

    fn run_command(&self, command: &str, wait: Option<Duration>) -> DeviceResult<()> {
        run::user_command(command, wait)
    }

    fn exit_code(&self, pid: u32) -> Option<u32> {
        run::exit_code(pid)
    }

    fn snapshot(&self) -> Vec<ProcessEntry> {
        let boot = run::read_limited(Path::new("/proc/stat"), 1 << 20)
            .and_then(|s| text::parse_boot_time(&s));
        let Ok(dir) = std::fs::read_dir("/proc") else {
            return Vec::new();
        };
        dir.filter_map(Result::ok)
            .filter_map(|e| {
                let pid: u32 = e.file_name().to_str()?.parse().ok()?;
                let stat = run::read_limited(&e.path().join("stat"), 64 << 10)?;
                let (ppid, started_at) = text::parse_proc_stat(&stat, boot)?;
                Some(ProcessEntry {
                    pid,
                    parent_pid: (ppid != 0).then_some(ppid),
                    // Processos de outros usuários não deixam ler o link: fica sem caminho.
                    exe_path: std::fs::read_link(e.path().join("exe"))
                        .ok()
                        .map(|p| p.to_string_lossy().into_owned()),
                    started_at,
                })
            })
            .collect()
    }

    fn request_close(&self, pids: &[u32]) -> DeviceResult<()> {
        run::terminate(pids)
    }

    fn force_close(&self, pids: &[u32]) -> DeviceResult<()> {
        run::kill(pids)
    }

    fn steam_running_app_id(&self) -> u32 {
        let Some(home) = run::home() else {
            return 0;
        };
        run::steam_app_id(&[
            home.join(".steam/registry.vdf"),
            home.join(".var/app/com.valvesoftware.Steam/.steam/registry.vdf"),
        ])
    }

    fn open_folder(&self, path: &str) -> DeviceResult<()> {
        if !Path::new(path).is_dir() {
            return Err(DeviceError::NotFound(path.into()));
        }
        open_with_system(path).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_targets_purely_without_spawning_processes() {
        assert_eq!(classify_target_with("", |_| true), LaunchTarget::Empty);
        assert_eq!(classify_target_with("   ", |_| true), LaunchTarget::Empty);
        assert_eq!(
            classify_target_with("steam://rungameid/123", |_| false),
            LaunchTarget::Uri("steam://rungameid/123")
        );
        assert_eq!(
            classify_target_with("bottles-cli", |_| false),
            LaunchTarget::Command(Path::new("bottles-cli"))
        );
        assert_eq!(
            classify_target_with("flatpak", |_| false),
            LaunchTarget::Command(Path::new("flatpak"))
        );
        assert_eq!(
            classify_target_with("/usr/bin/missing", |_| false),
            LaunchTarget::MissingPath("/usr/bin/missing")
        );
        assert_eq!(
            classify_target_with("./local/missing", |_| false),
            LaunchTarget::MissingPath("./local/missing")
        );
        assert_eq!(
            classify_target_with("/usr/bin/existing", |p| p == Path::new("/usr/bin/existing")),
            LaunchTarget::Command(Path::new("/usr/bin/existing"))
        );
        assert_eq!(
            classify_target_with("/usr/share/app.desktop", |_| true),
            LaunchTarget::DesktopFile(Path::new("/usr/share/app.desktop"))
        );
    }

    #[test]
    fn launches_and_sees_the_process() {
        let p = LinuxProcesses;
        let pid = p
            .launch("/bin/sh", Some("-c 'sleep 30'"), Some("/tmp"))
            .unwrap()
            .unwrap();
        let me = p.snapshot().into_iter().find(|e| e.pid == pid).unwrap();
        assert_eq!(me.parent_pid, Some(std::process::id()));
        assert!(me.started_at.is_some());
        p.request_close(&[pid]).unwrap();
        assert!(matches!(
            p.launch("/no/such/game", None, None),
            Err(DeviceError::NotFound(_))
        ));
        assert!(p.launch("  ", None, None).is_err());
    }

    #[test]
    fn environment_reaches_the_game_and_commands_run_through_sh() {
        let p = LinuxProcesses;
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
