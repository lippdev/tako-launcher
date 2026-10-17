//! Implementações em memória (port de `ConsoleMode.Runtime/Simulation/*`): testes, CI e `--simulate`.

use super::*;
use std::sync::Mutex;

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Telas e os modos de cada uma.
type Desk = (Vec<ConnectedDisplay>, Vec<(String, Vec<DisplayMode>)>);

/// Mesa simulada: telas, modos e topologia.
pub struct FakeDisplays {
    pub state: Mutex<Desk>,
    pub log: Mutex<Vec<String>>,
}

impl FakeDisplays {
    /// Uma TV 4K (desligada) e um monitor 1080p principal.
    pub fn desk() -> Self {
        let monitor = ConnectedDisplay {
            id: "DISPLAY1".into(),
            name: "Monitor".into(),
            is_active: true,
            is_primary: true,
            bounds: Some(ScreenBounds {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            }),
            current_mode: Some(DisplayMode::new(1920, 1080, 144)),
            hdr: HdrStatus::Unsupported,
        };
        let tv = ConnectedDisplay {
            id: "DISPLAY2".into(),
            name: "TV".into(),
            is_active: false,
            is_primary: false,
            bounds: None,
            current_mode: None,
            hdr: HdrStatus::Off,
        };
        let modes = vec![
            (
                "DISPLAY1".into(),
                vec![
                    DisplayMode::new(1920, 1080, 144),
                    DisplayMode::new(1920, 1080, 60),
                    DisplayMode::new(1280, 720, 60),
                ],
            ),
            (
                "DISPLAY2".into(),
                vec![
                    DisplayMode::new(3840, 2160, 120),
                    DisplayMode::new(3840, 2160, 60),
                    DisplayMode::new(1920, 1080, 120),
                ],
            ),
        ];
        FakeDisplays {
            state: Mutex::new((vec![monitor, tv], modes)),
            log: Mutex::new(Vec::new()),
        }
    }
}

impl Displays for FakeDisplays {
    fn list(&self) -> DeviceResult<Vec<ConnectedDisplay>> {
        Ok(lock(&self.state).0.clone())
    }
    fn modes(&self, id: &str) -> DeviceResult<Vec<DisplayMode>> {
        lock(&self.state)
            .1
            .iter()
            .find(|(d, _)| d == id)
            .map(|(_, m)| m.clone())
            .ok_or_else(|| DeviceError::NotFound(id.into()))
    }
    fn set_mode(&self, id: &str, mode: DisplayMode) -> DeviceResult<()> {
        let mut s = lock(&self.state);
        let d =
            s.0.iter_mut()
                .find(|d| d.id == id)
                .ok_or_else(|| DeviceError::NotFound(id.into()))?;
        d.current_mode = Some(mode);
        if let Some(b) = d.bounds.as_mut() {
            b.width = mode.width as i32;
            b.height = mode.height as i32;
        }
        lock(&self.log).push(format!("mode {id} {}", mode.key()));
        Ok(())
    }
    fn set_active(&self, active: &[String], primary: &str) -> DeviceResult<()> {
        let mut s = lock(&self.state);
        for d in s.0.iter_mut() {
            d.is_active = active.contains(&d.id);
            d.is_primary = d.id == primary;
            if d.is_active && d.current_mode.is_none() {
                d.current_mode = Some(DisplayMode::new(1920, 1080, 60));
                d.bounds = Some(ScreenBounds {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                });
            }
        }
        lock(&self.log).push(format!("active {} primary {primary}", active.join(",")));
        Ok(())
    }
    fn set_hdr(&self, id: &str, enabled: bool) -> DeviceResult<()> {
        let mut s = lock(&self.state);
        let d =
            s.0.iter_mut()
                .find(|d| d.id == id)
                .ok_or_else(|| DeviceError::NotFound(id.into()))?;
        if d.hdr == HdrStatus::Unsupported {
            return Err(DeviceError::Unsupported("HDR".into()));
        }
        d.hdr = if enabled {
            HdrStatus::On
        } else {
            HdrStatus::Off
        };
        Ok(())
    }
    fn capture(&self) -> DeviceResult<DisplayTopology> {
        let s = lock(&self.state);
        Ok(DisplayTopology {
            monitors: s
                .0
                .iter()
                .filter(|d| d.is_active)
                .map(|d| {
                    let m = d.current_mode.unwrap_or(DisplayMode::new(1920, 1080, 60));
                    let b = d.bounds.unwrap_or(ScreenBounds {
                        x: 0,
                        y: 0,
                        width: 0,
                        height: 0,
                    });
                    MonitorState {
                        id: d.id.clone(),
                        name: d.name.clone(),
                        is_primary: d.is_primary,
                        x: b.x,
                        y: b.y,
                        width: m.width,
                        height: m.height,
                        refresh_hz: m.refresh_hz as f64,
                        rotation: 0,
                    }
                })
                .collect(),
            platform_data: None,
        })
    }
    fn apply(&self, t: &DisplayTopology) -> DeviceResult<()> {
        let mut s = lock(&self.state);
        for d in s.0.iter_mut() {
            match t.find(&d.id) {
                Some(m) => {
                    d.is_active = true;
                    d.is_primary = m.is_primary;
                    d.current_mode = Some(DisplayMode::new(
                        m.width,
                        m.height,
                        m.refresh_hz.round() as u32,
                    ));
                    d.bounds = Some(ScreenBounds {
                        x: m.x,
                        y: m.y,
                        width: m.width as i32,
                        height: m.height as i32,
                    });
                }
                None => {
                    d.is_active = false;
                    d.is_primary = false;
                }
            }
        }
        lock(&self.log).push("apply".into());
        Ok(())
    }
}

pub struct FakeAudio {
    pub state: Mutex<AudioState>,
}

impl FakeAudio {
    pub fn desk() -> Self {
        FakeAudio {
            state: Mutex::new(AudioState {
                devices: vec![
                    AudioDevice {
                        id: "speakers".into(),
                        name: "Alto-falantes".into(),
                    },
                    AudioDevice {
                        id: "tv".into(),
                        name: "TV (HDMI)".into(),
                    },
                ],
                default_id: Some("speakers".into()),
                levels: vec![
                    AudioLevel {
                        device_id: "speakers".into(),
                        volume: 0.5,
                        muted: false,
                    },
                    AudioLevel {
                        device_id: "tv".into(),
                        volume: 0.8,
                        muted: false,
                    },
                ],
            }),
        }
    }
}

use super::audio::AudioLevel;

impl AudioOutputs for FakeAudio {
    fn can_set_default(&self) -> bool {
        true
    }
    fn capture(&self) -> DeviceResult<AudioState> {
        Ok(lock(&self.state).clone())
    }
    fn set_default(&self, id: &str) -> DeviceResult<()> {
        let mut s = lock(&self.state);
        if s.find(id).is_none() {
            return Err(DeviceError::NotFound(id.into()));
        }
        s.default_id = Some(id.into());
        Ok(())
    }
    fn set_volume(&self, id: &str, v: f32) -> DeviceResult<()> {
        let mut s = lock(&self.state);
        let l = s
            .levels
            .iter_mut()
            .find(|l| l.device_id == id)
            .ok_or_else(|| DeviceError::NotFound(id.into()))?;
        l.volume = v.clamp(0.0, 1.0);
        Ok(())
    }
    fn set_mute(&self, id: &str, m: bool) -> DeviceResult<()> {
        let mut s = lock(&self.state);
        let l = s
            .levels
            .iter_mut()
            .find(|l| l.device_id == id)
            .ok_or_else(|| DeviceError::NotFound(id.into()))?;
        l.muted = m;
        Ok(())
    }
}

#[derive(Default)]
pub struct FakePointer {
    pub hidden: Mutex<bool>,
    pub events: Mutex<Vec<String>>,
}

impl CursorControl for FakePointer {
    fn set_hidden(&self, hidden: bool) -> DeviceResult<()> {
        *lock(&self.hidden) = hidden;
        Ok(())
    }
    fn is_hidden(&self) -> bool {
        *lock(&self.hidden)
    }
    fn recover_after_crash(&self) {
        *lock(&self.hidden) = false;
    }
}

impl PointerInjector for FakePointer {
    fn move_by(&self, dx: i32, dy: i32) -> DeviceResult<()> {
        lock(&self.events).push(format!("move {dx} {dy}"));
        Ok(())
    }
    fn button(&self, b: MouseButton, down: bool) -> DeviceResult<()> {
        lock(&self.events).push(format!("{b:?} {}", if down { "down" } else { "up" }));
        Ok(())
    }
    fn wheel(&self, v: i32, h: i32) -> DeviceResult<()> {
        lock(&self.events).push(format!("wheel {v} {h}"));
        Ok(())
    }
}

/// Processos simulados: `launch` cria um processo que fica "rodando" até `exit`.
#[derive(Default)]
pub struct FakeProcesses {
    pub running: Mutex<Vec<ProcessEntry>>,
    pub launched: Mutex<Vec<String>>,
    pub steam_app: Mutex<u32>,
    /// Variáveis da última abertura com `launch_with_env`.
    pub env: Mutex<Vec<(String, String)>>,
    /// Comandos rodados por `run_command`.
    pub commands: Mutex<Vec<String>>,
    /// Códigos de saída dos processos que terminaram com [`FakeProcesses::exit_with`].
    pub exit_codes: Mutex<Vec<(u32, u32)>>,
    next_pid: Mutex<u32>,
}

impl FakeProcesses {
    pub fn exit(&self, pid: u32) {
        lock(&self.running).retain(|p| p.pid != pid);
    }

    /// Termina com um código de saída (o Tako abriu o executável direto).
    pub fn exit_with(&self, pid: u32, code: u32) {
        self.exit(pid);
        lock(&self.exit_codes).push((pid, code));
    }
}

impl ProcessLauncher for FakeProcesses {
    fn launch(
        &self,
        target: &str,
        args: Option<&str>,
        _cwd: Option<&str>,
    ) -> DeviceResult<Option<u32>> {
        let mut n = lock(&self.next_pid);
        *n += 1;
        let pid = 1000 + *n;
        lock(&self.launched).push(format!("{target} {}", args.unwrap_or("")).trim().to_owned());
        lock(&self.running).push(ProcessEntry {
            pid,
            parent_pid: None,
            exe_path: Some(target.to_owned()),
            started_at: Some(chrono::Utc::now()),
        });
        Ok(Some(pid))
    }
    fn launch_with_env(
        &self,
        target: &str,
        args: Option<&str>,
        cwd: Option<&str>,
        env: &[(String, String)],
    ) -> DeviceResult<Option<u32>> {
        *lock(&self.env) = env.to_vec();
        self.launch(target, args, cwd)
    }
    fn run_command(&self, command: &str, _wait: Option<std::time::Duration>) -> DeviceResult<()> {
        lock(&self.commands).push(command.to_owned());
        Ok(())
    }
    fn snapshot(&self) -> Vec<ProcessEntry> {
        lock(&self.running).clone()
    }
    fn exit_code(&self, pid: u32) -> Option<u32> {
        let mut codes = lock(&self.exit_codes);
        let i = codes.iter().position(|(p, _)| *p == pid)?;
        Some(codes.remove(i).1)
    }
    fn request_close(&self, pids: &[u32]) -> DeviceResult<()> {
        lock(&self.running).retain(|p| !pids.contains(&p.pid));
        Ok(())
    }
    fn force_close(&self, pids: &[u32]) -> DeviceResult<()> {
        self.request_close(pids)
    }
    fn steam_running_app_id(&self) -> u32 {
        *lock(&self.steam_app)
    }
    fn open_folder(&self, _path: &str) -> DeviceResult<()> {
        Ok(())
    }
}

/// Driver de esconder controles simulado: guarda o que está escondido.
#[derive(Debug)]
pub struct FakeHiding {
    pub installed: bool,
    pub hidden: Mutex<Vec<(u16, u16)>>,
    pub log: Mutex<Vec<String>>,
}

impl FakeHiding {
    pub fn new(installed: bool) -> Self {
        FakeHiding {
            installed,
            hidden: Mutex::new(Vec::new()),
            log: Mutex::new(Vec::new()),
        }
    }
}

impl ControllerHiding for FakeHiding {
    fn available(&self) -> bool {
        self.installed
    }
    fn supported(&self) -> bool {
        true
    }
    fn hide(&self, pads: &[(u16, u16)]) -> DeviceResult<()> {
        if !self.installed {
            return Err(DeviceError::NotFound("driver".into()));
        }
        *lock(&self.hidden) = pads.to_vec();
        lock(&self.log).push(format!("hide {pads:04x?}"));
        Ok(())
    }
    fn reveal(&self) -> DeviceResult<()> {
        if !std::mem::take(&mut *lock(&self.hidden)).is_empty() {
            lock(&self.log).push("reveal".into());
        }
        Ok(())
    }
    fn is_hidden(&self) -> bool {
        !lock(&self.hidden).is_empty()
    }
}
