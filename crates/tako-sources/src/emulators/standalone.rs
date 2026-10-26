//! Bibliotecas dos emuladores avulsos: as pastas de jogos que o usuário já configurou em cada um (e, no RPCS3, a
//! lista de jogos dele). Cada leitor é uma função pura sobre o arquivo de configuração; [`config_files`] diz onde
//! esse arquivo costuma estar em cada SO (incluindo a versão "portátil", ao lado do executável).

use super::emulator::{Emulator, Env};
use super::ini::Ini;
use std::path::{Path, PathBuf};
use tako_core::paths::Os;

/// Pastas a varrer (`true` = com subpastas) e jogos já conhecidos (caminho, nome).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Library {
    pub dirs: Vec<(PathBuf, bool)>,
    pub games: Vec<(PathBuf, Option<String>)>,
}

/// Arquivos de configuração possíveis, do mais provável ao menos.
pub fn config_files(emu: Emulator, env: &Env, exe_dir: Option<&Path>) -> Vec<PathBuf> {
    let os = env.os();
    let docs = env.documents();
    let appdata = env.var("APPDATA").map(Path::to_path_buf);
    let local = env.var("LOCALAPPDATA").map(Path::to_path_buf);
    let fp = |id: &str, rest: &str| env.flatpak_home(id).join(rest);
    let mut v: Vec<PathBuf> = Vec::new();
    match emu {
        Emulator::RetroArch => {}
        Emulator::Dolphin => {
            if let Some(d) = exe_dir {
                v.push(d.join("User").join("Config").join("Dolphin.ini"));
            }
            match os {
                Os::Windows => {
                    v.extend(appdata.map(|a| {
                        a.join("Dolphin Emulator")
                            .join("Config")
                            .join("Dolphin.ini")
                    }));
                    v.extend(docs.iter().map(|d| {
                        d.join("Dolphin Emulator")
                            .join("Config")
                            .join("Dolphin.ini")
                    }));
                }
                Os::Linux => {
                    v.push(env.config_home().join("dolphin-emu").join("Dolphin.ini"));
                    v.push(
                        env.home
                            .join(".dolphin-emu")
                            .join("Config")
                            .join("Dolphin.ini"),
                    );
                    v.push(fp(
                        "org.DolphinEmu.dolphin-emu",
                        "config/dolphin-emu/Dolphin.ini",
                    ));
                }
                Os::MacOs => v.push(
                    env.mac_support()
                        .join("Dolphin")
                        .join("Config")
                        .join("Dolphin.ini"),
                ),
            }
        }
        Emulator::Pcsx2 => {
            if let Some(d) = exe_dir {
                v.push(d.join("inis").join("PCSX2.ini"));
            }
            match os {
                Os::Windows => v.extend(
                    docs.iter()
                        .map(|d| d.join("PCSX2").join("inis").join("PCSX2.ini")),
                ),
                Os::Linux => {
                    v.push(
                        env.config_home()
                            .join("PCSX2")
                            .join("inis")
                            .join("PCSX2.ini"),
                    );
                    v.push(fp("net.pcsx2.PCSX2", "config/PCSX2/inis/PCSX2.ini"));
                }
                Os::MacOs => v.push(
                    env.mac_support()
                        .join("PCSX2")
                        .join("inis")
                        .join("PCSX2.ini"),
                ),
            }
        }
        Emulator::DuckStation => {
            if let Some(d) = exe_dir {
                v.push(d.join("settings.ini"));
            }
            match os {
                Os::Windows => {
                    v.extend(local.map(|l| l.join("DuckStation").join("settings.ini")));
                    v.extend(
                        docs.iter()
                            .map(|d| d.join("DuckStation").join("settings.ini")),
                    );
                }
                Os::Linux => {
                    v.push(env.data_home().join("duckstation").join("settings.ini"));
                    v.push(fp(
                        "org.duckstation.DuckStation",
                        "data/duckstation/settings.ini",
                    ));
                }
                Os::MacOs => v.push(env.mac_support().join("DuckStation").join("settings.ini")),
            }
        }
        Emulator::Ppsspp => {
            let rel = Path::new("PSP").join("SYSTEM").join("ppsspp.ini");
            if let Some(d) = exe_dir {
                v.push(d.join("memstick").join(&rel));
            }
            match os {
                Os::Windows => v.extend(docs.iter().map(|d| d.join("PPSSPP").join(&rel))),
                Os::Linux | Os::MacOs => {
                    v.push(env.config_home().join("ppsspp").join(&rel));
                    v.push(fp("org.ppsspp.PPSSPP", "config/ppsspp").join(&rel));
                }
            }
        }
        Emulator::Ryujinx => {
            if let Some(d) = exe_dir {
                v.push(d.join("portable").join("Config.json"));
            }
            match os {
                Os::Windows => v.extend(appdata.map(|a| a.join("Ryujinx").join("Config.json"))),
                Os::Linux => {
                    v.push(env.config_home().join("Ryujinx").join("Config.json"));
                    for id in Emulator::Ryujinx.flatpak_ids() {
                        v.push(fp(id, "config/Ryujinx/Config.json"));
                    }
                }
                Os::MacOs => v.push(env.mac_support().join("Ryujinx").join("Config.json")),
            }
        }
        Emulator::Cemu => {
            if let Some(d) = exe_dir {
                v.push(d.join("settings.xml"));
            }
            match os {
                Os::Windows => v.extend(appdata.map(|a| a.join("Cemu").join("settings.xml"))),
                Os::Linux => {
                    v.push(env.config_home().join("Cemu").join("settings.xml"));
                    v.push(fp("info.cemu.Cemu", "config/Cemu/settings.xml"));
                }
                Os::MacOs => v.push(env.mac_support().join("Cemu").join("settings.xml")),
            }
        }
        Emulator::Rpcs3 => {
            if let Some(d) = exe_dir {
                v.push(d.join("config").join("games.yml"));
                v.push(d.join("games.yml"));
            }
            match os {
                Os::Windows => {}
                Os::Linux => {
                    v.push(env.config_home().join("rpcs3").join("games.yml"));
                    v.push(fp("net.rpcs3.RPCS3", "config/rpcs3/games.yml"));
                }
                Os::MacOs => v.push(env.mac_support().join("rpcs3").join("games.yml")),
            }
        }
    }
    v
}

fn dir(s: &str) -> PathBuf {
    PathBuf::from(s.trim().trim_matches('"'))
}

/// `Dolphin.ini`: `[General] ISOPath0..` e `RecursiveISOPaths`.
pub fn dolphin(text: &str) -> Library {
    let ini = Ini::parse(text);
    let recursive = ini
        .get("General", "RecursiveISOPaths")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    Library {
        dirs: ini
            .numbered("General", "ISOPath")
            .iter()
            .map(|p| (dir(p), recursive))
            .collect(),
        games: Vec::new(),
    }
}

/// `PCSX2.ini` e o `settings.ini` do DuckStation: `[GameList] Paths` e `RecursivePaths` (repetidas).
pub fn game_list_ini(text: &str) -> Library {
    let ini = Ini::parse(text);
    Library {
        dirs: ini
            .all("GameList", "Paths")
            .map(|p| (dir(p), false))
            .chain(
                ini.all("GameList", "RecursivePaths")
                    .map(|p| (dir(p), true)),
            )
            .filter(|(p, _)| !p.as_os_str().is_empty())
            .collect(),
        games: Vec::new(),
    }
}

/// `ppsspp.ini`: a pasta aberta por último, as fixadas e os recentes.
pub fn ppsspp(text: &str) -> Library {
    let ini = Ini::parse(text);
    let mut dirs: Vec<(PathBuf, bool)> = ini
        .numbered("PinnedPaths", "Path")
        .iter()
        .map(|p| (dir(p), false))
        .collect();
    if let Some(c) = ini.get("General", "CurrentDirectory") {
        dirs.push((dir(c), false));
    }
    Library {
        dirs,
        games: ini
            .numbered("Recent", "FileName")
            .iter()
            .map(|p| (dir(p), None))
            .collect(),
    }
}

/// `Config.json` do Ryujinx: `game_dirs`.
pub fn ryujinx(text: &str) -> Library {
    let dirs = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v.get("game_dirs").and_then(|d| d.as_array()).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|d| d.as_str())
        .map(|d| (dir(d), false))
        .collect();
    Library {
        dirs,
        games: Vec::new(),
    }
}

/// `settings.xml` do Cemu: `<GamePaths><Entry>`.
pub fn cemu(text: &str) -> Library {
    let Ok(doc) = roxmltree::Document::parse(text) else {
        return Library::default();
    };
    let dirs = doc
        .descendants()
        .filter(|n| n.has_tag_name("GamePaths"))
        .flat_map(|n| n.children().filter(|c| c.has_tag_name("Entry")))
        .filter_map(|e| e.text())
        .map(|t| (dir(t), true))
        .collect();
    Library {
        dirs,
        games: Vec::new(),
    }
}

/// `games.yml` do RPCS3: `SERIAL: caminho/do/jogo/` por linha.
pub fn rpcs3(text: &str) -> Library {
    let games = text
        .lines()
        .filter_map(|l| {
            let (serial, path) = l.split_once(':')?;
            let serial = serial.trim();
            // Caminho do Windows ("D:/x") também tem `:`; o serial nunca tem espaço nem barra.
            if serial.is_empty() || serial.contains(['/', '\\', ' ']) || l.starts_with([' ', '#']) {
                return None;
            }
            let path = path.trim().trim_matches(['"', '\'']);
            (!path.is_empty()).then(|| (PathBuf::from(path), None))
        })
        .collect();
    Library {
        dirs: Vec::new(),
        games,
    }
}

/// Lê a biblioteca de um emulador pelo arquivo de configuração.
pub fn parse(emu: Emulator, text: &str) -> Library {
    match emu {
        Emulator::RetroArch => Library::default(),
        Emulator::Dolphin => dolphin(text),
        Emulator::Pcsx2 | Emulator::DuckStation => game_list_ini(text),
        Emulator::Ppsspp => ppsspp(text),
        Emulator::Ryujinx => ryujinx(text),
        Emulator::Cemu => cemu(text),
        Emulator::Rpcs3 => rpcs3(text),
    }
}

/// Título de um jogo de PS3 pelo `PARAM.SFO` (formato fixo da Sony: cabeçalho, chaves e valores).
pub fn sfo_title(data: &[u8]) -> Option<String> {
    let u32_at = |o: usize| {
        data.get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let u16_at = |o: usize| data.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    if data.get(..4)? != b"\0PSF" {
        return None;
    }
    let keys = u32_at(8)? as usize;
    let values = u32_at(12)? as usize;
    let count = (u32_at(16)? as usize).min(256);
    for i in 0..count {
        let e = 20 + i * 16;
        let key_off = keys + u16_at(e)? as usize;
        let len = u32_at(e + 4)? as usize;
        let val_off = values + u32_at(e + 12)? as usize;
        let key_end = data.get(key_off..)?.iter().position(|b| *b == 0)? + key_off;
        if &data[key_off..key_end] == b"TITLE" {
            let raw = data.get(val_off..val_off + len.min(512))?;
            let raw = raw.split(|b| *b == 0).next()?;
            let title = String::from_utf8_lossy(raw).trim().to_owned();
            return (!title.is_empty()).then_some(title);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_reader() {
        let d = dolphin(
            "[General]\nISOPaths = 2\nISOPath0 = D:\\GC\nISOPath1 = /wii\nRecursiveISOPaths = True\n",
        );
        assert_eq!(
            d.dirs,
            [
                (PathBuf::from("D:\\GC"), true),
                (PathBuf::from("/wii"), true)
            ]
        );
        let p = game_list_ini("[GameList]\nRecursivePaths = /ps2\nPaths = /ps2b\n");
        assert_eq!(
            p.dirs,
            [
                (PathBuf::from("/ps2b"), false),
                (PathBuf::from("/ps2"), true)
            ]
        );
        let pp = ppsspp(
            "[General]\nCurrentDirectory = /psp/\n[Recent]\nFileName0 = /psp/a.iso\n[PinnedPaths]\nPath0 = /pinned\n",
        );
        assert_eq!(pp.dirs.len(), 2);
        assert_eq!(pp.games, [(PathBuf::from("/psp/a.iso"), None)]);
        let r = ryujinx(r#"{"game_dirs":["/switch","C:\\Switch"],"x":1}"#);
        assert_eq!(r.dirs.len(), 2);
        assert!(ryujinx("{lixo").dirs.is_empty());
        let c = cemu("<content><GamePaths><Entry>/wiiu</Entry></GamePaths></content>");
        assert_eq!(c.dirs, [(PathBuf::from("/wiiu"), true)]);
        let y = rpcs3(
            "BLUS30443: D:/PS3/Demons Souls/\nNPUB30910: \"/home/u/.config/rpcs3/dev_hdd0/game/NPUB30910/\"\n# x: y\n",
        );
        assert_eq!(y.games.len(), 2);
        assert_eq!(y.games[0].0, PathBuf::from("D:/PS3/Demons Souls/"));
    }

    #[test]
    fn ps3_title_from_param_sfo() {
        // Cabeçalho + 1 entrada: chave "TITLE", valor "Jogo".
        let mut d = Vec::new();
        d.extend(b"\0PSF");
        d.extend(1u32.to_le_bytes()); // versão
        let keys = 20 + 16;
        d.extend((keys as u32).to_le_bytes());
        d.extend(((keys + 8) as u32).to_le_bytes());
        d.extend(1u32.to_le_bytes());
        d.extend(0u16.to_le_bytes()); // offset da chave
        d.extend(0x0204u16.to_le_bytes());
        d.extend(5u32.to_le_bytes()); // tamanho
        d.extend(8u32.to_le_bytes());
        d.extend(0u32.to_le_bytes()); // offset do valor
        d.extend(b"TITLE\0\0\0");
        d.extend(b"Jogo\0\0\0\0");
        assert_eq!(sfo_title(&d).as_deref(), Some("Jogo"));
        assert_eq!(sfo_title(b"lixo"), None);
        assert_eq!(sfo_title(&d[..30]), None);
    }
}
