//! Emuladores: onde cada um costuma estar em cada SO, onde guarda a configuração e o comando que abre uma ROM.
//!
//! A descoberta recebe o ambiente ([`Env`]) e uma função "existe?", para os testes valerem em qualquer SO.

use std::path::{Path, PathBuf};
use tako_core::paths::Os;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Emulator {
    RetroArch,
    Dolphin,
    Pcsx2,
    DuckStation,
    Ppsspp,
    Ryujinx,
    Cemu,
    Rpcs3,
}

impl Emulator {
    pub const ALL: [Emulator; 8] = [
        Emulator::RetroArch,
        Emulator::Dolphin,
        Emulator::Pcsx2,
        Emulator::DuckStation,
        Emulator::Ppsspp,
        Emulator::Ryujinx,
        Emulator::Cemu,
        Emulator::Rpcs3,
    ];

    /// Nome gravado nas configurações.
    pub fn key(self) -> &'static str {
        match self {
            Emulator::RetroArch => "retroarch",
            Emulator::Dolphin => "dolphin",
            Emulator::Pcsx2 => "pcsx2",
            Emulator::DuckStation => "duckstation",
            Emulator::Ppsspp => "ppsspp",
            Emulator::Ryujinx => "ryujinx",
            Emulator::Cemu => "cemu",
            Emulator::Rpcs3 => "rpcs3",
        }
    }

    pub fn from_key(key: &str) -> Option<Emulator> {
        Emulator::ALL
            .into_iter()
            .find(|e| e.key().eq_ignore_ascii_case(key.trim()))
    }

    pub fn name(self) -> &'static str {
        match self {
            Emulator::RetroArch => "RetroArch",
            Emulator::Dolphin => "Dolphin",
            Emulator::Pcsx2 => "PCSX2",
            Emulator::DuckStation => "DuckStation",
            Emulator::Ppsspp => "PPSSPP",
            Emulator::Ryujinx => "Ryujinx",
            Emulator::Cemu => "Cemu",
            Emulator::Rpcs3 => "RPCS3",
        }
    }

    /// Executáveis no Windows e no Linux (no macOS, ver [`Emulator::mac_app`]).
    fn exe_names(self, os: Os) -> &'static [&'static str] {
        match (self, os) {
            (Emulator::RetroArch, Os::Windows) => &["retroarch.exe"],
            (Emulator::RetroArch, _) => &["retroarch"],
            (Emulator::Dolphin, Os::Windows) => &["Dolphin.exe"],
            (Emulator::Dolphin, _) => &["dolphin-emu"],
            (Emulator::Pcsx2, Os::Windows) => &["pcsx2-qt.exe", "pcsx2.exe"],
            (Emulator::Pcsx2, _) => &["pcsx2-qt", "pcsx2"],
            (Emulator::DuckStation, Os::Windows) => {
                &["duckstation-qt-x64-ReleaseLTCG.exe", "duckstation-qt.exe"]
            }
            (Emulator::DuckStation, _) => &["duckstation-qt", "duckstation"],
            (Emulator::Ppsspp, Os::Windows) => &["PPSSPPWindows64.exe", "PPSSPPWindows.exe"],
            (Emulator::Ppsspp, _) => &["PPSSPPSDL", "ppsspp", "PPSSPPQt"],
            (Emulator::Ryujinx, Os::Windows) => &["Ryujinx.exe"],
            (Emulator::Ryujinx, _) => &["Ryujinx", "ryujinx"],
            (Emulator::Cemu, Os::Windows) => &["Cemu.exe"],
            (Emulator::Cemu, _) => &["cemu", "Cemu"],
            (Emulator::Rpcs3, Os::Windows) => &["rpcs3.exe"],
            (Emulator::Rpcs3, _) => &["rpcs3"],
        }
    }

    /// Pastas de instalação comuns no Windows (relativas às raízes de programas).
    fn windows_dirs(self) -> &'static [&'static str] {
        match self {
            Emulator::RetroArch => &["RetroArch-Win64", "RetroArch"],
            Emulator::Dolphin => &["Dolphin", "Dolphin-x64", "Dolphin Emulator"],
            Emulator::Pcsx2 => &["PCSX2"],
            Emulator::DuckStation => &["DuckStation"],
            Emulator::Ppsspp => &["PPSSPP"],
            Emulator::Ryujinx => &["Ryujinx"],
            Emulator::Cemu => &["Cemu"],
            Emulator::Rpcs3 => &["RPCS3"],
        }
    }

    /// `Contents/MacOS/<binário>` dentro do `.app`.
    fn mac_app(self) -> (&'static str, &'static str) {
        match self {
            Emulator::RetroArch => ("RetroArch.app", "RetroArch"),
            Emulator::Dolphin => ("Dolphin.app", "Dolphin"),
            Emulator::Pcsx2 => ("PCSX2.app", "PCSX2"),
            Emulator::DuckStation => ("DuckStation.app", "DuckStation"),
            Emulator::Ppsspp => ("PPSSPPSDL.app", "PPSSPPSDL"),
            Emulator::Ryujinx => ("Ryujinx.app", "Ryujinx"),
            Emulator::Cemu => ("Cemu.app", "Cemu"),
            Emulator::Rpcs3 => ("RPCS3.app", "rpcs3"),
        }
    }

    /// Ids do Flathub (Linux).
    pub fn flatpak_ids(self) -> &'static [&'static str] {
        match self {
            Emulator::RetroArch => &["org.libretro.RetroArch"],
            Emulator::Dolphin => &["org.DolphinEmu.dolphin-emu"],
            Emulator::Pcsx2 => &["net.pcsx2.PCSX2"],
            Emulator::DuckStation => &["org.duckstation.DuckStation"],
            Emulator::Ppsspp => &["org.ppsspp.PPSSPP"],
            Emulator::Ryujinx => &["io.github.ryubing.Ryujinx", "org.ryujinx.Ryujinx"],
            Emulator::Cemu => &["info.cemu.Cemu"],
            Emulator::Rpcs3 => &["net.rpcs3.RPCS3"],
        }
    }

    /// Pacote do Scoop (Windows).
    fn scoop(self) -> &'static str {
        self.key()
    }

    /// Argumentos que abrem `rom` em tela cheia, sem a janela de biblioteca do emulador. `core`: só RetroArch.
    pub fn args(self, rom: &str, core: Option<&str>) -> Vec<String> {
        let rom = rom.to_owned();
        match self {
            Emulator::RetroArch => match core {
                Some(c) => vec!["-f".into(), "-L".into(), c.to_owned(), rom],
                None => vec!["-f".into(), rom],
            },
            Emulator::Dolphin => vec!["-b".into(), "-e".into(), rom],
            Emulator::Pcsx2 | Emulator::DuckStation => {
                vec!["-batch".into(), "-fullscreen".into(), "--".into(), rom]
            }
            Emulator::Ppsspp | Emulator::Ryujinx => vec!["--fullscreen".into(), rom],
            Emulator::Cemu => vec!["-f".into(), "-g".into(), rom],
            Emulator::Rpcs3 => vec!["--no-gui".into(), rom],
        }
    }
}

/// O que o Tako precisa saber do ambiente para achar emuladores e configurações.
#[derive(Debug, Clone, Default)]
pub struct Env {
    pub os: Option<Os>,
    pub home: PathBuf,
    /// Variáveis (Windows: `APPDATA`, `LOCALAPPDATA`, `USERPROFILE`, `ProgramFiles`, `SystemDrive`, `OneDrive`;
    /// Linux: `XDG_CONFIG_HOME`, `XDG_DATA_HOME`).
    pub vars: Vec<(String, PathBuf)>,
    /// Pastas do `PATH`.
    pub path: Vec<PathBuf>,
    /// Raiz da Steam (o RetroArch também sai pela Steam).
    pub steam_root: Option<PathBuf>,
    /// Windows: `(nome, pasta)` dos programas instalados (chaves `Uninstall`).
    pub programs: Vec<(String, PathBuf)>,
}

impl Env {
    pub fn current() -> Env {
        let os = Os::current();
        let names = [
            "APPDATA",
            "LOCALAPPDATA",
            "USERPROFILE",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "SystemDrive",
            "OneDrive",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
        ];
        let vars = names
            .iter()
            .filter_map(|k| {
                let v = std::env::var_os(k)?;
                let p = PathBuf::from(&v);
                // `SystemDrive` vem sem a barra ("C:").
                let p = if *k == "SystemDrive" {
                    PathBuf::from(format!("{}\\", v.to_string_lossy()))
                } else {
                    p
                };
                Some(((*k).to_owned(), p))
            })
            .collect();
        let home = std::env::var_os(if os == Os::Windows {
            "USERPROFILE"
        } else {
            "HOME"
        })
        .map(PathBuf::from)
        .unwrap_or_default();
        Env {
            os: Some(os),
            home,
            vars,
            path: std::env::var_os("PATH")
                .map(|p| std::env::split_paths(&p).collect())
                .unwrap_or_default(),
            steam_root: crate::steam::find_root(),
            programs: installed_programs(),
        }
    }

    pub fn os(&self) -> Os {
        self.os.unwrap_or(Os::current())
    }

    pub fn var(&self, key: &str) -> Option<&Path> {
        self.vars
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_path())
            .filter(|p| !p.as_os_str().is_empty())
    }

    /// `~/.config` (ou `XDG_CONFIG_HOME`).
    pub fn config_home(&self) -> PathBuf {
        self.var("XDG_CONFIG_HOME")
            .filter(|p| p.has_root())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.home.join(".config"))
    }

    /// `~/.local/share` (ou `XDG_DATA_HOME`).
    pub fn data_home(&self) -> PathBuf {
        self.var("XDG_DATA_HOME")
            .filter(|p| p.has_root())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.home.join(".local").join("share"))
    }

    /// `~/Library/Application Support`.
    pub fn mac_support(&self) -> PathBuf {
        self.home.join("Library").join("Application Support")
    }

    /// Documentos do Windows (o OneDrive costuma levar a pasta).
    pub fn documents(&self) -> Vec<PathBuf> {
        let mut v = vec![self.home.join("Documents")];
        if let Some(od) = self.var("OneDrive") {
            v.push(od.join("Documents"));
        }
        v
    }

    /// `~/.var/app/<id>` de um Flatpak.
    pub fn flatpak_home(&self, id: &str) -> PathBuf {
        self.home.join(".var").join("app").join(id)
    }
}

#[cfg(windows)]
fn installed_programs() -> Vec<(String, PathBuf)> {
    let reg = crate::registry_windows::WindowsRegistry;
    crate::uninstall::read(&reg)
        .into_iter()
        .filter_map(|e| {
            let name = e.display_name?;
            let dir = e
                .install_location
                .filter(|l| !l.trim().is_empty())
                .map(|l| PathBuf::from(l.trim().trim_matches('"')))
                .or_else(|| {
                    let icon = e.display_icon?;
                    let icon = icon.split(',').next()?.trim().trim_matches('"').to_owned();
                    PathBuf::from(icon).parent().map(Path::to_path_buf)
                })?;
            Some((name, dir))
        })
        .collect()
}

#[cfg(not(windows))]
fn installed_programs() -> Vec<(String, PathBuf)> {
    Vec::new()
}

/// Como abrir o emulador: o executável e o que vem antes dos argumentos (Flatpak: `flatpak run <id>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub target: PathBuf,
    pub prefix: Vec<String>,
    /// Pasta do executável (configuração "portátil" mora ao lado dele).
    pub dir: Option<PathBuf>,
    pub flatpak: Option<String>,
}

impl Program {
    pub fn exe(path: PathBuf) -> Program {
        Program {
            dir: path.parent().map(Path::to_path_buf),
            target: path,
            prefix: Vec::new(),
            flatpak: None,
        }
    }
}

/// Onde procurar o executável, na ordem.
pub fn candidates(emu: Emulator, env: &Env) -> Vec<PathBuf> {
    let os = env.os();
    let names = emu.exe_names(os);
    let mut dirs: Vec<PathBuf> = Vec::new();
    match os {
        Os::Windows => {
            let mut roots: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)", "SystemDrive"]
                .iter()
                .filter_map(|k| env.var(k).map(Path::to_path_buf))
                .collect();
            if let Some(l) = env.var("LOCALAPPDATA") {
                roots.push(l.join("Programs"));
            }
            if let Some(a) = env.var("APPDATA") {
                roots.push(a.to_path_buf());
            }
            for r in &roots {
                for d in emu.windows_dirs() {
                    dirs.push(r.join(d));
                }
            }
            dirs.push(
                env.home
                    .join("scoop")
                    .join("apps")
                    .join(emu.scoop())
                    .join("current"),
            );
            if emu == Emulator::RetroArch
                && let Some(steam) = &env.steam_root
            {
                dirs.push(steam.join("steamapps").join("common").join("RetroArch"));
            }
            for (name, dir) in &env.programs {
                if name
                    .to_ascii_lowercase()
                    .contains(&emu.key().to_ascii_lowercase())
                {
                    dirs.push(dir.clone());
                }
            }
        }
        Os::Linux => {
            dirs.extend(env.path.iter().cloned());
            if emu == Emulator::RetroArch
                && let Some(steam) = &env.steam_root
            {
                dirs.push(steam.join("steamapps").join("common").join("RetroArch"));
            }
        }
        Os::MacOs => {
            let (app, bin) = emu.mac_app();
            return [
                PathBuf::from("/Applications"),
                env.home.join("Applications"),
            ]
            .into_iter()
            .map(|d| d.join(app).join("Contents").join("MacOS").join(bin))
            .chain(
                env.path
                    .iter()
                    .flat_map(|d| names.iter().map(move |n| d.join(n))),
            )
            .collect();
        }
    }
    dirs.iter()
        .flat_map(|d| names.iter().map(move |n| d.join(n)))
        .collect()
}

/// Acha o emulador: o caminho escolhido pelo usuário, os lugares comuns, AppImages (`~/Applications`) e Flatpak.
pub fn find(
    emu: Emulator,
    env: &Env,
    chosen: Option<&Path>,
    exists: &dyn Fn(&Path) -> bool,
    list_dir: &dyn Fn(&Path) -> Vec<PathBuf>,
) -> Option<Program> {
    if let Some(p) = chosen {
        return exists(p).then(|| Program::exe(p.to_path_buf()));
    }
    if let Some(p) = candidates(emu, env).into_iter().find(|p| exists(p)) {
        return Some(Program::exe(p));
    }
    if env.os() != Os::Linux {
        return None;
    }
    // AppImage com o nome do emulador ("DuckStation-x64.AppImage").
    let key = emu.key();
    for dir in [
        env.home.join("Applications"),
        env.home.join("AppImages"),
        env.home.join(".local").join("bin"),
    ] {
        let found = list_dir(&dir).into_iter().find(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
                .is_some_and(|n| n.starts_with(key) && n.ends_with(".appimage"))
        });
        if let Some(p) = found {
            return Some(Program::exe(p));
        }
    }
    let flatpak = env
        .path
        .iter()
        .map(|d| d.join("flatpak"))
        .chain([PathBuf::from("/usr/bin/flatpak")])
        .find(|p| exists(p))?;
    let id = emu.flatpak_ids().iter().find(|id| {
        exists(&env.flatpak_home(id))
            || exists(&Path::new("/var/lib/flatpak/app").join(id))
            || exists(&env.data_home().join("flatpak").join("app").join(id))
    })?;
    Some(Program {
        target: flatpak,
        prefix: vec!["run".into(), (*id).to_owned()],
        dir: None,
        flatpak: Some((*id).to_owned()),
    })
}

/// Um argumento na linha de comando do SO: no Windows, a linha vai inteira para o `CreateProcess`; nos outros, o
/// Tako separa como o shell (`split_args`).
pub fn quote(os: Os, arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg.chars().all(|c| {
            c.is_alphanumeric() || "-_./:=+,@".contains(c) || (os == Os::Windows && c == '\\')
        });
    if plain {
        return arg.to_owned();
    }
    let mut out = String::from("\"");
    if os == Os::Windows {
        // Regras do `CommandLineToArgvW`: barras antes de aspas dobram.
        let mut slashes = 0;
        for c in arg.chars() {
            match c {
                '\\' => slashes += 1,
                '"' => {
                    out.push_str(&"\\".repeat(slashes * 2 + 1));
                    out.push('"');
                    slashes = 0;
                }
                c => {
                    out.push_str(&"\\".repeat(slashes));
                    out.push(c);
                    slashes = 0;
                }
            }
        }
        out.push_str(&"\\".repeat(slashes * 2));
    } else {
        for c in arg.chars() {
            if matches!(c, '"' | '\\' | '$' | '`') {
                out.push('\\');
            }
            out.push(c);
        }
    }
    out.push('"');
    out
}

/// A linha de argumentos completa (prefixo do Flatpak e argumentos do emulador).
pub fn command_line(os: Os, program: &Program, args: &[String]) -> String {
    program
        .prefix
        .iter()
        .chain(args)
        .map(|a| quote(os, a))
        .collect::<Vec<_>>()
        .join(" ")
}
