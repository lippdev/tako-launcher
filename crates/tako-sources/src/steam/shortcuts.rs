//! Jogos não Steam adicionados à Steam (SRC-004): `userdata/<conta>/config/shortcuts.vdf`, em VDF binário.
//! Abrem por `steam://rungameid/<id>`, então ganham o overlay e o Steam Input como na Steam.

use super::ACCENTS;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use tako_core::library::{Game, GameStore};

/// Um `shortcuts.vdf` de verdade tem poucos KB; acima disso não é o arquivo da Steam.
const MAX_FILE: u64 = 4 << 20;
const MAX_DEPTH: usize = 8;

/// Atalho de um jogo não Steam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamShortcut {
    pub app_id: u32,
    pub name: String,
    pub exe: String,
    pub start_dir: String,
    pub last_played: Option<DateTime<Utc>>,
}

impl SteamShortcut {
    /// Id que a Steam usa para abrir o atalho: o appid nos 32 bits altos mais a marca de atalho.
    pub fn game_id(&self) -> u64 {
        (u64::from(self.app_id) << 32) | 0x0200_0000
    }
}

#[derive(Debug, PartialEq)]
enum Value {
    Map(Vec<(String, Value)>),
    Text(String),
    Int(u32),
}

impl Value {
    fn get(&self, key: &str) -> Option<&Value> {
        match self {
            // A Steam já gravou "AppName" e "appname": a chave não diferencia maiúsculas.
            Value::Map(m) => m
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            _ => None,
        }
    }

    fn text(&self, key: &str) -> String {
        match self.get(key) {
            Some(Value::Text(s)) => s.clone(),
            _ => String::new(),
        }
    }

    fn int(&self, key: &str) -> Option<u32> {
        match self.get(key) {
            Some(Value::Int(n)) => Some(*n),
            _ => None,
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let b = *self.data.get(self.at)?;
        self.at += 1;
        Some(b)
    }

    fn cstr(&mut self) -> Option<String> {
        let rest = self.data.get(self.at..)?;
        let end = rest.iter().position(|&b| b == 0)?;
        self.at += end + 1;
        Some(String::from_utf8_lossy(&rest[..end]).into_owned())
    }

    fn u32(&mut self) -> Option<u32> {
        let b = self.data.get(self.at..self.at + 4)?;
        self.at += 4;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Itens de um mapa até o 0x08 que o fecha.
    fn map(&mut self, depth: usize) -> Option<Vec<(String, Value)>> {
        if depth > MAX_DEPTH {
            return None;
        }
        let mut items = Vec::new();
        loop {
            let kind = self.byte()?;
            if kind == 0x08 {
                return Some(items);
            }
            let key = self.cstr()?;
            let value = match kind {
                0x00 => Value::Map(self.map(depth + 1)?),
                0x01 => Value::Text(self.cstr()?),
                0x02 => Value::Int(self.u32()?),
                _ => return None,
            };
            items.push((key, value));
        }
    }
}

/// Atalhos visíveis de um `shortcuts.vdf`; arquivo quebrado ou truncado não traz nenhum.
pub fn parse(data: &[u8]) -> Vec<SteamShortcut> {
    let mut r = Reader { data, at: 0 };
    let Some(root) = r.map(0).map(Value::Map) else {
        return Vec::new();
    };
    let Some(Value::Map(list)) = root.get("shortcuts") else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|(_, s)| {
            let app_id = s.int("appid")?;
            let name = s.text("AppName").trim().to_owned();
            if name.is_empty() || s.int("IsHidden").unwrap_or(0) != 0 {
                return None;
            }
            Some(SteamShortcut {
                app_id,
                name,
                exe: unquote(&s.text("Exe")),
                start_dir: unquote(&s.text("StartDir")),
                last_played: s
                    .int("LastPlayTime")
                    .filter(|t| *t > 0)
                    .and_then(|t| DateTime::from_timestamp(i64::from(t), 0)),
            })
        })
        .collect()
}

/// A Steam grava os caminhos entre aspas.
fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_owned()
}

/// O próprio Tako adicionado à Steam (comum para abrir pelo modo Big Picture) não é jogo.
fn is_tako(exe: &str) -> bool {
    Path::new(&exe.replace('\\', "/"))
        .file_stem()
        .is_some_and(|s| s.eq_ignore_ascii_case("tako"))
}

/// Atalhos da conta em `config` (`userdata/<conta>/config`), já como jogos.
pub fn scan(config: &Path) -> Vec<Game> {
    let path = config.join("shortcuts.vdf");
    let too_big = std::fs::metadata(&path).map_or(true, |m| m.len() > MAX_FILE);
    if too_big {
        return Vec::new();
    }
    let Ok(data) = std::fs::read(&path) else {
        return Vec::new();
    };
    parse(&data)
        .iter()
        .filter(|s| !is_tako(&s.exe))
        .map(|s| to_game(s, &config.join("grid")))
        .collect()
}

/// Id `steam-shortcut:<appid>`; a capa e o fundo vêm das imagens que o usuário pôs na Steam, se houver.
pub fn to_game(s: &SteamShortcut, grid: &Path) -> Game {
    let art = |suffix: &str| -> Option<String> {
        ["png", "jpg"]
            .iter()
            .map(|ext| grid.join(format!("{}{suffix}.{ext}", s.app_id)))
            .find(|p| p.is_file())
            .map(|p: PathBuf| p.to_string_lossy().into_owned())
    };
    Game {
        id: format!("steam-shortcut:{}", s.app_id),
        title: s.name.clone(),
        store: Some(GameStore::Steam),
        accent: ACCENTS[s.app_id as usize % ACCENTS.len()].to_owned(),
        cover_uri: art("p"),
        hero_uri: art("_hero"),
        is_installed: true,
        last_played_at: s.last_played,
        launch_target: Some(format!("steam://rungameid/{}", s.game_id())),
        // Acompanhado pela pasta do jogo: a Steam não informa o RunningAppID de atalhos.
        install_path: Some(s.start_dir.clone()).filter(|d| !d.is_empty()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(out: &mut Vec<u8>, key: &str, value: &str) {
        out.push(0x01);
        out.extend_from_slice(key.as_bytes());
        out.push(0);
        out.extend_from_slice(value.as_bytes());
        out.push(0);
    }

    fn int(out: &mut Vec<u8>, key: &str, value: u32) {
        out.push(0x02);
        out.extend_from_slice(key.as_bytes());
        out.push(0);
        out.extend_from_slice(&value.to_le_bytes());
    }

    fn open(out: &mut Vec<u8>, key: &str) {
        out.push(0x00);
        out.extend_from_slice(key.as_bytes());
        out.push(0);
    }

    /// O formato que a Steam grava, com dois atalhos visíveis, um escondido e o próprio Tako.
    fn sample() -> Vec<u8> {
        let mut v = Vec::new();
        open(&mut v, "shortcuts");
        let entries: [(&str, u32, &str, u32, u32); 4] = [
            (
                "Jogo Livre",
                3_000_000_001,
                r#""C:\Jogos\Livre\livre.exe""#,
                0,
                1_700_000_000,
            ),
            ("Emulador", 2_900_000_000, "\"/usr/bin/emu\"", 0, 0),
            ("Escondido", 12, "x.exe", 1, 0),
            ("Tako", 13, r#""C:\Program Files\Tako\Tako.exe""#, 0, 0),
        ];
        for (i, (name, id, exe, hidden, t)) in entries.iter().enumerate() {
            open(&mut v, &i.to_string());
            int(&mut v, "appid", *id);
            text(&mut v, if i == 1 { "appname" } else { "AppName" }, name);
            text(&mut v, "Exe", exe);
            text(&mut v, "StartDir", r#""C:\Jogos\Livre\""#);
            int(&mut v, "IsHidden", *hidden);
            int(&mut v, "LastPlayTime", *t);
            open(&mut v, "tags");
            text(&mut v, "0", "favorite");
            v.push(0x08);
            v.push(0x08);
        }
        v.push(0x08);
        v.push(0x08);
        v
    }

    #[test]
    fn reads_visible_shortcuts_with_any_key_case() {
        let list = parse(&sample());
        let names: Vec<_> = list.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Jogo Livre", "Emulador", "Tako"]);
        assert_eq!(list[0].exe, r"C:\Jogos\Livre\livre.exe");
        assert_eq!(list[0].start_dir, r"C:\Jogos\Livre\");
        assert_eq!(list[0].last_played.unwrap().timestamp(), 1_700_000_000);
        assert_eq!(list[1].last_played, None);
    }

    #[test]
    fn launches_through_steam_with_the_shortcut_game_id() {
        let s = &parse(&sample())[0];
        let g = to_game(s, Path::new("/nao/existe"));
        assert_eq!(g.id, "steam-shortcut:3000000001");
        assert_eq!(g.store(), GameStore::Steam);
        assert_eq!(
            g.launch_target.as_deref(),
            Some("steam://rungameid/12884901892328521728")
        );
        assert_eq!(g.steam_app_id, None);
        assert_eq!(g.cover_uri, None);
    }

    #[test]
    fn skips_tako_itself_and_uses_the_steam_grid_art() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("shortcuts.vdf"), sample()).unwrap();
        std::fs::create_dir(dir.path().join("grid")).unwrap();
        std::fs::write(dir.path().join("grid").join("3000000001p.png"), b"").unwrap();
        let games = scan(dir.path());
        let titles: Vec<_> = games.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, ["Jogo Livre", "Emulador"]);
        assert!(
            games[0]
                .cover_uri
                .as_deref()
                .unwrap()
                .ends_with("3000000001p.png")
        );
        assert_eq!(games[0].hero_uri, None);
    }

    #[test]
    fn broken_files_bring_nothing_and_never_panic() {
        let full = sample();
        for cut in 0..full.len() {
            let _ = parse(&full[..cut]);
        }
        assert!(parse(&full[..full.len() / 2]).is_empty());
        assert!(parse(b"").is_empty());
        assert!(parse(&[0x07, b'x', 0]).is_empty());
        // Mapas aninhados sem fim não estouram a pilha.
        let deep: Vec<u8> = std::iter::repeat_n([0x00, b'a', 0], 10_000)
            .flatten()
            .collect();
        assert!(parse(&deep).is_empty());
        assert!(scan(Path::new("/nao/existe")).is_empty());
    }
}
