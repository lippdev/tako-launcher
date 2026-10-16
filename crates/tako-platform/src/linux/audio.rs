//! Saída de áudio no PipeWire/PulseAudio pelo `pactl` (ids = nomes dos sinks, estáveis) ou, sem
//! ele, pelo `wpctl` (ids numéricos do PipeWire). shortcut: os ids do `wpctl` mudam entre sessões,
//! então a saída por jogo só é estável com o `pactl`; resolver pelo `node.name` se pedirem.

use crate::unix::pulse::{self, PA_VOLUME_NORM, Sink};
use crate::unix::run;
use std::sync::OnceLock;
use tako_core::devices::audio::AudioLevel;
use tako_core::devices::{AudioDevice, AudioOutputs, AudioState, DeviceError, DeviceResult};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Pactl,
    Wpctl,
    None,
}

#[derive(Default)]
pub struct PipeWireAudio(OnceLock<Tool>);

fn pactl_sinks() -> DeviceResult<Vec<Sink>> {
    let json = run::tool(
        "pactl",
        &["-f".into(), "json".into(), "list".into(), "sinks".into()],
    )?;
    pulse::parse_pactl_sinks(&json).map_err(DeviceError::Failed)
}

impl PipeWireAudio {
    /// Decide uma vez: `pactl` antigo (sem `-f json`) cai no `wpctl`.
    fn tool(&self) -> Tool {
        *self.0.get_or_init(|| {
            if run::has_tool("pactl") && pactl_sinks().is_ok() {
                Tool::Pactl
            } else if run::has_tool("wpctl") {
                Tool::Wpctl
            } else {
                Tool::None
            }
        })
    }

    fn sinks(&self) -> DeviceResult<(Vec<Sink>, Option<String>)> {
        match self.tool() {
            Tool::Pactl => {
                let sinks = pactl_sinks()?;
                let default = run::tool("pactl", &["get-default-sink".into()])
                    .ok()
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty());
                Ok((sinks, default))
            }
            Tool::Wpctl => {
                let sinks = pulse::parse_wpctl_status(&run::tool("wpctl", &["status".into()])?);
                let default = sinks.iter().find(|s| s.is_default).map(|s| s.id.clone());
                Ok((sinks, default))
            }
            Tool::None => Err(unsupported()),
        }
    }

    fn run(&self, pactl: [&str; 3], wpctl: [&str; 3]) -> DeviceResult<()> {
        let (program, args) = match self.tool() {
            Tool::Pactl => ("pactl", pactl),
            Tool::Wpctl => ("wpctl", wpctl),
            Tool::None => return Err(unsupported()),
        };
        let args: Vec<String> = args
            .iter()
            .filter(|a| !a.is_empty())
            .map(|a| a.to_string())
            .collect();
        run::tool(program, &args).map(drop)
    }

    fn check(&self, id: &str) -> DeviceResult<()> {
        if self.sinks()?.0.iter().any(|s| s.id == id) {
            Ok(())
        } else {
            Err(DeviceError::NotFound(id.into()))
        }
    }
}

fn unsupported() -> DeviceError {
    DeviceError::Unsupported("nem pactl nem wpctl encontrados".into())
}

impl AudioOutputs for PipeWireAudio {
    fn can_set_default(&self) -> bool {
        self.tool() != Tool::None
    }

    fn capture(&self) -> DeviceResult<AudioState> {
        let (sinks, default_id) = self.sinks()?;
        Ok(AudioState {
            levels: sinks
                .iter()
                .map(|s| AudioLevel {
                    device_id: s.id.clone(),
                    volume: s.volume,
                    muted: s.muted,
                })
                .collect(),
            devices: sinks
                .into_iter()
                .map(|s| AudioDevice {
                    id: s.id,
                    name: s.description,
                })
                .collect(),
            default_id,
        })
    }

    fn set_default(&self, id: &str) -> DeviceResult<()> {
        self.check(id)?;
        self.run(["set-default-sink", id, ""], ["set-default", id, ""])
    }

    fn set_volume(&self, id: &str, volume: f32) -> DeviceResult<()> {
        self.check(id)?;
        let v = volume.clamp(0.0, 1.0) as f64;
        let raw = ((v * PA_VOLUME_NORM).round() as u32).to_string();
        let scalar = format!("{v:.3}");
        self.run(["set-sink-volume", id, &raw], ["set-volume", id, &scalar])
    }

    fn set_mute(&self, id: &str, muted: bool) -> DeviceResult<()> {
        self.check(id)?;
        let m = if muted { "1" } else { "0" };
        self.run(["set-sink-mute", id, m], ["set-mute", id, m])
    }
}
