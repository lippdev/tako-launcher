//! Telas no macOS pelo CoreGraphics. A principal é a que fica na origem (0, 0); desligar uma tela
//! não existe na API pública (`Unsupported`).

use crate::unix::outputs::{Output, OutputBackend, OutputMode};
use objc2_core_foundation::{CFArray, CFRetained};
use objc2_core_graphics::{
    CGBeginDisplayConfiguration, CGCancelDisplayConfiguration, CGCompleteDisplayConfiguration,
    CGConfigureDisplayOrigin, CGConfigureDisplayWithDisplayMode, CGConfigureOption,
    CGDirectDisplayID, CGDisplayBounds, CGDisplayConfigRef, CGDisplayCopyAllDisplayModes,
    CGDisplayCopyDisplayMode, CGDisplayIsActive, CGDisplayIsBuiltin, CGDisplayIsMain,
    CGDisplayMirrorsDisplay, CGDisplayMode, CGDisplayModelNumber, CGDisplayRotation,
    CGDisplaySerialNumber, CGDisplayVendorNumber, CGError, CGGetOnlineDisplayList,
};
use tako_core::devices::{DeviceError, DeviceResult, HdrStatus, VrrStatus};
use tako_core::i18n::{t, tf};

pub struct CoreGraphics;

fn cg(what: &str, e: CGError) -> DeviceResult<()> {
    if e == CGError::Success {
        Ok(())
    } else {
        Err(DeviceError::Failed(format!("{what}: CGError {}", e.0)))
    }
}

fn online() -> DeviceResult<Vec<CGDirectDisplayID>> {
    let mut ids = [0u32; 16];
    let mut n = 0u32;
    // SAFETY: o buffer tem 16 posições e o tamanho passado é 16; `n` recebe quantas foram escritas.
    let e = unsafe { CGGetOnlineDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut n) };
    cg("CGGetOnlineDisplayList", e)?;
    Ok(ids[..(n as usize).min(ids.len())].to_vec())
}

/// Id estável entre execuções (o `CGDirectDisplayID` pode mudar): fabricante, modelo e série.
fn stable_id(d: CGDirectDisplayID) -> String {
    format!(
        "{:04x}-{:04x}-{}",
        CGDisplayVendorNumber(d),
        CGDisplayModelNumber(d),
        CGDisplaySerialNumber(d)
    )
}

fn all_modes(d: CGDirectDisplayID) -> Vec<CFRetained<CGDisplayMode>> {
    // SAFETY: sem dicionário de opções.
    let Some(arr): Option<CFRetained<CFArray>> = (unsafe { CGDisplayCopyAllDisplayModes(d, None) })
    else {
        return Vec::new();
    };
    (0..arr.count())
        .filter_map(|i| {
            // SAFETY: `i` está dentro do array, cujos itens são `CGDisplayMode`; reter mantém o
            // modo vivo depois que o array for solto.
            let p = unsafe { arr.value_at_index(i) } as *mut CGDisplayMode;
            let p = std::ptr::NonNull::new(p)?;
            // SAFETY: `p` é um CGDisplayMode válido do array (acima).
            Some(unsafe { CFRetained::retain(p) })
        })
        .collect()
}

fn mode_of(m: &CGDisplayMode) -> OutputMode {
    let (w, h) = (
        CGDisplayMode::width(Some(m)),
        CGDisplayMode::height(Some(m)),
    );
    let (pw, ph) = (
        CGDisplayMode::pixel_width(Some(m)),
        CGDisplayMode::pixel_height(Some(m)),
    );
    // Telas integradas informam 0 Hz.
    let hz = match CGDisplayMode::refresh_rate(Some(m)) {
        r if r > 0.0 => r,
        _ => 60.0,
    };
    OutputMode {
        // Mesmo tamanho em pontos pode ter densidades diferentes (HiDPI): a chave separa.
        key: format!("{w}x{h}@{hz:.3}:{pw}x{ph}"),
        width: w as u32,
        height: h as u32,
        refresh: hz,
    }
}

impl OutputBackend for CoreGraphics {
    fn outputs(&self) -> DeviceResult<Vec<Output>> {
        let mut n = 0;
        Ok(online()?
            .into_iter()
            // Telas espelhando outra aparecem só pela principal do espelho.
            .filter(|&d| CGDisplayMirrorsDisplay(d) == 0)
            .map(|d| {
                let active = CGDisplayIsActive(d);
                let b = CGDisplayBounds(d);
                let name = if CGDisplayIsBuiltin(d) {
                    t("platform.unix.builtin_display")
                } else {
                    n += 1;
                    tf("platform.unix.display_n", &[("n", &n.to_string())])
                };
                Output {
                    id: stable_id(d),
                    name,
                    enabled: active,
                    primary: CGDisplayIsMain(d),
                    x: b.origin.x as i32,
                    y: b.origin.y as i32,
                    current: CGDisplayCopyDisplayMode(d).map(|m| mode_of(&m)),
                    modes: all_modes(d)
                        .iter()
                        .filter(|m| CGDisplayMode::is_usable_for_desktop_gui(Some(m)))
                        .map(|m| mode_of(m))
                        .collect(),
                    rotation: (CGDisplayRotation(d).round() as i64).rem_euclid(360) as u16,
                    scale: 1.0,
                    hdr: HdrStatus::Unsupported,
                    vrr: VrrStatus::Unsupported,
                }
            })
            .collect())
    }

    /// ProMotion e Adaptive Sync só mudam em Ajustes do Sistema: nada público no CoreGraphics nem no IOKit.
    fn vrr_limitation(&self) -> Option<String> {
        Some(t("vrr.platform.macos"))
    }

    fn commit(&self, desired: &[Output]) -> DeviceResult<()> {
        let ids = online()?;
        let current = self.outputs()?;
        if desired
            .iter()
            .any(|d| !d.enabled && current.iter().any(|c| c.id == d.id && c.enabled))
        {
            return Err(DeviceError::Unsupported(
                "o macOS não deixa desligar uma tela".into(),
            ));
        }
        let mut config: CGDisplayConfigRef = std::ptr::null_mut();
        // SAFETY: `config` recebe uma configuração nova, cancelada ou concluída abaixo.
        cg("CGBeginDisplayConfiguration", unsafe {
            CGBeginDisplayConfiguration(&mut config)
        })?;
        let result = (|| {
            for d in desired.iter().filter(|d| d.enabled) {
                let Some(&cid) = ids.iter().find(|&&i| stable_id(i) == d.id) else {
                    continue;
                };
                if let Some(want) = &d.current {
                    let modes = all_modes(cid);
                    let mode = modes
                        .iter()
                        .find(|m| mode_of(m).key == want.key)
                        .ok_or_else(|| DeviceError::NotFound(want.key.clone()))?;
                    // SAFETY: `config` está aberta e `mode` pertence a esta tela; sem opções.
                    cg("CGConfigureDisplayWithDisplayMode", unsafe {
                        CGConfigureDisplayWithDisplayMode(config, cid, Some(mode), None)
                    })?;
                }
                // SAFETY: `config` está aberta.
                cg("CGConfigureDisplayOrigin", unsafe {
                    CGConfigureDisplayOrigin(config, cid, d.x, d.y)
                })?;
            }
            Ok(())
        })();
        match result {
            // SAFETY: conclui a configuração aberta; só vale até o fim da sessão do usuário.
            Ok(()) => cg("CGCompleteDisplayConfiguration", unsafe {
                CGCompleteDisplayConfiguration(config, CGConfigureOption::ForSession)
            }),
            Err(e) => {
                // SAFETY: descarta a configuração aberta.
                unsafe { CGCancelDisplayConfiguration(config) };
                Err(e)
            }
        }
    }
}
