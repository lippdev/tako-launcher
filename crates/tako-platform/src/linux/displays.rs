//! Telas no Linux, por sessão: X11 (XRandR), KDE (`kscreen-doctor`), GNOME (Mutter por D-Bus),
//! wlroots (`wlr-randr`) e gamescope (só leitura).

use crate::unix::outputs::{
    NoDisplays, Output, OutputBackend, OutputDisplays, OutputMode, x11_refresh,
};
use crate::unix::session::Session;
use crate::unix::{kscreen, mutter, run, wlr};
use std::collections::HashMap;
use tako_core::devices::{DeviceError, DeviceResult, Displays, HdrStatus};

pub fn for_session(session: Session) -> Box<dyn Displays> {
    match session {
        Session::X11 => Box::new(OutputDisplays::new(X11)),
        Session::Gamescope => Box::new(OutputDisplays {
            backend: X11,
            managed_by_session: true,
        }),
        Session::KdeWayland => Box::new(OutputDisplays::new(Kscreen)),
        Session::GnomeWayland => Box::new(OutputDisplays::new(Mutter)),
        Session::Wlroots => Box::new(OutputDisplays::new(Wlr)),
        Session::OtherWayland => Box::new(NoDisplays(
            "compositor Wayland sem API conhecida de telas".into(),
        )),
        Session::Unknown => Box::new(NoDisplays("sem sessão gráfica".into())),
    }
}

// ---------------------------------------------------------------- X11

struct X11;

fn x11_err(e: impl std::fmt::Display) -> DeviceError {
    DeviceError::Failed(format!("X11: {e}"))
}

mod xr {
    pub use x11rb::connection::Connection;
    pub use x11rb::protocol::randr::{self, ConnectionExt as _};
    pub use x11rb::protocol::xproto::ConnectionExt as _;
    pub use x11rb::rust_connection::RustConnection;
}

struct XState {
    conn: xr::RustConnection,
    root: u32,
    res: xr::randr::GetScreenResourcesCurrentReply,
    /// (saída, nome, info).
    outputs: Vec<(u32, String, xr::randr::GetOutputInfoReply)>,
    crtcs: HashMap<u32, xr::randr::GetCrtcInfoReply>,
    primary: u32,
}

fn rotation_deg(r: xr::randr::Rotation) -> u16 {
    use xr::randr::Rotation as R;
    let bits = u16::from(r);
    [(R::ROTATE90, 90), (R::ROTATE180, 180), (R::ROTATE270, 270)]
        .into_iter()
        .find(|(f, _)| bits & u16::from(*f) != 0)
        .map_or(0, |(_, d)| d)
}

fn rotation_flag(deg: u16) -> xr::randr::Rotation {
    use xr::randr::Rotation as R;
    match deg {
        90 => R::ROTATE90,
        180 => R::ROTATE180,
        270 => R::ROTATE270,
        _ => R::ROTATE0,
    }
}

impl X11 {
    fn read() -> DeviceResult<XState> {
        use xr::*;
        let (conn, screen) = x11rb::connect(None).map_err(x11_err)?;
        let root = conn
            .setup()
            .roots
            .get(screen)
            .ok_or_else(|| x11_err("tela X inexistente"))?
            .root;
        conn.randr_query_version(1, 5)
            .map_err(x11_err)?
            .reply()
            .map_err(x11_err)?;
        let res = conn
            .randr_get_screen_resources_current(root)
            .map_err(x11_err)?
            .reply()
            .map_err(x11_err)?;
        let ts = res.config_timestamp;
        let mut outputs = Vec::new();
        for &o in &res.outputs {
            let info = conn
                .randr_get_output_info(o, ts)
                .map_err(x11_err)?
                .reply()
                .map_err(x11_err)?;
            if info.connection == randr::Connection::CONNECTED {
                outputs.push((o, String::from_utf8_lossy(&info.name).into_owned(), info));
            }
        }
        let mut crtcs = HashMap::new();
        for &c in &res.crtcs {
            let info = conn
                .randr_get_crtc_info(c, ts)
                .map_err(x11_err)?
                .reply()
                .map_err(x11_err)?;
            crtcs.insert(c, info);
        }
        let primary = conn
            .randr_get_output_primary(root)
            .map_err(x11_err)?
            .reply()
            .map_err(x11_err)?
            .output;
        Ok(XState {
            conn,
            root,
            res,
            outputs,
            crtcs,
            primary,
        })
    }

    fn mode(st: &XState, id: u32) -> Option<OutputMode> {
        use xr::randr::ModeFlag;
        let m = st.res.modes.iter().find(|m| m.id == id)?;
        let flags = u32::from(m.mode_flags);
        Some(OutputMode {
            key: id.to_string(),
            width: m.width.into(),
            height: m.height.into(),
            refresh: x11_refresh(
                m.dot_clock,
                m.htotal,
                m.vtotal,
                flags & u32::from(ModeFlag::INTERLACE) != 0,
                flags & u32::from(ModeFlag::DOUBLE_SCAN) != 0,
            ),
        })
    }

    fn to_outputs(st: &XState) -> Vec<Output> {
        st.outputs
            .iter()
            .map(|(o, name, info)| {
                let crtc = st.crtcs.get(&info.crtc).filter(|c| c.mode != 0);
                Output {
                    id: name.clone(),
                    name: name.clone(),
                    enabled: crtc.is_some(),
                    primary: *o == st.primary,
                    x: crtc.map_or(0, |c| c.x.into()),
                    y: crtc.map_or(0, |c| c.y.into()),
                    current: crtc.and_then(|c| X11::mode(st, c.mode)),
                    modes: info
                        .modes
                        .iter()
                        .filter_map(|&m| X11::mode(st, m))
                        .collect(),
                    rotation: crtc.map_or(0, |c| rotation_deg(c.rotation)),
                    scale: 1.0,
                    hdr: HdrStatus::Unsupported,
                }
            })
            .collect()
    }
}

impl OutputBackend for X11 {
    fn outputs(&self) -> DeviceResult<Vec<Output>> {
        Ok(X11::to_outputs(&X11::read()?))
    }

    /// Como o `xrandr`: desliga os CRTCs que mudam, ajusta o tamanho da tela e liga de novo.
    fn commit(&self, desired: &[Output]) -> DeviceResult<()> {
        use xr::*;
        let st = X11::read()?;
        let ts = st.res.config_timestamp;
        // (crtc, x, y, modo, rotação, saída) de cada tela ligada.
        let mut plan: Vec<(u32, i16, i16, u32, u16, u32)> = Vec::new();
        let mut used = Vec::new();
        for d in desired.iter().filter(|d| d.enabled) {
            let Some((out, _, info)) = st.outputs.iter().find(|(_, n, _)| *n == d.id) else {
                continue;
            };
            let mode: u32 = d
                .current
                .as_ref()
                .and_then(|m| m.key.parse().ok())
                .ok_or_else(|| DeviceError::NotFound(format!("modo de {}", d.id)))?;
            let crtc = Some(info.crtc)
                .filter(|c| *c != 0)
                .or_else(|| {
                    info.crtcs.iter().copied().find(|c| {
                        !used.contains(c) && st.crtcs.get(c).is_some_and(|ci| ci.outputs.is_empty())
                    })
                })
                .ok_or_else(|| DeviceError::Failed(format!("sem CRTC livre para {}", d.id)))?;
            used.push(crtc);
            let (x, y) = (
                i16::try_from(d.x).map_err(x11_err)?,
                i16::try_from(d.y).map_err(x11_err)?,
            );
            plan.push((crtc, x, y, mode, d.rotation, *out));
        }
        let (w, h) = desired
            .iter()
            .filter(|d| d.enabled)
            .map(|d| {
                let (ew, eh) = d.extent();
                (d.x + ew, d.y + eh)
            })
            .fold((0, 0), |(a, b), (c, d)| (a.max(c), b.max(d)));
        let screen = st
            .conn
            .setup()
            .roots
            .iter()
            .find(|r| r.root == st.root)
            .ok_or_else(|| x11_err("tela X inexistente"))?;
        let mm = |px: i32, cur_px: u16, cur_mm: u16| {
            if cur_px == 0 {
                (px as f64 * 25.4 / 96.0) as u32
            } else {
                (px as f64 * cur_mm as f64 / cur_px as f64) as u32
            }
        };
        let (w16, h16) = (
            u16::try_from(w).map_err(x11_err)?,
            u16::try_from(h).map_err(x11_err)?,
        );
        let c = &st.conn;
        c.grab_server().map_err(x11_err)?;
        let result = (|| -> DeviceResult<()> {
            for (&crtc, info) in &st.crtcs {
                let keep = plan.iter().any(|p| {
                    p.0 == crtc
                        && (p.1, p.2, p.3, p.4)
                            == (info.x, info.y, info.mode, rotation_deg(info.rotation))
                        && info.outputs == [p.5]
                });
                if info.mode != 0 && !keep {
                    c.randr_set_crtc_config(crtc, 0, ts, 0, 0, 0, randr::Rotation::ROTATE0, &[])
                        .map_err(x11_err)?
                        .reply()
                        .map_err(x11_err)?;
                }
            }
            c.randr_set_screen_size(
                st.root,
                w16,
                h16,
                mm(w, screen.width_in_pixels, screen.width_in_millimeters),
                mm(h, screen.height_in_pixels, screen.height_in_millimeters),
            )
            .map_err(x11_err)?
            .check()
            .map_err(x11_err)?;
            for &(crtc, x, y, mode, rot, out) in &plan {
                let r = c
                    .randr_set_crtc_config(crtc, 0, ts, x, y, mode, rotation_flag(rot), &[out])
                    .map_err(x11_err)?
                    .reply()
                    .map_err(x11_err)?;
                if r.status != randr::SetConfig::SUCCESS {
                    return Err(x11_err(format!("SetCrtcConfig {:?}", r.status)));
                }
            }
            if let Some(p) = desired.iter().find(|d| d.enabled && d.primary)
                && let Some((out, _, _)) = st.outputs.iter().find(|(_, n, _)| *n == p.id)
            {
                c.randr_set_output_primary(st.root, *out)
                    .map_err(x11_err)?
                    .check()
                    .map_err(x11_err)?;
            }
            Ok(())
        })();
        let _ = c.ungrab_server();
        let _ = c.flush();
        result
    }
}

// ---------------------------------------------------------------- KDE

struct Kscreen;

impl OutputBackend for Kscreen {
    fn outputs(&self) -> DeviceResult<Vec<Output>> {
        kscreen::parse(&run::tool("kscreen-doctor", &["-j".into()])?).map_err(DeviceError::Failed)
    }
    fn commit(&self, desired: &[Output]) -> DeviceResult<()> {
        run::tool("kscreen-doctor", &kscreen::args(desired)).map(drop)
    }
    fn set_hdr(&self, id: &str, enabled: bool) -> DeviceResult<()> {
        let o = self
            .outputs()?
            .into_iter()
            .find(|o| o.id == id)
            .ok_or_else(|| DeviceError::NotFound(id.into()))?;
        if o.hdr == HdrStatus::Unsupported {
            return Err(DeviceError::Unsupported("HDR".into()));
        }
        let state = if enabled { "enable" } else { "disable" };
        run::tool("kscreen-doctor", &[format!("output.{id}.hdr.{state}")]).map(drop)
    }
}

// ---------------------------------------------------------------- wlroots

struct Wlr;

impl OutputBackend for Wlr {
    fn outputs(&self) -> DeviceResult<Vec<Output>> {
        wlr::parse(&run::tool("wlr-randr", &["--json".into()])?).map_err(DeviceError::Failed)
    }
    fn commit(&self, desired: &[Output]) -> DeviceResult<()> {
        run::tool("wlr-randr", &wlr::args(desired)).map(drop)
    }
}

// ---------------------------------------------------------------- GNOME

struct Mutter;

type Props = HashMap<String, zbus::zvariant::OwnedValue>;
type DbusMode = (String, i32, i32, f64, f64, Vec<f64>, Props);
type DbusMonitor = ((String, String, String, String), Vec<DbusMode>, Props);
type DbusLogical = (
    i32,
    i32,
    f64,
    u32,
    bool,
    Vec<(String, String, String, String)>,
    Props,
);
type DbusState = (u32, Vec<DbusMonitor>, Vec<DbusLogical>, Props);

fn dbus_err(e: impl std::fmt::Display) -> DeviceError {
    DeviceError::Failed(format!("Mutter: {e}"))
}

fn prop_bool(p: &Props, k: &str) -> bool {
    p.get(k)
        .and_then(|v| bool::try_from(v).ok())
        .unwrap_or(false)
}

impl Mutter {
    fn proxy() -> DeviceResult<zbus::blocking::Proxy<'static>> {
        let conn = zbus::blocking::Connection::session().map_err(dbus_err)?;
        zbus::blocking::Proxy::new(
            &conn,
            "org.gnome.Mutter.DisplayConfig",
            "/org/gnome/Mutter/DisplayConfig",
            "org.gnome.Mutter.DisplayConfig",
        )
        .map_err(dbus_err)
    }

    fn state(proxy: &zbus::blocking::Proxy<'_>) -> DeviceResult<mutter::MutterState> {
        let (serial, monitors, logical, props): DbusState =
            proxy.call("GetCurrentState", &()).map_err(dbus_err)?;
        Ok(mutter::MutterState {
            serial,
            monitors: monitors
                .into_iter()
                .map(|((connector, ..), modes, props)| mutter::MutterMonitor {
                    connector,
                    display_name: props
                        .get("display-name")
                        .and_then(|v| String::try_from(v.clone()).ok())
                        .unwrap_or_default(),
                    modes: modes
                        .into_iter()
                        .map(
                            |(id, w, h, refresh, preferred_scale, scales, p)| mutter::MutterMode {
                                current: prop_bool(&p, "is-current"),
                                id,
                                width: w.max(0) as u32,
                                height: h.max(0) as u32,
                                refresh,
                                preferred_scale,
                                scales,
                            },
                        )
                        .collect(),
                })
                .collect(),
            logical: logical
                .into_iter()
                .map(
                    |(x, y, scale, transform, primary, mons, _)| mutter::MutterLogical {
                        x,
                        y,
                        scale,
                        transform,
                        primary,
                        connectors: mons.into_iter().map(|m| m.0).collect(),
                    },
                )
                .collect(),
            physical_layout: props.get("layout-mode").and_then(|v| u32::try_from(v).ok())
                == Some(2),
        })
    }
}

impl OutputBackend for Mutter {
    fn outputs(&self) -> DeviceResult<Vec<Output>> {
        Ok(mutter::outputs(&Mutter::state(&Mutter::proxy()?)?))
    }

    fn commit(&self, desired: &[Output]) -> DeviceResult<()> {
        type Mon<'a> = (String, String, HashMap<String, zbus::zvariant::Value<'a>>);
        let proxy = Mutter::proxy()?;
        let state = Mutter::state(&proxy)?;
        let logical: Vec<(i32, i32, f64, u32, bool, Vec<Mon>)> =
            mutter::logical_config(&state, desired)
                .into_iter()
                .map(|(x, y, s, t, p, mons)| {
                    let mons = mons
                        .into_iter()
                        .map(|(c, m)| (c, m, HashMap::new()))
                        .collect();
                    (x, y, s, t, p, mons)
                })
                .collect();
        let props: HashMap<String, zbus::zvariant::Value> = HashMap::new();
        // Método 1 = temporário: não grava no monitors.xml; o Tako restaura depois.
        proxy
            .call::<_, _, ()>("ApplyMonitorsConfig", &(state.serial, 1u32, logical, props))
            .map_err(dbus_err)
    }
}
