//! "Diagnóstico do sistema": o usuário copia ou anexa o texto ao rodar o checklist de hardware.
//! Port de `Diagnostics/SystemReport.cs`. Os fatos chegam prontos (quem coleta é a plataforma); o texto
//! inteiro passa pelo [`LogRedactor`] e nomes de usuário e de computador viram marcadores.

use super::LogRedactor;
use crate::i18n::{Lang, t_in};
use chrono::{DateTime, FixedOffset};
use regex::Regex;
use std::fmt::Write;

pub const USER_MARKER: &str = "[usuário]";
pub const MACHINE_MARKER: &str = "[computador]";

/// Tela conectada, em pixels físicos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayInfo {
    /// Nome do SO (`\\.\DISPLAY1`, `HDMI-A-1`).
    pub device: String,
    /// Placa de vídeo que alimenta a tela, quando o SO informa.
    pub adapter: Option<String>,
    pub width: u32,
    pub height: u32,
    pub scale_percent: Option<u32>,
    pub refresh_hz: Option<u32>,
    pub is_primary: bool,
}

impl DisplayInfo {
    /// Proporção aproximada (16:9, 21:9...), para casar com o checklist de hardware.
    pub fn aspect_ratio(&self) -> String {
        if self.width == 0 || self.height == 0 {
            return "?".into();
        }
        let ratio = f64::from(self.width) / f64::from(self.height);
        let known = [
            ("4:3", 4.0 / 3.0),
            ("16:10", 16.0 / 10.0),
            ("16:9", 16.0 / 9.0),
            ("21:9", 21.0 / 9.0),
            ("32:9", 32.0 / 9.0),
            ("3:2", 3.0 / 2.0),
        ];
        let (label, value) = known
            .into_iter()
            .min_by(|a, b| (a.1 - ratio).abs().total_cmp(&(b.1 - ratio).abs()))
            .unwrap_or(("?", 0.0));
        if (value - ratio).abs() < 0.07 {
            label.into()
        } else {
            format!("{ratio:.2}:1")
        }
    }
}

/// Fatos do sistema: só o que ajuda a reproduzir um problema de hardware (nada de contas ou lista de jogos).
#[derive(Debug, Clone, Default)]
pub struct SystemSnapshot {
    pub generated_at: DateTime<FixedOffset>,
    pub tako_version: String,
    /// "instalada", "portátil"... já no idioma do relatório.
    pub install_kind: String,
    pub data_folder: Option<String>,
    /// "Windows 11 Pro", "Fedora Linux 41", "macOS".
    pub os_name: Option<String>,
    /// "24H2", "15.1".
    pub os_version: Option<String>,
    /// Build do Windows, versão do kernel ou do Darwin.
    pub os_build: Option<String>,
    pub architecture: String,
    /// Sessão gráfica (ex.: "Wayland · KDE", "gamescope").
    pub desktop_session: Option<String>,
    pub cpu: Option<String>,
    pub logical_processors: u32,
    pub memory_bytes: Option<u64>,
    pub displays: Vec<DisplayInfo>,
    pub controllers: Vec<String>,
    pub active_controller: Option<String>,
    /// Linhas de [`crate::input::controller_test::lines`]; vazio quando não foi lido.
    pub controller_test: Vec<String>,
    pub steam_found: bool,
    pub steam_games: usize,
    pub manual_games: usize,
    pub total_games: usize,
    pub safe_mode: bool,
    pub safe_mode_detail: Option<String>,
    pub theme: Option<String>,
}

/// `diagnostico-AAAAMMDD-HHMMSS.txt`, na pasta de logs.
pub fn file_name(time: DateTime<FixedOffset>) -> String {
    format!("diagnostico-{}.txt", time.format("%Y%m%d-%H%M%S"))
}

pub fn format(
    s: &SystemSnapshot,
    redactor: &LogRedactor,
    user_name: Option<&str>,
    machine_name: Option<&str>,
    lang: Lang,
) -> String {
    let tr = |k: &str| t_in(lang, &format!("diagnostics.report.{k}"));
    let mut out = String::new();
    let line = |out: &mut String, label: &str, value: Option<String>| {
        let value = value
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| tr("missing"));
        let _ = writeln!(out, "{:<22} {value}", format!("{}:", tr(label)));
    };

    let _ = writeln!(out, "{}", tr("title"));
    let when = s.generated_at.format("%Y-%m-%d %H:%M:%S %:z").to_string();
    let _ = writeln!(out, "{}\n", tr("generated").replace("{when}", &when));

    out.push_str("[Tako]\n");
    line(&mut out, "version", Some(s.tako_version.clone()));
    line(&mut out, "install", Some(s.install_kind.clone()));
    line(&mut out, "data_folder", s.data_folder.clone());
    line(&mut out, "theme", s.theme.clone());
    let safe = if s.safe_mode {
        match s.safe_mode_detail.as_deref().filter(|d| !d.is_empty()) {
            Some(d) => format!("{} ({d})", tr("safe_on")),
            None => tr("safe_on"),
        }
    } else {
        tr("safe_off")
    };
    line(&mut out, "safe_mode", Some(safe));
    out.push('\n');

    let _ = writeln!(out, "[{}]", tr("system_section"));
    let os = [&s.os_name, &s.os_version]
        .into_iter()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    line(&mut out, "os", Some(os));
    line(&mut out, "build", s.os_build.clone());
    line(&mut out, "arch", Some(s.architecture.clone()));
    line(&mut out, "session", s.desktop_session.clone());
    let cpu = s
        .cpu
        .as_ref()
        .map(|c| format!("{c} ({} threads)", s.logical_processors));
    line(&mut out, "cpu", cpu);
    let gb = |b: u64| format!("{:.1} GB", b as f64 / (1024.0 * 1024.0 * 1024.0));
    line(&mut out, "memory", s.memory_bytes.map(gb));
    out.push('\n');

    let _ = writeln!(out, "[{}] {}", tr("displays_section"), s.displays.len());
    if s.displays.is_empty() {
        let _ = writeln!(out, "  {}", tr("no_displays"));
    }
    for d in &s.displays {
        let primary = if d.is_primary {
            format!(" ({})", tr("primary"))
        } else {
            String::new()
        };
        let scale = d.scale_percent.map_or("?".into(), |v| format!("{v}%"));
        let hz = d.refresh_hz.map_or("?".into(), |v| v.to_string());
        let _ = write!(
            out,
            "  {}{primary}: {}x{} {}, {} {scale}, {hz} Hz",
            d.device,
            d.width,
            d.height,
            d.aspect_ratio(),
            tr("scale")
        );
        match d.adapter.as_deref().filter(|a| !a.is_empty()) {
            Some(a) => {
                let _ = writeln!(out, " · {a}");
            }
            None => out.push('\n'),
        }
    }
    out.push('\n');

    let _ = writeln!(
        out,
        "[{}] {}",
        tr("controllers_section"),
        s.controllers.len()
    );
    if s.controllers.is_empty() {
        let _ = writeln!(out, "  {}", tr("no_controllers"));
    }
    for c in &s.controllers {
        let _ = writeln!(out, "  {c}");
    }
    let active = s
        .active_controller
        .clone()
        .unwrap_or_else(|| tr("keyboard"));
    let _ = writeln!(out, "{:<22} {active}", format!("  {}:", tr("in_use")));
    out.push('\n');

    if !s.controller_test.is_empty() {
        let _ = writeln!(out, "[{}]", tr("controller_test_section"));
        for l in &s.controller_test {
            let _ = writeln!(out, "  {l}");
        }
        out.push('\n');
    }

    let _ = writeln!(out, "[{}]", tr("library_section"));
    let steam = if s.steam_found {
        tr("steam_found").replace("{n}", &s.steam_games.to_string())
    } else {
        tr("steam_missing")
    };
    line(&mut out, "steam", Some(steam));
    line(&mut out, "manual", Some(s.manual_games.to_string()));
    line(&mut out, "total", Some(s.total_games.to_string()));

    let report = redactor.redact(&out);
    // Computador antes do usuário: nomes como "PC-DO-FULANO" contêm o nome do usuário.
    let report = replace_word(&report, machine_name, MACHINE_MARKER);
    replace_word(&report, user_name, USER_MARKER)
}

/// Troca nomes só como palavra inteira ("Ana" não vira marcador dentro de "Banana").
fn replace_word(text: &str, word: Option<&str>, marker: &str) -> String {
    let Some(word) = word.map(str::trim).filter(|w| w.chars().count() >= 2) else {
        return text.to_owned();
    };
    let Ok(re) = Regex::new(&format!("(?i){}", regex::escape(word))) else {
        return text.to_owned();
    };
    let is_word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for m in re.find_iter(text) {
        let before = text[..m.start()].chars().next_back();
        let after = text[m.end()..].chars().next();
        out.push_str(&text[last..m.start()]);
        out.push_str(if is_word(before) || is_word(after) {
            m.as_str()
        } else {
            marker
        });
        last = m.end();
    }
    out.push_str(&text[last..]);
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::input::controller_test;

    fn sample() -> SystemSnapshot {
        SystemSnapshot {
            generated_at: DateTime::parse_from_rfc3339("2026-10-15T14:30:05-03:00").unwrap(),
            tako_version: "0.1.0-alpha.7".into(),
            install_kind: "instalada".into(),
            data_folder: Some(r"C:\Users\Fulano\AppData\Local\Tako".into()),
            os_name: Some("Windows 11 Pro".into()),
            os_version: Some("24H2".into()),
            os_build: Some("26100.4351".into()),
            architecture: "x86_64".into(),
            desktop_session: None,
            cpu: Some("AMD Ryzen 7 7840U".into()),
            logical_processors: 16,
            memory_bytes: Some(16 * 1024 * 1024 * 1024),
            displays: vec![
                DisplayInfo {
                    device: r"\\.\DISPLAY1".into(),
                    adapter: Some("AMD Radeon 780M".into()),
                    width: 1920,
                    height: 1080,
                    scale_percent: Some(125),
                    refresh_hz: Some(120),
                    is_primary: true,
                },
                DisplayInfo {
                    device: r"\\.\DISPLAY2".into(),
                    adapter: None,
                    width: 3440,
                    height: 1440,
                    scale_percent: None,
                    refresh_hz: None,
                    is_primary: false,
                },
            ],
            controllers: vec!["Xbox (posição 1)".into(), "DualSense (USB)".into()],
            active_controller: Some("Controle Xbox".into()),
            controller_test: vec![],
            steam_found: true,
            steam_games: 42,
            manual_games: 3,
            total_games: 45,
            safe_mode: false,
            safe_mode_detail: None,
            theme: Some("Padrão".into()),
        }
    }

    fn pt(
        s: &SystemSnapshot,
        r: &LogRedactor,
        user: Option<&str>,
        machine: Option<&str>,
    ) -> String {
        format(s, r, user, machine, Lang::PtBr)
    }

    #[test]
    fn report_lists_every_section_of_the_hardware_checklist() {
        let r = pt(
            &sample(),
            &LogRedactor::new(Some(r"C:\Users\Fulano")),
            None,
            None,
        );
        for part in [
            "0.1.0-alpha.7",
            "instalada",
            "Windows 11 Pro 24H2",
            "26100.4351",
            r"\\.\DISPLAY1 (principal): 1920x1080 16:9, escala 125%, 120 Hz · AMD Radeon 780M",
            "3440x1440 21:9, escala ?, ? Hz",
            "DualSense (USB)",
            "encontrada, 42 jogos",
            "desligado",
            "16.0 GB",
            "Gerado em 2026-10-15 14:30:05 -03:00",
        ] {
            assert!(r.contains(part), "{part}\n{r}");
        }
    }

    #[test]
    fn report_has_no_personal_data() {
        let s = SystemSnapshot {
            safe_mode: true,
            safe_mode_detail: Some(
                r"tema em C:\Users\Fulano\Desktop\x falhou no PC-DO-FULANO".into(),
            ),
            ..sample()
        };
        let r = pt(
            &s,
            &LogRedactor::new(Some(r"C:\Users\Fulano")),
            Some("Fulano"),
            Some("PC-DO-FULANO"),
        );
        assert!(!r.to_lowercase().contains("fulano"), "{r}");
        assert!(r.contains(r"%USERPROFILE%\AppData\Local\Tako"));
        assert!(r.contains(MACHINE_MARKER));
    }

    #[test]
    fn short_names_are_replaced_only_as_whole_words() {
        let s = SystemSnapshot {
            cpu: Some("Banana CPU".into()),
            theme: Some("Ana".into()),
            ..sample()
        };
        let r = pt(&s, &LogRedactor::new(None), Some("ana"), None);
        assert!(r.contains("Banana CPU"));
        assert!(
            r.contains(&format!("Tema:                  {USER_MARKER}")),
            "{r}"
        );
    }

    #[test]
    fn missing_values_are_reported_instead_of_hidden() {
        let s = SystemSnapshot {
            displays: vec![],
            controllers: vec![],
            cpu: None,
            memory_bytes: None,
            steam_found: false,
            active_controller: None,
            ..sample()
        };
        let r = pt(&s, &LogRedactor::new(None), None, None);
        assert!(r.contains("nenhuma tela lida"));
        assert!(r.contains("nenhum controle detectado"));
        assert!(r.contains("não encontrada"));
        assert!(r.contains("Processador:           não informado"), "{r}");
        assert!(r.contains("teclado"));
    }

    #[test]
    fn report_is_translated() {
        let r = format(&sample(), &LogRedactor::new(None), None, None, Lang::EnUs);
        assert!(r.contains("Tako · System diagnostics"));
        assert!(r.contains("found, 42 games"));
        assert!(r.contains("Processor:"));
    }

    #[test]
    fn aspect_ratio_matches_the_checklist_labels() {
        for (w, h, e) in [
            (2560, 1080, "21:9"),
            (5120, 1440, "32:9"),
            (1280, 800, "16:10"),
            (3840, 2160, "16:9"),
            (1000, 1000, "1.00:1"),
            (0, 1000, "?"),
        ] {
            let d = DisplayInfo {
                device: "d".into(),
                adapter: None,
                width: w,
                height: h,
                scale_percent: Some(100),
                refresh_hz: Some(60),
                is_primary: true,
            };
            assert_eq!(d.aspect_ratio(), e);
        }
    }

    #[test]
    fn file_name_is_sortable() {
        assert_eq!(
            file_name(sample().generated_at),
            "diagnostico-20261015-143005.txt"
        );
    }

    #[test]
    fn report_includes_the_controller_test_section() {
        let mut s = sample();
        s.controller_test = controller_test::lines(&[], Some(true));
        let r = pt(&s, &LogRedactor::new(None), None, None);
        assert!(
            r.contains(
                "[Teste de controle]\n  Steam em execução: sim\n  nenhum controle lido agora\n"
            ),
            "{r}"
        );
        s.controller_test.clear();
        assert!(!pt(&s, &LogRedactor::new(None), None, None).contains("[Teste de controle]"));
    }
}
