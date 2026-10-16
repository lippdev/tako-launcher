//! Telas pela API CCD (`QueryDisplayConfig`/`SetDisplayConfig`) e pelo GDI (`ChangeDisplaySettingsEx`).
//! Port de `ConsoleMode.Runtime.Windows/{Ccd,WindowsDisplayControl,WindowsDisplayConfiguration}.cs`.
//!
//! A captura guarda a configuração ativa inteira (caminhos e modos CCD) em `platform_data`; `apply` a devolve ao
//! `SetDisplayConfig`. Se falhar (ex.: depois de reiniciar, os LUIDs dos adaptadores mudam), restaura monitor a
//! monitor: religa/desconecta pela CCD e aplica modo, posição e principal pelo GDI.

use crate::windows_identity::{self as identity, SourceRequest};
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use tako_core::devices::{
    ConnectedDisplay, DeviceError, DeviceResult, DisplayMode, DisplayTopology, Displays, HdrStatus,
    MonitorState, ScreenBounds,
};
use windows::Win32::Devices::Display::*;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, LUID};
use windows::Win32::Graphics::Gdi::*;
use windows::core::PCWSTR;

fn fail(msg: impl Into<String>) -> DeviceError {
    DeviceError::Failed(msg.into())
}

const APPLY_FLAGS: SET_DISPLAY_CONFIG_FLAGS = SET_DISPLAY_CONFIG_FLAGS(
    SDC_APPLY.0 | SDC_USE_SUPPLIED_DISPLAY_CONFIG.0 | SDC_SAVE_TO_DATABASE.0 | SDC_ALLOW_CHANGES.0,
);

type Paths = Vec<DISPLAYCONFIG_PATH_INFO>;
type Modes = Vec<DISPLAYCONFIG_MODE_INFO>;

fn query(flags: QUERY_DISPLAY_CONFIG_FLAGS) -> DeviceResult<(Paths, Modes)> {
    // A configuração pode mudar entre a consulta do tamanho e a leitura: tenta de novo algumas vezes.
    for _ in 0..5 {
        let (mut np, mut nm) = (0u32, 0u32);
        // SAFETY: só escreve nos dois contadores locais.
        let sizes = unsafe { GetDisplayConfigBufferSizes(flags, &mut np, &mut nm) };
        if sizes != ERROR_SUCCESS {
            return Err(fail(format!(
                "GetDisplayConfigBufferSizes falhou ({})",
                sizes.0
            )));
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
        // SAFETY: os vetores têm exatamente `np`/`nm` elementos, como os contadores informam.
        let r = unsafe {
            QueryDisplayConfig(
                flags,
                &mut np,
                paths.as_mut_ptr(),
                &mut nm,
                modes.as_mut_ptr(),
                None,
            )
        };
        if r == ERROR_SUCCESS {
            paths.truncate(np as usize);
            modes.truncate(nm as usize);
            return Ok((paths, modes));
        }
        if r != ERROR_INSUFFICIENT_BUFFER {
            return Err(fail(format!("QueryDisplayConfig falhou ({})", r.0)));
        }
    }
    Err(fail(
        "QueryDisplayConfig: a configuração de monitores não parou de mudar",
    ))
}

fn set(paths: &[DISPLAYCONFIG_PATH_INFO], modes: &[DISPLAYCONFIG_MODE_INFO]) -> i32 {
    // SAFETY: fatias válidas de estruturas CCD; o Windows valida o conteúdo.
    unsafe { SetDisplayConfig(Some(paths), Some(modes), APPLY_FLAGS) }
}

fn source_mode_idx(p: &DISPLAYCONFIG_PATH_INFO) -> u32 {
    // SAFETY: os dois ramos da união são u32; sem DISPLAYCONFIG_PATH_SUPPORT_VIRTUAL_MODE vale `modeInfoIdx`.
    unsafe { p.sourceInfo.Anonymous.modeInfoIdx }
}

fn source_mode(modes: &[DISPLAYCONFIG_MODE_INFO], idx: u32) -> Option<DISPLAYCONFIG_SOURCE_MODE> {
    let m = modes.get(idx as usize)?;
    if m.infoType != DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
        return None;
    }
    // SAFETY: o tipo foi conferido; com `infoType` de fonte a união guarda `sourceMode`.
    Some(unsafe { m.Anonymous.sourceMode })
}

fn is_active(p: &DISPLAYCONFIG_PATH_INFO) -> bool {
    p.flags & DISPLAYCONFIG_PATH_ACTIVE != 0
}

fn luid_key(l: LUID) -> i64 {
    (i64::from(l.HighPart) << 32) | i64::from(l.LowPart)
}

fn header(
    kind: DISPLAYCONFIG_DEVICE_INFO_TYPE,
    size: usize,
    adapter: LUID,
    id: u32,
) -> DISPLAYCONFIG_DEVICE_INFO_HEADER {
    DISPLAYCONFIG_DEVICE_INFO_HEADER {
        r#type: kind,
        size: size as u32,
        adapterId: adapter,
        id,
    }
}

fn wide(buf: &[u16]) -> Option<String> {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    (end > 0).then(|| String::from_utf16_lossy(&buf[..end]))
}

/// Nome amigável e caminho estável do monitor.
fn target_name(adapter: LUID, target: u32) -> (Option<String>, Option<String>) {
    let mut r = DISPLAYCONFIG_TARGET_DEVICE_NAME {
        header: header(
            DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>(),
            adapter,
            target,
        ),
        ..Default::default()
    };
    // SAFETY: o cabeçalho declara o tamanho real da estrutura que o contém.
    if unsafe { DisplayConfigGetDeviceInfo(&mut r.header) } != 0 {
        return (None, None);
    }
    (
        wide(&r.monitorFriendlyDeviceName),
        wide(&r.monitorDevicePath),
    )
}

/// Nome GDI (`\\.\DISPLAY1`) da fonte.
fn source_gdi_name(adapter: LUID, source: u32) -> Option<String> {
    let mut r = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
        header: header(
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>(),
            adapter,
            source,
        ),
        ..Default::default()
    };
    // SAFETY: o cabeçalho declara o tamanho real da estrutura que o contém.
    (unsafe { DisplayConfigGetDeviceInfo(&mut r.header) } == 0)
        .then(|| wide(&r.viewGdiDeviceName))
        .flatten()
}

fn get_hdr(adapter: LUID, target: u32) -> HdrStatus {
    let mut r = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
        header: header(
            DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
            size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>(),
            adapter,
            target,
        ),
        ..Default::default()
    };
    // SAFETY: o cabeçalho declara o tamanho real da estrutura que o contém.
    if unsafe { DisplayConfigGetDeviceInfo(&mut r.header) } != 0 {
        return HdrStatus::Unknown;
    }
    // SAFETY: a união é um u32 de bits (bit 0 = suportado, bit 1 = ligado).
    let v = unsafe { r.Anonymous.value };
    match (v & 1 != 0, v & 2 != 0) {
        (false, _) => HdrStatus::Unsupported,
        (true, true) => HdrStatus::On,
        (true, false) => HdrStatus::Off,
    }
}

fn path_id(p: &DISPLAYCONFIG_PATH_INFO) -> String {
    let t = &p.targetInfo;
    let (_, device_path) = target_name(t.adapterId, t.id);
    identity::ccd_monitor_id(
        t.adapterId.HighPart,
        t.adapterId.LowPart,
        t.id,
        device_path.as_deref(),
    )
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn devmode() -> DEVMODEW {
    DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    }
}

fn current_devmode(gdi: &[u16]) -> DeviceResult<DEVMODEW> {
    let mut dm = devmode();
    // SAFETY: `gdi` termina em 0 e `dm` tem `dmSize` preenchido.
    if !unsafe {
        EnumDisplaySettingsExW(
            PCWSTR(gdi.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut dm,
            ENUM_DISPLAY_SETTINGS_FLAGS(0),
        )
    }
    .as_bool()
    {
        return Err(fail(format!(
            "EnumDisplaySettingsEx falhou para {}",
            String::from_utf16_lossy(gdi)
        )));
    }
    Ok(dm)
}

fn change(gdi: &[u16], dm: &DEVMODEW, flags: CDS_TYPE) -> DISP_CHANGE {
    // SAFETY: `gdi` termina em 0 e `dm` é um DEVMODEW completo.
    unsafe { ChangeDisplaySettingsExW(PCWSTR(gdi.as_ptr()), Some(dm), None, flags, None) }
}

/// Aplica o que ficou na fila com `CDS_NORESET`.
fn commit() -> DISP_CHANGE {
    // SAFETY: chamada sem dispositivo nem modo: só aplica as mudanças pendentes.
    unsafe { ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_TYPE(0), None) }
}

fn ok(code: DISP_CHANGE) -> bool {
    code == DISP_CHANGE_SUCCESSFUL || code == DISP_CHANGE_RESTART
}

#[derive(Clone)]
struct Monitor {
    id: String,
    name: String,
    gdi: Option<String>,
    adapter: LUID,
    source: u32,
    target: u32,
    active: bool,
    bounds: Option<ScreenBounds>,
    mode: Option<DisplayMode>,
}

/// Monitores conectados (CCD, Windows).
#[derive(Default)]
pub struct WindowsDisplays {
    /// Alvo (adaptador, id) → fonte que tinha enquanto ativo, para religar com o mesmo `\\.\DISPLAYn`.
    last_source: Mutex<HashMap<(i64, u32), u32>>,
}

impl WindowsDisplays {
    /// Monitores conectados e a fonte de cada um (a atual, ou a que usaria se fosse ligado).
    fn connected(&self) -> DeviceResult<Vec<Monitor>> {
        let (paths, modes) = query(QDC_ALL_PATHS)?;
        let mut targets: Vec<((i64, u32), LUID)> = Vec::new();
        let mut candidates: HashMap<(i64, u32), Vec<u32>> = HashMap::new();
        let mut active: HashMap<(i64, u32), usize> = HashMap::new();
        for (i, p) in paths.iter().enumerate() {
            if !p.targetInfo.targetAvailable.as_bool() {
                continue;
            }
            let key = (luid_key(p.targetInfo.adapterId), p.targetInfo.id);
            let list = candidates.entry(key).or_insert_with(|| {
                targets.push((key, p.targetInfo.adapterId));
                Vec::new()
            });
            if !list.contains(&p.sourceInfo.id) {
                list.push(p.sourceInfo.id);
            }
            if is_active(p) {
                active.entry(key).or_insert(i);
            }
        }

        let assigned = {
            let mut last = self
                .last_source
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            for (key, &i) in &active {
                last.insert(*key, paths[i].sourceInfo.id);
            }
            // As fontes são por adaptador: a chave inclui o adaptador para nunca colidir entre placas.
            let requests: Vec<_> = targets
                .iter()
                .map(|&(key, _)| SourceRequest {
                    target: key,
                    active: active.get(&key).map(|&i| (key.0, paths[i].sourceInfo.id)),
                    preferred: last.get(&key).map(|&s| (key.0, s)),
                    candidates: candidates[&key].iter().map(|&s| (key.0, s)).collect(),
                })
                .collect();
            identity::assign_sources(&requests)
        };

        let mut result: Vec<Monitor> = Vec::new();
        for &(key, adapter) in &targets {
            let Some(&(_, source)) = assigned.get(&key) else {
                continue;
            };
            let (friendly, device_path) = target_name(adapter, key.1);
            let id = identity::ccd_monitor_id(
                adapter.HighPart,
                adapter.LowPart,
                key.1,
                device_path.as_deref(),
            );
            if result.iter().any(|m| m.id.eq_ignore_ascii_case(&id)) {
                continue; // tela clonada: um monitor só
            }
            let gdi = source_gdi_name(adapter, source);
            let (mut bounds, mut mode) = (None, None);
            if let Some(&i) = active.get(&key)
                && let Some(sm) = source_mode(&modes, source_mode_idx(&paths[i]))
            {
                bounds = Some(ScreenBounds {
                    x: sm.position.x,
                    y: sm.position.y,
                    width: sm.width as i32,
                    height: sm.height as i32,
                });
                let rate = paths[i].targetInfo.refreshRate;
                let hz = if rate.Denominator == 0 {
                    0
                } else {
                    (f64::from(rate.Numerator) / f64::from(rate.Denominator)).round() as u32
                };
                mode = Some(DisplayMode::new(sm.width, sm.height, hz));
            }
            result.push(Monitor {
                name: friendly
                    .or_else(|| gdi.clone())
                    .unwrap_or_else(|| id.clone()),
                id,
                gdi,
                adapter,
                source,
                target: key.1,
                active: active.contains_key(&key),
                bounds,
                mode,
            });
        }
        Ok(result)
    }

    fn find_active(&self, id: &str) -> DeviceResult<Monitor> {
        self.connected()?
            .into_iter()
            .find(|m| m.active && m.id.eq_ignore_ascii_case(id))
            .ok_or_else(|| DeviceError::NotFound(format!("Tela desligada: {id}")))
    }

    fn enable(&self, ids: &[String]) -> DeviceResult<()> {
        let wanted: Vec<Monitor> = self
            .connected()?
            .into_iter()
            .filter(|m| !m.active && ids.iter().any(|i| i.eq_ignore_ascii_case(&m.id)))
            .collect();
        if wanted.is_empty() {
            return Ok(());
        }
        let (paths, modes) = query(QDC_ALL_PATHS)?;
        let mut selected: Paths = paths.iter().filter(|p| is_active(p)).copied().collect();
        for m in &wanted {
            let Some(p) = paths.iter().find(|p| {
                p.targetInfo.targetAvailable.as_bool()
                    && p.targetInfo.id == m.target
                    && p.sourceInfo.id == m.source
                    && luid_key(p.targetInfo.adapterId) == luid_key(m.adapter)
            }) else {
                continue;
            };
            let mut p = *p;
            p.flags |= DISPLAYCONFIG_PATH_ACTIVE;
            p.sourceInfo.Anonymous.modeInfoIdx = DISPLAYCONFIG_PATH_MODE_IDX_INVALID;
            p.targetInfo.Anonymous.modeInfoIdx = DISPLAYCONFIG_PATH_MODE_IDX_INVALID;
            selected.push(p);
        }
        let code = set(&selected, &modes);
        if code != 0 {
            // Último recurso do Console Mode: estender a área de trabalho para todos os conectados.
            // SAFETY: sem caminhos nem modos; o Windows escolhe a topologia estendida.
            let extend = unsafe { SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_EXTEND) };
            if extend != 0 {
                let names: Vec<_> = wanted.iter().map(|w| w.name.as_str()).collect();
                return Err(fail(format!(
                    "SetDisplayConfig não ligou {} ({code}; estender: {extend})",
                    names.join(", ")
                )));
            }
        }
        Ok(())
    }

    fn disable(&self, ids: &[String]) -> DeviceResult<()> {
        let (mut paths, modes) = query(QDC_ONLY_ACTIVE_PATHS)?;
        let mut remaining = 0;
        let mut changed = false;
        for p in &mut paths {
            let id = path_id(p);
            if ids.iter().any(|i| i.eq_ignore_ascii_case(&id)) {
                p.flags = 0;
                changed = true;
            } else {
                remaining += 1;
            }
        }
        if !changed {
            return Ok(());
        }
        if remaining == 0 {
            return Err(fail("Desconectar todas as telas deixaria o PC sem imagem"));
        }
        match set(&paths, &modes) {
            0 => Ok(()),
            code => Err(fail(format!(
                "SetDisplayConfig não desconectou as telas ({code})"
            ))),
        }
    }

    fn set_primary(&self, id: &str) -> DeviceResult<()> {
        let (paths, mut modes) = query(QDC_ONLY_ACTIVE_PATHS)?;
        let origin = paths
            .iter()
            .find(|p| path_id(p).eq_ignore_ascii_case(id))
            .and_then(|p| source_mode(&modes, source_mode_idx(p)))
            .ok_or_else(|| fail(format!("Tela desligada não pode ser a principal: {id}")))?
            .position;
        if origin.x == 0 && origin.y == 0 {
            return Ok(());
        }
        // A principal é a que está em (0,0): desloca todas as fontes para a escolhida cair na origem.
        for m in &mut modes {
            if m.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE {
                // SAFETY: o tipo foi conferido; a união guarda `sourceMode`.
                let pos = unsafe { &mut m.Anonymous.sourceMode.position };
                pos.x -= origin.x;
                pos.y -= origin.y;
            }
        }
        match set(&paths, &modes) {
            0 => Ok(()),
            code => Err(fail(format!(
                "SetDisplayConfig não trocou a tela principal ({code})"
            ))),
        }
    }

    /// Religa os monitores que estavam ativos e desconecta os que não estavam (como o `RestoreBackup` do
    /// Console Mode). Devolve os religados, para entrarem no passo de modo e posição.
    fn match_active(&self, target: &DisplayTopology) -> DeviceResult<Vec<String>> {
        let connected = self.connected()?;
        let want_on: Vec<String> = target
            .monitors
            .iter()
            .filter(|m| {
                connected
                    .iter()
                    .any(|c| !c.active && c.id.eq_ignore_ascii_case(&m.id))
            })
            .map(|m| m.id.clone())
            .collect();
        if !want_on.is_empty() {
            self.enable(&want_on)?;
        }
        let want_off: Vec<String> = connected
            .iter()
            .filter(|c| c.active && target.find(&c.id).is_none())
            .map(|c| c.id.clone())
            .collect();
        if !want_off.is_empty() && !target.monitors.is_empty() {
            self.disable(&want_off)?;
        }
        Ok(want_on)
    }

    /// Caminho GDI: resolução, taxa, rotação, posição e principal dos monitores alterados que estão ativos.
    fn apply_per_monitor(&self, target: &DisplayTopology, changed: &[String]) -> DeviceResult<()> {
        let gdi_names: HashMap<String, String> = self
            .connected()?
            .into_iter()
            .filter(|m| m.active)
            .filter_map(|m| Some((m.id.to_lowercase(), m.gdi?)))
            .collect();
        let mut inactive = Vec::new();
        let mut pending = 0;
        for m in target
            .monitors
            .iter()
            .filter(|m| changed.iter().any(|c| c.eq_ignore_ascii_case(&m.id)))
        {
            let Some(gdi) = gdi_names.get(&m.id.to_lowercase()) else {
                inactive.push(m.name.clone());
                continue;
            };
            let gdi = to_wide(gdi);
            let mut dm = current_devmode(&gdi)?;
            dm.dmPelsWidth = m.width;
            dm.dmPelsHeight = m.height;
            // O GDI usa Hz inteiros, arredondados para baixo (59,94 Hz aparece como 59).
            dm.dmDisplayFrequency = (m.refresh_hz + 0.01).floor() as u32;
            dm.Anonymous1.Anonymous2 = DEVMODEW_0_1 {
                dmPosition: windows::Win32::Foundation::POINTL { x: m.x, y: m.y },
                dmDisplayOrientation: DEVMODE_DISPLAY_ORIENTATION(u32::from(m.rotation / 90 % 4)),
                // SAFETY: lê o campo do ramo de tela que o próprio Windows preencheu.
                dmDisplayFixedOutput: unsafe { dm.Anonymous1.Anonymous2.dmDisplayFixedOutput },
            };
            dm.dmFields = DM_PELSWIDTH
                | DM_PELSHEIGHT
                | DM_DISPLAYFREQUENCY
                | DM_POSITION
                | DM_DISPLAYORIENTATION;
            let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET;
            if m.is_primary {
                flags |= CDS_SET_PRIMARY;
            }
            let code = change(&gdi, &dm, flags);
            if !ok(code) {
                return Err(fail(format!(
                    "ChangeDisplaySettingsEx recusou {} ({})",
                    m.name, code.0
                )));
            }
            pending += 1;
        }
        if pending > 0 {
            let code = commit();
            if !ok(code) {
                return Err(fail(format!(
                    "ChangeDisplaySettingsEx não aplicou as mudanças ({})",
                    code.0
                )));
            }
        }
        if !inactive.is_empty() {
            return Err(fail(format!(
                "Monitor(es) inativo(s) não podem ser religados sem a configuração original: {}",
                inactive.join(", ")
            )));
        }
        Ok(())
    }
}

/// Bytes crus de estruturas CCD (POD `repr(C)`, sem ponteiros).
fn as_bytes<T: Copy>(items: &[T]) -> &[u8] {
    // SAFETY: T é uma estrutura C só de inteiros; ler seus bytes (incluindo enchimento zerado pelo
    // `Default`/Windows) é válido pelo tamanho exato da fatia.
    unsafe { std::slice::from_raw_parts(items.as_ptr().cast::<u8>(), size_of_val(items)) }
}

fn from_bytes<T: Copy + Default>(bytes: &[u8]) -> Vec<T> {
    bytes
        .chunks_exact(size_of::<T>())
        // SAFETY: cada pedaço tem `size_of::<T>()` bytes e T aceita qualquer padrão de bits (só inteiros).
        .map(|c| unsafe { std::ptr::read_unaligned(c.as_ptr().cast::<T>()) })
        .collect()
}

fn decode(data: Option<&str>) -> Option<(Paths, Modes)> {
    let ps = size_of::<DISPLAYCONFIG_PATH_INFO>();
    let (np, _, bytes) = identity::decode_ccd(data?, ps, size_of::<DISPLAYCONFIG_MODE_INFO>())?;
    let (p, m) = bytes.split_at(np * ps);
    Some((from_bytes(p), from_bytes(m)))
}

impl Displays for WindowsDisplays {
    fn list(&self) -> DeviceResult<Vec<ConnectedDisplay>> {
        Ok(self
            .connected()?
            .into_iter()
            .map(|m| ConnectedDisplay {
                is_primary: m.active && m.bounds.is_some_and(|b| b.x == 0 && b.y == 0),
                hdr: if m.active {
                    get_hdr(m.adapter, m.target)
                } else {
                    HdrStatus::Unknown
                },
                bounds: m.bounds,
                current_mode: m.mode,
                is_active: m.active,
                id: m.id,
                name: m.name,
            })
            .collect())
    }

    fn modes(&self, display_id: &str) -> DeviceResult<Vec<DisplayMode>> {
        let Some(gdi) = self.find_active(display_id)?.gdi else {
            return Ok(Vec::new());
        };
        let gdi = to_wide(&gdi);
        let mut modes = Vec::new();
        let mut dm = devmode();
        let mut i = 0;
        // SAFETY: `gdi` termina em 0 e `dm` tem `dmSize` preenchido.
        while unsafe {
            EnumDisplaySettingsExW(
                PCWSTR(gdi.as_ptr()),
                ENUM_DISPLAY_SETTINGS_MODE(i),
                &mut dm,
                ENUM_DISPLAY_SETTINGS_FLAGS(0),
            )
        }
        .as_bool()
        {
            // Taxa 0 ou 1 é "padrão do hardware": não serve para escolher.
            if dm.dmDisplayFrequency > 1 && (dm.dmBitsPerPel == 0 || dm.dmBitsPerPel >= 24) {
                modes.push(DisplayMode::new(
                    dm.dmPelsWidth,
                    dm.dmPelsHeight,
                    dm.dmDisplayFrequency,
                ));
            }
            i += 1;
        }
        Ok(DisplayMode::normalize(modes))
    }

    fn set_mode(&self, display_id: &str, mode: DisplayMode) -> DeviceResult<()> {
        let m = self.find_active(display_id)?;
        let (Some(gdi), Some(bounds)) = (m.gdi, m.bounds) else {
            return Err(DeviceError::NotFound(format!(
                "Tela desligada: {display_id}"
            )));
        };
        let gdi = to_wide(&gdi);
        let mut dm = current_devmode(&gdi)?;
        dm.dmPelsWidth = mode.width;
        dm.dmPelsHeight = mode.height;
        dm.dmDisplayFrequency = mode.refresh_hz;
        dm.Anonymous1.Anonymous2.dmPosition = windows::Win32::Foundation::POINTL {
            x: bounds.x,
            y: bounds.y,
        };
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_POSITION;
        let queued = change(&gdi, &dm, CDS_UPDATEREGISTRY | CDS_NORESET);
        let applied = if ok(queued) { commit() } else { queued };
        if !ok(applied) {
            return Err(fail(format!(
                "ChangeDisplaySettingsEx recusou {mode} em {} ({})",
                m.name, applied.0
            )));
        }
        if bounds.x == 0 && bounds.y == 0 {
            self.set_primary(display_id)?;
        }
        Ok(())
    }

    fn set_active(&self, active_ids: &[String], primary: &str) -> DeviceResult<()> {
        if active_ids.is_empty() {
            return Err(fail("Desconectar todas as telas deixaria o PC sem imagem"));
        }
        self.enable(active_ids)?;
        let off: Vec<String> = self
            .connected()?
            .into_iter()
            .filter(|m| m.active && !active_ids.iter().any(|i| i.eq_ignore_ascii_case(&m.id)))
            .map(|m| m.id)
            .collect();
        if !off.is_empty() {
            self.disable(&off)?;
        }
        self.set_primary(primary)
    }

    fn set_hdr(&self, display_id: &str, enabled: bool) -> DeviceResult<()> {
        let m = self.find_active(display_id)?;
        let mut s = DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE {
            header: header(
                DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
                size_of::<DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE>(),
                m.adapter,
                m.target,
            ),
            ..Default::default()
        };
        s.Anonymous.value = u32::from(enabled);
        // SAFETY: o cabeçalho declara o tamanho real da estrutura que o contém.
        match unsafe { DisplayConfigSetDeviceInfo(&s.header) } {
            0 => Ok(()),
            code => Err(fail(format!(
                "HDR {} falhou em {} ({code})",
                if enabled { "ligar" } else { "desligar" },
                m.name
            ))),
        }
    }

    fn capture(&self) -> DeviceResult<DisplayTopology> {
        let (paths, modes) = query(QDC_ONLY_ACTIVE_PATHS)?;
        let mut monitors: Vec<MonitorState> = Vec::new();
        for p in paths.iter().filter(|p| is_active(p)) {
            let Some(sm) = source_mode(&modes, source_mode_idx(p)) else {
                continue;
            };
            let t = &p.targetInfo;
            let (friendly, device_path) = target_name(t.adapterId, t.id);
            let id = identity::ccd_monitor_id(
                t.adapterId.HighPart,
                t.adapterId.LowPart,
                t.id,
                device_path.as_deref(),
            );
            if monitors.iter().any(|m| m.id.eq_ignore_ascii_case(&id)) {
                continue;
            }
            let rate = t.refreshRate;
            monitors.push(MonitorState {
                name: friendly
                    .or_else(|| source_gdi_name(p.sourceInfo.adapterId, p.sourceInfo.id))
                    .unwrap_or_else(|| id.clone()),
                id,
                is_primary: sm.position.x == 0 && sm.position.y == 0,
                x: sm.position.x,
                y: sm.position.y,
                width: sm.width,
                height: sm.height,
                refresh_hz: if rate.Denominator == 0 {
                    0.0
                } else {
                    f64::from(rate.Numerator) / f64::from(rate.Denominator)
                },
                rotation: match t.rotation {
                    DISPLAYCONFIG_ROTATION_ROTATE90 => 90,
                    DISPLAYCONFIG_ROTATION_ROTATE180 => 180,
                    DISPLAYCONFIG_ROTATION_ROTATE270 => 270,
                    _ => 0,
                },
            });
        }
        Ok(DisplayTopology {
            monitors,
            platform_data: Some(identity::encode_ccd(
                paths.len(),
                modes.len(),
                &[as_bytes(&paths), as_bytes(&modes)].concat(),
            )),
        })
    }

    fn apply(&self, topology: &DisplayTopology) -> DeviceResult<()> {
        let mut ccd_error = 0;
        if let Some((paths, modes)) = decode(topology.platform_data.as_deref()) {
            ccd_error = set(&paths, &modes);
            if ccd_error == 0 {
                return Ok(());
            }
        }
        let changed = identity::changed_ids(&self.capture()?, topology);
        let fallback = self.match_active(topology).and_then(|relinked| {
            let extra: Vec<String> = relinked
                .into_iter()
                .filter(|r| !changed.iter().any(|c| c.eq_ignore_ascii_case(r)))
                .collect();
            self.apply_per_monitor(topology, &[changed, extra].concat())
        });
        match fallback {
            Err(e) if ccd_error != 0 => Err(fail(format!(
                "SetDisplayConfig falhou ({ccd_error}); restauração monitor a monitor: {e}"
            ))),
            other => other,
        }
    }
}
