//! Chamadas `system.*`: o que o sistema operacional sabe (telas, saídas de áudio) e a interface só lista, para
//! o usuário escolher onde o jogo aparece e toca. Quem lê é a plataforma (`tako-platform`); em demonstração
//! (`simulate`) valem as versões em memória do núcleo, que não tocam na máquina.

use crate::protocol::*;
use std::sync::Arc;
use tako_core::devices::{
    self, DeviceError,
    fake::{FakeAudio, FakeDisplays},
};

/// As funções de plataforma que as consultas `system.*` usam.
pub(crate) struct Devices {
    displays: Arc<dyn devices::Displays>,
    audio: Arc<dyn devices::AudioOutputs>,
}

impl Devices {
    pub(crate) fn new(platform: &tako_platform::Platform, simulate: bool) -> Self {
        if simulate {
            Self {
                displays: Arc::new(FakeDisplays::desk()),
                audio: Arc::new(FakeAudio::desk()),
            }
        } else {
            Self {
                displays: Arc::clone(&platform.displays),
                audio: Arc::clone(&platform.audio),
            }
        }
    }
}

/// `Unsupported` e `PermissionDenied` são "válido, mas não agora" (o sistema não deixa); o texto fica no log.
fn device_error(what: &str, error: DeviceError) -> BridgeError {
    tracing::warn!("{what}: {error}");
    let (code, message) = match error {
        DeviceError::Unsupported(_) | DeviceError::PermissionDenied(_) => {
            (ErrorCode::Conflict, "o sistema não permite")
        }
        DeviceError::NotFound(_) => (ErrorCode::NotFound, "não encontrado"),
        DeviceError::Failed(_) => (ErrorCode::Internal, "falha ao consultar o sistema"),
    };
    BridgeError::new(code, format!("{what}: {message}"))
}

fn feature(status: devices::HdrStatus) -> DisplayFeature {
    match status {
        devices::HdrStatus::Unknown => DisplayFeature::Unknown,
        devices::HdrStatus::Unsupported => DisplayFeature::Unsupported,
        devices::HdrStatus::Off => DisplayFeature::Off,
        devices::HdrStatus::On => DisplayFeature::On,
    }
}

pub(crate) fn displays(devices: &Devices) -> Result<DisplayList, BridgeError> {
    let connected = devices
        .displays
        .list()
        .map_err(|e| device_error("telas", e))?;
    let displays = connected
        .into_iter()
        .map(|screen| {
            // Sem a lista de modos a tela ainda serve: a interface só não oferece resolução.
            let modes = devices
                .displays
                .modes(&screen.id)
                .map(devices::DisplayMode::normalize)
                .unwrap_or_else(|e| {
                    tracing::warn!("modos da tela {}: {e}", screen.id);
                    Vec::new()
                });
            let name = if screen.name.trim().is_empty() {
                screen.id.clone()
            } else {
                screen.name
            };
            DisplayInfo {
                primary: screen.is_primary,
                active: screen.is_active,
                hdr: feature(screen.hdr),
                vrr: feature(screen.vrr),
                modes: modes.iter().map(devices::DisplayMode::key).collect(),
                id: screen.id,
                name,
            }
        })
        .collect();
    Ok(DisplayList {
        displays,
        managed_by_session: devices.displays.is_managed_by_session(),
        vrr_limitation: devices.displays.vrr_limitation(),
    })
}

pub(crate) fn audio_outputs(devices: &Devices) -> Result<AudioOutputList, BridgeError> {
    let state = devices
        .audio
        .capture()
        .map_err(|e| device_error("saídas de áudio", e))?;
    let outputs = state
        .devices
        .iter()
        .map(|device| AudioOutput {
            id: device.id.clone(),
            name: if device.name.trim().is_empty() {
                device.id.clone()
            } else {
                device.name.clone()
            },
            default: state
                .default_id
                .as_deref()
                .is_some_and(|id| id.eq_ignore_ascii_case(&device.id)),
        })
        .collect();
    Ok(AudioOutputList {
        outputs,
        can_switch: devices.audio.can_set_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppService, Core};
    use serde_json::{Value, json};
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    /// Plataforma com a leitura de áudio quebrada, para ver o erro e para ver que a demonstração a ignora.
    struct BrokenAudio;
    impl devices::AudioOutputs for BrokenAudio {
        fn can_set_default(&self) -> bool {
            false
        }
        fn capture(&self) -> devices::DeviceResult<devices::AudioState> {
            Err(DeviceError::Failed("WASAPI indisponível".into()))
        }
        fn set_default(&self, _: &str) -> devices::DeviceResult<()> {
            Ok(())
        }
        fn set_volume(&self, _: &str, _: f32) -> devices::DeviceResult<()> {
            Ok(())
        }
        fn set_mute(&self, _: &str, _: bool) -> devices::DeviceResult<()> {
            Ok(())
        }
    }

    fn service(simulate: bool) -> (AppService, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let platform = Platform {
            audio: Arc::new(BrokenAudio),
            ..Platform::simulated()
        };
        let core =
            Core::open_with_sources(DataPaths::at(dir.path()), platform, simulate, vec![]).unwrap();
        (AppService::with_core("test", core), dir)
    }

    /// Passa pelo mesmo analisador que o hospedeiro usa e devolve a resposta como JSON.
    fn wire(app: &mut AppService, name: &str, payload: Option<Value>) -> Value {
        let mut value = json!({"v": 1, "id": 3, "name": name});
        if let Some(payload) = payload {
            value["payload"] = payload;
        }
        let response = match parse_request(&value.to_string()) {
            Ok(request) => match app.handle(request.call) {
                Ok(reply) => Response::ok(request.id, reply),
                Err(error) => Response::err(request.id, error),
            },
            Err((id, error)) => Response::err(id, error),
        };
        serde_json::to_value(response).unwrap()
    }

    #[test]
    fn displays_list_the_simulated_desk_with_their_modes() {
        let (mut app, _dir) = service(true);
        let reply = wire(&mut app, "system.displays", None);
        assert_eq!(reply["ok"], true, "{reply}");
        let data = &reply["reply"]["data"];
        assert_eq!(data["managedBySession"], false);
        assert_eq!(data["vrrLimitation"], Value::Null);
        let displays = data["displays"].as_array().unwrap();
        assert_eq!(displays.len(), 2);
        assert_eq!(displays[0]["id"], "DISPLAY1");
        assert_eq!(displays[0]["name"], "Monitor");
        assert_eq!(displays[0]["primary"], true);
        assert_eq!(displays[0]["active"], true);
        assert_eq!(displays[0]["hdr"], "unsupported");
        assert_eq!(
            displays[0]["modes"],
            json!(["1920x1080@144", "1920x1080@60", "1280x720@60"])
        );
        assert_eq!(displays[1]["id"], "DISPLAY2");
        assert_eq!(displays[1]["active"], false);
        assert_eq!(displays[1]["hdr"], "off");
        assert_eq!(displays[1]["vrr"], "off");
        assert_eq!(displays[1]["modes"][0], "3840x2160@120");
    }

    #[test]
    fn audio_outputs_mark_the_default_and_whether_switching_is_allowed() {
        let (mut app, _dir) = service(true);
        let reply = wire(&mut app, "system.audio_outputs", None);
        assert_eq!(reply["ok"], true, "{reply}");
        assert_eq!(
            reply["reply"]["data"],
            json!({
                "outputs": [
                    {"id": "speakers", "name": "Alto-falantes", "default": true},
                    {"id": "tv", "name": "TV (HDMI)", "default": false},
                ],
                "canSwitch": true,
            })
        );
    }

    #[test]
    fn a_failing_platform_becomes_a_bridge_error_but_demo_mode_never_asks_it() {
        let (mut app, _dir) = service(false);
        let reply = wire(&mut app, "system.audio_outputs", None);
        assert_eq!(reply["ok"], false);
        assert_eq!(reply["error"]["code"], "internal");
        // Quem não é do sistema real não pode ser alterado por ele: a demonstração usa as falsas.
        let (mut demo, _dir) = service(true);
        assert_eq!(wire(&mut demo, "system.audio_outputs", None)["ok"], true);
    }

    #[test]
    fn device_errors_map_to_bridge_codes() {
        let code = |e| device_error("x", e).code;
        assert_eq!(
            code(DeviceError::Unsupported("a".into())),
            ErrorCode::Conflict
        );
        assert_eq!(
            code(DeviceError::PermissionDenied("a".into())),
            ErrorCode::Conflict
        );
        assert_eq!(code(DeviceError::NotFound("a".into())), ErrorCode::NotFound);
        assert_eq!(code(DeviceError::Failed("a".into())), ErrorCode::Internal);
    }

    #[test]
    fn the_calls_take_no_arguments_and_need_the_core() {
        let (mut app, _dir) = service(true);
        for name in ["system.displays", "system.audio_outputs"] {
            let reply = wire(&mut app, name, Some(json!({})));
            assert_eq!(reply["error"]["code"], "bad_request", "{name}");
        }
        let mut bare = AppService::new("1");
        for call in [Call::SystemDisplays, Call::SystemAudioOutputs] {
            assert_eq!(bare.handle(call).unwrap_err().code, ErrorCode::Internal);
        }
    }
}
