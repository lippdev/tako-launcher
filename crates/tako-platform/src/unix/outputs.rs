//! Modelo comum de telas do Linux e do macOS. Cada backend (XRandR, kscreen-doctor, Mutter,
//! wlr-randr, CoreGraphics) só lê as saídas e aplica um estado completo; achar o modo, ligar e
//! desligar, reposicionar e restaurar ficam aqui, testados sem SO.

use tako_core::devices::{
    ConnectedDisplay, DeviceError, DeviceResult, DisplayMode, DisplayTopology, Displays, HdrStatus,
    MonitorState, ScreenBounds,
};

#[derive(Debug, Clone, PartialEq)]
pub struct OutputMode {
    /// Id do modo no backend (XRandR: id numérico; KDE: nome; Mutter: id; macOS: chave própria).
    pub key: String,
    pub width: u32,
    pub height: u32,
    pub refresh: f64,
}

impl OutputMode {
    pub fn display_mode(&self) -> DisplayMode {
        DisplayMode::new(self.width, self.height, self.refresh.round() as u32)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub primary: bool,
    pub x: i32,
    pub y: i32,
    pub current: Option<OutputMode>,
    pub modes: Vec<OutputMode>,
    /// 0, 90, 180 ou 270.
    pub rotation: u16,
    /// Escala lógica (Wayland); 1 no X11 e no macOS.
    pub scale: f64,
    pub hdr: HdrStatus,
}

impl Output {
    /// Tamanho ocupado no layout (com rotação e escala).
    pub fn extent(&self) -> (i32, i32) {
        let Some(m) = &self.current else {
            return (0, 0);
        };
        let s = if self.scale > 0.0 { self.scale } else { 1.0 };
        let (w, h) = (
            (m.width as f64 / s).round() as i32,
            (m.height as f64 / s).round() as i32,
        );
        if self.rotation % 180 == 90 {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// Modo com o tamanho pedido e a taxa mais próxima (até meio hertz de diferença).
    pub fn find_mode(&self, width: u32, height: u32, refresh: f64) -> Option<&OutputMode> {
        self.modes
            .iter()
            .filter(|m| m.width == width && m.height == height)
            .map(|m| (m, (m.refresh - refresh).abs()))
            .filter(|(_, d)| *d < 0.5)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(m, _)| m)
    }

    fn best_mode(&self) -> Option<&OutputMode> {
        self.modes.iter().max_by(|a, b| {
            (a.width * a.height, a.width)
                .cmp(&(b.width * b.height, b.width))
                .then(a.refresh.total_cmp(&b.refresh))
        })
    }
}

/// Taxa de um modo XRandR a partir do relógio e dos totais (como o `xrandr` calcula).
pub fn x11_refresh(dot_clock: u32, htotal: u16, vtotal: u16, interlace: bool, double: bool) -> f64 {
    let mut v = vtotal as f64;
    if double {
        v *= 2.0;
    }
    if interlace {
        v /= 2.0;
    }
    let total = htotal as f64 * v;
    if total > 0.0 {
        dot_clock as f64 / total
    } else {
        0.0
    }
}

/// O que cada backend implementa.
pub trait OutputBackend: Send + Sync {
    fn outputs(&self) -> DeviceResult<Vec<Output>>;
    /// Aplica o estado inteiro: saídas com `enabled == false` (ou ausentes) são desligadas.
    fn commit(&self, desired: &[Output]) -> DeviceResult<()>;
    fn set_hdr(&self, _id: &str, _enabled: bool) -> DeviceResult<()> {
        Err(DeviceError::Unsupported("HDR".into()))
    }
}

/// [`Displays`] sobre um [`OutputBackend`]. `managed_by_session` (gamescope): só leitura.
pub struct OutputDisplays<B> {
    pub backend: B,
    pub managed_by_session: bool,
}

impl<B: OutputBackend> OutputDisplays<B> {
    pub fn new(backend: B) -> Self {
        OutputDisplays {
            backend,
            managed_by_session: false,
        }
    }

    fn writable(&self) -> DeviceResult<()> {
        if self.managed_by_session {
            return Err(DeviceError::Unsupported(
                "a resolução é definida pela sessão (gamescope)".into(),
            ));
        }
        Ok(())
    }

    fn commit(&self, desired: &[Output]) -> DeviceResult<()> {
        // Nunca deixar o usuário sem nenhuma tela.
        if !desired.iter().any(|o| o.enabled && o.current.is_some()) {
            return Err(DeviceError::Failed("nenhuma tela ficaria ligada".into()));
        }
        self.backend.commit(desired)
    }
}

fn find<'a>(outs: &'a mut [Output], id: &str) -> DeviceResult<&'a mut Output> {
    outs.iter_mut()
        .find(|o| o.id.eq_ignore_ascii_case(id))
        .ok_or_else(|| DeviceError::NotFound(id.into()))
}

/// Liga as telas lado a lado na ordem atual, a principal primeiro em (0, 0) (exigência do macOS e
/// do Windows). shortcut: telas empilhadas na vertical viram horizontais; refazer se pedirem.
pub fn reflow(outs: &mut [Output]) {
    let mut order: Vec<usize> = (0..outs.len()).filter(|&i| outs[i].enabled).collect();
    order.sort_by_key(|&i| (!outs[i].primary, outs[i].x, outs[i].y));
    let mut x = 0;
    for i in order {
        outs[i].x = x;
        outs[i].y = 0;
        x += outs[i].extent().0;
    }
}

impl<B: OutputBackend> Displays for OutputDisplays<B> {
    fn list(&self) -> DeviceResult<Vec<ConnectedDisplay>> {
        Ok(self
            .backend
            .outputs()?
            .into_iter()
            .map(|o| {
                let active = o.enabled && o.current.is_some();
                let (w, h) = o.extent();
                ConnectedDisplay {
                    bounds: active.then_some(ScreenBounds {
                        x: o.x,
                        y: o.y,
                        width: w,
                        height: h,
                    }),
                    current_mode: o
                        .current
                        .as_ref()
                        .filter(|_| active)
                        .map(|m| m.display_mode()),
                    id: o.id,
                    name: o.name,
                    is_active: active,
                    is_primary: o.primary,
                    hdr: o.hdr,
                }
            })
            .collect())
    }

    fn modes(&self, display_id: &str) -> DeviceResult<Vec<DisplayMode>> {
        let mut outs = self.backend.outputs()?;
        let o = find(&mut outs, display_id)?;
        Ok(DisplayMode::normalize(
            o.modes.iter().map(OutputMode::display_mode),
        ))
    }

    fn set_mode(&self, display_id: &str, mode: DisplayMode) -> DeviceResult<()> {
        self.writable()?;
        let mut outs = self.backend.outputs()?;
        let o = find(&mut outs, display_id)?;
        let new = o
            .find_mode(mode.width, mode.height, mode.refresh_hz as f64)
            .cloned()
            .ok_or_else(|| DeviceError::NotFound(format!("modo {} em {display_id}", mode.key())))?;
        let (ox, oy, (ow, oh)) = (o.x, o.y, o.extent());
        o.current = Some(new);
        let (nw, nh) = o.extent();
        let id = o.id.clone();
        // Empurra quem estava à direita ou abaixo, para não sobrepor nem abrir buraco.
        for other in outs.iter_mut().filter(|x| x.enabled && x.id != id) {
            if other.x >= ox + ow {
                other.x += nw - ow;
            }
            if other.y >= oy + oh {
                other.y += nh - oh;
            }
        }
        self.commit(&outs)
    }

    fn set_active(&self, active_ids: &[String], primary: &str) -> DeviceResult<()> {
        self.writable()?;
        if !active_ids.iter().any(|a| a.eq_ignore_ascii_case(primary)) {
            return Err(DeviceError::Failed(
                "a principal precisa estar ligada".into(),
            ));
        }
        let mut outs = self.backend.outputs()?;
        for id in active_ids {
            find(&mut outs, id)?;
        }
        for o in outs.iter_mut() {
            o.enabled = active_ids.iter().any(|a| a.eq_ignore_ascii_case(&o.id));
            o.primary = o.id.eq_ignore_ascii_case(primary);
            if o.enabled && o.current.is_none() {
                o.current = o.best_mode().cloned();
            }
        }
        reflow(&mut outs);
        self.commit(&outs)
    }

    fn set_hdr(&self, display_id: &str, enabled: bool) -> DeviceResult<()> {
        self.writable()?;
        self.backend.set_hdr(display_id, enabled)
    }

    fn capture(&self) -> DeviceResult<DisplayTopology> {
        Ok(DisplayTopology {
            monitors: self
                .backend
                .outputs()?
                .into_iter()
                .filter(|o| o.enabled)
                .filter_map(|o| {
                    let m = o.current.as_ref()?;
                    Some(MonitorState {
                        id: o.id.clone(),
                        name: o.name.clone(),
                        is_primary: o.primary,
                        x: o.x,
                        y: o.y,
                        width: m.width,
                        height: m.height,
                        refresh_hz: m.refresh,
                        rotation: o.rotation,
                    })
                })
                .collect(),
            platform_data: None,
        })
    }

    fn apply(&self, topology: &DisplayTopology) -> DeviceResult<()> {
        self.writable()?;
        let mut outs = self.backend.outputs()?;
        if !outs.iter().any(|o| topology.find(&o.id).is_some()) {
            return Err(DeviceError::NotFound(
                "nenhuma tela do registro está conectada".into(),
            ));
        }
        for o in outs.iter_mut() {
            let Some(m) = topology.find(&o.id) else {
                o.enabled = false;
                o.primary = false;
                continue;
            };
            o.enabled = true;
            o.primary = m.is_primary;
            o.x = m.x;
            o.y = m.y;
            o.rotation = m.rotation;
            match o.find_mode(m.width, m.height, m.refresh_hz).cloned() {
                Some(mode) => o.current = Some(mode),
                // Restaura o resto mesmo sem o modo exato (ex.: a tela mudou de cabo).
                None => tracing::warn!(id = %o.id, "modo {}x{} sumiu", m.width, m.height),
            }
            if o.current.is_none() {
                o.current = o.best_mode().cloned();
            }
        }
        self.commit(&outs)
    }

    fn is_managed_by_session(&self) -> bool {
        self.managed_by_session
    }
}

/// Telas num ambiente sem backend conhecido: tudo `Unsupported`, nunca pânico.
pub struct NoDisplays(pub String);

impl NoDisplays {
    fn err<T>(&self) -> DeviceResult<T> {
        Err(DeviceError::Unsupported(self.0.clone()))
    }
}

impl Displays for NoDisplays {
    fn list(&self) -> DeviceResult<Vec<ConnectedDisplay>> {
        self.err()
    }
    fn modes(&self, _: &str) -> DeviceResult<Vec<DisplayMode>> {
        self.err()
    }
    fn set_mode(&self, _: &str, _: DisplayMode) -> DeviceResult<()> {
        self.err()
    }
    fn set_active(&self, _: &[String], _: &str) -> DeviceResult<()> {
        self.err()
    }
    fn set_hdr(&self, _: &str, _: bool) -> DeviceResult<()> {
        self.err()
    }
    fn capture(&self) -> DeviceResult<DisplayTopology> {
        self.err()
    }
    fn apply(&self, _: &DisplayTopology) -> DeviceResult<()> {
        self.err()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::Mutex;

    pub fn mode(w: u32, h: u32, hz: f64) -> OutputMode {
        OutputMode {
            key: format!("{w}x{h}@{hz}"),
            width: w,
            height: h,
            refresh: hz,
        }
    }

    pub fn output(id: &str, on: bool, primary: bool, x: i32, modes: Vec<OutputMode>) -> Output {
        Output {
            id: id.into(),
            name: id.into(),
            enabled: on,
            primary,
            x,
            y: 0,
            current: on.then(|| modes[0].clone()),
            modes,
            rotation: 0,
            scale: 1.0,
            hdr: HdrStatus::Unsupported,
        }
    }

    /// Monitor 1080p (144 e 59.94/60) principal + TV 4K desligada.
    struct Fake(Mutex<Vec<Output>>, Mutex<u32>);

    impl OutputBackend for Fake {
        fn outputs(&self) -> DeviceResult<Vec<Output>> {
            Ok(self.0.lock().unwrap().clone())
        }
        fn commit(&self, d: &[Output]) -> DeviceResult<()> {
            *self.0.lock().unwrap() = d.to_vec();
            *self.1.lock().unwrap() += 1;
            Ok(())
        }
    }

    fn desk() -> OutputDisplays<Fake> {
        OutputDisplays::new(Fake(
            Mutex::new(vec![
                output(
                    "DP-1",
                    true,
                    true,
                    0,
                    vec![
                        mode(1920, 1080, 144.0),
                        mode(1920, 1080, 59.94),
                        mode(1920, 1080, 60.0),
                        mode(1280, 720, 60.0),
                    ],
                ),
                output(
                    "HDMI-1",
                    false,
                    false,
                    0,
                    vec![mode(3840, 2160, 60.0), mode(3840, 2160, 120.0)],
                ),
            ]),
            Mutex::new(0),
        ))
    }

    fn state(d: &OutputDisplays<Fake>) -> Vec<Output> {
        d.backend.0.lock().unwrap().clone()
    }

    #[test]
    fn lists_and_normalizes_modes() {
        let d = desk();
        let l = d.list().unwrap();
        assert!(l[0].is_active && l[0].is_primary);
        assert_eq!(l[0].current_mode, Some(DisplayMode::new(1920, 1080, 144)));
        assert!(!l[1].is_active && l[1].bounds.is_none());
        assert_eq!(
            d.modes("dp-1").unwrap(),
            vec![
                DisplayMode::new(1920, 1080, 144),
                DisplayMode::new(1920, 1080, 60),
                DisplayMode::new(1280, 720, 60)
            ]
        );
        assert!(matches!(d.modes("X"), Err(DeviceError::NotFound(_))));
    }

    #[test]
    fn set_mode_picks_closest_refresh_and_pushes_neighbours() {
        let d = desk();
        d.set_active(&["DP-1".into(), "HDMI-1".into()], "DP-1")
            .unwrap();
        assert_eq!(state(&d)[1].x, 1920);
        d.set_mode("DP-1", DisplayMode::new(1920, 1080, 60))
            .unwrap();
        assert_eq!(state(&d)[0].current.as_ref().unwrap().refresh, 60.0);
        d.set_mode("DP-1", DisplayMode::new(1280, 720, 60)).unwrap();
        assert_eq!(state(&d)[1].x, 1280);
        assert!(matches!(
            d.set_mode("DP-1", DisplayMode::new(800, 600, 60)),
            Err(DeviceError::NotFound(_))
        ));
    }

    #[test]
    fn set_active_puts_primary_first_and_picks_best_mode() {
        let d = desk();
        d.set_active(&["HDMI-1".into()], "HDMI-1").unwrap();
        let s = state(&d);
        assert!(!s[0].enabled && s[1].enabled && s[1].primary);
        assert_eq!(s[1].current.as_ref().unwrap().refresh, 120.0);
        assert_eq!((s[1].x, s[1].y), (0, 0));
        assert!(d.set_active(&["DP-1".into()], "HDMI-1").is_err());
        assert!(matches!(
            d.set_active(&["NOPE".into()], "NOPE"),
            Err(DeviceError::NotFound(_))
        ));
    }

    #[test]
    fn capture_then_apply_restores_and_is_idempotent() {
        let d = desk();
        let before = d.capture().unwrap();
        d.set_active(&["HDMI-1".into()], "HDMI-1").unwrap();
        d.apply(&before).unwrap();
        assert!(d.capture().unwrap().is_equivalent_to(&before));
        d.apply(&before).unwrap();
        assert!(d.capture().unwrap().is_equivalent_to(&before));
        // Registro de outra mesa: não desliga tudo.
        let other = DisplayTopology {
            monitors: vec![MonitorState {
                id: "VGA-9".into(),
                ..before.monitors[0].clone()
            }],
            platform_data: None,
        };
        assert!(d.apply(&other).is_err());
        assert!(state(&d)[0].enabled);
    }

    #[test]
    fn managed_session_is_read_only() {
        let mut d = desk();
        d.managed_by_session = true;
        assert!(d.list().is_ok());
        assert!(matches!(
            d.set_mode("DP-1", DisplayMode::new(1920, 1080, 60)),
            Err(DeviceError::Unsupported(_))
        ));
        assert_eq!(*d.backend.1.lock().unwrap(), 0);
    }

    #[test]
    fn extent_handles_rotation_and_scale() {
        let mut o = output("A", true, true, 0, vec![mode(3840, 2160, 60.0)]);
        o.scale = 2.0;
        assert_eq!(o.extent(), (1920, 1080));
        o.rotation = 90;
        assert_eq!(o.extent(), (1080, 1920));
    }

    #[test]
    fn x11_refresh_like_xrandr() {
        // 1920x1080@60 CEA: 148.5 MHz, 2200 x 1125.
        assert!((x11_refresh(148_500_000, 2200, 1125, false, false) - 60.0).abs() < 1e-9);
        assert!((x11_refresh(74_250_000, 2200, 1125, true, false) - 60.0).abs() < 1e-9);
        assert!((x11_refresh(148_500_000, 2200, 1125, false, true) - 30.0).abs() < 1e-9);
        assert_eq!(x11_refresh(1, 0, 0, false, false), 0.0);
    }

    #[test]
    fn no_displays_never_panics() {
        let d = NoDisplays("compositor desconhecido".into());
        assert!(matches!(d.list(), Err(DeviceError::Unsupported(_))));
        assert!(!d.is_managed_by_session());
    }
}
