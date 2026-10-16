//! Saída de áudio pelo Core Audio: port de `ConsoleMode.Runtime.Windows/{AudioNative,WindowsAudioEndpoints}.cs`.
//! Trocar a saída padrão usa `IPolicyConfig`, NÃO documentada: se não existir, só essa troca fica desligada.

use std::ffi::c_void;
use std::sync::OnceLock;
use tako_core::devices::{
    AudioDevice, AudioOutputs, AudioState, DeviceError, DeviceResult, audio::AudioLevel,
};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc};
use windows::Win32::System::Com::*;
use windows::core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, PCWSTR, PWSTR};

#[allow(non_snake_case)]
mod policy_config {
    use super::*;

    /// `IPolicyConfig` (Windows 7+): só `SetDefaultEndpoint` é chamado; os anteriores ocupam a tabela virtual.
    #[windows::core::interface("F8679F50-850A-41CF-9C72-430F290290C8")]
    pub unsafe trait IPolicyConfig: IUnknown {
        fn GetMixFormat(&self, id: PCWSTR, format: *mut *mut c_void) -> HRESULT;
        fn GetDeviceFormat(&self, id: PCWSTR, default: i32, format: *mut *mut c_void) -> HRESULT;
        fn ResetDeviceFormat(&self, id: PCWSTR) -> HRESULT;
        fn SetDeviceFormat(&self, id: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
        fn GetProcessingPeriod(
            &self,
            id: PCWSTR,
            default: i32,
            period: *mut i64,
            min: *mut i64,
        ) -> HRESULT;
        fn SetProcessingPeriod(&self, id: PCWSTR, period: *mut i64) -> HRESULT;
        fn GetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
        fn SetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
        fn GetPropertyValue(&self, id: PCWSTR, key: *const c_void, value: *mut c_void) -> HRESULT;
        fn SetPropertyValue(&self, id: PCWSTR, key: *const c_void, value: *mut c_void) -> HRESULT;
        pub fn SetDefaultEndpoint(&self, id: PCWSTR, role: ERole) -> HRESULT;
    }
}
use policy_config::IPolicyConfig;

const POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

/// PKEY_Device_FriendlyName.
const FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

thread_local! {
    // As chamadas chegam de threads quaisquer: cada uma entra no COM uma vez (falha = já iniciado em outro modo).
    static COM: () = {
        // SAFETY: inicialização do COM da thread atual, sem parâmetros reservados.
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    };
}

fn com() {
    COM.with(|_| {});
}

fn err(call: &str, e: windows::core::Error) -> DeviceError {
    DeviceError::Failed(format!("{call} falhou ({e})"))
}

fn enumerator() -> DeviceResult<IMMDeviceEnumerator> {
    com();
    // SAFETY: classe e interface do Core Audio, criadas no processo.
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_INPROC_SERVER) }
        .map_err(|e| err("MMDeviceEnumerator", e))
}

fn policy() -> Option<IPolicyConfig> {
    com();
    // SAFETY: interface não documentada; falha só desliga a troca da saída padrão.
    unsafe { CoCreateInstance(&POLICY_CONFIG_CLIENT, None, CLSCTX_ALL) }.ok()
}

fn id_of(device: &IMMDevice) -> Option<String> {
    // SAFETY: o texto é alocado pelo Windows (CoTaskMemAlloc): lido e liberado aqui.
    unsafe {
        let p = device.GetId().ok()?;
        let id = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const c_void));
        id
    }
}

fn friendly_name(device: &IMMDevice) -> Option<String> {
    // SAFETY: o PROPVARIANT e o texto convertido são liberados depois de lidos.
    unsafe {
        let store = device.OpenPropertyStore(STGM_READ).ok()?;
        let mut value = store.GetValue(&FRIENDLY_NAME).ok()?;
        let text: Option<PWSTR> = PropVariantToStringAlloc(&value).ok();
        let _ = PropVariantClear(&mut value); // falha ao liberar só vaza o texto
        let p = text?;
        let name = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const c_void));
        name.filter(|n| !n.is_empty())
    }
}

fn volume(device: &IMMDevice) -> Option<IAudioEndpointVolume> {
    // SAFETY: ativação documentada do controle de volume do endpoint.
    unsafe { device.Activate(CLSCTX_ALL, None) }.ok()
}

fn volume_of(device_id: &str) -> DeviceResult<IAudioEndpointVolume> {
    let id: Vec<u16> = device_id.encode_utf16().chain(Some(0)).collect();
    // SAFETY: `id` termina em 0 e vive durante a chamada.
    let device = unsafe { enumerator()?.GetDevice(PCWSTR(id.as_ptr())) }
        .map_err(|_| DeviceError::NotFound(device_id.to_string()))?;
    volume(&device)
        .ok_or_else(|| DeviceError::Failed(format!("Sem controle de volume em {device_id}")))
}

/// Saídas de áudio do Windows.
#[derive(Default)]
pub struct WindowsAudio {
    can_set_default: OnceLock<bool>,
}

impl AudioOutputs for WindowsAudio {
    fn can_set_default(&self) -> bool {
        *self.can_set_default.get_or_init(|| policy().is_some())
    }

    fn capture(&self) -> DeviceResult<AudioState> {
        let e = enumerator()?;
        let mut state = AudioState::default();
        // SAFETY: chamadas COM documentadas; os objetos devolvidos são liberados pelo Drop.
        unsafe {
            let collection = e
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(|x| err("EnumAudioEndpoints", x))?;
            let count = collection.GetCount().map_err(|x| err("GetCount", x))?;
            for i in 0..count {
                let Ok(device) = collection.Item(i) else {
                    continue;
                };
                let Some(id) = id_of(&device) else { continue };
                if let Some(v) = volume(&device)
                    && let (Ok(level), Ok(muted)) = (v.GetMasterVolumeLevelScalar(), v.GetMute())
                {
                    state.levels.push(AudioLevel {
                        device_id: id.clone(),
                        volume: level,
                        muted: muted.as_bool(),
                    });
                }
                state.devices.push(AudioDevice {
                    name: friendly_name(&device).unwrap_or_else(|| id.clone()),
                    id,
                });
            }
            // Sem saída padrão (nenhum dispositivo) vira None.
            state.default_id = e
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .ok()
                .and_then(|d| id_of(&d));
        }
        Ok(state)
    }

    fn set_default(&self, device_id: &str) -> DeviceResult<()> {
        let policy = policy().ok_or_else(|| {
            DeviceError::Unsupported(
                "Trocar a saída padrão não está disponível neste Windows (IPolicyConfig)".into(),
            )
        })?;
        let id: Vec<u16> = device_id.encode_utf16().chain(Some(0)).collect();
        for role in [eConsole, eMultimedia, eCommunications] {
            // SAFETY: `id` termina em 0 e vive durante a chamada.
            unsafe { policy.SetDefaultEndpoint(PCWSTR(id.as_ptr()), role) }
                .ok()
                .map_err(|e| err("IPolicyConfig.SetDefaultEndpoint", e))?;
        }
        Ok(())
    }

    fn set_volume(&self, device_id: &str, volume: f32) -> DeviceResult<()> {
        // SAFETY: chamada COM documentada, sem contexto de evento.
        unsafe {
            volume_of(device_id)?
                .SetMasterVolumeLevelScalar(volume.clamp(0.0, 1.0), std::ptr::null())
        }
        .map_err(|e| err("SetMasterVolumeLevelScalar", e))
    }

    fn set_mute(&self, device_id: &str, muted: bool) -> DeviceResult<()> {
        // SAFETY: chamada COM documentada, sem contexto de evento.
        unsafe { volume_of(device_id)?.SetMute(muted, std::ptr::null()) }
            .map_err(|e| err("SetMute", e))
    }
}
