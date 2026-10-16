//! Diário de sessão e proteção contra loop de travamento (port de `Resilience/{SessionJournal,FaultBudget}.cs`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Marcador "sessão em andamento" gravado na abertura (`session.json`, mesmos nomes do C#).
/// `running` continua verdadeiro se o Tako não fechou limpo; `unclean_exits` conta as saídas sujas seguidas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SessionMarker {
    pub running: bool,
    pub unclean_exits: u32,
    pub started_at: Option<DateTime<Utc>>,
}

impl SessionMarker {
    /// Arquivo ilegível: o processo morreu no meio da gravação, conta como uma saída suja.
    fn unreadable() -> Self {
        SessionMarker {
            running: true,
            unclean_exits: 0,
            started_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupReason {
    Normal,
    /// As últimas aberturas não fecharam limpo.
    CrashLoop,
    /// `--safe-mode`.
    Requested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupDecision {
    pub safe_mode: bool,
    pub reason: StartupReason,
    pub unclean_exits: u32,
    pub marker: SessionMarker,
}

/// Saídas sujas seguidas que levam a próxima abertura ao modo seguro.
pub const UNCLEAN_EXITS_FOR_SAFE_MODE: u32 = 2;

pub fn decide(
    previous: Option<&SessionMarker>,
    safe_mode_requested: bool,
    now: DateTime<Utc>,
) -> StartupDecision {
    let unclean = match previous {
        Some(m) if m.running => m.unclean_exits.saturating_add(1),
        _ => 0,
    };
    let reason = if safe_mode_requested {
        StartupReason::Requested
    } else if unclean >= UNCLEAN_EXITS_FOR_SAFE_MODE {
        StartupReason::CrashLoop
    } else {
        StartupReason::Normal
    };
    StartupDecision {
        safe_mode: reason != StartupReason::Normal,
        reason,
        unclean_exits: unclean,
        marker: SessionMarker {
            running: true,
            unclean_exits: unclean,
            started_at: Some(now),
        },
    }
}

/// Marcador do fechamento limpo: zera a contagem.
pub fn clean_exit() -> SessionMarker {
    SessionMarker {
        running: false,
        unclean_exits: 0,
        started_at: None,
    }
}

/// Arquivo do marcador. Falhas de leitura e gravação nunca impedem a abertura: sem gravar, o Tako abre
/// normalmente (só perde a proteção nesta sessão).
#[derive(Debug)]
pub struct SessionJournal {
    path: PathBuf,
    current: Option<SessionMarker>,
}

const MAX_MARKER_BYTES: u64 = 64 * 1024;

impl SessionJournal {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        SessionJournal {
            path: path.into(),
            current: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Lê o marcador anterior, decide o modo e grava "sessão em andamento".
    pub fn begin(&mut self, safe_mode_requested: bool, now: DateTime<Utc>) -> StartupDecision {
        let decision = decide(self.read().as_ref(), safe_mode_requested, now);
        self.save(decision.marker.clone());
        decision
    }

    /// Fechamento limpo.
    pub fn end_clean(&mut self) {
        self.save(clean_exit());
    }

    /// Sair do modo seguro nesta sessão: as falhas anteriores deixam de contar.
    pub fn forgive(&mut self, now: DateTime<Utc>) {
        let mut marker = self.current.clone().unwrap_or(SessionMarker {
            running: true,
            unclean_exits: 0,
            started_at: Some(now),
        });
        marker.unclean_exits = 0;
        self.save(marker);
    }

    /// Marcador gravado; ausente = `None`; ilegível conta como sessão que não fechou limpo.
    pub fn read(&self) -> Option<SessionMarker> {
        if !self.path.exists() {
            return None;
        }
        let mut text = String::new();
        let ok = std::fs::File::open(&self.path)
            .and_then(|f| f.take(MAX_MARKER_BYTES).read_to_string(&mut text))
            .is_ok();
        Some(
            ok.then(|| serde_json::from_str(&text).ok())
                .flatten()
                .unwrap_or_else(SessionMarker::unreadable),
        )
    }

    fn save(&mut self, marker: SessionMarker) {
        if let Err(e) = write_atomic(&self.path, &marker) {
            tracing::warn!(
                "diário de sessão não gravado em {}: {e}",
                self.path.display()
            );
        }
        self.current = Some(marker);
    }
}

/// Grava JSON num `.tmp` e troca pelo arquivo (uma queda no meio não deixa o arquivo pela metade).
pub(crate) fn write_atomic<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)
}

/// Limite de erros não tratados numa janela: um isolado mostra a tela de recuperação; muitos seguidos
/// (um erro a cada quadro) significam que a tela não tem conserto e o Tako devolve o desktop.
#[derive(Debug)]
pub struct FaultBudget {
    max_faults: usize,
    window: Duration,
    faults: VecDeque<Duration>,
}

impl Default for FaultBudget {
    fn default() -> Self {
        Self::new(3, Duration::from_secs(10))
    }
}

impl FaultBudget {
    pub fn new(max_faults: usize, window: Duration) -> Self {
        FaultBudget {
            max_faults,
            window,
            faults: VecDeque::new(),
        }
    }

    /// Registra um erro em `now` (relógio monotônico); falso quando o limite da janela estourou.
    pub fn try_record(&mut self, now: Duration) -> bool {
        while self
            .faults
            .front()
            .is_some_and(|f| now.saturating_sub(*f) >= self.window)
        {
            self.faults.pop_front();
        }
        self.faults.push_back(now);
        self.faults.len() <= self.max_faults
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-15T20:00:00Z")
            .map(|d| d.to_utc())
            .unwrap_or_default()
    }

    fn marker(running: bool, unclean: u32) -> SessionMarker {
        SessionMarker {
            running,
            unclean_exits: unclean,
            started_at: Some(now()),
        }
    }

    #[test]
    fn first_start_is_normal() {
        assert!(!decide(None, false, now()).safe_mode);
    }

    #[test]
    fn start_after_clean_exit_is_normal() {
        let d = decide(Some(&clean_exit()), false, now());
        assert!(!d.safe_mode);
        assert_eq!(d.unclean_exits, 0);
        assert!(d.marker.running);
    }

    #[test]
    fn one_unclean_exit_is_not_enough_for_safe_mode() {
        let d = decide(Some(&marker(true, 0)), false, now());
        assert!(!d.safe_mode);
        assert_eq!(d.unclean_exits, 1);
    }

    #[test]
    fn two_unclean_exits_in_a_row_start_in_safe_mode() {
        let d = decide(Some(&marker(true, 1)), false, now());
        assert!(d.safe_mode);
        assert_eq!(d.reason, StartupReason::CrashLoop);
        assert_eq!(d.unclean_exits, 2);
    }

    #[test]
    fn requested_safe_mode_wins_over_a_clean_history() {
        let d = decide(Some(&clean_exit()), true, now());
        assert!(d.safe_mode);
        assert_eq!(d.reason, StartupReason::Requested);
    }

    #[test]
    fn journal_counts_crashes_and_clears_on_clean_exit() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let file = dir.path().join("session.json");
        assert!(!SessionJournal::new(&file).begin(false, now()).safe_mode);
        assert!(!SessionJournal::new(&file).begin(false, now()).safe_mode);
        let mut third = SessionJournal::new(&file);
        assert!(third.begin(false, now()).safe_mode);
        third.end_clean();
        assert!(!SessionJournal::new(&file).begin(false, now()).safe_mode);
    }

    #[test]
    fn leaving_safe_mode_forgives_previous_crashes() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let file = dir.path().join("session.json");
        SessionJournal::new(&file).begin(false, now());
        SessionJournal::new(&file).begin(false, now());
        let mut safe = SessionJournal::new(&file);
        assert!(safe.begin(false, now()).safe_mode);
        safe.forgive(now());
        assert!(!SessionJournal::new(&file).begin(false, now()).safe_mode);
    }

    #[test]
    fn corrupted_marker_counts_as_one_unclean_exit() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let file = dir.path().join("session.json");
        std::fs::write(&file, "{\"Running\": tr").unwrap_or_default();
        let d = SessionJournal::new(&file).begin(false, now());
        assert!(!d.safe_mode);
        assert_eq!(d.unclean_exits, 1);
    }

    #[test]
    fn reads_the_marker_written_by_the_csharp_app() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let file = dir.path().join("session.json");
        let csharp =
            r#"{ "Running": true, "UncleanExits": 1, "StartedAt": "2026-10-15T20:00:00+00:00" }"#;
        std::fs::write(&file, csharp).unwrap_or_default();
        assert_eq!(SessionJournal::new(&file).read(), Some(marker(true, 1)));
    }

    #[test]
    fn unwritable_folder_still_starts() {
        let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("{e}"));
        let blocker = dir.path().join("arquivo");
        std::fs::write(&blocker, "").unwrap_or_default();
        let mut j = SessionJournal::new(blocker.join("session.json"));
        assert!(!j.begin(false, now()).safe_mode);
        j.end_clean();
    }

    #[test]
    fn fault_budget_allows_isolated_errors() {
        let mut b = FaultBudget::new(3, Duration::from_secs(10));
        for s in [0, 20, 40, 60] {
            assert!(b.try_record(Duration::from_secs(s)));
        }
    }

    #[test]
    fn fault_budget_trips_on_a_burst() {
        let mut b = FaultBudget::default();
        assert!(b.try_record(Duration::from_secs(1)));
        assert!(b.try_record(Duration::from_secs(2)));
        assert!(b.try_record(Duration::from_secs(3)));
        assert!(!b.try_record(Duration::from_secs(4)));
    }
}
