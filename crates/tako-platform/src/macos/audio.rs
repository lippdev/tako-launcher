//! Saída de áudio no macOS pelo CoreAudio. Id = UID do dispositivo (estável entre execuções).

use objc2_core_audio::{
    AudioObjectGetPropertyData, AudioObjectGetPropertyDataSize, AudioObjectID,
    AudioObjectPropertyAddress, AudioObjectSetPropertyData, kAudioDevicePropertyDeviceUID,
    kAudioDevicePropertyMute, kAudioDevicePropertyStreams, kAudioDevicePropertyVolumeScalar,
    kAudioHardwarePropertyDefaultOutputDevice, kAudioHardwarePropertyDevices,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyName, kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject,
};
use objc2_core_foundation::{CFRetained, CFString};
use std::ffi::c_void;
use std::ptr::NonNull;
use tako_core::devices::audio::AudioLevel;
use tako_core::devices::{AudioDevice, AudioOutputs, AudioState, DeviceError, DeviceResult};

pub struct CoreAudio;

const SYSTEM: AudioObjectID = kAudioObjectSystemObject as AudioObjectID;

fn addr(selector: u32, scope: u32, element: u32) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: element,
    }
}

fn get<T: Copy + Default>(obj: AudioObjectID, a: AudioObjectPropertyAddress) -> Option<T> {
    let mut v = T::default();
    let mut size = size_of::<T>() as u32;
    // SAFETY: `v` tem `size` bytes e é um tipo simples (número); o CoreAudio escreve no máximo isso.
    let st = unsafe {
        AudioObjectGetPropertyData(
            obj,
            NonNull::from(&a),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut v).cast::<c_void>(),
        )
    };
    (st == 0).then_some(v)
}

fn set<T: Copy>(obj: AudioObjectID, a: AudioObjectPropertyAddress, v: T) -> bool {
    // SAFETY: `v` vive até o fim da chamada e o tamanho passado é o dele.
    let st = unsafe {
        AudioObjectSetPropertyData(
            obj,
            NonNull::from(&a),
            0,
            std::ptr::null(),
            size_of::<T>() as u32,
            NonNull::from(&v).cast::<c_void>(),
        )
    };
    st == 0
}

fn size_of_prop(obj: AudioObjectID, a: AudioObjectPropertyAddress) -> u32 {
    let mut size = 0u32;
    // SAFETY: só lê o tamanho para `size`.
    let st = unsafe {
        AudioObjectGetPropertyDataSize(
            obj,
            NonNull::from(&a),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
        )
    };
    if st == 0 { size } else { 0 }
}

fn string(obj: AudioObjectID, selector: u32) -> Option<String> {
    let p: Option<NonNull<CFString>> = get(
        obj,
        addr(
            selector,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        ),
    )?;
    // SAFETY: as propriedades de texto devolvem um CFString com retenção (+1), solto aqui.
    let s = unsafe { CFRetained::from_raw(p?) };
    Some(s.to_string())
}

fn devices() -> Vec<AudioObjectID> {
    let a = addr(
        kAudioHardwarePropertyDevices,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    let n = size_of_prop(SYSTEM, a) as usize / size_of::<AudioObjectID>();
    let mut ids = vec![0 as AudioObjectID; n];
    let mut size = (n * size_of::<AudioObjectID>()) as u32;
    if n == 0 {
        return ids;
    }
    // SAFETY: o buffer tem `size` bytes.
    let st = unsafe {
        AudioObjectGetPropertyData(
            SYSTEM,
            NonNull::from(&a),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::new(ids.as_mut_ptr().cast::<c_void>()).unwrap_or(NonNull::dangling()),
        )
    };
    if st != 0 {
        return Vec::new();
    }
    ids.truncate(size as usize / size_of::<AudioObjectID>());
    ids
}

fn is_output(d: AudioObjectID) -> bool {
    size_of_prop(
        d,
        addr(
            kAudioDevicePropertyStreams,
            kAudioObjectPropertyScopeOutput,
            kAudioObjectPropertyElementMain,
        ),
    ) > 0
}

fn outputs() -> Vec<(AudioObjectID, String, String)> {
    devices()
        .into_iter()
        .filter(|&d| is_output(d))
        .filter_map(|d| {
            let uid = string(d, kAudioDevicePropertyDeviceUID)?;
            let name = string(d, kAudioObjectPropertyName).unwrap_or_else(|| uid.clone());
            Some((d, uid, name))
        })
        .collect()
}

fn find(uid: &str) -> DeviceResult<AudioObjectID> {
    outputs()
        .into_iter()
        .find(|(_, u, _)| u == uid)
        .map(|(d, ..)| d)
        .ok_or_else(|| DeviceError::NotFound(uid.into()))
}

/// Volume do canal principal; sem ele, a média dos canais 1 e 2.
fn volume(d: AudioObjectID) -> Option<f32> {
    let at = |e| {
        get::<f32>(
            d,
            addr(
                kAudioDevicePropertyVolumeScalar,
                kAudioObjectPropertyScopeOutput,
                e,
            ),
        )
    };
    at(kAudioObjectPropertyElementMain).or_else(|| {
        let ch: Vec<f32> = [1, 2].into_iter().filter_map(at).collect();
        (!ch.is_empty()).then(|| ch.iter().sum::<f32>() / ch.len() as f32)
    })
}

impl AudioOutputs for CoreAudio {
    fn can_set_default(&self) -> bool {
        true
    }

    fn capture(&self) -> DeviceResult<AudioState> {
        let outs = outputs();
        let default = get::<AudioObjectID>(
            SYSTEM,
            addr(
                kAudioHardwarePropertyDefaultOutputDevice,
                kAudioObjectPropertyScopeGlobal,
                kAudioObjectPropertyElementMain,
            ),
        );
        Ok(AudioState {
            default_id: outs
                .iter()
                .find(|(d, ..)| Some(*d) == default)
                .map(|(_, u, _)| u.clone()),
            levels: outs
                .iter()
                .filter_map(|(d, uid, _)| {
                    Some(AudioLevel {
                        device_id: uid.clone(),
                        volume: volume(*d)?,
                        muted: get::<u32>(
                            *d,
                            addr(
                                kAudioDevicePropertyMute,
                                kAudioObjectPropertyScopeOutput,
                                kAudioObjectPropertyElementMain,
                            ),
                        )
                        .is_some_and(|m| m != 0),
                    })
                })
                .collect(),
            devices: outs
                .into_iter()
                .map(|(_, id, name)| AudioDevice { id, name })
                .collect(),
        })
    }

    fn set_default(&self, id: &str) -> DeviceResult<()> {
        let d = find(id)?;
        let a = addr(
            kAudioHardwarePropertyDefaultOutputDevice,
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        );
        if set(SYSTEM, a, d) {
            Ok(())
        } else {
            Err(DeviceError::Failed(format!("saída padrão {id}")))
        }
    }

    fn set_volume(&self, id: &str, volume: f32) -> DeviceResult<()> {
        let d = find(id)?;
        let v = volume.clamp(0.0, 1.0);
        let at = |e| {
            set(
                d,
                addr(
                    kAudioDevicePropertyVolumeScalar,
                    kAudioObjectPropertyScopeOutput,
                    e,
                ),
                v,
            )
        };
        // `|` (não `||`): os dois canais precisam receber o volume.
        if at(kAudioObjectPropertyElementMain) || (at(1) | at(2)) {
            Ok(())
        } else {
            Err(DeviceError::Unsupported(format!("volume de {id}")))
        }
    }

    fn set_mute(&self, id: &str, muted: bool) -> DeviceResult<()> {
        let d = find(id)?;
        let a = addr(
            kAudioDevicePropertyMute,
            kAudioObjectPropertyScopeOutput,
            kAudioObjectPropertyElementMain,
        );
        if set(d, a, u32::from(muted)) {
            Ok(())
        } else {
            Err(DeviceError::Unsupported(format!("mudo de {id}")))
        }
    }
}
