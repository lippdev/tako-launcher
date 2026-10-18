//! TelaCarregamento (UPD-001): a janela de abertura enquanto a primeira leitura das lojas monta a biblioteca.
//! O Tako não aplica atualização na abertura (baixa em segundo plano e instala ao sair), então a barra mostra o
//! progresso de verdade da leitura: lojas lidas sobre o total. Some quando a biblioteca está pronta.

use crate::{bridge, theme};
use slint::ComponentHandle;
use slint_interpreter::{ComponentInstance, Value};
use std::path::Path;
use std::time::{Duration, Instant};
use tako_core::i18n::t;

/// Texto, progresso (0 a 1; negativo = sem número ainda) e porcentagem da barra.
pub fn state(scanning: bool, permille: u32) -> (String, f32, String) {
    if !scanning {
        return (t("first_run.splash.starting"), 1.0, String::new());
    }
    if permille == 0 {
        // Nenhuma loja lida ainda: a barra corre sem número.
        return (t("first_run.splash.scanning"), -1.0, String::new());
    }
    let permille = permille.min(1000);
    (
        t("first_run.splash.scanning"),
        permille as f32 / 1000.0,
        format!("{}%", permille / 10),
    )
}

pub fn set(ui: &ComponentInstance, (status, progress, percent): (String, f32, String)) {
    bridge::set(ui, "status", Value::String(status.into()));
    bridge::set(ui, "progress", Value::Number(progress.into()));
    bridge::set(ui, "percent", Value::String(percent.into()));
}

/// Abre a janela no centro da tela; `None` se o tema não compilar (a abertura segue sem ela).
pub fn open(builtin: &Path, reduce_motion: bool) -> Option<ComponentInstance> {
    let def = theme::compile_splash(builtin)
        .map_err(|e| tracing::warn!("janela de abertura: {e}"))
        .ok()?;
    let ui = def.create().ok()?;
    bridge::set_global(&ui, "Ui", "reduce-motion", Value::Bool(reduce_motion));
    set(&ui, state(true, 0));
    ui.show().ok()?;
    use slint::winit_030::WinitWindowAccessor;
    ui.window().with_winit_window(|w| {
        if let Some(m) = w.current_monitor() {
            let (screen, at, size) = (m.size(), m.position(), w.outer_size());
            w.set_outer_position(slint::winit_030::winit::dpi::PhysicalPosition::new(
                at.x + (screen.width.saturating_sub(size.width) / 2) as i32,
                at.y + (screen.height.saturating_sub(size.height) / 2) as i32,
            ));
        }
    });
    Some(ui)
}

/// Segue a leitura: atualiza a barra e, quando a biblioteca fica pronta, mostra a janela principal (`ready`) e fecha
/// a de abertura. `hold` deixa os últimos tentáculos assentarem.
pub fn follow(
    ui: ComponentInstance,
    scanning: impl Fn() -> (bool, u32) + 'static,
    hold: Duration,
    ready: impl FnOnce() + 'static,
) {
    // shortcut: uma leitura travada (rede da Steam) não prende a abertura além de 15 s; a leitura continua atrás.
    const LIMIT: Duration = Duration::from_secs(15);
    let started = Instant::now();
    let mut ready = Some(ready);
    let mut done_at: Option<Instant> = None;
    let timer = std::rc::Rc::new(slint::Timer::default());
    let me = std::rc::Rc::downgrade(&timer);
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            let (busy, permille) = scanning();
            let busy = busy && started.elapsed() < LIMIT;
            set(&ui, state(busy, permille));
            if busy {
                return;
            }
            let at = *done_at.get_or_insert_with(Instant::now);
            if at.elapsed() >= hold
                && let Some(show) = ready.take()
            {
                show();
                let _ = ui.hide();
                if let Some(t) = me.upgrade() {
                    t.stop();
                }
            }
        },
    );
    // Uma vez por abertura: o timer (parado no fim) fica até o processo sair.
    std::mem::forget(timer);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_splash_window_compiles_with_its_properties() {
        let dir = tempfile::tempdir().unwrap();
        let builtin = theme::install_builtin(dir.path()).unwrap();
        let def = theme::compile_splash(&builtin).unwrap_or_else(|e| panic!("{e}"));
        let props: Vec<String> = def.properties().map(|(n, _)| n).collect();
        for p in ["progress", "status", "percent", "sub"] {
            assert!(props.iter().any(|n| n == p), "{p}");
        }
    }

    #[test]
    fn the_bar_shows_only_real_progress() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        // Nenhuma loja lida: sem número, barra indeterminada.
        let (status, p, percent) = state(true, 0);
        assert_eq!(status, "Procurando seus jogos…");
        assert!(p < 0.0 && percent.is_empty());
        // 3 de 7 lojas: 42%.
        let (_, p, percent) = state(true, 3 * 1000 / 7);
        assert!((p - 0.428).abs() < 0.001);
        assert_eq!(percent, "42%");
        // Leitura pronta: barra cheia e "Iniciando o Tako…".
        assert_eq!(
            state(false, 500),
            ("Iniciando o Tako…".into(), 1.0, String::new())
        );
    }
}
