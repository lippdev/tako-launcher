//! Memória e CPU de um processo do Tako (PERF-001): a própria interface e o supervisor. Só leitura; acesso
//! negado ou processo que já saiu vira `None`.

use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProcessUsage {
    /// Memória residente (working set no Windows).
    pub memory_bytes: Option<u64>,
    /// CPU usada desde o início (usuário + sistema).
    pub cpu_time: Option<Duration>,
}

/// Uso do processo `pid` (`std::process::id()` para este).
pub fn process(pid: u32) -> Option<ProcessUsage> {
    let u = imp(pid);
    (u.memory_bytes.is_some() || u.cpu_time.is_some()).then_some(u)
}

#[cfg(target_os = "windows")]
fn imp(pid: u32) -> ProcessUsage {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let ticks = |ft: FILETIME| (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    // SAFETY: o handle é fechado no fim; a estrutura vai com o tamanho dela em `cb`.
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return ProcessUsage::default();
        };
        let mut counters = PROCESS_MEMORY_COUNTERS {
            cb: size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
            ..Default::default()
        };
        let memory_bytes = GetProcessMemoryInfo(h, &mut counters, counters.cb)
            .ok()
            .map(|()| counters.WorkingSetSize as u64);
        let (mut created, mut exit, mut kernel, mut user) = Default::default();
        let cpu_time = GetProcessTimes(h, &mut created, &mut exit, &mut kernel, &mut user)
            .ok()
            .map(|()| Duration::from_nanos((ticks(kernel) + ticks(user)) * 100));
        let _ = CloseHandle(h);
        ProcessUsage {
            memory_bytes,
            cpu_time,
        }
    }
}

#[cfg(target_os = "linux")]
fn imp(pid: u32) -> ProcessUsage {
    // SAFETY: `sysconf` só lê uma constante do sistema.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    let read = |f: &str| std::fs::read_to_string(format!("/proc/{pid}/{f}")).ok();
    ProcessUsage {
        memory_bytes: read("status").as_deref().and_then(rss_from_status),
        cpu_time: read("stat")
            .as_deref()
            .and_then(|s| cpu_from_stat(s, u64::try_from(hz).ok()?)),
    }
}

#[cfg(target_os = "macos")]
fn imp(pid: u32) -> ProcessUsage {
    // SAFETY: `proc_taskinfo` só tem números; zerada é um valor válido.
    let mut info: libc::proc_taskinfo = unsafe { std::mem::zeroed() };
    let size = size_of::<libc::proc_taskinfo>() as libc::c_int;
    // SAFETY: o buffer é uma `proc_taskinfo` inteira, com o tamanho informado.
    let read = unsafe {
        libc::proc_pidinfo(
            pid as libc::c_int,
            libc::PROC_PIDTASKINFO,
            0,
            (&raw mut info).cast(),
            size,
        )
    };
    // Os tempos de `proc_taskinfo` vêm em unidades do relógio da máquina; para este processo `getrusage`
    // dá microssegundos, e o supervisor fica só com a memória.
    let cpu_time = (pid == std::process::id()).then(|| {
        // SAFETY: `rusage` só tem números; zerada é um valor válido.
        let mut r: libc::rusage = unsafe { std::mem::zeroed() };
        // SAFETY: `r` é uma `rusage` válida para escrita.
        (unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut r) } == 0).then(|| {
            let us = |t: libc::timeval| t.tv_sec as u64 * 1_000_000 + t.tv_usec as u64;
            Duration::from_micros(us(r.ru_utime) + us(r.ru_stime))
        })
    });
    ProcessUsage {
        memory_bytes: (read == size).then_some(info.pti_resident_size),
        cpu_time: cpu_time.flatten(),
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn imp(_pid: u32) -> ProcessUsage {
    ProcessUsage::default()
}

/// `VmRSS:  123456 kB` de `/proc/<pid>/status`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn rss_from_status(status: &str) -> Option<u64> {
    let kb: u64 = status
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()?;
    kb.checked_mul(1024)
}

/// `utime` + `stime` (campos 14 e 15) de `/proc/<pid>/stat`. O nome do processo vem entre parênteses e pode
/// ter espaços: conta depois do último `)`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn cpu_from_stat(stat: &str, hz: u64) -> Option<Duration> {
    if hz == 0 {
        return None;
    }
    let mut after = stat.get(stat.rfind(')')? + 1..)?.split_whitespace();
    // Depois do nome: estado é o campo 3, então utime (14) é o 12º daqui.
    let utime: u64 = after.nth(11)?.parse().ok()?;
    let stime: u64 = after.next()?.parse().ok()?;
    let ticks = utime.checked_add(stime)?;
    Some(Duration::from_millis(ticks.checked_mul(1000)? / hz))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_process_reports_memory_and_cpu() {
        let u = process(std::process::id()).expect("o próprio processo");
        if cfg!(any(
            target_os = "windows",
            target_os = "linux",
            target_os = "macos"
        )) {
            assert!(u.memory_bytes.is_some_and(|m| m > 1024 * 1024));
            assert!(u.cpu_time.is_some());
        }
        assert_eq!(process(u32::MAX), None);
    }

    #[test]
    fn proc_files_parse_and_broken_ones_do_not() {
        assert_eq!(
            rss_from_status("Name:\ttako\nVmRSS:\t  2048 kB\n"),
            Some(2048 * 1024)
        );
        assert_eq!(rss_from_status("VmRSS: x kB"), None);
        assert_eq!(rss_from_status(""), None);
        let stat = "42 (tako (ui) x) S 1 42 42 0 -1 4194560 100 0 0 0 250 50 0 0 20 0 9 0 1 2 3";
        assert_eq!(cpu_from_stat(stat, 100), Some(Duration::from_secs(3)));
        assert_eq!(cpu_from_stat(stat, 0), None);
        assert_eq!(cpu_from_stat("42 (tako) S 1", 100), None);
        assert_eq!(cpu_from_stat("sem parênteses", 100), None);
    }
}
