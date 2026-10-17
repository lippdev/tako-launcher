//! PERF-001: painel "Desempenho do Tako" em Configurações › Sistema. Memória e CPU da interface e do supervisor
//! (atualizados a cada segundo enquanto o painel está aberto) e o tempo até a primeira tela desta abertura.

use super::Cmd;
use super::Ctx;
use super::layer::{Flow, Layer, MenuItem, MenuLayer};
use crate::model::{Hint, Overlay, PanelRow};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tako_platform::usage::{self, ProcessUsage};

static STARTED: OnceLock<Instant> = OnceLock::new();
static FIRST_FRAME: OnceLock<Duration> = OnceLock::new();

/// Início do processo da interface (o `main`); só vale a primeira chamada.
pub fn mark_start(at: Instant) {
    let _ = STARTED.set(at);
}

/// A primeira tela foi desenhada: guarda (e registra no log) quanto levou desde o início.
pub fn first_frame_drawn() {
    if let Some(start) = STARTED.get()
        && FIRST_FRAME.set(start.elapsed()).is_ok()
    {
        tracing::info!("primeira tela em {:?}", start.elapsed());
    }
}

pub fn opened_in() -> Option<Duration> {
    FIRST_FRAME.get().copied()
}

/// CPU entre duas leituras, em % da máquina inteira (como o Gerenciador de Tarefas).
fn cpu_percent(before: (Instant, Duration), now: (Instant, Duration), cores: usize) -> Option<f64> {
    let wall = now.0.checked_duration_since(before.0)?.as_secs_f64();
    let cpu = now.1.checked_sub(before.1)?.as_secs_f64();
    (wall > 0. && cores > 0).then(|| (cpu / wall / cores as f64 * 100.).min(100.))
}

fn megabytes(bytes: u64) -> String {
    format!("{} MB", (bytes + 512 * 1024) / (1024 * 1024))
}

fn percent(p: f64, ctx: &Ctx) -> String {
    let s = format!("{p:.1} %");
    if ctx.lang() == tako_core::i18n::Lang::PtBr {
        s.replace('.', ",")
    } else {
        s
    }
}

/// Supervisor desta interface (`TAKO_SUPERVISOR_PID`, posto por ele ao abrir a UI).
fn supervisor_pid() -> Option<u32> {
    std::env::var("TAKO_SUPERVISOR_PID")
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Última leitura de cada processo, para a CPU sair da diferença.
#[derive(Default)]
struct Samples(HashMap<u32, (Instant, Duration)>);

impl Samples {
    fn rows(
        &mut self,
        ctx: &Ctx,
        key: &str,
        pid: u32,
        read: Option<ProcessUsage>,
    ) -> Vec<PanelRow> {
        let unknown = || ctx.t("settings.performance.unknown");
        let memory = read
            .and_then(|u| u.memory_bytes)
            .map_or_else(unknown, megabytes);
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        let now = read.and_then(|u| u.cpu_time).map(|c| (Instant::now(), c));
        let cpu = match (now, self.0.get(&pid)) {
            (Some(now), Some(before)) => cpu_percent(*before, now, cores),
            _ => None,
        };
        if let Some(now) = now {
            self.0.insert(pid, now);
        }
        vec![
            PanelRow::info(
                format!("{key}-memory"),
                ctx.t("settings.performance.memory"),
                memory,
            ),
            PanelRow::info(
                format!("{key}-cpu"),
                ctx.t("settings.performance.cpu"),
                cpu.map_or_else(
                    || {
                        if now.is_some() {
                            ctx.t("settings.performance.measuring")
                        } else {
                            unknown()
                        }
                    },
                    |p| percent(p, ctx),
                ),
            ),
        ]
    }
}

fn build(ctx: &Ctx, samples: &RefCell<Samples>, supervisor: Option<u32>) -> Vec<MenuItem> {
    let mut samples = samples.borrow_mut();
    let me = std::process::id();
    let mut rows = vec![PanelRow::header(ctx.t("settings.performance.ui"))];
    rows.extend(samples.rows(ctx, "ui", me, usage::process(me)));
    rows.push(PanelRow::info(
        "opened",
        ctx.t("settings.performance.opened"),
        opened_in().map_or_else(
            || ctx.t("settings.performance.unknown"),
            |d| format!("{} ms", d.as_millis()),
        ),
    ));
    if let Some(pid) = supervisor {
        rows.push(PanelRow::header(ctx.t("settings.performance.supervisor")));
        rows.extend(samples.rows(ctx, "supervisor", pid, usage::process(pid)));
    }
    rows.into_iter()
        .map(|r| {
            let desc = match r.key.as_str() {
                "opened" => ctx.t("settings.performance.opened_desc"),
                "supervisor-memory" => ctx.t("settings.performance.supervisor_desc"),
                _ => String::new(),
            };
            MenuItem::new(r.desc(desc))
        })
        .collect()
}

/// O painel de menu de sempre, redesenhado a cada segundo (`Layer::live`).
pub struct PerformancePanel(MenuLayer);

pub fn panel(ctx: &Ctx) -> PerformancePanel {
    let samples = Rc::new(RefCell::new(Samples::default()));
    let supervisor = supervisor_pid();
    PerformancePanel(MenuLayer::panel(
        ctx.t("settings.performance.title"),
        move |ctx| build(ctx, &samples, supervisor),
    ))
}

impl Layer for PerformancePanel {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        self.0.handle(cmd, ctx)
    }
    fn render(&self, ctx: &Ctx) -> Overlay {
        self.0.render(ctx)
    }
    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        self.0.hints(ctx)
    }
    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        self.0.click(zone, index, ctx)
    }
    fn hover(&mut self, zone: &str, index: usize) {
        self.0.hover(zone, index);
    }
    fn live(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_comes_from_the_difference_between_two_readings() {
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(2);
        let p = cpu_percent((t0, Duration::ZERO), (t1, Duration::from_secs(1)), 4).unwrap();
        assert!((p - 12.5).abs() < 1e-9);
        assert_eq!(
            cpu_percent((t1, Duration::ZERO), (t0, Duration::ZERO), 4),
            None
        );
        assert_eq!(
            cpu_percent((t0, Duration::from_secs(1)), (t1, Duration::ZERO), 4),
            None
        );
        assert_eq!(
            cpu_percent((t0, Duration::ZERO), (t0, Duration::ZERO), 4),
            None
        );
        assert_eq!(megabytes(150 * 1024 * 1024 + 1), "150 MB");
    }

    #[test]
    fn panel_shows_this_process_and_refreshes_while_open() {
        let dir = tempfile::tempdir().unwrap();
        let (_guard, ctx) = crate::metadata::test_ctx(dir.path(), true);
        let p = panel(&ctx);
        assert!(p.live());
        let first = p.render(&ctx).rows;
        let value = |rows: &[PanelRow], key: &str| {
            rows.iter()
                .find(|r| r.key == key)
                .map(|r| r.value.clone())
                .unwrap_or_else(|| panic!("falta {key}"))
        };
        assert!(value(&first, "ui-memory").ends_with(" MB"));
        assert_eq!(
            value(&first, "ui-cpu"),
            ctx.t("settings.performance.measuring")
        );
        // Gasta um pouco de CPU para a segunda leitura ter diferença.
        let spin = Instant::now();
        while spin.elapsed() < Duration::from_millis(30) {
            std::hint::black_box(0u64.wrapping_add(1));
        }
        let second = p.render(&ctx).rows;
        assert!(value(&second, "ui-cpu").ends_with(" %"));
        assert!(second.iter().any(|r| r.key == "opened"));
    }
}
