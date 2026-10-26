//! Logs e diagnóstico (MIG-015): redação de dados sensíveis, arquivo diário `logs/tako-AAAAMMDD.log`
//! guardado por 7 dias e o relatório do sistema. Port de `Diagnostics/{LogRedactor,FileLoggerProvider}.cs`.

pub mod report;

use chrono::{DateTime, Duration as Days, FixedOffset, Local, NaiveDate};
use regex::{NoExpand, Regex};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::{Layer, filter::LevelFilter};

/// Remove dados sensíveis antes de gravar: a pasta do usuário vira `%USERPROFILE%`, URLs perdem a query
/// (onde ficam tokens e chaves) e pares como `token=...` ou `Bearer ...` são mascarados.
#[derive(Debug, Clone)]
pub struct LogRedactor {
    profile: Option<Regex>,
}

pub const MASK: &str = "[removido]";
pub const PROFILE_MARKER: &str = "%USERPROFILE%";

fn rx(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("regex constante"))
}

fn url_query() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    rx(
        &R,
        r#"(?i)(?<base>\b[a-z][a-z0-9+.\-]*://[^\s?#"'<>]+)\?[^\s#"'<>]*"#,
    )
}

fn bearer() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    rx(
        &R,
        r"(?i)(?<scheme>\b(?:Bearer|Basic))\s+[A-Za-z0-9\-._~+/]+=*",
    )
}

fn secret_pair() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    rx(
        &R,
        r#"(?i)(?<key>\b(?:access_token|refresh_token|id_token|token|api[_-]?key|apikey|client_secret|secret|password|passwd|pwd|authorization|sig|signature)\b)(?<sep>\s*[=:]\s*)[^\s&;,"']+"#,
    )
}

impl LogRedactor {
    /// `user_profile`: pasta do usuário (`C:\Users\Ana`, `/home/ana`); vazia = não troca.
    pub fn new(user_profile: Option<&str>) -> Self {
        let profile = user_profile
            .map(|p| p.trim_end_matches(['\\', '/']))
            .filter(|p| !p.trim().is_empty())
            .and_then(|p| {
                // A mesma pasta escrita com \ ou / (URLs file:///C:/Users/...).
                let forward = p.replace('\\', "/");
                let pattern = if forward == p {
                    regex::escape(p)
                } else {
                    format!("{}|{}", regex::escape(p), regex::escape(&forward))
                };
                Regex::new(&format!("(?i){pattern}")).ok()
            });
        LogRedactor { profile }
    }

    /// Com a pasta do usuário atual (`USERPROFILE` ou `HOME`).
    pub fn for_current_user() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok();
        Self::new(home.as_deref())
    }

    pub fn redact(&self, text: &str) -> String {
        let mut text = match &self.profile {
            Some(p) => p.replace_all(text, NoExpand(PROFILE_MARKER)).into_owned(),
            None => text.to_owned(),
        };
        text = url_query()
            .replace_all(&text, format!("${{base}}?{MASK}"))
            .into_owned();
        text = bearer()
            .replace_all(&text, format!("${{scheme}} {MASK}"))
            .into_owned();
        secret_pair()
            .replace_all(&text, format!("${{key}}${{sep}}{MASK}"))
            .into_owned()
    }
}

pub const RETENTION_DAYS: i64 = 7;

/// `tako-AAAAMMDD.log`.
pub fn log_file_name(date: NaiveDate) -> String {
    format!("tako-{}.log", date.format("%Y%m%d"))
}

/// Apaga os `tako-*.log` com mais de [`RETENTION_DAYS`] dias (o de hoje conta como um).
pub fn delete_expired(dir: &Path, today: NaiveDate) {
    let oldest = today - Days::days(RETENTION_DAYS - 1);
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(stamp) = name
            .to_str()
            .and_then(|n| n.strip_prefix("tako-"))
            .and_then(|n| n.strip_suffix(".log"))
        else {
            continue;
        };
        if NaiveDate::parse_from_str(stamp, "%Y%m%d").is_ok_and(|d| d < oldest) {
            // Aberto por outro programa: tenta de novo no próximo dia.
            let _ = fs::remove_file(entry.path());
        }
    }
}

type Clock = fn() -> DateTime<FixedOffset>;

fn local_now() -> DateTime<FixedOffset> {
    Local::now().fixed_offset()
}

struct DailyState {
    dir: PathBuf,
    redactor: LogRedactor,
    clock: Clock,
    file: Mutex<Option<(NaiveDate, File)>>,
}

impl DailyState {
    fn append(&self, line: &[u8]) {
        let text = self.redactor.redact(&String::from_utf8_lossy(line));
        let today = (self.clock)().date_naive();
        let mut slot = self.file.lock().unwrap_or_else(|e| e.into_inner());
        if slot.as_ref().is_none_or(|(day, _)| *day != today) {
            *slot = None;
            // Sem onde gravar (disco cheio, pasta protegida): descarta, mas nunca derruba o Tako.
            if fs::create_dir_all(&self.dir).is_ok() {
                delete_expired(&self.dir, today);
                let path = self.dir.join(log_file_name(today));
                if let Ok(f) = OpenOptions::new().create(true).append(true).open(path) {
                    *slot = Some((today, f));
                }
            }
        }
        if let Some((_, f)) = slot.as_mut() {
            let _ = f.write_all(text.as_bytes());
        }
    }
}

/// Destino do `fmt` do `tracing`: cada evento chega inteiro num [`LineWriter`], é redigido e anexado
/// ao arquivo do dia. shortcut: grava na thread de quem registra; trocar por fila se o log pesar na UI.
#[derive(Clone)]
pub struct DailyFile(Arc<DailyState>);

impl DailyFile {
    pub fn new(dir: impl Into<PathBuf>, redactor: LogRedactor) -> Self {
        Self::with_clock(dir, redactor, local_now)
    }

    /// Relógio fixo para testes.
    pub fn with_clock(dir: impl Into<PathBuf>, redactor: LogRedactor, clock: Clock) -> Self {
        DailyFile(Arc::new(DailyState {
            dir: dir.into(),
            redactor,
            clock,
            file: Mutex::new(None),
        }))
    }
}

pub struct LineWriter<'a> {
    state: &'a DailyState,
    buf: Vec<u8>,
}

impl Write for LineWriter<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for LineWriter<'_> {
    fn drop(&mut self) {
        if !self.buf.is_empty() {
            self.state.append(&self.buf);
        }
    }
}

impl<'a> MakeWriter<'a> for DailyFile {
    type Writer = LineWriter<'a>;
    fn make_writer(&'a self) -> Self::Writer {
        LineWriter {
            state: &self.0,
            buf: Vec::new(),
        }
    }
}

/// Hora local com fuso, como no C# (`2026-10-15 12:00:00.000 -03:00`).
struct LocalTimer(Clock);

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", (self.0)().format("%Y-%m-%d %H:%M:%S%.3f %:z"))
    }
}

/// Camada do `tracing` que grava `Info` e acima, redigido, em `logs_dir/tako-AAAAMMDD.log`.
pub fn file_layer<S>(file: DailyFile) -> impl Layer<S> + Send + Sync
where
    S: tracing::Subscriber + for<'s> tracing_subscriber::registry::LookupSpan<'s>,
{
    let clock = file.0.clock;
    tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(LocalTimer(clock))
        .with_writer(file)
        .with_filter(LevelFilter::INFO)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    #[test]
    fn redactor_replaces_the_user_profile() {
        let r = LogRedactor::new(Some(r"C:\Users\Fulano"));
        for (input, expected) in [
            (
                r"Falha em C:\Users\Fulano\AppData\Local\Tako\tako.db",
                r"Falha em %USERPROFILE%\AppData\Local\Tako\tako.db",
            ),
            (r"c:\users\fulano\Jogos\x.exe", r"%USERPROFILE%\Jogos\x.exe"),
            (
                "file:///C:/Users/Fulano/capa.png",
                "file:///%USERPROFILE%/capa.png",
            ),
            (r"D:\Jogos\x.exe", r"D:\Jogos\x.exe"),
        ] {
            assert_eq!(r.redact(input), expected);
        }
        let home = LogRedactor::new(Some("/home/fulano/"));
        assert_eq!(
            home.redact("/home/fulano/.local/share/tako"),
            "%USERPROFILE%/.local/share/tako"
        );
        assert_eq!(LogRedactor::new(Some("  ")).redact("a"), "a");
    }

    #[test]
    fn redactor_removes_tokens_and_authenticated_queries() {
        let r = LogRedactor::new(None);
        for (input, expected) in [
            (
                "GET https://api.exemplo.com/v1/jogos?access_token=abc123&x=1 falhou",
                "GET https://api.exemplo.com/v1/jogos?[removido] falhou",
            ),
            (
                "baixando https://cdn.exemplo.com/a.zip?sig=XYZ",
                "baixando https://cdn.exemplo.com/a.zip?[removido]",
            ),
            (
                "Authorization: Bearer eyJhbGciOi.payload.sig",
                "Authorization: [removido] [removido]",
            ),
            (
                "token=segredo; apiKey: 123",
                "token=[removido]; apiKey: [removido]",
            ),
            (
                "https://store.steampowered.com/app/570",
                "https://store.steampowered.com/app/570",
            ),
        ] {
            assert_eq!(r.redact(input), expected);
        }
    }

    fn fixed() -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339("2026-10-15T12:00:00-03:00").unwrap()
    }

    #[test]
    fn file_layer_writes_information_and_above_redacted() {
        let dir = tempfile::tempdir().unwrap();
        let file = DailyFile::with_clock(
            dir.path(),
            LogRedactor::new(Some(r"C:\Users\Fulano")),
            fixed,
        );
        let subscriber = tracing_subscriber::registry().with(file_layer(file));
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!("não aparece");
            tracing::info!(r"abrindo C:\Users\Fulano\jogo.exe");
            tracing::error!(error = "quebrou", "erro com https://x.com/a?token=1");
        });
        let path = dir.path().join(log_file_name(fixed().date_naive()));
        let text = fs::read_to_string(path).unwrap();
        assert!(!text.contains("não aparece"));
        assert!(text.contains(r"abrindo %USERPROFILE%\jogo.exe"), "{text}");
        assert!(text.contains("erro com https://x.com/a?[removido]"));
        assert!(text.contains("quebrou"));
        assert!(text.contains("2026-10-15 12:00:00.000 -03:00"));
        assert!(!text.contains("Fulano"));
    }

    #[test]
    fn file_layer_is_safe_from_many_threads() {
        let dir = tempfile::tempdir().unwrap();
        let file = DailyFile::with_clock(dir.path(), LogRedactor::new(None), fixed);
        let dispatch =
            tracing::Dispatch::new(tracing_subscriber::registry().with(file_layer(file)));
        let threads: Vec<_> = (0..8)
            .map(|t| {
                let d = dispatch.clone();
                std::thread::spawn(move || {
                    tracing::dispatcher::with_default(&d, || {
                        for i in 0..100 {
                            tracing::info!("linha {t}-{i}");
                        }
                    })
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        let path = dir.path().join(log_file_name(fixed().date_naive()));
        assert_eq!(fs::read_to_string(path).unwrap().lines().count(), 800);
    }

    #[test]
    fn expired_logs_are_deleted_after_seven_days() {
        let dir = tempfile::tempdir().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
        for ago in [0, 6, 7, 30] {
            fs::write(dir.path().join(log_file_name(today - Days::days(ago))), "x").unwrap();
        }
        fs::write(dir.path().join("outro.log"), "x").unwrap();
        fs::write(dir.path().join("tako-lixo.log"), "x").unwrap();
        delete_expired(dir.path(), today);
        let mut left: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "outro.log",
                "tako-20261009.log",
                "tako-20261015.log",
                "tako-lixo.log"
            ]
        );
        delete_expired(&dir.path().join("não existe"), today);
    }
}
