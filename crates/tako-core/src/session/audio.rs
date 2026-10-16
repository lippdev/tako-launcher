//! Saída de áudio durante o jogo (port de `ConsoleMode.Runtime/Audio/AudioOutputSwitch.cs` e da parte de
//! áudio de `Session/Devices/GameSessionDevices.cs`): no início, a saída escolhida vira a padrão; no fim,
//! volta a anterior só se a do jogo continuar padrão (se o usuário trocou durante o jogo, a escolha fica).

use crate::devices::{AudioDevice, AudioOutputs, AudioState, DeviceResult};
use serde::{Deserialize, Serialize};

/// Saída escolhida nas configurações: id estável e o nome visto por último.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioOutputChoice {
    pub device_id: String,
    pub name: String,
}

impl AudioOutputChoice {
    pub fn new(device_id: &str, name: &str) -> Self {
        AudioOutputChoice {
            device_id: device_id.to_owned(),
            name: name.to_owned(),
        }
    }

    /// Pelo id e, se o id sumiu, pelo nome (a mesma TV noutra porta HDMI ganha outro id). Nome repetido
    /// em mais de um dispositivo não conta: seria um palpite.
    pub fn find_in<'a>(&self, state: &'a AudioState) -> Option<&'a AudioDevice> {
        if let Some(d) = state.find(&self.device_id) {
            return Some(d);
        }
        let mut by_name = state
            .devices
            .iter()
            .filter(|d| d.name.to_lowercase() == self.name.to_lowercase());
        match (by_name.next(), by_name.next()) {
            (Some(d), None) => Some(d),
            _ => None,
        }
    }
}

/// O que a troca fez, para desfazer no fim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSwitchRecord {
    pub previous_id: Option<String>,
    pub device_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSwitchOutcome {
    Switched,
    /// A saída escolhida já era a padrão.
    AlreadyDefault,
    /// A saída escolhida não está ativa (ex.: TV desligada).
    DeviceMissing,
    /// O SO não deixa trocar a saída padrão.
    Unsupported,
}

fn same(a: Option<&str>, b: &str) -> bool {
    a.is_some_and(|a| a.eq_ignore_ascii_case(b))
}

/// Torna `choice` a saída padrão. O registro devolvido desfaz exatamente o que mudou.
pub fn apply(
    audio: &dyn AudioOutputs,
    choice: &AudioOutputChoice,
) -> DeviceResult<(AudioSwitchOutcome, Option<AudioSwitchRecord>)> {
    if !audio.can_set_default() {
        return Ok((AudioSwitchOutcome::Unsupported, None));
    }
    let state = audio.capture()?;
    let Some(device) = choice.find_in(&state) else {
        return Ok((AudioSwitchOutcome::DeviceMissing, None));
    };
    if same(state.default_id.as_deref(), &device.id) {
        return Ok((AudioSwitchOutcome::AlreadyDefault, None));
    }
    audio.set_default(&device.id)?;
    let record = AudioSwitchRecord {
        previous_id: state.default_id.clone(),
        device_id: device.id.clone(),
    };
    Ok((AudioSwitchOutcome::Switched, Some(record)))
}

/// Devolve a saída anterior se ela ainda existe e a do jogo continua padrão. Verdadeiro se voltou.
pub fn restore(audio: &dyn AudioOutputs, record: &AudioSwitchRecord) -> DeviceResult<bool> {
    let state = audio.capture()?;
    let Some(previous) = record
        .previous_id
        .as_deref()
        .filter(|p| state.find(p).is_some())
    else {
        return Ok(false);
    };
    if !same(state.default_id.as_deref(), &record.device_id) {
        return Ok(false);
    }
    audio.set_default(previous)?;
    Ok(true)
}

/// Áudio de uma sessão de jogo: troca no início e devolve no fim. Nada aqui derruba a sessão: falhas
/// vão para o log. Terminar duas vezes não desfaz nada a mais.
#[derive(Debug, Default)]
pub struct GameSessionAudio {
    record: Option<AudioSwitchRecord>,
}

impl GameSessionAudio {
    pub fn begin(
        &mut self,
        audio: &dyn AudioOutputs,
        game_id: &str,
        choice: Option<&AudioOutputChoice>,
    ) -> Option<AudioSwitchOutcome> {
        self.record = None;
        let choice = choice?;
        match apply(audio, choice) {
            Ok((outcome, record)) => {
                tracing::info!("áudio de {game_id}: {} → {outcome:?}", choice.name);
                self.record = record;
                Some(outcome)
            }
            Err(e) => {
                tracing::warn!(
                    "áudio de {game_id}: falha ao trocar a saída para {}: {e}",
                    choice.name
                );
                None
            }
        }
    }

    /// Verdadeiro se a saída de antes voltou.
    pub fn end(&mut self, audio: &dyn AudioOutputs, game_id: &str) -> bool {
        let Some(record) = self.record.take() else {
            return false;
        };
        match restore(audio, &record) {
            Ok(restored) => {
                tracing::info!("áudio de {game_id}: saída de antes devolvida: {restored}");
                restored
            }
            Err(e) => {
                tracing::warn!("áudio de {game_id}: falha ao devolver a saída de antes: {e}");
                false
            }
        }
    }

    /// Há uma troca a desfazer.
    pub fn is_active(&self) -> bool {
        self.record.is_some()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::devices::fake::FakeAudio;
    use crate::devices::{AudioState, DeviceError};

    fn default_of(a: &FakeAudio) -> Option<String> {
        a.capture().unwrap().default_id
    }

    /// Saída padrão que o SO não deixa trocar.
    struct Locked(FakeAudio);
    impl AudioOutputs for Locked {
        fn can_set_default(&self) -> bool {
            false
        }
        fn capture(&self) -> DeviceResult<AudioState> {
            self.0.capture()
        }
        fn set_default(&self, _: &str) -> DeviceResult<()> {
            Err(DeviceError::Unsupported("padrão".into()))
        }
        fn set_volume(&self, id: &str, v: f32) -> DeviceResult<()> {
            self.0.set_volume(id, v)
        }
        fn set_mute(&self, id: &str, m: bool) -> DeviceResult<()> {
            self.0.set_mute(id, m)
        }
    }

    #[test]
    fn switch_makes_the_choice_default_and_restore_brings_back_the_previous_one() {
        let audio = FakeAudio::desk();
        let before = audio.capture().unwrap();
        let (outcome, record) = apply(&audio, &AudioOutputChoice::new("tv", "TV (HDMI)")).unwrap();
        assert_eq!(outcome, AudioSwitchOutcome::Switched);
        assert_eq!(default_of(&audio).as_deref(), Some("tv"));
        assert!(restore(&audio, &record.unwrap()).unwrap());
        assert!(audio.capture().unwrap().is_equivalent_to(&before));
    }

    #[test]
    fn restore_keeps_a_choice_the_user_made_during_the_game() {
        let audio = FakeAudio::desk();
        let (_, record) = apply(&audio, &AudioOutputChoice::new("tv", "TV")).unwrap();
        audio.state.lock().unwrap().devices.push(AudioDevice {
            id: "fone".into(),
            name: "Fone".into(),
        });
        audio.set_default("fone").unwrap(); // trocou durante o jogo
        assert!(!restore(&audio, &record.unwrap()).unwrap());
        assert_eq!(default_of(&audio).as_deref(), Some("fone"));
    }

    #[test]
    fn restore_skips_a_previous_device_that_was_disconnected() {
        let audio = FakeAudio::desk();
        let (_, record) = apply(&audio, &AudioOutputChoice::new("tv", "TV")).unwrap();
        audio
            .state
            .lock()
            .unwrap()
            .devices
            .retain(|d| d.id != "speakers");
        assert!(!restore(&audio, &record.unwrap()).unwrap());
        assert_eq!(default_of(&audio).as_deref(), Some("tv"));
    }

    #[test]
    fn nothing_changes_when_the_choice_is_missing_already_default_or_unsupported() {
        let audio = FakeAudio::desk();
        let missing = apply(&audio, &AudioOutputChoice::new("fone", "Fone")).unwrap();
        assert_eq!(missing, (AudioSwitchOutcome::DeviceMissing, None));
        let already = apply(&audio, &AudioOutputChoice::new("speakers", "Alto-falantes")).unwrap();
        assert_eq!(already, (AudioSwitchOutcome::AlreadyDefault, None));
        let locked = Locked(FakeAudio::desk());
        let unsupported = apply(&locked, &AudioOutputChoice::new("tv", "TV")).unwrap();
        assert_eq!(unsupported, (AudioSwitchOutcome::Unsupported, None));
        assert_eq!(default_of(&audio).as_deref(), Some("speakers"));
    }

    #[test]
    fn a_choice_is_found_by_id_then_by_a_unique_name() {
        let mut state = FakeAudio::desk().capture().unwrap();
        let find = |c: AudioOutputChoice, s: &AudioState| c.find_in(s).map(|d| d.id.clone());
        assert_eq!(
            find(AudioOutputChoice::new("tv", "outro nome"), &state).as_deref(),
            Some("tv")
        );
        assert_eq!(
            find(AudioOutputChoice::new("id-antigo", "tv (hdmi)"), &state).as_deref(),
            Some("tv")
        );
        assert_eq!(
            find(AudioOutputChoice::new("id-antigo", "Outra TV"), &state),
            None
        );
        state.devices.push(AudioDevice {
            id: "tv-2".into(),
            name: "TV (HDMI)".into(),
        });
        assert_eq!(
            find(AudioOutputChoice::new("id-antigo", "TV (HDMI)"), &state),
            None
        );
    }

    #[test]
    fn audio_switches_at_the_start_and_comes_back_at_the_end_once() {
        let audio = FakeAudio::desk();
        let before = audio.capture().unwrap();
        let mut session = GameSessionAudio::default();
        let choice = AudioOutputChoice::new("tv", "TV (HDMI)");
        assert_eq!(
            session.begin(&audio, "steam:1", Some(&choice)),
            Some(AudioSwitchOutcome::Switched)
        );
        assert_eq!(default_of(&audio).as_deref(), Some("tv"));
        assert!(session.end(&audio, "steam:1"));
        assert!(!session.end(&audio, "steam:1")); // terminar de novo não faz nada
        assert!(audio.capture().unwrap().is_equivalent_to(&before));
    }

    #[test]
    fn nothing_happens_when_no_output_is_chosen() {
        let audio = FakeAudio::desk();
        let mut session = GameSessionAudio::default();
        assert_eq!(session.begin(&audio, "steam:1", None), None);
        assert!(!session.is_active());
        assert!(!session.end(&audio, "steam:1"));
        assert_eq!(default_of(&audio).as_deref(), Some("speakers"));
    }
}
