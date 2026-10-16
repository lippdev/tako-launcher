//! Restauração de telas e áudio (port de `Session/Restore/*` e de `SessionEnvironmentSnapshot.cs`):
//! registra o estado antes do jogo em `session-environment.json` e, no fim (ou na abertura depois de
//! uma queda), restaura só o que mudou, com conferência. Restaurar duas vezes não faz nada a mais.

use crate::devices::audio::AudioLevel;
use crate::devices::{
    AudioOutputs, AudioState, DeviceError, DisplayTopology, Displays, MonitorState,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Estado de telas e áudio registrado antes de uma sessão. `display` ou `audio` ficam vazios quando a
/// leitura daquela parte falhou: a outra parte continua valendo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEnvironmentSnapshot {
    pub schema_version: u32,
    pub captured_at: DateTime<Utc>,
    pub context: Option<String>,
    pub display: Option<DisplayTopology>,
    pub audio: Option<AudioState>,
}

impl SessionEnvironmentSnapshot {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;
    const MAX_FILE_BYTES: u64 = 1024 * 1024;

    pub fn new(
        captured_at: DateTime<Utc>,
        context: Option<&str>,
        display: Option<DisplayTopology>,
        audio: Option<AudioState>,
    ) -> Self {
        SessionEnvironmentSnapshot {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            captured_at,
            context: context.map(Into::into),
            display,
            audio,
        }
    }

    /// Partes ausentes em qualquer lado não são comparadas.
    pub fn is_equivalent_to(&self, other: &Self) -> bool {
        let display = match (&self.display, &other.display) {
            (Some(a), Some(b)) => a.is_equivalent_to(b),
            _ => true,
        };
        let audio = match (&self.audio, &other.audio) {
            (Some(a), Some(b)) => a.is_equivalent_to(b),
            _ => true,
        };
        display && audio
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Texto inválido, incompleto ou de versão desconhecida devolve `None`.
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str::<Self>(json)
            .ok()
            .filter(|s| s.schema_version == Self::CURRENT_SCHEMA_VERSION)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorChangeKind {
    /// Estava ligada antes e não está mais.
    Missing,
    /// Não estava ligada antes.
    Added,
    /// Resolução ou taxa.
    Mode,
    Position,
    Rotation,
    Primary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorChange {
    pub monitor_id: String,
    pub name: String,
    pub kind: MonitorChangeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioDefaultChange {
    pub expected: String,
    pub actual: Option<String>,
}

/// Volume ou mudo. `actual` vazio: o dispositivo não está mais disponível.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioLevelChange {
    pub expected: AudioLevel,
    pub actual: Option<AudioLevel>,
}

impl AudioLevelChange {
    pub fn device_id(&self) -> &str {
        &self.expected.device_id
    }
    pub fn volume_changed(&self) -> bool {
        self.actual
            .as_ref()
            .is_none_or(|a| (a.volume - self.expected.volume).abs() > AudioState::VOLUME_TOLERANCE)
    }
    pub fn mute_changed(&self) -> bool {
        self.actual
            .as_ref()
            .is_none_or(|a| a.muted != self.expected.muted)
    }
}

/// Diferenças entre o registrado (`expected`) e o atual. Parte que falta em qualquer lado (leitura
/// falhou) não gera diferença: sem saber o estado, nada é alterado.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnvironmentDiff {
    pub monitors: Vec<MonitorChange>,
    pub audio_default: Option<AudioDefaultChange>,
    pub audio_levels: Vec<AudioLevelChange>,
}

impl EnvironmentDiff {
    pub fn is_empty(&self) -> bool {
        self.monitors.is_empty() && !self.has_audio_changes()
    }
    pub fn has_display_changes(&self) -> bool {
        !self.monitors.is_empty()
    }
    pub fn has_audio_changes(&self) -> bool {
        self.audio_default.is_some() || !self.audio_levels.is_empty()
    }

    pub fn between(
        expected: &SessionEnvironmentSnapshot,
        actual: &SessionEnvironmentSnapshot,
    ) -> Self {
        let monitors = match (&expected.display, &actual.display) {
            (Some(e), Some(a)) => compare_display(e, a),
            _ => vec![],
        };
        let (Some(e), Some(a)) = (&expected.audio, &actual.audio) else {
            return EnvironmentDiff {
                monitors,
                ..Default::default()
            };
        };
        let audio_default = e
            .default_id
            .as_ref()
            .filter(|want| {
                !a.default_id
                    .as_ref()
                    .is_some_and(|now| now.eq_ignore_ascii_case(want))
            })
            .map(|want| AudioDefaultChange {
                expected: want.clone(),
                actual: a.default_id.clone(),
            });
        let audio_levels = e
            .levels
            .iter()
            .map(|want| AudioLevelChange {
                expected: want.clone(),
                actual: a.level_of(&want.device_id).cloned(),
            })
            .filter(|c| c.volume_changed() || c.mute_changed())
            .collect();
        EnvironmentDiff {
            monitors,
            audio_default,
            audio_levels,
        }
    }

    /// Só as diferenças que `plan` tentou corrigir (para conferir a restauração).
    pub fn restrict_to(self, plan: &RestorePlan) -> Self {
        let display = plan
            .steps
            .iter()
            .any(|s| matches!(s, RestoreStep::Display { .. }));
        let default = plan
            .steps
            .iter()
            .any(|s| matches!(s, RestoreStep::SetDefaultAudio { .. }));
        let devices: Vec<&str> = plan
            .steps
            .iter()
            .filter_map(|s| match s {
                RestoreStep::SetVolume { device_id, .. }
                | RestoreStep::SetMute { device_id, .. } => Some(device_id.as_str()),
                _ => None,
            })
            .collect();
        EnvironmentDiff {
            monitors: if display { self.monitors } else { vec![] },
            audio_default: self.audio_default.filter(|_| default),
            audio_levels: self
                .audio_levels
                .into_iter()
                .filter(|c| {
                    devices
                        .iter()
                        .any(|d| d.eq_ignore_ascii_case(c.device_id()))
                })
                .collect(),
        }
    }
}

fn compare_display(expected: &DisplayTopology, actual: &DisplayTopology) -> Vec<MonitorChange> {
    let change = |m: &MonitorState, kind| MonitorChange {
        monitor_id: m.id.clone(),
        name: m.name.clone(),
        kind,
    };
    let mut out = vec![];
    for want in &expected.monitors {
        let Some(now) = actual.find(&want.id) else {
            out.push(change(want, MonitorChangeKind::Missing));
            continue;
        };
        if !want.has_same_mode(now) {
            out.push(change(want, MonitorChangeKind::Mode));
        }
        if (want.x, want.y) != (now.x, now.y) {
            out.push(change(want, MonitorChangeKind::Position));
        }
        if want.rotation != now.rotation {
            out.push(change(want, MonitorChangeKind::Rotation));
        }
        if want.is_primary != now.is_primary {
            out.push(change(want, MonitorChangeKind::Primary));
        }
    }
    out.extend(
        actual
            .monitors
            .iter()
            .filter(|m| expected.find(&m.id).is_none())
            .map(|m| change(m, MonitorChangeKind::Added)),
    );
    out
}

/// Uma alteração a fazer no SO para voltar ao estado registrado.
#[derive(Debug, Clone, PartialEq)]
pub enum RestoreStep {
    /// Volta a configuração de telas inteira (topologia, principal, resolução, taxa, rotação, posição).
    Display {
        target: DisplayTopology,
        changed_ids: Vec<String>,
    },
    SetDefaultAudio {
        device_id: String,
        name: String,
    },
    SetVolume {
        device_id: String,
        name: String,
        volume: f32,
    },
    SetMute {
        device_id: String,
        name: String,
        muted: bool,
    },
}

impl RestoreStep {
    /// Texto para o log.
    pub fn description(&self) -> String {
        match self {
            RestoreStep::Display { changed_ids, .. } => {
                format!("telas ({} alterada(s))", changed_ids.len())
            }
            RestoreStep::SetDefaultAudio { name, .. } => format!("saída padrão: {name}"),
            RestoreStep::SetVolume { name, volume, .. } => {
                format!("volume de {name}: {:.0}%", volume * 100.0)
            }
            RestoreStep::SetMute { name, muted, .. } => {
                format!("{name}: {}", if *muted { "mudo" } else { "com som" })
            }
        }
    }
}

/// Diferença que não será corrigida, com o motivo (vai para o log).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRestore {
    pub what: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RestorePlan {
    pub steps: Vec<RestoreStep>,
    pub skipped: Vec<SkippedRestore>,
}

impl RestorePlan {
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Só o que mudou, só o que ainda existe e só o que a plataforma consegue fazer. Telas primeiro
    /// (trocar de tela pode trocar a saída HDMI), depois a saída padrão, depois volume e mudo.
    pub fn new(
        expected: &SessionEnvironmentSnapshot,
        actual: &SessionEnvironmentSnapshot,
        can_set_default: bool,
    ) -> Self {
        let diff = EnvironmentDiff::between(expected, actual);
        let mut steps = vec![];
        let mut skipped = vec![];
        let skip = |what: &str, reason: &str| SkippedRestore {
            what: what.into(),
            reason: reason.into(),
        };

        if let (true, Some(display)) = (diff.has_display_changes(), &expected.display) {
            if display.primary().is_none() {
                skipped.push(skip("telas", "o estado registrado não tem tela principal"));
            } else {
                let mut ids: Vec<String> = vec![];
                for m in &diff.monitors {
                    if !ids.iter().any(|i| i.eq_ignore_ascii_case(&m.monitor_id)) {
                        ids.push(m.monitor_id.clone());
                    }
                }
                steps.push(RestoreStep::Display {
                    target: display.clone(),
                    changed_ids: ids,
                });
            }
        }

        let now = actual.audio.clone().unwrap_or_default();
        let name_of = |id: &str| {
            expected
                .audio
                .as_ref()
                .and_then(|a| a.find(id))
                .map_or(id.to_owned(), |d| d.name.clone())
        };
        if let Some(change) = &diff.audio_default {
            let name = name_of(&change.expected);
            if now.find(&change.expected).is_none() {
                skipped.push(skip("saída padrão", &format!("{name} não está conectado")));
            } else if !can_set_default {
                let why = "trocar a saída padrão não está disponível neste sistema";
                skipped.push(skip("saída padrão", why));
            } else {
                steps.push(RestoreStep::SetDefaultAudio {
                    device_id: change.expected.clone(),
                    name,
                });
            }
        }
        for change in &diff.audio_levels {
            let id = change.device_id().to_owned();
            let name = name_of(&id);
            if change.actual.is_none() || now.find(&id).is_none() {
                skipped.push(skip(
                    &format!("volume de {name}"),
                    "dispositivo não está conectado",
                ));
                continue;
            }
            let volume = change.volume_changed().then(|| RestoreStep::SetVolume {
                device_id: id.clone(),
                name: name.clone(),
                volume: change.expected.volume,
            });
            let mute = change.mute_changed().then(|| RestoreStep::SetMute {
                device_id: id.clone(),
                name: name.clone(),
                muted: change.expected.muted,
            });
            // Sem pico de som: para voltar ao mudo, silencia antes do volume; para tirar do mudo, ajusta antes.
            let ordered = if change.expected.muted {
                [mute, volume]
            } else {
                [volume, mute]
            };
            steps.extend(ordered.into_iter().flatten());
        }
        RestorePlan { steps, skipped }
    }
}

/// Resultado de uma restauração, com a conferência feita logo depois.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RestoreReport {
    pub plan: RestorePlan,
    pub applied: Vec<RestoreStep>,
    pub failed: Vec<SkippedRestore>,
    pub remaining: EnvironmentDiff,
}

impl RestoreReport {
    /// Nada diferente do registrado (ou nada que dê para corrigir).
    pub fn nothing_to_do(&self) -> bool {
        self.plan.is_empty()
    }
    /// Tudo o que foi planejado foi aplicado e a conferência confirmou.
    pub fn succeeded(&self) -> bool {
        self.failed.is_empty() && self.remaining.is_empty()
    }
}

/// Snapshot pendente em disco: gravado antes do jogo, apagado depois da restauração. Se o Tako cair no
/// meio, o arquivo fica para a próxima abertura. Falhas de disco nunca impedem o jogo de abrir.
#[derive(Debug, Clone)]
pub struct PendingEnvironmentStore {
    path: PathBuf,
}

impl PendingEnvironmentStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        PendingEnvironmentStore { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write(&self, snapshot: &SessionEnvironmentSnapshot) -> bool {
        crate::supervision::journal::write_atomic(&self.path, snapshot).is_ok()
    }

    /// Ausente, ilegível, grande demais ou de versão desconhecida: `None`.
    pub fn read(&self) -> Option<SessionEnvironmentSnapshot> {
        let mut text = String::new();
        std::fs::File::open(&self.path)
            .and_then(|f| {
                f.take(SessionEnvironmentSnapshot::MAX_FILE_BYTES)
                    .read_to_string(&mut text)
            })
            .ok()?;
        SessionEnvironmentSnapshot::from_json(&text)
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn delete(&self) {
        // Se não apagar, fica para a próxima abertura, que compara antes de oferecer qualquer coisa.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// As funções de plataforma que a restauração usa.
#[derive(Clone, Copy)]
pub struct Env<'a> {
    pub displays: &'a dyn Displays,
    pub audio: &'a dyn AudioOutputs,
}

/// Registra telas e áudio antes do jogo e restaura depois só o que mudou. Falha numa parte não
/// impede a outra e nunca derruba a sessão.
#[derive(Debug)]
pub struct SessionEnvironmentGuard {
    store: PendingEnvironmentStore,
    active: Option<SessionEnvironmentSnapshot>,
}

impl SessionEnvironmentGuard {
    pub fn new(store: PendingEnvironmentStore) -> Self {
        SessionEnvironmentGuard {
            store,
            active: None,
        }
    }

    /// Snapshot da sessão em andamento.
    pub fn active(&self) -> Option<&SessionEnvironmentSnapshot> {
        self.active.as_ref()
    }

    /// Antes de abrir o jogo: lê telas e áudio e grava em disco. Com `enabled` falso, não registra.
    /// `None` se nenhuma parte pôde ser lida.
    pub fn begin_session(
        &mut self,
        env: Env<'_>,
        context: &str,
        enabled: bool,
        now: DateTime<Utc>,
    ) -> Option<&SessionEnvironmentSnapshot> {
        self.active = None;
        if !enabled {
            self.store.delete();
            return None;
        }
        let snapshot = capture(env, Some(context), now);
        if snapshot.display.is_none() && snapshot.audio.is_none() {
            self.store.delete();
            return None;
        }
        if !self.store.write(&snapshot) {
            tracing::warn!(
                "estado de telas e áudio não gravado em {}; a restauração só vale nesta abertura",
                self.store.path().display()
            );
        }
        self.active = Some(snapshot);
        self.active.as_ref()
    }

    /// Fim do jogo (estado `Restoring`): restaura só o que mudou. Chamar de novo não faz nada.
    pub fn end_session(&mut self, env: Env<'_>, now: DateTime<Utc>) -> RestoreReport {
        match self.active.take() {
            Some(s) => self.restore_from(env, &s, now),
            None => RestoreReport::default(),
        }
    }

    /// A sessão terminou sem jogo para restaurar: esquece o registro.
    pub fn discard(&mut self) {
        self.active = None;
        self.store.delete();
    }

    /// Na abertura: snapshot deixado por uma sessão que caiu e que ainda difere do estado atual.
    pub fn pending_from_previous_run(
        &self,
        env: Env<'_>,
        now: DateTime<Utc>,
    ) -> Option<SessionEnvironmentSnapshot> {
        if !self.store.exists() {
            return None;
        }
        let Some(pending) = self.store.read() else {
            tracing::warn!("estado pendente de telas e áudio ilegível; descartado");
            self.store.delete();
            return None;
        };
        let current = capture(env, pending.context.as_deref(), now);
        if RestorePlan::new(&pending, &current, env.audio.can_set_default()).is_empty() {
            self.store.delete();
            return None;
        }
        Some(pending)
    }

    /// Diferença, plano, execução passo a passo e conferência.
    pub fn restore_from(
        &self,
        env: Env<'_>,
        expected: &SessionEnvironmentSnapshot,
        now: DateTime<Utc>,
    ) -> RestoreReport {
        let ctx = expected.context.as_deref().unwrap_or("-");
        let current = capture(env, expected.context.as_deref(), now);
        let plan = RestorePlan::new(expected, &current, env.audio.can_set_default());
        for s in &plan.skipped {
            tracing::info!("restauração após {ctx}: {} mantido ({})", s.what, s.reason);
        }
        if plan.is_empty() {
            self.store.delete();
            tracing::info!("restauração após {ctx}: telas e áudio já estavam como antes");
            return RestoreReport {
                plan,
                ..Default::default()
            };
        }
        let mut applied = vec![];
        let mut failed = vec![];
        for step in &plan.steps {
            match execute(env, step) {
                Ok(()) => {
                    tracing::info!("restauração após {ctx}: {}", step.description());
                    applied.push(step.clone());
                }
                Err(e) => {
                    tracing::warn!(
                        "restauração após {ctx}: falha em {}: {e}",
                        step.description()
                    );
                    failed.push(SkippedRestore {
                        what: step.description(),
                        reason: e.to_string(),
                    });
                }
            }
        }
        let after = capture(env, expected.context.as_deref(), now);
        let remaining = EnvironmentDiff::between(expected, &after).restrict_to(&plan);
        let report = RestoreReport {
            plan,
            applied,
            failed,
            remaining,
        };
        if report.succeeded() {
            tracing::info!(
                "restauração após {ctx} conferida: {} alteração(ões)",
                report.applied.len()
            );
        } else {
            tracing::warn!("restauração após {ctx} incompleta: {:?}", report.remaining);
        }
        self.store.delete();
        report
    }
}

fn capture(env: Env<'_>, context: Option<&str>, now: DateTime<Utc>) -> SessionEnvironmentSnapshot {
    let display = env
        .displays
        .capture()
        .inspect_err(|e| tracing::warn!("não foi possível ler as telas: {e}"))
        .ok();
    let audio = env
        .audio
        .capture()
        .inspect_err(|e| tracing::warn!("não foi possível ler o áudio: {e}"))
        .ok();
    SessionEnvironmentSnapshot::new(now, context, display, audio)
}

fn execute(env: Env<'_>, step: &RestoreStep) -> Result<(), DeviceError> {
    match step {
        RestoreStep::Display { target, .. } => env.displays.apply(target),
        RestoreStep::SetDefaultAudio { device_id, .. } => env.audio.set_default(device_id),
        RestoreStep::SetVolume {
            device_id, volume, ..
        } => env.audio.set_volume(device_id, *volume),
        RestoreStep::SetMute {
            device_id, muted, ..
        } => env.audio.set_mute(device_id, *muted),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::devices::fake::{FakeAudio, FakeDisplays};
    use crate::devices::{AudioDevice, DeviceResult, DisplayMode};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-15T20:00:00Z")
            .unwrap()
            .to_utc()
    }

    fn monitor(id: &str, primary: bool, x: i32, w: u32, h: u32, hz: f64) -> MonitorState {
        MonitorState {
            id: id.into(),
            name: id.into(),
            is_primary: primary,
            x,
            y: 0,
            width: w,
            height: h,
            refresh_hz: hz,
            rotation: 0,
        }
    }
    fn tv() -> MonitorState {
        monitor("TV", true, 0, 3840, 2160, 60.0)
    }
    fn side() -> MonitorState {
        monitor("SIDE", false, 3840, 1920, 1080, 144.0)
    }
    fn topo(m: Vec<MonitorState>) -> Option<DisplayTopology> {
        Some(DisplayTopology {
            monitors: m,
            platform_data: None,
        })
    }
    fn level(id: &str, volume: f32, muted: bool) -> AudioLevel {
        AudioLevel {
            device_id: id.into(),
            volume,
            muted,
        }
    }
    fn sound() -> AudioState {
        FakeAudio::desk().capture().unwrap()
    }
    fn snap(
        display: Option<DisplayTopology>,
        audio: Option<AudioState>,
    ) -> SessionEnvironmentSnapshot {
        SessionEnvironmentSnapshot::new(now(), Some("teste"), display, audio)
    }

    // Diferença

    #[test]
    fn same_state_has_no_differences() {
        let before = snap(topo(vec![tv(), side()]), Some(sound()));
        let after = snap(topo(vec![side(), tv()]), Some(sound()));
        assert!(EnvironmentDiff::between(&before, &after).is_empty());
        assert!(before.is_equivalent_to(&after));
    }

    #[test]
    fn equivalent_refresh_fractions_are_not_a_change() {
        let before = snap(
            topo(vec![MonitorState {
                refresh_hz: 59.94,
                ..tv()
            }]),
            None,
        );
        let after = snap(
            topo(vec![MonitorState {
                refresh_hz: 59.97,
                ..tv()
            }]),
            None,
        );
        assert!(EnvironmentDiff::between(&before, &after).is_empty());
    }

    #[test]
    fn resolution_refresh_position_rotation_and_primary_changes_are_reported_per_monitor() {
        use MonitorChangeKind::*;
        let before = snap(topo(vec![tv(), side()]), None);
        let after = snap(
            topo(vec![
                MonitorState {
                    width: 1920,
                    height: 1080,
                    is_primary: false,
                    x: 1920,
                    ..tv()
                },
                MonitorState {
                    is_primary: true,
                    x: 0,
                    rotation: 90,
                    ..side()
                },
            ]),
            None,
        );
        let diff = EnvironmentDiff::between(&before, &after);
        let kinds = |id: &str| {
            diff.monitors
                .iter()
                .filter(|m| m.monitor_id == id)
                .map(|m| m.kind)
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds("TV"), [Mode, Position, Primary]);
        assert_eq!(kinds("SIDE"), [Position, Rotation, Primary]);
    }

    #[test]
    fn monitor_turned_off_or_on_changes_the_topology() {
        let both = snap(topo(vec![tv(), side()]), None);
        let tv_only = snap(topo(vec![tv()]), None);
        let off = EnvironmentDiff::between(&both, &tv_only).monitors;
        assert_eq!(
            off.iter().map(|m| m.kind).collect::<Vec<_>>(),
            [MonitorChangeKind::Missing]
        );
        let on = EnvironmentDiff::between(&tv_only, &both).monitors;
        assert_eq!(
            on.iter().map(|m| m.kind).collect::<Vec<_>>(),
            [MonitorChangeKind::Added]
        );
    }

    #[test]
    fn missing_part_in_either_snapshot_is_never_a_difference() {
        let full = snap(topo(vec![tv()]), Some(sound()));
        let no_display = snap(
            None,
            Some(AudioState {
                levels: vec![],
                default_id: None,
                ..sound()
            }),
        );
        let no_audio = snap(
            topo(vec![MonitorState {
                width: 1280,
                ..tv()
            }]),
            None,
        );
        assert!(!EnvironmentDiff::between(&full, &no_display).has_display_changes());
        assert!(!EnvironmentDiff::between(&full, &no_audio).has_audio_changes());
    }

    #[test]
    fn audio_default_device_volume_and_mute_changes_are_reported() {
        let before = snap(None, Some(sound()));
        let after = snap(
            None,
            Some(AudioState {
                default_id: Some("tv".into()),
                levels: vec![level("speakers", 0.2, true), level("tv", 0.8, false)],
                ..sound()
            }),
        );
        let diff = EnvironmentDiff::between(&before, &after);
        assert_eq!(
            diff.audio_default,
            Some(AudioDefaultChange {
                expected: "speakers".into(),
                actual: Some("tv".into())
            })
        );
        assert_eq!(diff.audio_levels.len(), 1);
        let c = &diff.audio_levels[0];
        assert_eq!(c.device_id(), "speakers");
        assert!(c.volume_changed() && c.mute_changed());
    }

    #[test]
    fn volume_within_tolerance_is_not_a_change() {
        let before = snap(None, Some(sound()));
        let after = snap(
            None,
            Some(AudioState {
                levels: vec![level("speakers", 0.503, false), level("tv", 0.8, false)],
                ..sound()
            }),
        );
        assert!(EnvironmentDiff::between(&before, &after).is_empty());
    }

    // Plano

    #[test]
    fn plan_restores_display_in_one_step_with_the_changed_monitors() {
        let before = snap(topo(vec![tv(), side()]), None);
        let after = snap(
            topo(vec![
                MonitorState {
                    width: 1920,
                    height: 1080,
                    ..tv()
                },
                side(),
            ]),
            None,
        );
        let plan = RestorePlan::new(&before, &after, true);
        assert_eq!(
            plan.steps,
            [RestoreStep::Display {
                target: before.display.clone().unwrap(),
                changed_ids: vec!["TV".into()]
            }]
        );
    }

    #[test]
    fn plan_restores_only_what_changed_and_orders_display_before_audio() {
        let before = snap(topo(vec![tv()]), Some(sound()));
        let after = snap(
            topo(vec![MonitorState {
                refresh_hz: 120.0,
                ..tv()
            }]),
            Some(AudioState {
                default_id: Some("tv".into()),
                ..sound()
            }),
        );
        let plan = RestorePlan::new(&before, &after, true);
        assert_eq!(plan.steps.len(), 2);
        assert!(matches!(plan.steps[0], RestoreStep::Display { .. }));
        assert_eq!(
            plan.steps[1],
            RestoreStep::SetDefaultAudio {
                device_id: "speakers".into(),
                name: "Alto-falantes".into()
            }
        );
        assert!(plan.skipped.is_empty());
    }

    #[test]
    fn unplugged_device_is_skipped_with_a_reason() {
        let before = snap(None, Some(sound()));
        let after = snap(
            None,
            Some(AudioState {
                devices: vec![AudioDevice {
                    id: "tv".into(),
                    name: "TV (HDMI)".into(),
                }],
                default_id: Some("tv".into()),
                levels: vec![level("tv", 0.8, false)],
            }),
        );
        let plan = RestorePlan::new(&before, &after, true);
        assert!(plan.is_empty());
        assert_eq!(plan.skipped.len(), 2); // saída padrão + volume dos alto-falantes
        assert!(
            plan.skipped
                .iter()
                .all(|s| s.reason.contains("não está conectado"))
        );
    }

    #[test]
    fn without_default_device_support_only_volume_and_mute_are_restored() {
        let before = snap(None, Some(sound()));
        let after = snap(
            None,
            Some(AudioState {
                default_id: Some("tv".into()),
                levels: vec![level("speakers", 0.5, true), level("tv", 0.8, false)],
                ..sound()
            }),
        );
        let plan = RestorePlan::new(&before, &after, false);
        assert_eq!(
            plan.steps,
            [RestoreStep::SetMute {
                device_id: "speakers".into(),
                name: "Alto-falantes".into(),
                muted: false
            }]
        );
        assert_eq!(plan.skipped.len(), 1);
        assert!(plan.skipped[0].reason.contains("não está disponível"));
    }

    #[test]
    fn unmuting_sets_the_volume_first_and_muting_comes_before_the_volume() {
        let loud_muted = snap(
            None,
            Some(AudioState {
                levels: vec![level("speakers", 1.0, true)],
                ..sound()
            }),
        );
        let quiet = snap(
            None,
            Some(AudioState {
                levels: vec![level("speakers", 0.1, false)],
                ..sound()
            }),
        );
        let unmute = RestorePlan::new(&quiet, &loud_muted, true).steps;
        let mute = RestorePlan::new(&loud_muted, &quiet, true).steps;
        assert!(matches!(
            unmute[..],
            [
                RestoreStep::SetVolume { .. },
                RestoreStep::SetMute { muted: false, .. }
            ]
        ));
        assert!(matches!(
            mute[..],
            [
                RestoreStep::SetMute { muted: true, .. },
                RestoreStep::SetVolume { .. }
            ]
        ));
    }

    #[test]
    fn recorded_state_without_primary_monitor_is_not_applied() {
        let bogus = snap(topo(vec![side()]), None);
        let plan = RestorePlan::new(&bogus, &snap(topo(vec![tv()]), None), true);
        assert!(plan.is_empty());
        assert_eq!(plan.skipped.len(), 1);
    }

    // JSON

    #[test]
    fn snapshot_round_trips_through_json() {
        let mut s = snap(topo(vec![tv(), side()]), Some(sound()));
        if let Some(d) = s.display.as_mut() {
            d.platform_data = Some("AAEC".into());
        }
        let back = SessionEnvironmentSnapshot::from_json(&s.to_json()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn invalid_or_unknown_json_is_rejected() {
        for json in [
            "",
            "{",
            r#"{"schemaVersion": 99, "capturedAt": "2026-10-15T20:00:00Z"}"#,
            r#"{"schemaVersion": 1, "capturedAt": "2026-10-15T20:00:00Z", "display": {"monitors": [{"name": "sem id"}]}}"#,
            r#"{"schemaVersion": 1, "capturedAt": "2026-10-15T20:00:00Z", "audio": {"devices": []}}"#,
        ] {
            assert_eq!(SessionEnvironmentSnapshot::from_json(json), None, "{json}");
        }
    }

    // Sessão, contra os fakes com falhas injetadas.

    #[derive(Default)]
    struct Flaky<T> {
        inner: T,
        calls: Mutex<Vec<String>>,
        fail_capture: AtomicBool,
        fail_apply: bool,
    }

    impl<T> Flaky<T> {
        fn new(inner: T) -> Self {
            Flaky {
                inner,
                calls: Mutex::new(vec![]),
                fail_capture: AtomicBool::new(false),
                fail_apply: false,
            }
        }
        fn log(&self, s: String) {
            self.calls.lock().unwrap().push(s);
        }
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
        fn check_capture(&self) -> DeviceResult<()> {
            if self.fail_capture.swap(false, Ordering::SeqCst) {
                return Err(DeviceError::Failed("leitura falhou (simulado)".into()));
            }
            Ok(())
        }
    }

    impl Displays for Flaky<FakeDisplays> {
        fn list(&self) -> DeviceResult<Vec<crate::devices::ConnectedDisplay>> {
            self.inner.list()
        }
        fn modes(&self, id: &str) -> DeviceResult<Vec<DisplayMode>> {
            self.inner.modes(id)
        }
        fn set_mode(&self, id: &str, m: DisplayMode) -> DeviceResult<()> {
            self.inner.set_mode(id, m)
        }
        fn set_active(&self, ids: &[String], p: &str) -> DeviceResult<()> {
            self.inner.set_active(ids, p)
        }
        fn set_hdr(&self, id: &str, on: bool) -> DeviceResult<()> {
            self.inner.set_hdr(id, on)
        }
        fn capture(&self) -> DeviceResult<DisplayTopology> {
            self.check_capture()?;
            self.inner.capture()
        }
        fn apply(&self, t: &DisplayTopology) -> DeviceResult<()> {
            self.log("apply".into());
            if self.fail_apply {
                return Err(DeviceError::Failed("aplicar falhou (simulado)".into()));
            }
            self.inner.apply(t)
        }
    }

    impl AudioOutputs for Flaky<FakeAudio> {
        fn can_set_default(&self) -> bool {
            true
        }
        fn capture(&self) -> DeviceResult<AudioState> {
            self.check_capture()?;
            self.inner.capture()
        }
        fn set_default(&self, id: &str) -> DeviceResult<()> {
            self.log(format!("default {id}"));
            self.inner.set_default(id)
        }
        fn set_volume(&self, id: &str, v: f32) -> DeviceResult<()> {
            self.log(format!("volume {id} {v}"));
            self.inner.set_volume(id, v)
        }
        fn set_mute(&self, id: &str, m: bool) -> DeviceResult<()> {
            self.log(format!("mute {id} {m}"));
            self.inner.set_mute(id, m)
        }
    }

    struct Rig {
        displays: Flaky<FakeDisplays>,
        audio: Flaky<FakeAudio>,
        dir: tempfile::TempDir,
    }

    impl Rig {
        fn new() -> Self {
            Rig {
                displays: Flaky::new(FakeDisplays::desk()),
                audio: Flaky::new(FakeAudio::desk()),
                dir: tempfile::tempdir().unwrap(),
            }
        }
        fn env(&self) -> Env<'_> {
            Env {
                displays: &self.displays,
                audio: &self.audio,
            }
        }
        fn store(&self) -> PendingEnvironmentStore {
            PendingEnvironmentStore::new(self.dir.path().join("session-environment.json"))
        }
        fn guard(&self) -> SessionEnvironmentGuard {
            SessionEnvironmentGuard::new(self.store())
        }
        /// O jogo troca a resolução do monitor.
        fn game_changes_resolution(&self) {
            self.displays
                .inner
                .set_mode("DISPLAY1", DisplayMode::new(1280, 720, 60))
                .unwrap();
        }
    }

    #[test]
    fn session_captures_before_the_game_and_restores_what_the_game_changed() {
        let rig = Rig::new();
        let before = snap(
            Some(rig.displays.capture().unwrap()),
            Some(rig.audio.capture().unwrap()),
        );
        let mut g = rig.guard();
        g.begin_session(rig.env(), "jogo-1", true, now());
        assert!(rig.store().exists());
        rig.game_changes_resolution();
        rig.audio.inner.set_default("tv").unwrap();

        let report = g.end_session(rig.env(), now());
        assert!(report.succeeded());
        assert_eq!(report.applied.len(), 2);
        assert_eq!(rig.displays.calls(), ["apply"]);
        assert_eq!(rig.audio.calls(), ["default speakers"]);
        let after = snap(
            Some(rig.displays.capture().unwrap()),
            Some(rig.audio.capture().unwrap()),
        );
        assert!(after.is_equivalent_to(&before));
        assert!(!rig.store().exists());
        assert!(g.active().is_none());

        // Restaurar de novo não faz nada a mais.
        assert!(g.end_session(rig.env(), now()).nothing_to_do());
        assert!(g.restore_from(rig.env(), &before, now()).nothing_to_do());
        assert_eq!(rig.displays.calls(), ["apply"]);
        assert_eq!(rig.audio.calls(), ["default speakers"]);
    }

    #[test]
    fn nothing_changed_means_no_call_to_the_platform() {
        let rig = Rig::new();
        let mut g = rig.guard();
        g.begin_session(rig.env(), "jogo-1", true, now());
        let report = g.end_session(rig.env(), now());
        assert!(report.nothing_to_do());
        assert!(rig.displays.calls().is_empty() && rig.audio.calls().is_empty());
        assert!(!rig.store().exists());
    }

    #[test]
    fn disabled_option_records_nothing_and_restores_nothing() {
        let rig = Rig::new();
        let mut g = rig.guard();
        assert!(g.begin_session(rig.env(), "jogo-1", false, now()).is_none());
        rig.game_changes_resolution();
        assert!(g.end_session(rig.env(), now()).nothing_to_do());
        assert!(rig.displays.calls().is_empty());
        assert!(!rig.store().exists());
    }

    #[test]
    fn display_failure_does_not_stop_audio_restore_and_is_reported() {
        let mut rig = Rig::new();
        let mut g = rig.guard();
        let expected = g
            .begin_session(rig.env(), "jogo-1", true, now())
            .cloned()
            .unwrap();
        rig.game_changes_resolution();
        rig.audio.inner.set_mute("speakers", true).unwrap();
        rig.displays.fail_apply = true;

        let report = g.restore_from(rig.env(), &expected, now());
        assert!(!report.succeeded());
        assert_eq!(report.failed.len(), 1);
        assert!(
            report
                .applied
                .iter()
                .any(|s| matches!(s, RestoreStep::SetMute { muted: false, .. }))
        );
        assert!(report.remaining.has_display_changes());
        assert!(!report.remaining.has_audio_changes());
    }

    #[test]
    fn capture_failure_in_one_part_keeps_the_other() {
        let rig = Rig::new();
        rig.displays.fail_capture.store(true, Ordering::SeqCst);
        let mut g = rig.guard();
        let s = g.begin_session(rig.env(), "jogo-1", true, now()).unwrap();
        assert!(s.display.is_none() && s.audio.is_some());
    }

    #[test]
    fn capture_failure_in_both_parts_records_nothing() {
        let rig = Rig::new();
        rig.displays.fail_capture.store(true, Ordering::SeqCst);
        rig.audio.fail_capture.store(true, Ordering::SeqCst);
        let mut g = rig.guard();
        assert!(g.begin_session(rig.env(), "jogo-1", true, now()).is_none());
        assert!(!rig.store().exists());
    }

    #[test]
    fn discard_forgets_the_session() {
        let rig = Rig::new();
        let mut g = rig.guard();
        g.begin_session(rig.env(), "jogo-1", true, now());
        rig.game_changes_resolution();
        g.discard();
        assert!(g.end_session(rig.env(), now()).nothing_to_do());
        assert!(!rig.store().exists());
    }

    #[test]
    fn snapshot_left_by_a_crash_is_offered_on_the_next_start_and_restored() {
        let rig = Rig::new();
        let before = rig.displays.capture().unwrap();
        rig.guard().begin_session(rig.env(), "jogo-1", true, now());
        rig.game_changes_resolution();
        // O Tako cai: próxima abertura, com o mesmo arquivo.
        let next = rig.guard();
        let pending = next.pending_from_previous_run(rig.env(), now()).unwrap();
        assert_eq!(pending.context.as_deref(), Some("jogo-1"));
        assert!(next.restore_from(rig.env(), &pending, now()).succeeded());
        assert!(rig.displays.capture().unwrap().is_equivalent_to(&before));
        assert!(!rig.store().exists());
    }

    #[test]
    fn snapshot_left_by_a_crash_without_differences_is_dropped_silently() {
        let rig = Rig::new();
        let mut g = rig.guard();
        g.begin_session(rig.env(), "jogo-1", true, now());
        assert!(g.pending_from_previous_run(rig.env(), now()).is_none());
        assert!(!rig.store().exists());
    }

    #[test]
    fn unreadable_pending_file_is_dropped() {
        let rig = Rig::new();
        std::fs::write(rig.store().path(), "{ quebrado").unwrap();
        assert!(
            rig.guard()
                .pending_from_previous_run(rig.env(), now())
                .is_none()
        );
        assert!(!rig.store().exists());
    }

    #[test]
    fn fakes_change_only_memory() {
        // EnvironmentContractTests: o que a restauração espera dos fakes.
        let rig = Rig::new();
        let target = DisplayTopology {
            monitors: vec![monitor("DISPLAY1", true, 0, 1280, 720, 60.0)],
            platform_data: Some("opaco".into()),
        };
        rig.displays.apply(&target).unwrap();
        assert_eq!(rig.displays.capture().unwrap().monitors[0].width, 1280);
        assert_eq!(rig.displays.capture().unwrap().platform_data, None);
        rig.audio.set_default("tv").unwrap();
        rig.audio.set_volume("tv", 0.25).unwrap();
        let a = rig.audio.capture().unwrap();
        assert_eq!(a.default_id.as_deref(), Some("tv"));
        assert_eq!(a.level_of("tv").map(|l| l.volume), Some(0.25));
        assert!(rig.audio.set_default("desconectado").is_err());
        assert!(sound().is_equivalent_to(&AudioState {
            levels: vec![sound().levels[0].clone()],
            ..sound()
        }));
        assert!(!sound().is_equivalent_to(&AudioState {
            levels: vec![level("speakers", 0.5, true)],
            ..sound()
        }));
        assert!(!sound().is_equivalent_to(&AudioState {
            default_id: Some("tv".into()),
            ..sound()
        }));
    }
}
