//! Sistemas (consoles) que o Tako reconhece: id no padrão do ES-DE (o nome das pastas de ROMs mais comum),
//! extensões das ROMs, núcleos do RetroArch em ordem de preferência e emuladores avulsos.

use super::emulator::Emulator;

#[derive(Debug, PartialEq, Eq)]
pub struct System {
    /// Id do ES-DE ("snes", "psx"); vai no `Game::platform`.
    pub id: &'static str,
    pub name: &'static str,
    /// Extensões em minúsculas, sem ponto.
    pub extensions: &'static [&'static str],
    /// Núcleos do RetroArch (nome do arquivo sem `_libretro.<ext>`), do preferido ao último.
    pub cores: &'static [&'static str],
    /// Emuladores avulsos, preferidos ao RetroArch quando existem.
    pub standalone: &'static [Emulator],
    /// Outros nomes de pasta e de banco do RetroArch para o mesmo sistema.
    pub aliases: &'static [&'static str],
}

use Emulator::*;

pub const SYSTEMS: &[System] = &[
    System {
        id: "nes",
        name: "Nintendo Entertainment System",
        extensions: &["nes", "fds", "unf", "unif"],
        cores: &["mesen", "nestopia", "fceumm"],
        standalone: &[],
        aliases: &["famicom", "fds", "nintendo - nintendo entertainment system"],
    },
    System {
        id: "snes",
        name: "Super Nintendo",
        extensions: &["sfc", "smc", "fig", "swc", "bs"],
        cores: &["snes9x", "bsnes"],
        standalone: &[],
        aliases: &[
            "sfc",
            "superfamicom",
            "nintendo - super nintendo entertainment system",
        ],
    },
    System {
        id: "n64",
        name: "Nintendo 64",
        extensions: &["n64", "z64", "v64"],
        cores: &["mupen64plus_next", "parallel_n64"],
        standalone: &[],
        aliases: &["nintendo64", "nintendo - nintendo 64"],
    },
    System {
        id: "gb",
        name: "Game Boy",
        extensions: &["gb"],
        cores: &["gambatte", "sameboy", "mgba"],
        standalone: &[],
        aliases: &["gameboy", "nintendo - game boy"],
    },
    System {
        id: "gbc",
        name: "Game Boy Color",
        extensions: &["gbc"],
        cores: &["gambatte", "sameboy", "mgba"],
        standalone: &[],
        aliases: &["gameboycolor", "nintendo - game boy color"],
    },
    System {
        id: "gba",
        name: "Game Boy Advance",
        extensions: &["gba"],
        cores: &["mgba", "vba_next", "gpsp"],
        standalone: &[],
        aliases: &["gameboyadvance", "nintendo - game boy advance"],
    },
    System {
        id: "nds",
        name: "Nintendo DS",
        extensions: &["nds"],
        cores: &["melonds", "melondsds", "desmume"],
        standalone: &[],
        aliases: &["ds", "nintendo - nintendo ds"],
    },
    System {
        id: "n3ds",
        name: "Nintendo 3DS",
        extensions: &["3ds", "cci", "cxi"],
        cores: &["citra"],
        standalone: &[],
        aliases: &["3ds", "nintendo - nintendo 3ds"],
    },
    System {
        id: "gc",
        name: "GameCube",
        extensions: &["gcm", "iso", "rvz", "gcz", "ciso", "wia", "dol", "elf"],
        cores: &["dolphin"],
        standalone: &[Dolphin],
        aliases: &["gamecube", "ngc", "nintendo - gamecube"],
    },
    System {
        id: "wii",
        name: "Wii",
        extensions: &["wbfs", "iso", "rvz", "gcz", "wia", "wad", "ciso"],
        cores: &["dolphin"],
        standalone: &[Dolphin],
        aliases: &["nintendo - wii"],
    },
    System {
        id: "wiiu",
        name: "Wii U",
        extensions: &["wua", "wud", "wux", "rpx"],
        cores: &[],
        standalone: &[Cemu],
        aliases: &["nintendo - wii u"],
    },
    System {
        id: "switch",
        name: "Nintendo Switch",
        extensions: &["nsp", "xci", "nro"],
        cores: &[],
        standalone: &[Ryujinx],
        aliases: &["nintendoswitch", "nintendo - switch"],
    },
    System {
        id: "psx",
        name: "PlayStation",
        extensions: &[
            "cue", "chd", "pbp", "m3u", "iso", "img", "ecm", "mds", "ccd",
        ],
        cores: &[
            "swanstation",
            "mednafen_psx_hw",
            "mednafen_psx",
            "pcsx_rearmed",
        ],
        standalone: &[DuckStation],
        aliases: &["ps1", "playstation", "sony - playstation"],
    },
    System {
        id: "ps2",
        name: "PlayStation 2",
        extensions: &["iso", "chd", "cso", "zso", "gz", "cue", "elf"],
        cores: &["pcsx2"],
        standalone: &[Pcsx2],
        aliases: &["playstation2", "sony - playstation 2"],
    },
    System {
        id: "ps3",
        name: "PlayStation 3",
        extensions: &[],
        cores: &[],
        standalone: &[Rpcs3],
        aliases: &["playstation3", "sony - playstation 3"],
    },
    System {
        id: "psp",
        name: "PlayStation Portable",
        extensions: &["iso", "cso", "chd", "pbp"],
        cores: &["ppsspp"],
        standalone: &[Ppsspp],
        aliases: &["sony - playstation portable"],
    },
    System {
        id: "megadrive",
        name: "Mega Drive",
        extensions: &["md", "gen", "smd", "bin"],
        cores: &["genesis_plus_gx", "picodrive"],
        standalone: &[],
        aliases: &["genesis", "sega - mega drive - genesis"],
    },
    System {
        id: "mastersystem",
        name: "Master System",
        extensions: &["sms"],
        cores: &["genesis_plus_gx", "picodrive"],
        standalone: &[],
        aliases: &["sms", "sega - master system - mark iii"],
    },
    System {
        id: "gamegear",
        name: "Game Gear",
        extensions: &["gg"],
        cores: &["genesis_plus_gx"],
        standalone: &[],
        aliases: &["gg", "sega - game gear"],
    },
    System {
        id: "segacd",
        name: "Sega CD",
        extensions: &["cue", "chd", "iso", "m3u"],
        cores: &["genesis_plus_gx", "picodrive"],
        standalone: &[],
        aliases: &["megacd", "sega - mega-cd - sega cd"],
    },
    System {
        id: "sega32x",
        name: "Sega 32X",
        extensions: &["32x"],
        cores: &["picodrive"],
        standalone: &[],
        aliases: &["32x", "sega - 32x"],
    },
    System {
        id: "saturn",
        name: "Sega Saturn",
        extensions: &["cue", "chd", "ccd", "m3u"],
        cores: &["mednafen_saturn", "yabasanshiro"],
        standalone: &[],
        aliases: &["sega - saturn"],
    },
    System {
        id: "dreamcast",
        name: "Dreamcast",
        extensions: &["gdi", "chd", "cdi", "cue", "m3u"],
        cores: &["flycast"],
        standalone: &[],
        aliases: &["dc", "sega - dreamcast"],
    },
    System {
        id: "pcengine",
        name: "PC Engine",
        extensions: &["pce", "cue", "chd"],
        cores: &["mednafen_pce", "mednafen_pce_fast"],
        standalone: &[],
        aliases: &["tg16", "turbografx16", "nec - pc engine - turbografx 16"],
    },
    System {
        id: "atari2600",
        name: "Atari 2600",
        extensions: &["a26", "bin"],
        cores: &["stella"],
        standalone: &[],
        aliases: &["atari - 2600"],
    },
    System {
        id: "lynx",
        name: "Atari Lynx",
        extensions: &["lnx"],
        cores: &["handy", "mednafen_lynx"],
        standalone: &[],
        aliases: &["atari - lynx"],
    },
    System {
        id: "ngp",
        name: "Neo Geo Pocket",
        extensions: &["ngp", "ngc"],
        cores: &["mednafen_ngp"],
        standalone: &[],
        aliases: &["ngpc", "snk - neo geo pocket color"],
    },
    System {
        id: "wonderswan",
        name: "WonderSwan",
        extensions: &["ws", "wsc"],
        cores: &["mednafen_wswan"],
        standalone: &[],
        aliases: &["wonderswancolor", "bandai - wonderswan color"],
    },
    System {
        id: "virtualboy",
        name: "Virtual Boy",
        extensions: &["vb"],
        cores: &["mednafen_vb"],
        standalone: &[],
        aliases: &["nintendo - virtual boy"],
    },
    System {
        id: "msx",
        name: "MSX",
        extensions: &["rom", "mx1", "mx2", "dsk"],
        cores: &["bluemsx", "fmsx"],
        standalone: &[],
        aliases: &["microsoft - msx"],
    },
    System {
        id: "arcade",
        name: "Arcade",
        extensions: &["zip", "7z"],
        cores: &["fbneo", "mame"],
        standalone: &[],
        aliases: &[
            "mame",
            "fbneo",
            "fba",
            "fbneo - arcade games",
            "mame 2003-plus",
        ],
    },
    System {
        id: "neogeo",
        name: "Neo Geo",
        extensions: &["zip", "7z"],
        cores: &["fbneo"],
        standalone: &[],
        aliases: &["snk - neo geo"],
    },
];

pub fn by_id(id: &str) -> Option<&'static System> {
    SYSTEMS.iter().find(|s| s.id.eq_ignore_ascii_case(id))
}

/// Pelo nome da pasta (`~/ROMs/snes`, `Super Nintendo`) ou do banco do RetroArch
/// (`Nintendo - Super Nintendo Entertainment System.lpl`).
pub fn by_name(name: &str) -> Option<&'static System> {
    let name = name.trim();
    let name = name
        .strip_suffix(".lpl")
        .or_else(|| name.strip_suffix(".rdb"))
        .unwrap_or(name);
    let compact = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let want = compact(name);
    if want.is_empty() {
        return None;
    }
    SYSTEMS.iter().find(|s| {
        compact(s.id) == want
            || compact(s.name) == want
            || s.aliases.iter().any(|a| compact(a) == want)
    })
}

/// Sistemas com esta extensão (`iso` é de vários).
pub fn by_extension(ext: &str) -> impl Iterator<Item = &'static System> + '_ {
    SYSTEMS
        .iter()
        .filter(move |s| s.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

/// Chave de plataforma do Tako (a dos metadados: `igdb_parse::PLATFORMS`) para o id do sistema.
pub fn platform_key(id: &str) -> &str {
    match id {
        "gc" => "gamecube",
        "psx" => "ps1",
        "n3ds" => "3ds",
        "sega32x" => "32x",
        other => other,
    }
}
