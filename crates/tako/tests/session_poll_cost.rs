//! Medição manual da leitura usada pela sessão no Windows, sem abrir nem encerrar processos.

#![cfg(target_os = "windows")]

use std::time::{Duration, Instant};
use tako_core::session::GameSession;
use tako_platform::Platform;

#[test]
#[ignore = "medição manual: 80 leituras reais a cada 250 ms; rode com --ignored --nocapture"]
fn cached_process_snapshot_cost() {
    let root = std::env::temp_dir().join("tako-session-poll-cost");
    let platform = Platform::native(&root);
    let processes = platform.processes;
    let cold = Instant::now();
    let count = processes.snapshot().len();
    let cold = cold.elapsed();
    let cpu = || {
        tako_platform::usage::process(std::process::id())
            .and_then(|usage| usage.cpu_time)
            .expect("CPU do próprio processo")
    };
    let cpu_start = cpu();
    let elapsed = Instant::now();
    let mut samples = Vec::new();
    for _ in 0..80 {
        std::thread::sleep(GameSession::POLL);
        let start = Instant::now();
        std::hint::black_box(processes.snapshot());
        samples.push(start.elapsed());
    }
    let cpu_time = cpu().saturating_sub(cpu_start);
    let cpu_percent = cpu_time.as_secs_f64() / elapsed.elapsed().as_secs_f64() * 100.0;
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    samples.sort();
    let total: Duration = samples.iter().sum();
    println!(
        "processos={count}; fria={cold:?}; n={}; p50={:?}; p95={:?}; max={:?}; tempo em leitura={total:?}; fração do intervalo={:.3}% de um núcleo (limite por tempo de parede)",
        samples.len(),
        samples[39],
        samples[75],
        samples[79],
        total.as_secs_f64() / (GameSession::POLL.as_secs_f64() * samples.len() as f64) * 100.0
    );
    println!(
        "CPU usuário+sistema={cpu_time:?}; CPU de um núcleo={cpu_percent:.3}%; CPU do PC={:.3}% ({threads} threads)",
        cpu_percent / threads as f64
    );
}
