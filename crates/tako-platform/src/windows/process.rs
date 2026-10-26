//! Abrir jogos e observar processos: `ShellExecuteExW`, retrato pelo Toolhelp (port de
//! `Launcher.UI.Wpf/Services/ProcessSnapshot.cs`) e `WM_CLOSE` nas janelas do jogo.

use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, PoisonError};
use tako_core::devices::{DeviceError, DeviceResult, GameWindow, ProcessEntry, ProcessLauncher};
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, HWND, LPARAM, WPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, PCWSTR, PWSTR};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// FILETIME (100 ns desde 1601) → UTC.
fn from_filetime(ft: FILETIME) -> Option<DateTime<Utc>> {
    let ticks = (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    let secs = (ticks / 10_000_000) as i64 - 11_644_473_600;
    DateTime::from_timestamp(secs, (ticks % 10_000_000) as u32 * 100)
}

/// Caminho do executável e início.
type ProcessInfo = (Option<String>, Option<DateTime<Utc>>);

/// Caminho e início do processo; acesso negado (sistema, outro usuário, anti-cheat) vira `None`, sem erro.
fn query(pid: u32) -> ProcessInfo {
    // SAFETY: o handle é fechado no fim; os buffers têm o tamanho informado.
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return (None, None);
        };
        let mut buf = [0u16; 1024];
        let mut size = buf.len() as u32;
        let path =
            QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut size)
                .ok()
                .map(|()| String::from_utf16_lossy(&buf[..size as usize]));
        let (mut created, mut exit, mut kernel, mut user) = Default::default();
        let started = GetProcessTimes(h, &mut created, &mut exit, &mut kernel, &mut user)
            .ok()
            .and_then(|()| from_filetime(created));
        let _ = CloseHandle(h);
        (path, started)
    }
}

/// Processos do Windows. Caminhos ficam em cache por PID: cada retrato só consulta os novos.
#[derive(Default)]
pub struct WindowsProcesses {
    cache: Mutex<HashMap<u32, ProcessInfo>>,
    /// Processos raiz dos jogos (SESSION-003): com um handle aberto, o Windows guarda o código de saída depois
    /// que o processo termina. Guardados como número porque `HANDLE` não é `Send`.
    held: Mutex<Vec<(u32, isize)>>,
}

/// Bem mais jogos do que se abre numa sessão do Tako; passando disso, solta os mais antigos.
const MAX_HELD: usize = 16;
/// `STILL_ACTIVE`: o processo ainda está rodando.
const STILL_ACTIVE: u32 = 259;

fn close_raw(h: isize) {
    // SAFETY: o handle foi aberto por este módulo e sai da lista antes de fechar (fecha uma vez só).
    unsafe {
        let _ = CloseHandle(HANDLE(h as *mut _));
    }
}

impl WindowsProcesses {
    /// Guarda o handle do processo (fica com ele; fecha se o processo já estava guardado).
    fn hold(&self, pid: u32, h: HANDLE) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if held.iter().any(|(p, _)| *p == pid) {
            close_raw(h.0 as isize);
            return;
        }
        if held.len() >= MAX_HELD {
            let (_, old) = held.remove(0);
            close_raw(old);
        }
        held.push((pid, h.0 as isize));
    }
}

struct CloseTargets<'a> {
    pids: &'a HashSet<u32>,
    windows: Vec<HWND>,
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` é o `&mut CloseTargets` passado a `EnumWindows`, vivo durante a enumeração.
    let targets = unsafe { &mut *(lparam.0 as *mut CloseTargets) };
    let mut pid = 0;
    // SAFETY: `hwnd` vem do próprio `EnumWindows`.
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        // Só janelas principais: visíveis e sem dono (diálogos fecham junto com a dona).
        if targets.pids.contains(&pid)
            && IsWindowVisible(hwnd).as_bool()
            && GetWindow(hwnd, GW_OWNER).is_err()
        {
            targets.windows.push(hwnd);
        }
    }
    BOOL(1)
}

impl ProcessLauncher for WindowsProcesses {
    fn launch(
        &self,
        target: &str,
        arguments: Option<&str>,
        working_dir: Option<&str>,
    ) -> DeviceResult<Option<u32>> {
        let file = wide(target);
        let args = arguments.map(wide);
        let dir = working_dir.map(wide);
        let ptr = |v: &Option<Vec<u16>>| v.as_ref().map_or(PCWSTR::null(), |v| PCWSTR(v.as_ptr()));
        let mut info = SHELLEXECUTEINFOW {
            cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: SEE_MASK_NOCLOSEPROCESS,
            lpFile: PCWSTR(file.as_ptr()),
            lpParameters: ptr(&args),
            lpDirectory: ptr(&dir),
            nShow: SW_SHOWNORMAL.0,
            ..Default::default()
        };
        // SAFETY: os textos terminam em 0 e vivem até o fim da função.
        unsafe { ShellExecuteExW(&mut info) }.map_err(|e| {
            DeviceError::Failed(format!("ShellExecuteEx falhou para {target}: {e}"))
        })?;
        // URIs e `shell:AppsFolder\…` não devolvem processo.
        if info.hProcess.is_invalid() {
            return Ok(None);
        }
        // SAFETY: handle devolvido pelo `SEE_MASK_NOCLOSEPROCESS`; fica guardado para o código de saída.
        let pid = unsafe { GetProcessId(info.hProcess) };
        if pid == 0 {
            close_raw(info.hProcess.0 as isize);
        } else {
            self.hold(pid, info.hProcess);
        }
        Ok((pid != 0).then_some(pid))
    }

    /// Executável com variáveis: `CreateProcess` pelo `Command` (o `ShellExecuteEx` não recebe ambiente). Atalhos,
    /// URIs e jogos que pedem administrador seguem pelo `launch`, sem as variáveis.
    fn launch_with_env(
        &self,
        target: &str,
        arguments: Option<&str>,
        working_dir: Option<&str>,
        env: &[(String, String)],
    ) -> DeviceResult<Option<u32>> {
        use std::os::windows::process::CommandExt;
        let is_exe = std::path::Path::new(target)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"));
        if env.is_empty() || !is_exe {
            if !env.is_empty() {
                tracing::warn!("variáveis de ambiente só valem para executáveis: {target}");
            }
            return self.launch(target, arguments, working_dir);
        }
        let mut cmd = std::process::Command::new(target);
        if let Some(a) = arguments {
            cmd.raw_arg(a);
        }
        if let Some(d) = working_dir {
            cmd.current_dir(d);
        }
        cmd.envs(env.iter().map(|(k, v)| (k, v)));
        const ERROR_ELEVATION_REQUIRED: i32 = 740;
        match cmd.spawn() {
            Ok(child) => {
                // O handle do `Child` ainda está aberto: o processo existe, mesmo que já tenha saído.
                self.watch_exit(child.id());
                Ok(Some(child.id()))
            }
            Err(e) if e.raw_os_error() == Some(ERROR_ELEVATION_REQUIRED) => {
                tracing::warn!("{target} pede administrador; abrindo sem as variáveis de ambiente");
                self.launch(target, arguments, working_dir)
            }
            Err(e) => Err(DeviceError::Failed(format!(
                "não foi possível abrir {target}: {e}"
            ))),
        }
    }

    fn run_command(&self, command: &str, wait: Option<std::time::Duration>) -> DeviceResult<()> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut child = std::process::Command::new("cmd")
            .arg("/C")
            .raw_arg(command)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| DeviceError::Failed(format!("comando \"{command}\": {e}")))?;
        let Some(limit) = wait else {
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
                    return Err(DeviceError::Failed(format!(
                        "comando \"{command}\" não terminou em {} s",
                        limit.as_secs()
                    )));
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                Err(e) => return Err(DeviceError::Failed(format!("comando \"{command}\": {e}"))),
            }
        }
    }

    fn watch_exit(&self, pid: u32) {
        let known = self
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .any(|(p, _)| *p == pid);
        if known {
            return;
        }
        // SAFETY: só consulta; o handle fica guardado e é fechado em `exit_code` ou ao passar de `MAX_HELD`.
        match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
            Ok(h) => self.hold(pid, h),
            Err(e) => tracing::debug!("sem o código de saída de {pid}: {e}"),
        }
    }

    fn exit_code(&self, pid: u32) -> Option<u32> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        let i = held.iter().position(|(p, _)| *p == pid)?;
        let mut code = 0u32;
        // SAFETY: handle aberto por este módulo com `PROCESS_QUERY_LIMITED_INFORMATION`.
        let read = unsafe { GetExitCodeProcess(HANDLE(held[i].1 as *mut _), &mut code) };
        let code = match read {
            // Ainda rodando: continua guardado.
            Ok(()) if code == STILL_ACTIVE => return None,
            Ok(()) => Some(code),
            Err(e) => {
                tracing::debug!("código de saída de {pid}: {e}");
                None
            }
        };
        close_raw(held.remove(i).1);
        code
    }

    fn snapshot(&self) -> Vec<ProcessEntry> {
        // SAFETY: retrato de processos; o handle é fechado no fim.
        let Ok(snap) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
            return Vec::new();
        };
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        let mut entries = Vec::with_capacity(400);
        let mut seen = HashSet::new();
        let mut e = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        // SAFETY: `e` tem `dwSize` preenchido; `snap` é um retrato válido.
        let mut ok = unsafe { Process32FirstW(snap, &mut e) }.is_ok();
        while ok {
            let pid = e.th32ProcessID;
            seen.insert(pid);
            let (path, started) = cache.entry(pid).or_insert_with(|| query(pid)).clone();
            entries.push(ProcessEntry {
                pid,
                parent_pid: (e.th32ParentProcessID != 0).then_some(e.th32ParentProcessID),
                exe_path: path,
                started_at: started,
            });
            // SAFETY: como acima.
            ok = unsafe { Process32NextW(snap, &mut e) }.is_ok();
        }
        // SAFETY: handle aberto acima.
        let _ = unsafe { CloseHandle(snap) };
        cache.retain(|pid, _| seen.contains(pid));
        entries
    }

    fn request_close(&self, pids: &[u32]) -> DeviceResult<()> {
        let pids: HashSet<u32> = pids.iter().copied().collect();
        let mut targets = CloseTargets {
            pids: &pids,
            windows: Vec::new(),
        };
        // SAFETY: `targets` vive durante toda a enumeração, que é síncrona.
        let _ = unsafe {
            EnumWindows(
                Some(collect),
                LPARAM(&mut targets as *mut CloseTargets as isize),
            )
        };
        if targets.windows.is_empty() {
            return Err(DeviceError::NotFound(
                "nenhuma janela do jogo para fechar".into(),
            ));
        }
        for hwnd in targets.windows {
            // SAFETY: mensagem postada (sem esperar) para uma janela de outro processo.
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        }
        Ok(())
    }

    fn force_close(&self, pids: &[u32]) -> DeviceResult<()> {
        let mut failed = Vec::new();
        for &pid in pids.iter().filter(|&&p| p > 4) {
            // SAFETY: o handle aberto aqui é fechado logo depois; só encerra o processo pedido. Sem handle, o
            // processo já saiu entre a leitura e o pedido: nada a fazer.
            unsafe {
                if let Ok(h) = OpenProcess(PROCESS_TERMINATE, false, pid) {
                    if let Err(e) = TerminateProcess(h, 1) {
                        failed.push(format!("{pid}: {e}"));
                    }
                    let _ = CloseHandle(h);
                }
            }
        }
        if failed.is_empty() {
            Ok(())
        } else {
            Err(DeviceError::Failed(failed.join("; ")))
        }
    }

    fn game_window(&self, pids: &[u32]) -> GameWindow {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
        };
        let pids: HashSet<u32> = pids.iter().copied().collect();
        if pids.is_empty() {
            return GameWindow::None;
        }
        let mut targets = CloseTargets {
            pids: &pids,
            windows: Vec::new(),
        };
        // SAFETY: `targets` vive durante toda a enumeração, que é síncrona.
        let _ = unsafe {
            EnumWindows(
                Some(collect),
                LPARAM(&mut targets as *mut CloseTargets as isize),
            )
        };
        let mut best = GameWindow::None;
        for hwnd in targets.windows {
            let mut r = RECT::default();
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            // SAFETY: só lê o retângulo da janela (listada agora há pouco) e o do monitor em que ela está.
            let ok = unsafe {
                GetWindowRect(hwnd, &mut r).is_ok()
                    && GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut mi)
                        .as_bool()
            };
            // Janelas minúsculas (ocultas de serviço, 1x1) não contam como o jogo na tela.
            if !ok || r.right - r.left < 320 || r.bottom - r.top < 240 {
                continue;
            }
            // Com barra de título é janela (o RE2 abre assim, maximizado, antes de ir para tela cheia).
            // SAFETY: só lê o estilo da janela.
            let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
            let captioned = style & WS_CAPTION.0 == WS_CAPTION.0;
            let m = mi.rcMonitor;
            if !captioned
                && r.left <= m.left
                && r.top <= m.top
                && r.right >= m.right
                && r.bottom >= m.bottom
            {
                return GameWindow::Fullscreen;
            }
            best = GameWindow::Windowed;
        }
        best
    }

    fn steam_running_app_id(&self) -> u32 {
        windows_registry::CURRENT_USER
            .open(r"Software\Valve\Steam")
            .and_then(|k| k.get_u32("RunningAppID"))
            .unwrap_or(0)
    }

    fn open_folder(&self, path: &str) -> DeviceResult<()> {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map(drop)
            .map_err(|e| DeviceError::Failed(format!("explorer: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn keeps_the_exit_code_of_the_game_it_opened() {
        let p = WindowsProcesses::default();
        let cmd =
            std::env::var("ComSpec").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into());
        let env = [("TAKO_TEST".to_owned(), "1".to_owned())];
        // 0xC0000135 (DLL ausente) em decimal com sinal, como o `exit` do cmd recebe.
        let pid = p
            .launch_with_env(&cmd, Some("/c exit -1073741515"), None, &env)
            .unwrap()
            .unwrap();
        let mut code = None;
        for _ in 0..100 {
            code = p.exit_code(pid);
            if code.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(code, Some(0xC000_0135));
        // Lido uma vez só; processo que o Tako não guardou não tem código.
        assert_eq!(p.exit_code(pid), None);
        assert_eq!(p.exit_code(u32::MAX), None);
        // Um processo aberto por outro (o supervisor segura o do jogo pelo PID).
        let child = std::process::Command::new(&cmd)
            .args(["/c", "exit 7"])
            .spawn()
            .unwrap();
        p.watch_exit(child.id());
        let pid = child.id();
        drop(child);
        let mut code = None;
        for _ in 0..100 {
            code = p.exit_code(pid);
            if code.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(code, Some(7));
    }

    #[test]
    fn user_commands_report_failure_and_timeouts() {
        let p = WindowsProcesses::default();
        let wait = Some(Duration::from_secs(10));
        assert!(p.run_command("exit 0", wait).is_ok());
        let failed = p.run_command("exit 3", wait).unwrap_err().to_string();
        assert!(failed.contains("saiu com"), "{failed}");
        let slow = p.run_command("ping -n 5 127.0.0.1 >nul", Some(Duration::from_millis(300)));
        assert!(slow.unwrap_err().to_string().contains("não terminou"));
        assert!(p.run_command("exit 3", None).is_ok());
    }
}
