//! Cliente do driver HidHide (INPUT-005): esconde os controles dos outros programas enquanto a Central está
//! aberta sobre o jogo, e o Tako continua vendo (o executável dele entra na lista dos que veem).
//!
//! Fala com o driver por `\\.\HidHide` (sem administrador). Antes de mudar, grava em `<data_root>/
//! controllers-hidden.json` o que vai pôr nas listas; `reveal` tira só isso e volta o estado "ligado" de antes,
//! inclusive na próxima execução depois de uma queda (ver `session::recover_after_crash`).

use crate::windows_hidhide::{self as lists, Changes};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tako_core::devices::{ControllerHiding, DeviceError, DeviceResult};
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
    SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
};
use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING, QueryDosDeviceW,
};
use windows::Win32::System::IO::DeviceIoControl;
use windows::core::{PCWSTR, w};

/// `CTL_CODE(32769, n, METHOD_BUFFERED, FILE_READ_DATA)`.
const fn ioctl(n: u32) -> u32 {
    (32769 << 16) | (1 << 14) | (n << 2)
}
const GET_WHITELIST: u32 = ioctl(2048);
const SET_WHITELIST: u32 = ioctl(2049);
const GET_BLACKLIST: u32 = ioctl(2050);
const SET_BLACKLIST: u32 = ioctl(2051);
const GET_ACTIVE: u32 = ioctl(2052);
const SET_ACTIVE: u32 = ioctl(2053);

fn failed(what: &str, e: windows::core::Error) -> DeviceError {
    DeviceError::Failed(format!("HidHide: {what}: {e}"))
}

/// Canal com o driver; o driver aceita um por vez, então fecha logo.
struct Driver(HANDLE);

impl Driver {
    fn open() -> DeviceResult<Driver> {
        // SAFETY: abre o dispositivo de controle do driver pelo nome fixo; o handle é fechado no `Drop`.
        let h = unsafe {
            CreateFileW(
                w!(r"\\.\HidHide"),
                GENERIC_READ.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                None,
            )
        }
        .map_err(|e| DeviceError::NotFound(format!("HidHide: {e}")))?;
        Ok(Driver(h))
    }

    fn get_list(&self, code: u32) -> DeviceResult<Vec<String>> {
        let mut needed = 0u32;
        // SAFETY: só pergunta o tamanho (sem buffer de saída); `needed` vive até o fim da chamada.
        let _ = unsafe { DeviceIoControl(self.0, code, None, 0, None, 0, Some(&mut needed), None) };
        // Sempre lê de verdade (mesmo com tamanho 0): gravar a partir de uma lista mal lida apagaria a configuração
        // do usuário.
        let len = (needed as usize / 2).clamp(4096, lists::MAX_LIST);
        let mut buf = vec![0u16; len];
        let mut got = 0u32;
        // SAFETY: o buffer tem `len` u16 e o tamanho passado em bytes é o dele.
        unsafe {
            DeviceIoControl(
                self.0,
                code,
                None,
                0,
                Some(buf.as_mut_ptr() as *mut c_void),
                (len * 2) as u32,
                Some(&mut got),
                None,
            )
        }
        .map_err(|e| failed("ler lista", e))?;
        buf.truncate((got as usize / 2).min(len));
        Ok(lists::decode(&buf))
    }

    fn set_list(&self, code: u32, items: &[String]) -> DeviceResult<()> {
        let buf = lists::encode(items);
        // SAFETY: o buffer de entrada é uma lista MULTI_SZ completa e o tamanho em bytes é o dele.
        unsafe {
            DeviceIoControl(
                self.0,
                code,
                Some(buf.as_ptr() as *const c_void),
                (buf.len() * 2) as u32,
                None,
                0,
                None,
                None,
            )
        }
        .map_err(|e| failed("gravar lista", e))
    }

    fn active(&self) -> DeviceResult<bool> {
        let mut v = 0u8;
        let mut got = 0u32;
        // SAFETY: a saída é um BOOLEAN (1 byte) que vive até o fim da chamada.
        unsafe {
            DeviceIoControl(
                self.0,
                GET_ACTIVE,
                None,
                0,
                Some(&mut v as *mut u8 as *mut c_void),
                1,
                Some(&mut got),
                None,
            )
        }
        .map_err(|e| failed("ler estado", e))?;
        Ok(v != 0)
    }

    fn set_active(&self, on: bool) -> DeviceResult<()> {
        let v = u8::from(on);
        // SAFETY: a entrada é um BOOLEAN (1 byte) que vive até o fim da chamada.
        unsafe {
            DeviceIoControl(
                self.0,
                SET_ACTIVE,
                Some(&v as *const u8 as *const c_void),
                1,
                None,
                0,
                None,
                None,
            )
        }
        .map_err(|e| failed("ligar", e))
    }
}

impl Drop for Driver {
    fn drop(&mut self) {
        // SAFETY: o handle veio do `CreateFileW` e só é fechado aqui.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Instâncias presentes (HID e USB) dos controles `(vendor, product)`.
fn pad_instances(pads: &[(u16, u16)]) -> Vec<String> {
    let mut out = Vec::new();
    // SAFETY: lista todos os dispositivos presentes; a lista é destruída no fim desta função.
    let Ok(set) = (unsafe {
        SetupDiGetClassDevsW(None, PCWSTR::null(), None, DIGCF_ALLCLASSES | DIGCF_PRESENT)
    }) else {
        return out;
    };
    let mut i = 0;
    loop {
        let mut info = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: `info` tem o `cbSize` certo; o fim da lista devolve erro e encerra o laço.
        if unsafe { SetupDiEnumDeviceInfo(set, i, &mut info) }.is_err() {
            break;
        }
        i += 1;
        let mut buf = [0u16; 512];
        // SAFETY: o buffer tem 512 u16; o texto volta terminado em NUL.
        if unsafe { SetupDiGetDeviceInstanceIdW(set, &info, Some(&mut buf), None) }.is_err() {
            continue;
        }
        let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        let id = String::from_utf16_lossy(&buf[..end]);
        if pads
            .iter()
            .any(|(v, p)| lists::is_pad_instance(&id, *v, *p))
        {
            out.push(id);
        }
    }
    // SAFETY: a lista veio do `SetupDiGetClassDevsW` acima e não é usada depois.
    let _ = unsafe { SetupDiDestroyDeviceInfoList(set) };
    out
}

/// O executável do Tako como o driver espera (`\Device\HarddiskVolumeN\...`).
fn own_exe() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.to_string_lossy().into_owned();
    let exe = exe.strip_prefix(r"\\?\").unwrap_or(&exe).to_owned();
    let drive: Vec<u16> = exe.get(..2)?.encode_utf16().chain([0]).collect();
    let mut target = [0u16; 512];
    // SAFETY: `drive` termina em NUL e o buffer de saída tem 512 u16.
    let n = unsafe { QueryDosDeviceW(PCWSTR(drive.as_ptr()), Some(&mut target)) } as usize;
    let end = target[..n.min(512)]
        .iter()
        .position(|c| *c == 0)
        .unwrap_or(n.min(512));
    lists::device_path(&exe, &String::from_utf16_lossy(&target[..end]))
}

pub struct HidHide {
    marker: PathBuf,
    /// Uma operação por vez (UI e supervisor são processos diferentes; o driver também só aceita um).
    busy: Mutex<()>,
    /// Última resposta de `available` (a tela de Configurações pergunta a cada desenho).
    seen: Mutex<Option<(Instant, bool)>>,
}

impl HidHide {
    pub fn new(data_root: &std::path::Path) -> Self {
        HidHide {
            marker: data_root.join("controllers-hidden.json"),
            busy: Mutex::new(()),
            seen: Mutex::new(None),
        }
    }

    fn changes(&self) -> Changes {
        std::fs::read_to_string(&self.marker)
            .map(|t| Changes::parse(&t))
            .unwrap_or_default()
    }

    fn write_changes(&self, c: &Changes) -> DeviceResult<()> {
        let text = serde_json::to_string(c).map_err(|e| DeviceError::Failed(e.to_string()))?;
        std::fs::write(&self.marker, text)
            .map_err(|e| DeviceError::Failed(format!("marcador do HidHide: {e}")))
    }
}

impl ControllerHiding for HidHide {
    fn available(&self) -> bool {
        let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, ok)) = *seen
            && at.elapsed() < Duration::from_secs(5)
        {
            return ok;
        }
        let ok = Driver::open().is_ok();
        *seen = Some((Instant::now(), ok));
        ok
    }

    fn supported(&self) -> bool {
        true
    }

    fn hide(&self, pads: &[(u16, u16)]) -> DeviceResult<()> {
        let _g = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        let instances = pad_instances(pads);
        if instances.is_empty() {
            tracing::info!("HidHide: nenhum dispositivo dos controles {pads:04x?}");
            return Ok(());
        }
        let exe = own_exe().ok_or_else(|| DeviceError::Failed("caminho do Tako".into()))?;
        let d = Driver::open()?;
        let mut c = self.changes();
        let first = c.is_empty();
        // Grava antes de mudar: se cair no meio, a próxima execução desfaz.
        let (allow, allowed) = lists::add_to(&d.get_list(GET_WHITELIST)?, &[exe]);
        let (hide, hidden) = lists::add_to(&d.get_list(GET_BLACKLIST)?, &instances);
        if first {
            c.was_active = d.active()?;
        }
        c.allowed.extend(allowed);
        c.hidden.extend(hidden);
        self.write_changes(&c)?;
        d.set_list(SET_WHITELIST, &allow)?;
        d.set_list(SET_BLACKLIST, &hide)?;
        d.set_active(true)?;
        tracing::info!("HidHide: {} dispositivo(s) escondido(s)", instances.len());
        Ok(())
    }

    fn reveal(&self) -> DeviceResult<()> {
        let _g = self.busy.lock().unwrap_or_else(|e| e.into_inner());
        let c = self.changes();
        if c.is_empty() {
            let _ = std::fs::remove_file(&self.marker);
            return Ok(());
        }
        let d = Driver::open()?;
        let hide = lists::remove_from(&d.get_list(GET_BLACKLIST)?, &c.hidden);
        d.set_list(SET_BLACKLIST, &hide)?;
        let allow = lists::remove_from(&d.get_list(GET_WHITELIST)?, &c.allowed);
        d.set_list(SET_WHITELIST, &allow)?;
        if !c.was_active {
            d.set_active(false)?;
        }
        std::fs::remove_file(&self.marker)
            .map_err(|e| DeviceError::Failed(format!("marcador do HidHide: {e}")))?;
        tracing::info!("HidHide: controles devolvidos");
        Ok(())
    }

    fn is_hidden(&self) -> bool {
        !self.changes().is_empty()
    }
}
