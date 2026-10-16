//! Textos pequenos do Linux e do macOS: `/proc/<pid>/stat`, `registry.vdf` da Steam, argumentos
//! no estilo do shell, URIs e os arquivos de abrir com o sistema (`.desktop` e LaunchAgent).

use chrono::{DateTime, Utc};

/// Ticks por segundo de `/proc` (USER_HZ): fixo em 100 na ABI do Linux.
const USER_HZ: u64 = 100;

/// `(ppid, início)` de `/proc/<pid>/stat`. O nome do processo pode ter espaços e parênteses,
/// por isso os campos são contados depois do último `)`.
pub fn parse_proc_stat(stat: &str, boot_time: Option<i64>) -> Option<(u32, Option<DateTime<Utc>>)> {
    let rest = &stat[stat.rfind(')')? + 1..];
    let f: Vec<&str> = rest.split_whitespace().collect();
    // f[0] é o campo 3 (estado); ppid é o 4 e starttime o 22.
    let ppid = f.get(1)?.parse().ok()?;
    let started = f
        .get(19)
        .and_then(|t| t.parse::<u64>().ok())
        .zip(boot_time)
        .and_then(|(ticks, boot)| {
            DateTime::from_timestamp(boot.checked_add((ticks / USER_HZ) as i64)?, 0)
        });
    Some((ppid, started))
}

/// `btime` de `/proc/stat` (segundos desde 1970 em que o sistema ligou).
pub fn parse_boot_time(proc_stat: &str) -> Option<i64> {
    proc_stat
        .lines()
        .find_map(|l| l.strip_prefix("btime "))
        .and_then(|v| v.trim().parse().ok())
}

/// `RunningAppID` do `registry.vdf` da Steam (0 = nenhum ou arquivo ilegível).
pub fn steam_running_app_id(vdf: &str) -> u32 {
    let mut tokens = vdf_strings(vdf);
    while let Some(t) = tokens.next() {
        if t.eq_ignore_ascii_case("RunningAppID") {
            return tokens.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        }
    }
    0
}

/// As strings entre aspas de um VDF, na ordem (chaves e valores; `{`/`}` ignorados).
fn vdf_strings(text: &str) -> impl Iterator<Item = String> + '_ {
    let mut chars = text.chars();
    std::iter::from_fn(move || {
        chars.by_ref().find(|&c| c == '"')?;
        let mut s = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(s),
                '\\' => s.extend(chars.next()),
                c => s.push(c),
            }
        }
        None
    })
}

/// Separa argumentos como o shell: aspas simples e duplas, `\` escapa. Sem expansão de variáveis.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut has = false;
    let mut quote: Option<char> = None;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('\''), c) => cur.push(c),
            (Some(_), '\\') => match chars.next() {
                Some(n @ ('"' | '\\' | '$' | '`')) => cur.push(n),
                Some(n) => {
                    cur.push('\\');
                    cur.push(n);
                }
                None => cur.push('\\'),
            },
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                has = true;
            }
            (None, '\\') => {
                cur.extend(chars.next());
                has = true;
            }
            (None, c) if c.is_whitespace() => {
                if has || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                has = false;
            }
            (None, c) => cur.push(c),
        }
    }
    if has || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// `steam://run/1`, `heroic://launch/...`, `lutris:rungameid/3`. Caminhos e `C:` não contam.
pub fn is_uri(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    scheme.len() > 1
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
}

/// `~/.config/autostart/tako.desktop` (o `Exec` segue as regras de aspas da especificação).
pub fn autostart_desktop_entry(exe: &str) -> String {
    let mut quoted = String::from('"');
    for c in exe.chars() {
        if matches!(c, '"' | '`' | '$' | '\\') {
            quoted.push('\\');
        }
        quoted.push(c);
    }
    quoted.push('"');
    // `%` é especial no Exec e vira `%%`.
    let quoted = quoted.replace('%', "%%");
    format!(
        "[Desktop Entry]\nType=Application\nName=Tako\nExec={quoted}\nX-GNOME-Autostart-enabled=true\n"
    )
}

/// `~/Library/LaunchAgents/dev.lipp.tako.plist`.
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub fn launch_agent_plist(label: &str, exe: &str) -> String {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>{}</string>
  <key>ProgramArguments</key><array><string>{}</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
"#,
        esc(label),
        esc(exe)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_stat() {
        let stat = "1234 (Game (x86) ok) S 99 1234 1234 0 -1 4194560 100 0 0 0 5 3 0 0 20 0 4 0 25000 1000 200";
        let (ppid, started) = parse_proc_stat(stat, Some(1_700_000_000)).unwrap();
        assert_eq!(ppid, 99);
        assert_eq!(started.unwrap().timestamp(), 1_700_000_250);
        assert_eq!(parse_proc_stat(stat, None).unwrap().1, None);
        assert!(parse_proc_stat("garbage", None).is_none());
        assert!(parse_proc_stat("1 (x) S notanumber", None).is_none());
        assert_eq!(
            parse_boot_time("cpu 1 2\nbtime 1700000000\nprocesses 5"),
            Some(1_700_000_000)
        );
        assert_eq!(parse_boot_time("cpu 1"), None);
    }

    #[test]
    fn reads_running_app_id() {
        let vdf = r#""Registry"
{
	"HKCU"
	{
		"Software"
		{
			"Valve"
			{
				"Steam"
				{
					"language"		"brazilian"
					"Path\"x"		"C:\\a"
					"RunningAppID"		"1091500"
				}
			}
		}
	}
}"#;
        assert_eq!(steam_running_app_id(vdf), 1_091_500);
        assert_eq!(steam_running_app_id(r#""runningappid" "0""#), 0);
        assert_eq!(steam_running_app_id(r#""RunningAppID" "abc""#), 0);
        assert_eq!(steam_running_app_id(r#""RunningAppID"#), 0);
        assert_eq!(steam_running_app_id(""), 0);
    }

    #[test]
    fn splits_like_a_shell() {
        assert_eq!(
            split_args(r#"-w "my file" 'a b' c\ d "x\"y" '' -e"#),
            ["-w", "my file", "a b", "c d", "x\"y", "", "-e"]
        );
        assert!(split_args("   ").is_empty());
        assert_eq!(split_args(r#""unterminated"#), ["unterminated"]);
        assert_eq!(split_args(r#""C:\games""#), [r"C:\games"]);
    }

    #[test]
    fn detects_uris() {
        for u in [
            "steam://rungameid/10",
            "heroic://launch/legendary/x",
            "lutris:rungameid/3",
        ] {
            assert!(is_uri(u), "{u}");
        }
        for p in [
            "/usr/bin/game",
            "C:\\x",
            "game.sh",
            "./a:b",
            "1abc:x",
            "~/x",
        ] {
            assert!(!is_uri(p), "{p}");
        }
    }

    #[test]
    fn writes_autostart_files() {
        let d = autostart_desktop_entry("/opt/Tako $1/100%/tako");
        assert!(d.contains(r#"Exec="/opt/Tako \$1/100%%/tako""#), "{d}");
        let p = launch_agent_plist("dev.lipp.tako", "/Applications/A&B.app/Contents/MacOS/tako");
        assert!(p.contains("<string>/Applications/A&amp;B.app/Contents/MacOS/tako</string>"));
        assert!(p.contains("<key>RunAtLoad</key><true/>"));
    }
}
