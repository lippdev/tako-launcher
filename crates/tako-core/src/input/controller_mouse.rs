//! Cursor pelo controle (PRD D-03): analógico esquerdo move, direito rola, A clica.
//! Port de `ConsoleMode.Runtime/SessionTools/ControllerMouseMath.cs`; o SO entra por [`PointerInjector`].

use crate::devices::{DeviceResult, MouseButton, PointerInjector};
use std::time::Duration;

/// Zona morta radial do analógico esquerdo; analógicos gastos descansam um pouco fora do centro.
pub const MOVE_DEAD_ZONE: f64 = 0.18;
/// O direito (rolagem) pede um empurrão mais claro, para mirar com o esquerdo nunca rolar sem querer.
pub const SCROLL_DEAD_ZONE: f64 = 0.3;
/// No máximo, o ponteiro cruza tantas alturas de tela por segundo: uma TV 4K parece uma 1080p.
pub const SCREENS_PER_SECOND: f64 = 1.1;
pub const MIN_NOTCHES_PER_SECOND: f64 = 3.0;
pub const MAX_NOTCHES_PER_SECOND: f64 = 16.0;

/// Eixo do SDL (-32768..32767) para -1..1. O SDL usa Y para baixo: quem chama inverte o Y.
pub fn from_axis(value: i16) -> f64 {
    (value as f64 / 32767.0).max(-1.0)
}

/// Zona morta radial e curva quadrática: inclinação pequena mira com precisão, total percorre a tela.
pub fn curve(x: f64, y: f64, dead_zone: f64) -> (f64, f64) {
    let magnitude = (x * x + y * y).sqrt();
    if magnitude <= dead_zone {
        return (0.0, 0.0);
    }
    let scaled = ((magnitude - dead_zone) / (1.0 - dead_zone)).min(1.0);
    let factor = scaled * scaled / magnitude;
    (x * factor, y * factor)
}

/// Analógico esquerdo em pixels, guardando as frações entre amostras para o movimento lento não travar.
#[derive(Debug, Default)]
pub struct PointerMotion {
    rest_x: f64,
    rest_y: f64,
}

impl PointerMotion {
    /// `y` para cima positivo (o Y da tela cresce para baixo); `screen_height` em pixels.
    pub fn step(&mut self, x: f64, y: f64, seconds: f64, screen_height: u32) -> (i32, i32) {
        let (vx, vy) = curve(x, y, MOVE_DEAD_ZONE);
        if vx == 0.0 && vy == 0.0 {
            self.reset();
            return (0, 0);
        }
        let speed = f64::from(screen_height.max(720)) * SCREENS_PER_SECOND;
        self.rest_x += vx * speed * seconds;
        self.rest_y -= vy * speed * seconds;
        let dx = self.rest_x.trunc();
        let dy = self.rest_y.trunc();
        self.rest_x -= dx;
        self.rest_y -= dy;
        (dx as i32, dy as i32)
    }

    pub fn reset(&mut self) {
        self.rest_x = 0.0;
        self.rest_y = 0.0;
    }
}

/// Um eixo do analógico direito em "dentes" inteiros da roda (positivo = para cima / direita). O primeiro
/// sai assim que o analógico deixa a zona morta; alguns programas ignoram frações.
#[derive(Debug, Default)]
pub struct WheelMotion {
    pending: f64,
    moving: bool,
}

impl WheelMotion {
    pub fn step(&mut self, axis: f64, seconds: f64) -> i32 {
        let magnitude = axis.abs();
        if magnitude <= SCROLL_DEAD_ZONE {
            self.reset();
            return 0;
        }
        let scaled = ((magnitude - SCROLL_DEAD_ZONE) / (1.0 - SCROLL_DEAD_ZONE)).min(1.0);
        let rate = MIN_NOTCHES_PER_SECOND
            + (MAX_NOTCHES_PER_SECOND - MIN_NOTCHES_PER_SECOND) * scaled * scaled;
        if self.moving {
            self.pending += rate * seconds;
        } else {
            self.moving = true;
            self.pending = 1.0;
        }
        let notches = self.pending.trunc();
        self.pending -= notches;
        notches as i32 * axis.signum() as i32
    }

    pub fn reset(&mut self) {
        self.pending = 0.0;
        self.moving = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickEvent {
    None,
    /// A tocado uma vez.
    LeftClick,
    /// A tocado duas vezes.
    RightClick,
    /// A segurado: esquerdo desce, para arrastar.
    LeftDown,
    /// A solto depois de segurar.
    LeftUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    WaitRelease,
    Idle,
    FirstPress,
    WaitSecondTap,
    Dragging,
}

/// O botão A como mouse: toque = esquerdo, dois toques = direito, segurar = arrastar. O clique simples
/// espera [`ClickGesture::DOUBLE_TAP_WINDOW`] depois de soltar, para saber que não é o início de um direito.
#[derive(Debug)]
pub struct ClickGesture {
    phase: Phase,
    since: Duration,
}

impl Default for ClickGesture {
    fn default() -> Self {
        ClickGesture {
            phase: Phase::WaitRelease,
            since: Duration::ZERO,
        }
    }
}

impl ClickGesture {
    pub const HOLD_FOR_DRAG: Duration = Duration::from_millis(350);
    pub const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(250);

    /// Uma amostra do botão A em `now` (relógio monotônico).
    pub fn update(&mut self, held: bool, now: Duration) -> ClickEvent {
        let elapsed = now.saturating_sub(self.since);
        match self.phase {
            Phase::WaitRelease => {
                if !held {
                    self.phase = Phase::Idle;
                }
            }
            Phase::Idle => {
                if held {
                    self.enter(Phase::FirstPress, now);
                }
            }
            Phase::FirstPress => {
                if !held {
                    self.enter(Phase::WaitSecondTap, now);
                } else if elapsed >= Self::HOLD_FOR_DRAG {
                    self.phase = Phase::Dragging;
                    return ClickEvent::LeftDown;
                }
            }
            Phase::WaitSecondTap => {
                if held {
                    self.phase = Phase::WaitRelease;
                    return ClickEvent::RightClick;
                }
                if elapsed >= Self::DOUBLE_TAP_WINDOW {
                    self.phase = Phase::Idle;
                    return ClickEvent::LeftClick;
                }
            }
            Phase::Dragging => {
                if !held {
                    self.phase = Phase::Idle;
                    return ClickEvent::LeftUp;
                }
            }
        }
        ClickEvent::None
    }

    /// Recomeça ignorando o A até ser solto (o toque que fechou a Central não pode clicar).
    /// Verdadeiro quando o esquerdo ficou pressionado de um arraste e quem chamou precisa soltá-lo.
    pub fn reset(&mut self) -> bool {
        let was_dragging = self.phase == Phase::Dragging;
        self.phase = Phase::WaitRelease;
        was_dragging
    }

    fn enter(&mut self, phase: Phase, now: Duration) {
        self.phase = phase;
        self.since = now;
    }
}

/// Ponteiro pelo controle: junta movimento, rolagem e cliques e entrega ao [`PointerInjector`].
#[derive(Debug, Default)]
pub struct ControllerMouse {
    pub motion: PointerMotion,
    pub wheel_v: WheelMotion,
    pub wheel_h: WheelMotion,
    pub click: ClickGesture,
}

impl ControllerMouse {
    /// Uma amostra: analógicos em -1..1 com Y para cima, `a_held` = botão A.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        out: &dyn PointerInjector,
        (lx, ly): (f64, f64),
        (rx, ry): (f64, f64),
        a_held: bool,
        now: Duration,
        seconds: f64,
        screen_height: u32,
    ) -> DeviceResult<()> {
        let (dx, dy) = self.motion.step(lx, ly, seconds, screen_height);
        if dx != 0 || dy != 0 {
            out.move_by(dx, dy)?;
        }
        let (v, h) = (
            self.wheel_v.step(ry, seconds),
            self.wheel_h.step(rx, seconds),
        );
        if v != 0 || h != 0 {
            out.wheel(v, h)?;
        }
        match self.click.update(a_held, now) {
            ClickEvent::LeftClick => click(out, MouseButton::Left),
            ClickEvent::RightClick => click(out, MouseButton::Right),
            ClickEvent::LeftDown => out.button(MouseButton::Left, true),
            ClickEvent::LeftUp => out.button(MouseButton::Left, false),
            ClickEvent::None => Ok(()),
        }
    }

    /// Pausa ou sai do modo: solta o esquerdo de um arraste e ignora o A até ser solto.
    pub fn reset(&mut self, out: &dyn PointerInjector) -> DeviceResult<()> {
        self.motion.reset();
        self.wheel_v.reset();
        self.wheel_h.reset();
        if self.click.reset() {
            out.button(MouseButton::Left, false)?;
        }
        Ok(())
    }
}

fn click(out: &dyn PointerInjector, button: MouseButton) -> DeviceResult<()> {
    out.button(button, true)?;
    out.button(button, false)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::devices::fake::FakePointer;

    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    #[test]
    fn stick_inside_the_dead_zone_does_not_move() {
        for (x, y) in [(0.0, 0.0), (0.1, 0.1), (0.17, 0.0)] {
            assert_eq!(curve(x, y, MOVE_DEAD_ZONE), (0.0, 0.0));
        }
    }

    #[test]
    fn curve_keeps_the_direction_and_reaches_full_speed() {
        let (x, y) = curve(0.0, -1.0, MOVE_DEAD_ZONE);
        assert!(x.abs() < 1e-6 && (y + 1.0).abs() < 1e-6);
        let (dx, dy) = curve(0.5, 0.5, MOVE_DEAD_ZONE);
        assert!((dx - dy).abs() < 1e-6);
    }

    #[test]
    fn half_tilt_is_much_slower_than_full_tilt() {
        let (half, _) = curve(0.5, 0.0, MOVE_DEAD_ZONE);
        let (full, _) = curve(1.0, 0.0, MOVE_DEAD_ZONE);
        assert!(half < full / 3.0, "half={half} full={full}");
    }

    #[test]
    fn sdl_axes_are_normalized() {
        assert!((from_axis(32767) - 1.0).abs() < 1e-3);
        assert!((from_axis(-32768) + 1.0).abs() < 1e-3);
        assert_eq!(from_axis(0), 0.0);
    }

    #[test]
    fn pointer_moves_right_and_up_on_screen() {
        let (dx, dy) = PointerMotion::default().step(1.0, 1.0, 0.1, 1080);
        assert!(dx > 0);
        assert!(dy < 0, "analógico para cima sobe o ponteiro");
    }

    #[test]
    fn pointer_speed_follows_the_screen_height() {
        let hd = PointerMotion::default().step(1.0, 0.0, 0.1, 1080).0;
        let uhd = PointerMotion::default().step(1.0, 0.0, 0.1, 2160).0;
        assert!((hd * 2 - 1..=hd * 2 + 1).contains(&uhd));
    }

    #[test]
    fn slow_moves_accumulate_instead_of_stalling() {
        let mut m = PointerMotion::default();
        let total: i32 = (0..100).map(|_| m.step(0.25, 0.0, 0.008, 1080).0).sum();
        assert!(total > 0);
        assert_eq!(PointerMotion::default().step(0.25, 0.0, 0.008, 1080).0, 0);
    }

    #[test]
    fn wheel_sends_a_notch_at_once_then_by_rate() {
        let mut w = WheelMotion::default();
        assert_eq!(w.step(0.6, 0.008), 1);
        let total: i32 = (0..125).map(|_| w.step(1.0, 0.008)).sum();
        let max = MAX_NOTCHES_PER_SECOND as i32;
        assert!((max - 1..=max + 1).contains(&total), "{total}");
    }

    #[test]
    fn wheel_down_is_negative_and_the_dead_zone_resets_it() {
        let mut w = WheelMotion::default();
        assert_eq!(w.step(-0.9, 0.008), -1);
        assert_eq!(w.step(0.1, 0.008), 0);
        assert_eq!(w.step(0.9, 0.008), 1);
    }

    fn run(g: &mut ClickGesture, samples: &[(u64, bool)]) -> Vec<ClickEvent> {
        samples
            .iter()
            .map(|(t, h)| g.update(*h, ms(*t)))
            .filter(|e| *e != ClickEvent::None)
            .collect()
    }

    fn ready() -> ClickGesture {
        let mut g = ClickGesture::default();
        g.update(false, Duration::ZERO);
        g
    }

    #[test]
    fn a_tap_is_a_left_click_once_the_double_tap_window_passes() {
        let e = run(
            &mut ready(),
            &[(10, true), (80, false), (200, false), (340, false)],
        );
        assert_eq!(e, [ClickEvent::LeftClick]);
    }

    #[test]
    fn a_tap_does_not_click_before_the_window_ends() {
        assert!(run(&mut ready(), &[(10, true), (80, false), (200, false)]).is_empty());
    }

    #[test]
    fn two_quick_taps_are_one_right_click() {
        let s = [
            (10, true),
            (80, false),
            (180, true),
            (250, false),
            (700, false),
        ];
        assert_eq!(run(&mut ready(), &s), [ClickEvent::RightClick]);
    }

    #[test]
    fn holding_a_drags_with_the_left_button() {
        let s = [
            (10, true),
            (200, true),
            (370, true),
            (900, true),
            (950, false),
            (1500, false),
        ];
        assert_eq!(
            run(&mut ready(), &s),
            [ClickEvent::LeftDown, ClickEvent::LeftUp]
        );
    }

    #[test]
    fn a_held_when_the_mode_starts_is_ignored_until_released() {
        let mut g = ClickGesture::default();
        assert!(
            run(
                &mut g,
                &[(0, true), (500, true), (600, false), (1200, false)]
            )
            .is_empty()
        );
        let e = run(&mut g, &[(1300, true), (1350, false), (1700, false)]);
        assert_eq!(e, [ClickEvent::LeftClick]);
    }

    #[test]
    fn reset_mid_drag_asks_for_the_left_button_to_be_released() {
        let mut g = ready();
        run(&mut g, &[(10, true), (400, true)]);
        assert!(g.reset());
        assert!(!g.reset());
        assert!(run(&mut g, &[(500, true), (900, true)]).is_empty());
    }

    #[test]
    fn controller_mouse_drives_the_injector() {
        let out = FakePointer::default();
        let mut m = ControllerMouse::default();
        m.click.update(false, Duration::ZERO);
        m.step(&out, (1.0, 0.0), (0.0, 0.9), true, ms(10), 0.1, 1080)
            .unwrap();
        m.step(&out, (0.0, 0.0), (0.0, 0.0), true, ms(400), 0.1, 1080)
            .unwrap();
        m.reset(&out).unwrap();
        let events = out.events.lock().unwrap().clone();
        assert!(
            events[0].starts_with("move ") && events[1] == "wheel 1 0",
            "{events:?}"
        );
        assert_eq!(events[2..], ["Left down", "Left up"]);
    }
}
