//! Central rápida (WEB-22): intenção de abertura e controles do sistema pertencem ao núcleo.

use crate::protocol::*;
use crate::{AppService, system};
use tako_core::devices::{DeviceError, DisplayMode};
use tako_core::input::shortcut::KeyShortcut;

fn device_error(error: DeviceError) -> BridgeError {
    tracing::warn!(%error, "controle rápido");
    let code = match error {
        DeviceError::NotFound(_) => ErrorCode::NotFound,
        DeviceError::Unsupported(_) | DeviceError::PermissionDenied(_) => ErrorCode::Conflict,
        DeviceError::Failed(_) => ErrorCode::Internal,
    };
    BridgeError::new(code, error.to_string())
}

impl AppService {
    pub fn central_open(&self) -> bool {
        self.central_open && self.game_active()
    }

    pub fn set_central_open(&mut self, open: bool) {
        let open = open && self.game_active();
        if self.central_open != open {
            self.central_open = open;
            self.emit(Event::ControlCenterOpen(open));
        }
    }

    /// Mesma chave e mesmo padrão do supervisor Slint; fora da sessão não registra atalho.
    pub fn central_shortcut(&self) -> Option<KeyShortcut> {
        self.game_active().then(|| {
            KeyShortcut::parse(
                self.core
                    .as_ref()
                    .and_then(|c| c.setting("ControlCenterKey")),
            )
        })
    }

    fn central_state(&self) -> Result<ControlCenterState, BridgeError> {
        let devices = &self.require_core()?.devices;
        let audio = match devices.audio().capture() {
            Ok(state) => {
                let level = state.default_id.as_ref().and_then(|id| state.level_of(id));
                Some(ControlCenterAudio {
                    outputs: system::audio_outputs(devices)?,
                    default_id: state.default_id.clone(),
                    volume: level.map(|l| l.volume),
                    muted: level.is_some_and(|l| l.muted),
                })
            }
            Err(error) => {
                tracing::warn!(%error, "áudio da Central");
                None
            }
        };
        let displays = devices
            .displays()
            .list()
            .unwrap_or_else(|error| {
                tracing::warn!(%error, "telas da Central");
                Vec::new()
            })
            .into_iter()
            .filter(|d| d.is_active)
            .map(|d| {
                let modes = devices.displays().modes(&d.id).unwrap_or_else(|error| {
                    tracing::warn!(%error, "modos da Central");
                    Vec::new()
                });
                ControlCenterDisplay {
                    id: d.id,
                    name: d.name,
                    current_mode: d.current_mode.map(|m| m.key()),
                    modes: DisplayMode::normalize(modes)
                        .into_iter()
                        .map(|m| m.key())
                        .collect(),
                }
            })
            .collect();
        let usage = tako_platform::usage::process(std::process::id());
        Ok(ControlCenterState {
            open: self.central_open(),
            audio,
            displays,
            display_managed: devices.displays().is_managed_by_session(),
            controller: self.controller().clone(),
            memory_mib: usage
                .and_then(|u| u.memory_bytes)
                .map(|b| b as f64 / (1024.0 * 1024.0)),
            cpu_seconds: usage.and_then(|u| u.cpu_time).map(|t| t.as_secs_f64()),
        })
    }

    pub(crate) fn handle_control_center(&mut self, call: Call) -> Result<Reply, BridgeError> {
        match call {
            Call::ControlCenterState => Ok(Reply::ControlCenterState(self.central_state()?)),
            Call::ControlCenterSet(action) => {
                if !self.game_active() {
                    return Err(BridgeError::new(ErrorCode::Conflict, "nenhum jogo aberto"));
                }
                let devices = &self.require_core()?.devices;
                match action {
                    ControlCenterAction::Volume { value } => {
                        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                            return Err(BridgeError::new(
                                ErrorCode::BadRequest,
                                "volume fora do intervalo",
                            ));
                        }
                        let state = devices.audio().capture().map_err(device_error)?;
                        let id = state.default_id.ok_or_else(|| {
                            BridgeError::new(ErrorCode::NotFound, "sem saída padrão")
                        })?;
                        devices
                            .audio()
                            .set_volume(&id, value)
                            .map_err(device_error)?;
                    }
                    ControlCenterAction::Mute { muted } => {
                        let state = devices.audio().capture().map_err(device_error)?;
                        let id = state.default_id.ok_or_else(|| {
                            BridgeError::new(ErrorCode::NotFound, "sem saída padrão")
                        })?;
                        devices.audio().set_mute(&id, muted).map_err(device_error)?;
                    }
                    ControlCenterAction::AudioOutput { id } => {
                        let state = devices.audio().capture().map_err(device_error)?;
                        if state.find(&id).is_none() {
                            return Err(BridgeError::new(
                                ErrorCode::NotFound,
                                "saída não encontrada",
                            ));
                        }
                        devices.audio().set_default(&id).map_err(device_error)?;
                    }
                    ControlCenterAction::DisplayMode { id, mode } => {
                        let mode = DisplayMode::parse(&mode).ok_or_else(|| {
                            BridgeError::new(ErrorCode::BadRequest, "modo inválido")
                        })?;
                        if devices.displays().is_managed_by_session() {
                            return Err(BridgeError::new(
                                ErrorCode::Conflict,
                                "tela gerenciada pela sessão",
                            ));
                        }
                        if !devices
                            .displays()
                            .list()
                            .map_err(device_error)?
                            .iter()
                            .any(|d| d.id == id && d.is_active)
                            || !devices
                                .displays()
                                .modes(&id)
                                .map_err(device_error)?
                                .contains(&mode)
                        {
                            return Err(BridgeError::new(
                                ErrorCode::BadRequest,
                                "modo não oferecido pela tela",
                            ));
                        }
                        devices
                            .displays()
                            .set_mode(&id, mode)
                            .map_err(device_error)?;
                    }
                }
                Ok(Reply::ControlCenterSet(self.central_state()?))
            }
            _ => Err(BridgeError::new(
                ErrorCode::UnknownCall,
                "chamada fora da Central",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Core;
    use tako_core::paths::DataPaths;

    fn service() -> (AppService, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let core = Core::open(
            DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        )
        .unwrap();
        (AppService::with_core("test", core), dir)
    }

    #[test]
    fn shortcut_and_open_intent_only_exist_during_a_game() {
        let (mut app, _dir) = service();
        assert!(app.central_shortcut().is_none());
        app.set_central_open(true);
        assert!(!app.central_open());
        app.launch_game("steam:demo0");
        assert_eq!(app.central_shortcut(), Some(KeyShortcut::default()));
        app.set_central_open(true);
        assert!(app.central_open());
        app.set_central_open(false);
        assert!(!app.central_open());
    }

    #[test]
    fn controls_validate_input_and_change_the_devices() {
        let (mut app, _dir) = service();
        let set = |action| Call::ControlCenterSet(action);
        assert_eq!(
            app.handle(set(ControlCenterAction::Mute { muted: true }))
                .unwrap_err()
                .code,
            ErrorCode::Conflict
        );
        app.launch_game("steam:demo0");
        assert_eq!(
            app.handle(set(ControlCenterAction::Volume { value: 1.1 }))
                .unwrap_err()
                .code,
            ErrorCode::BadRequest
        );
        app.handle(set(ControlCenterAction::Volume { value: 0.35 }))
            .unwrap();
        assert_eq!(
            app.central_state().unwrap().audio.unwrap().volume,
            Some(0.35)
        );
        app.handle(set(ControlCenterAction::Mute { muted: true }))
            .unwrap();
        assert!(app.central_state().unwrap().audio.unwrap().muted);
        app.handle(set(ControlCenterAction::AudioOutput { id: "tv".into() }))
            .unwrap();
        assert_eq!(
            app.central_state()
                .unwrap()
                .audio
                .unwrap()
                .default_id
                .as_deref(),
            Some("tv")
        );
        assert!(
            app.handle(set(ControlCenterAction::AudioOutput {
                id: "missing".into()
            }))
            .is_err()
        );
        assert!(
            app.handle(set(ControlCenterAction::DisplayMode {
                id: "DISPLAY1".into(),
                mode: "1x1@1".into()
            }))
            .is_err()
        );
        app.handle(set(ControlCenterAction::DisplayMode {
            id: "DISPLAY1".into(),
            mode: "1280x720@60".into(),
        }))
        .unwrap();
        assert_eq!(
            app.central_state().unwrap().displays[0]
                .current_mode
                .as_deref(),
            Some("1280x720@60")
        );
    }

    #[test]
    fn wire_contract_rejects_malformed_payloads() {
        for text in [
            r#"{"v":1,"id":1,"name":"control_center.state","payload":{}}"#,
            r#"{"v":1,"id":1,"name":"control_center.set","payload":{"action":"volume","value":"x"}}"#,
            r#"{"v":1,"id":1,"name":"control_center.set","payload":{"action":"mute","muted":true,"extra":1}}"#,
        ] {
            assert_eq!(
                parse_request(text).unwrap_err().1.code,
                ErrorCode::BadRequest
            );
        }
        let call = parse_request(r#"{"v":1,"id":1,"name":"control_center.set","payload":{"action":"volume","value":0.5}}"#).unwrap().call;
        assert_eq!(
            call,
            Call::ControlCenterSet(ControlCenterAction::Volume { value: 0.5 })
        );
        assert_eq!(
            serde_json::to_value(Event::ControlCenterOpen(true)).unwrap(),
            serde_json::json!({"name":"control_center.open","payload":true})
        );
    }
}
