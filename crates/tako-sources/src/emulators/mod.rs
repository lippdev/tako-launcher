//! Emuladores e ROMs (SRC-001), nos três SOs. Junta numa loja só ("Emuladores"):
//!
//! - as playlists do RetroArch, com o núcleo escolhido em cada jogo;
//! - a pasta de ROMs do ES-DE, com os nomes e capas do `gamelist.xml`;
//! - as pastas de jogos configuradas no Dolphin, PCSX2, DuckStation, PPSSPP, Ryujinx e Cemu, e a lista do RPCS3;
//! - as pastas de ROMs que o usuário adicionou no Tako (uma por sistema, ou uma raiz com uma subpasta por sistema).
//!
//! Cada ROM abre com o comando certo: o emulador escolhido para o sistema em Configurações, senão o emulador de
//! onde ela veio, senão o avulso preferido do sistema, senão o RetroArch com o primeiro núcleo instalado. ROM sem
//! emulador para abrir fica de fora e entra no aviso da loja. Os arquivos de configuração e as pastas são dados
//! do usuário: limites de tamanho, de profundidade e de quantidade, e nada de pânico em entrada quebrada.

pub mod emulator;
pub mod esde;
pub mod ini;
pub mod retroarch;
pub mod standalone;
pub mod systems;

use crate::{LibraryScan, LibrarySource, file_uri};
use emulator::{Emulator, Env, Program};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use systems::System;
use tako_core::i18n::{t, tf};
use tako_core::library::{Game, GameStore, GameTrackingStrategy};
use tako_core::paths::Os;

/// Chaves das configurações (JSON).
pub mod keys {
    pub const ROM_FOLDERS: &str = "EmulatorRomFolders";
    pub const PATHS: &str = "EmulatorPaths";
    pub const BY_SYSTEM: &str = "EmulatorBySystem";
}

/// Limites da varredura (pastas do usuário).
pub const MAX_DEPTH: usize = 6;
pub const MAX_ENTRIES: usize = 200_000;
pub const MAX_GAMES: usize = 50_000;
const MAX_CONFIG: u64 = 4 * 1024 * 1024;
/// Pastas de mídia e de sistema que não têm ROMs.
const SKIP_DIRS: [&str; 8] = [
    "media",
    "images",
    "videos",
    "manuals",
    "downloaded_media",
    "snap",
    "boxart",
    "screenshots",
];

type ListFn = Arc<dyn Fn(&Path) -> Vec<PathBuf> + Send + Sync>;
type ReadFn = Arc<dyn Fn(&Path, u64) -> Option<Vec<u8>> + Send + Sync>;
type SizeFn = Arc<dyn Fn(&Path) -> Option<u64> + Send + Sync>;

/// Acesso ao disco atrás de funções (os testes usam pastas temporárias ou caminhos de outro SO).
#[derive(Clone)]
pub struct Fs {
    pub exists: Arc<dyn Fn(&Path) -> bool + Send + Sync>,
    pub is_dir: Arc<dyn Fn(&Path) -> bool + Send + Sync>,
    /// Entradas da pasta (vazio se não existe).
    pub list_dir: ListFn,
    /// Até `limit` bytes do arquivo.
    pub read: ReadFn,
    pub size: SizeFn,
}

impl Fs {
    pub fn real() -> Fs {
        Fs {
            exists: Arc::new(|p| p.exists()),
            is_dir: Arc::new(|p| p.is_dir()),
            list_dir: Arc::new(|p| {
                std::fs::read_dir(p)
                    .map(|d| d.filter_map(Result::ok).map(|e| e.path()).collect())
                    .unwrap_or_default()
            }),
            read: Arc::new(|p, limit| crate::read_limited(p, limit).ok()),
            size: Arc::new(|p| {
                std::fs::metadata(p)
                    .ok()
                    .filter(|m| m.is_file())
                    .map(|m| m.len())
            }),
        }
    }

    pub fn read_text(&self, p: &Path) -> Option<String> {
        (self.read)(p, MAX_CONFIG).map(|b| crate::text_of(&b))
    }
}

/// De onde a ROM veio (para escolher o emulador e para o aviso).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    RetroArch,
    EsDe,
    Emulator(Emulator),
    Folder,
}

/// Como abrir: o sistema decide, um emulador avulso, ou o RetroArch com este núcleo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Via {
    Auto,
    Emulator(Emulator),
    Core(PathBuf),
}

/// Uma ROM achada.
#[derive(Debug, Clone)]
pub struct Found {
    pub path: PathBuf,
    pub title: Option<String>,
    pub system: Option<&'static System>,
    pub via: Via,
    pub cover: Option<PathBuf>,
    pub origin: Origin,
}

/// Pasta de ROMs adicionada no Tako; `system` vazio = uma subpasta por sistema (`~/ROMs/snes`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RomFolder {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// Configuração do usuário (Configurações › Biblioteca › Emuladores e ROMs).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub rom_folders: Vec<RomFolder>,
    /// Executável escolhido à mão.
    pub paths: BTreeMap<Emulator, PathBuf>,
    /// Emulador por sistema (id do sistema).
    pub by_system: BTreeMap<String, Emulator>,
}

impl Config {
    /// Valores gravados; quebrados viram vazio (com log).
    pub fn parse(folders: Option<&str>, paths: Option<&str>, by_system: Option<&str>) -> Config {
        fn json<T: for<'de> Deserialize<'de> + Default>(key: &str, v: Option<&str>) -> T {
            match v.filter(|s| !s.trim().is_empty() && s.len() <= 256 * 1024) {
                None => T::default(),
                Some(s) => serde_json::from_str(s).unwrap_or_else(|e| {
                    tracing::warn!("{key} inválido: {e}");
                    T::default()
                }),
            }
        }
        let rom_folders: Vec<RomFolder> = json(keys::ROM_FOLDERS, folders);
        let paths: BTreeMap<String, PathBuf> = json(keys::PATHS, paths);
        let by_system: BTreeMap<String, String> = json(keys::BY_SYSTEM, by_system);
        Config {
            rom_folders: rom_folders
                .into_iter()
                .filter(|f| !f.path.as_os_str().is_empty())
                .take(256)
                .map(|f| RomFolder {
                    system: f.system.filter(|s| systems::by_id(s).is_some()),
                    path: f.path,
                })
                .collect(),
            paths: paths
                .into_iter()
                .filter_map(|(k, v)| Some((Emulator::from_key(&k)?, v)))
                .filter(|(_, v)| !v.as_os_str().is_empty())
                .collect(),
            by_system: by_system
                .into_iter()
                .filter(|(s, _)| systems::by_id(s).is_some())
                .filter_map(|(s, e)| Some((s, Emulator::from_key(&e)?)))
                .collect(),
        }
    }

    /// `(chave, valor)` a gravar.
    pub fn save(&self) -> [(&'static str, String); 3] {
        let paths: BTreeMap<&str, &PathBuf> =
            self.paths.iter().map(|(e, p)| (e.key(), p)).collect();
        let by_system: BTreeMap<&str, &str> = self
            .by_system
            .iter()
            .map(|(s, e)| (s.as_str(), e.key()))
            .collect();
        let j = |v: serde_json::Result<String>| v.unwrap_or_default();
        [
            (
                keys::ROM_FOLDERS,
                j(serde_json::to_string(&self.rom_folders)),
            ),
            (keys::PATHS, j(serde_json::to_string(&paths))),
            (keys::BY_SYSTEM, j(serde_json::to_string(&by_system))),
        ]
    }
}

/// Emuladores achados neste PC e a pasta de núcleos do RetroArch.
#[derive(Debug, Clone, Default)]
pub struct Installed {
    pub programs: BTreeMap<Emulator, Program>,
    pub retroarch: Option<retroarch::Dirs>,
}

impl Installed {
    pub fn detect(env: &Env, config: &Config, fs: &Fs) -> Installed {
        let exists = |p: &Path| (fs.exists)(p);
        let list = |p: &Path| (fs.list_dir)(p);
        let programs: BTreeMap<Emulator, Program> = Emulator::ALL
            .into_iter()
            .filter_map(|e| {
                let p = emulator::find(
                    e,
                    env,
                    config.paths.get(&e).map(PathBuf::as_path),
                    &exists,
                    &list,
                )?;
                Some((e, p))
            })
            .collect();
        let exe_dir = programs
            .get(&Emulator::RetroArch)
            .and_then(|p| p.dir.clone());
        let retroarch = retroarch::config_dirs(env, exe_dir.as_deref())
            .into_iter()
            .find_map(|d| {
                let cfg = fs.read_text(&d.join("retroarch.cfg"))?;
                Some(retroarch::read_dirs(
                    &cfg,
                    &d,
                    exe_dir.as_deref(),
                    &env.home,
                ))
            })
            .or_else(|| {
                // Sem `retroarch.cfg` ainda: as pastas padrão, ao lado do executável.
                exe_dir.as_ref().map(|d| retroarch::Dirs {
                    playlists: d.join("playlists"),
                    cores: d.join("cores"),
                    thumbnails: d.join("thumbnails"),
                })
            });
        Installed {
            programs,
            retroarch,
        }
    }

    /// O RetroArch tem algum núcleo do sistema.
    pub fn has_core(&self, system: &System, os: Os, fs: &Fs) -> bool {
        self.core_for(system, os, fs).is_some()
    }

    /// Primeiro núcleo do sistema que está instalado.
    fn core_for(&self, system: &System, os: Os, fs: &Fs) -> Option<PathBuf> {
        let dirs = self.retroarch.as_ref()?;
        system
            .cores
            .iter()
            .map(|c| retroarch::core_file(&dirs.cores, c, os))
            .find(|p| (fs.exists)(p))
    }
}

/// O comando que abre a ROM: emulador, executável e argumentos.
pub fn launcher(
    found: &Found,
    installed: &Installed,
    config: &Config,
    os: Os,
    fs: &Fs,
) -> Option<(Emulator, Program, Vec<String>)> {
    let rom = found.path.to_string_lossy().into_owned();
    let with = |e: Emulator, core: Option<PathBuf>| {
        let program = installed.programs.get(&e)?.clone();
        if e == Emulator::RetroArch && core.is_none() {
            return None;
        }
        let args = e.args(&rom, core.as_ref().map(|c| c.to_string_lossy()).as_deref());
        Some((e, program, args))
    };
    let ra_core = |s: &System| installed.core_for(s, os, fs);
    // 1. Escolhido pelo usuário para o sistema.
    if let Some(s) = found.system
        && let Some(&e) = config.by_system.get(s.id)
    {
        let core = (e == Emulator::RetroArch).then(|| ra_core(s)).flatten();
        if let Some(l) = with(e, core) {
            return Some(l);
        }
    }
    // 2. O emulador de onde a ROM veio.
    match &found.via {
        Via::Core(core) if (fs.exists)(core) => {
            if let Some(l) = with(Emulator::RetroArch, Some(core.clone())) {
                return Some(l);
            }
        }
        Via::Emulator(e) => {
            if let Some(l) = with(*e, None) {
                return Some(l);
            }
        }
        _ => {}
    }
    // 3. Avulso preferido do sistema, depois o RetroArch.
    let s = found.system?;
    s.standalone
        .iter()
        .find_map(|e| with(*e, None))
        .or_else(|| with(Emulator::RetroArch, ra_core(s)))
}

/// Nome de arquivo sem as marcas de região e versão: "Super Mario World (USA) [!]" → "Super Mario World".
pub fn clean_title(stem: &str) -> String {
    let mut s = stem.replace('_', " ");
    loop {
        let trimmed = s.trim_end();
        let cut = match trimmed.chars().last() {
            Some(')') => trimmed.rfind('('),
            Some(']') => trimmed.rfind('['),
            _ => None,
        };
        match cut {
            Some(i) if i > 0 => s.truncate(i),
            _ => break,
        }
    }
    let t = s.trim();
    if t.is_empty() {
        stem.trim().to_owned()
    } else {
        t.to_owned()
    }
}

/// GameCube ou Wii pelo cabeçalho do disco (ISO: as "palavras mágicas"; RVZ/WIA: o tipo do disco).
fn dolphin_system(path: &Path, fs: &Fs) -> &'static System {
    let ext = extension(path);
    let wii = systems::by_id("wii");
    let gc = systems::by_id("gc");
    let pick = |is_wii: bool| if is_wii { wii } else { gc };
    let found = match ext.as_str() {
        "wbfs" | "wad" => wii,
        "iso" | "gcm" => (fs.read)(path, 0x20).and_then(|h| {
            let be = |o: usize| {
                h.get(o..o + 4)
                    .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            };
            match (be(0x18), be(0x1C)) {
                (Some(0x5D1C_9EA3), _) => wii,
                (_, Some(0xC233_9F3D)) => gc,
                _ => None,
            }
        }),
        "rvz" | "wia" => (fs.read)(path, 0x4C).and_then(|h| {
            h.get(0x48..0x4C)
                .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                .and_then(|t| pick(t == 2))
        }),
        _ => None,
    };
    found.or(gc).unwrap_or(&systems::SYSTEMS[0])
}

pub fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// Arquivos que um `.m3u`, `.cue`, `.gdi` ou `.ccd` cita (os discos e faixas não aparecem sozinhos).
fn referenced(path: &Path, fs: &Fs) -> Vec<String> {
    let ext = extension(path);
    let Some(text) = (fs.read)(path, 256 * 1024).map(|b| crate::text_of(&b)) else {
        return Vec::new();
    };
    let mut out: Vec<String> = match ext.as_str() {
        "m3u" => text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_owned())
            .collect(),
        "cue" => text
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                let rest = l
                    .strip_prefix("FILE ")
                    .or_else(|| l.strip_prefix("file "))?;
                let name = match rest.strip_prefix('"') {
                    Some(q) => q.split('"').next()?,
                    None => rest.split_whitespace().next()?,
                };
                Some(name.to_owned())
            })
            .collect(),
        "gdi" => text
            .lines()
            .skip(1)
            .filter_map(|l| {
                let l = l.trim();
                // "1 0 4 2352 track01.bin 0" ou com o nome entre aspas.
                if let Some(start) = l.find('"') {
                    return l[start + 1..].split('"').next().map(str::to_owned);
                }
                l.split_whitespace().nth(4).map(str::to_owned)
            })
            .collect(),
        "ccd" => {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            vec![format!("{stem}.img"), format!("{stem}.sub")]
        }
        _ => Vec::new(),
    };
    for r in &mut out {
        // Só o nome: as faixas ficam na mesma pasta (ou numa subpasta citada pelo m3u).
        *r = r.replace('\\', "/").to_lowercase();
    }
    out
}

/// Varre `dir` atrás de ROMs de `systems_here`, até `depth` níveis. Recursiva: os argumentos são o estado da
/// varredura (origem, comando, disco, entradas que ainda dá para ler e o resultado).
#[allow(clippy::too_many_arguments)]
fn walk(
    dir: &Path,
    systems_here: &[&'static System],
    recursive: bool,
    origin: Origin,
    via: &Via,
    fs: &Fs,
    budget: &mut usize,
    out: &mut Vec<Found>,
    depth: usize,
) {
    let entries = (fs.list_dir)(dir);
    if *budget < entries.len() {
        tracing::warn!("pasta de ROMs grande demais: parei em {}", dir.display());
        *budget = 0;
        return;
    }
    *budget -= entries.len();
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for e in entries {
        let name = e
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name.starts_with('.') {
            continue;
        }
        if (fs.is_dir)(&e) {
            dirs.push(e);
        } else {
            files.push(e);
        }
    }
    // Wii U (pasta com code/*.rpx) e PS3 (PS3_GAME/USRDIR/EBOOT.BIN) são pastas, não arquivos.
    let wants = |id: &str| systems_here.iter().any(|s| s.id == id);
    if wants("ps3") {
        for base in [dir.join("PS3_GAME").join("USRDIR"), dir.join("USRDIR")] {
            let eboot = base.join("EBOOT.BIN");
            if (fs.exists)(&eboot) {
                out.push(folder_game(eboot, dir, "ps3", origin, via, fs));
                return;
            }
        }
    }
    if wants("wiiu") {
        let code = dir.join("code");
        if let Some(rpx) = (fs.list_dir)(&code)
            .into_iter()
            .find(|p| extension(p) == "rpx")
        {
            out.push(folder_game(rpx, dir, "wiiu", origin, via, fs));
            return;
        }
    }
    let lists: HashSet<String> = files
        .iter()
        .filter(|f| matches!(extension(f).as_str(), "m3u" | "cue" | "gdi" | "ccd"))
        .flat_map(|f| referenced(f, fs))
        .collect();
    for f in files {
        let ext = extension(&f);
        let name = f
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if lists.contains(&name) {
            continue;
        }
        let Some(system) = systems_here
            .iter()
            .find(|s| s.extensions.contains(&ext.as_str()))
        else {
            continue;
        };
        // `rpx` solto só conta dentro de `code/` (acima).
        if ext == "rpx" {
            continue;
        }
        let system = if matches!(system.id, "gc" | "wii") && systems_here.len() > 1 {
            dolphin_system(&f, fs)
        } else {
            system
        };
        out.push(Found {
            title: f.file_stem().map(|s| clean_title(&s.to_string_lossy())),
            path: f,
            system: Some(system),
            via: via.clone(),
            cover: None,
            origin,
        });
    }
    if recursive && depth > 0 {
        for d in dirs {
            let name = d
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            if SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk(
                &d,
                systems_here,
                true,
                origin,
                via,
                fs,
                budget,
                out,
                depth - 1,
            );
        }
    }
}

fn folder_game(
    file: PathBuf,
    dir: &Path,
    system: &str,
    origin: Origin,
    via: &Via,
    fs: &Fs,
) -> Found {
    let sfo = dir.join("PS3_GAME").join("PARAM.SFO");
    let title = (system == "ps3")
        .then(|| {
            (fs.read)(&sfo, 64 * 1024).or_else(|| (fs.read)(&dir.join("PARAM.SFO"), 64 * 1024))
        })
        .flatten()
        .and_then(|d| standalone::sfo_title(&d))
        .or_else(|| dir.file_name().map(|n| clean_title(&n.to_string_lossy())));
    Found {
        path: file,
        title,
        system: systems::by_id(system),
        via: via.clone(),
        cover: None,
        origin,
    }
}

/// ROMs de uma pasta do usuário: de um sistema só, ou uma subpasta por sistema.
pub fn scan_folder(folder: &RomFolder, origin: Origin, fs: &Fs, budget: &mut usize) -> Vec<Found> {
    let mut out = Vec::new();
    let assigned = folder
        .system
        .as_deref()
        .and_then(systems::by_id)
        .or_else(|| {
            folder
                .path
                .file_name()
                .and_then(|n| systems::by_name(&n.to_string_lossy()))
        });
    match assigned {
        Some(s) => walk(
            &folder.path,
            &[s],
            true,
            origin,
            &Via::Auto,
            fs,
            budget,
            &mut out,
            MAX_DEPTH,
        ),
        None => {
            for sub in (fs.list_dir)(&folder.path) {
                let Some(s) = sub
                    .file_name()
                    .and_then(|n| systems::by_name(&n.to_string_lossy()))
                else {
                    continue;
                };
                if (fs.is_dir)(&sub) {
                    walk(
                        &sub,
                        &[s],
                        true,
                        origin,
                        &Via::Auto,
                        fs,
                        budget,
                        &mut out,
                        MAX_DEPTH,
                    );
                }
            }
        }
    }
    out
}

/// ROMs das bibliotecas dos emuladores avulsos.
fn standalone_games(env: &Env, installed: &Installed, fs: &Fs, budget: &mut usize) -> Vec<Found> {
    let mut out = Vec::new();
    for emu in Emulator::ALL
        .into_iter()
        .filter(|e| *e != Emulator::RetroArch)
    {
        let exe_dir = installed.programs.get(&emu).and_then(|p| p.dir.clone());
        let Some(lib) = standalone::config_files(emu, env, exe_dir.as_deref())
            .into_iter()
            .find_map(|f| fs.read_text(&f))
            .map(|text| standalone::parse(emu, &text))
        else {
            continue;
        };
        let here: Vec<&'static System> = systems::SYSTEMS
            .iter()
            .filter(|s| s.standalone.contains(&emu))
            .collect();
        let via = Via::Emulator(emu);
        for (dir, recursive) in &lib.dirs {
            walk(
                dir,
                &here,
                *recursive,
                Origin::Emulator(emu),
                &via,
                fs,
                budget,
                &mut out,
                MAX_DEPTH,
            );
        }
        for (path, title) in &lib.games {
            if emu == Emulator::Rpcs3 {
                walk(
                    path,
                    &here,
                    false,
                    Origin::Emulator(emu),
                    &via,
                    fs,
                    budget,
                    &mut out,
                    0,
                );
            } else if (fs.exists)(path) {
                let ext = extension(path);
                let Some(system) = here.iter().find(|s| s.extensions.contains(&ext.as_str()))
                else {
                    continue;
                };
                out.push(Found {
                    title: title
                        .clone()
                        .or_else(|| path.file_stem().map(|s| clean_title(&s.to_string_lossy()))),
                    path: path.clone(),
                    system: Some(system),
                    via: via.clone(),
                    cover: None,
                    origin: Origin::Emulator(emu),
                });
            }
        }
    }
    out
}

/// Id estável pelo caminho da ROM.
pub fn game_id(path: &Path) -> String {
    let hash = Sha256::digest(esde::key_of(path).as_bytes());
    let hex: String = hash.iter().take(8).map(|b| format!("{b:02x}")).collect();
    format!("emulator:{hex}")
}

fn accent(system: Option<&System>) -> &'static str {
    match system.map(|s| s.id) {
        Some(
            "nes" | "snes" | "n64" | "gb" | "gbc" | "gba" | "nds" | "n3ds" | "switch"
            | "virtualboy",
        ) => "#8B1E2B",
        Some("gc" | "wii" | "wiiu") => "#4B3A8C",
        Some("psx" | "ps2" | "ps3" | "psp") => "#1F3F8F",
        Some(
            "megadrive" | "mastersystem" | "gamegear" | "segacd" | "sega32x" | "saturn"
            | "dreamcast",
        ) => "#1C4E80",
        _ => "#3A3F4B",
    }
}

/// Lista final: tira repetidos (a primeira origem ganha; nome e capa do ES-DE completam), escolhe o comando e
/// monta os jogos. Devolve também as ROMs sem emulador, por sistema.
pub fn build(
    found: Vec<Found>,
    esde: &esde::Library,
    installed: &Installed,
    config: &Config,
    os: Os,
    fs: &Fs,
) -> (Vec<Game>, BTreeMap<String, usize>) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut unique: Vec<Found> = Vec::new();
    for f in found {
        let key = esde::key_of(&f.path);
        match seen.get(&key) {
            Some(&i) => {
                let u = &mut unique[i];
                if u.cover.is_none() {
                    u.cover = f.cover;
                }
                if u.system.is_none() {
                    u.system = f.system;
                }
            }
            None => {
                seen.insert(key, unique.len());
                unique.push(f);
            }
        }
    }
    let mut games = Vec::new();
    let mut missing: BTreeMap<String, usize> = BTreeMap::new();
    for mut f in unique.into_iter().take(MAX_GAMES) {
        if let Some((entry, cover)) = esde.entries.get(&esde::key_of(&f.path)) {
            if entry.hidden {
                continue;
            }
            if let Some(n) = &entry.name {
                f.title = Some(n.clone());
            }
            if f.cover.is_none() {
                f.cover = cover.clone();
            }
        }
        let Some((_, program, args)) = launcher(&f, installed, config, os, fs) else {
            let name = f.system.map_or("?", |s| s.name);
            *missing.entry(name.to_owned()).or_default() += 1;
            continue;
        };
        let title = f.title.clone().unwrap_or_else(|| {
            f.path
                .file_stem()
                .map(|s| clean_title(&s.to_string_lossy()))
                .unwrap_or_default()
        });
        let size = (fs.size)(&f.path);
        games.push(Game {
            id: game_id(&f.path),
            title,
            store: Some(GameStore::Emulators),
            size_bytes: size,
            size_label: size.and_then(crate::steam::size_label),
            accent: accent(f.system).to_owned(),
            cover_uri: f.cover.as_deref().map(file_uri),
            has_fallback_art: true,
            is_installed: true,
            launch_target: Some(program.target.to_string_lossy().into_owned()),
            launch_arguments: Some(emulator::command_line(os, &program, &args)),
            working_directory: program
                .dir
                .as_ref()
                .map(|d| d.to_string_lossy().into_owned()),
            tracking: Some(GameTrackingStrategy::Process),
            platform: f.system.map(|s| systems::platform_key(s.id).to_owned()),
            rom_path: Some(f.path.to_string_lossy().into_owned()),
            ..Default::default()
        });
    }
    (games, missing)
}

/// Comando para uma ROM adicionada à mão (`platform` = a chave gravada nela): `(executável, argumentos, pasta)`.
/// `None` = nenhum emulador deste PC abre o sistema.
pub fn command_for(
    rom: &Path,
    platform: Option<&str>,
    config: &Config,
) -> Option<(String, String, Option<String>)> {
    let env = Env::current();
    let fs = Fs::real();
    let installed = Installed::detect(&env, config, &fs);
    let system = platform
        .and_then(systems::by_name)
        .or_else(|| systems::by_extension(&extension(rom)).next());
    let found = Found {
        path: rom.to_path_buf(),
        title: None,
        system,
        via: Via::Auto,
        cover: None,
        origin: Origin::Folder,
    };
    let (_, program, args) = launcher(&found, &installed, config, env.os(), &fs)?;
    Some((
        program.target.to_string_lossy().into_owned(),
        emulator::command_line(env.os(), &program, &args),
        program.dir.map(|d| d.to_string_lossy().into_owned()),
    ))
}

/// A loja "Emuladores".
pub struct EmulatorSource {
    /// `None` = o ambiente de verdade, lido só na varredura (ler o registro e a Steam custa).
    pub env: Option<Env>,
    pub config: Config,
    pub fs: Fs,
}

impl EmulatorSource {
    pub fn new(config: Config) -> EmulatorSource {
        EmulatorSource {
            env: None,
            config,
            fs: Fs::real(),
        }
    }

    pub fn env(&self) -> Env {
        self.env.clone().unwrap_or_else(Env::current)
    }

    pub fn installed(&self, env: &Env) -> Installed {
        Installed::detect(env, &self.config, &self.fs)
    }

    /// Tudo o que foi achado, antes de escolher o comando.
    pub fn found(&self, env: &Env, installed: &Installed, esde: &esde::Library) -> Vec<Found> {
        let fs = &self.fs;
        let mut budget = MAX_ENTRIES;
        let mut found = Vec::new();
        if let Some(dirs) = &installed.retroarch {
            let exists = |p: &Path| (fs.exists)(p);
            let read = |p: &Path| fs.read_text(p);
            let list = |p: &Path| (fs.list_dir)(p);
            found.extend(retroarch::read(dirs, &exists, &read, &list));
        }
        found.extend(standalone_games(env, installed, fs, &mut budget));
        if let Some(dir) = &esde.rom_dir {
            let folder = RomFolder {
                path: dir.clone(),
                system: None,
            };
            found.extend(scan_folder(&folder, Origin::EsDe, fs, &mut budget));
        }
        for folder in &self.config.rom_folders {
            found.extend(scan_folder(folder, Origin::Folder, fs, &mut budget));
        }
        found
    }
}

impl LibrarySource for EmulatorSource {
    fn store(&self) -> GameStore {
        GameStore::Emulators
    }

    fn scan(&self) -> LibraryScan {
        let env = self.env();
        let installed = self.installed(&env);
        let esde = esde::read(&env, &self.fs);
        let found = self.found(&env, &installed, &esde);
        let (games, missing) = build(found, &esde, &installed, &self.config, env.os(), &self.fs);
        let client_installed = !installed.programs.is_empty();
        let notice = if !client_installed && games.is_empty() {
            Some(t("sources.emulators.none"))
        } else if !missing.is_empty() {
            let count: usize = missing.values().sum();
            let systems: Vec<&str> = missing.keys().map(String::as_str).collect();
            Some(tf(
                "sources.emulators.no_emulator",
                &[
                    ("count", &count.to_string()),
                    ("systems", &systems.join(", ")),
                ],
            ))
        } else {
            None
        };
        tracing::info!(
            jogos = games.len(),
            emuladores = installed.programs.len(),
            sem_emulador = missing.values().sum::<usize>(),
            "emuladores"
        );
        LibraryScan {
            games,
            client_installed,
            notice,
        }
    }
}

#[cfg(test)]
mod tests;
