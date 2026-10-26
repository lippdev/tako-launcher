#![allow(clippy::unwrap_used)]

use super::emulator::{Emulator, Env, Program, command_line, quote};
use super::*;
use std::fs;
use std::path::{Path, PathBuf};

fn write(path: &Path, content: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Um Linux de mentira dentro de uma pasta temporária (vale em qualquer SO).
fn linux(home: &Path) -> Env {
    Env {
        os: Some(Os::Linux),
        home: home.to_path_buf(),
        vars: Vec::new(),
        path: vec![home.join("bin")],
        steam_root: None,
        programs: Vec::new(),
    }
}

fn titles(games: &[Game]) -> Vec<String> {
    let mut v: Vec<String> = games.iter().map(|g| g.title.clone()).collect();
    v.sort();
    v
}

fn find<'a>(games: &'a [Game], title: &str) -> &'a Game {
    games
        .iter()
        .find(|g| g.title == title)
        .unwrap_or_else(|| panic!("sem {title}"))
}

#[test]
fn imports_from_retroarch_standalone_esde_and_user_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let h = tmp.path();
    // RetroArch no PATH, com o snes9x instalado e uma playlist de SNES.
    write(&h.join("bin/retroarch"), b"");
    let cfg = h.join(".config/retroarch");
    write(
        &cfg.join("retroarch.cfg"),
        b"libretro_directory = \"~/cores\"\n",
    );
    write(&h.join("cores/snes9x_libretro.so"), b"");
    write(&h.join("roms/snes/Chrono Trigger (USA).sfc"), b"rom");
    let playlist = format!(
        r#"{{"default_core_path":"","items":[
            {{"path":"{}","label":"Chrono Trigger","core_path":"{}"}},
            {{"path":"{}","label":"Sumiu","core_path":"DETECT"}}]}}"#,
        h.join("roms/snes/Chrono Trigger (USA).sfc")
            .to_string_lossy()
            .replace('\\', "\\\\"),
        h.join("cores/snes9x_libretro.so")
            .to_string_lossy()
            .replace('\\', "\\\\"),
        h.join("roms/snes/nao-existe.sfc")
            .to_string_lossy()
            .replace('\\', "\\\\"),
    );
    write(
        &cfg.join("playlists/Nintendo - Super Nintendo Entertainment System.lpl"),
        playlist.as_bytes(),
    );
    write(&cfg.join("thumbnails/Nintendo - Super Nintendo Entertainment System/Named_Boxarts/Chrono Trigger.png"), b"png");
    // DuckStation com uma pasta de jogos: o .bin do .cue não aparece sozinho.
    write(&h.join("bin/duckstation-qt"), b"");
    let psx = h.join("psx");
    write(
        &h.join(".local/share/duckstation/settings.ini"),
        format!("[GameList]\nRecursivePaths = {}\n", psx.display()).as_bytes(),
    );
    write(
        &psx.join("Vagrant Story (USA).cue"),
        b"FILE \"Vagrant Story (USA).bin\" BINARY\n",
    );
    write(&psx.join("Vagrant Story (USA).bin"), b"bin");
    // ES-DE: a pasta de ROMs dele, com o nome do gamelist.
    write(
        &h.join("ES-DE/settings/es_settings.xml"),
        b"<?xml version=\"1.0\"?>\n<string name=\"ROMDirectory\" value=\"~/ROMs\" />\n",
    );
    write(&h.join("ROMs/psx/ff7.chd"), b"chd");
    write(&h.join("ROMs/psx/escondido.chd"), b"chd");
    write(
        &h.join("ES-DE/gamelists/psx/gamelist.xml"),
        b"<gameList><game><path>./ff7.chd</path><name>Final Fantasy VII</name></game><game><path>./escondido.chd</path><hidden>true</hidden></game></gameList>",
    );
    write(&h.join("ES-DE/downloaded_media/psx/covers/ff7.png"), b"png");
    // Pasta do usuário com uma subpasta por sistema; N64 e GameCube sem emulador.
    write(
        &h.join("Meus/snes/Super Metroid (Japan, USA) [!].sfc"),
        b"rom",
    );
    write(&h.join("Meus/n64/Zelda.z64"), b"rom");
    write(&h.join("Meus/gc/Melee.iso"), b"iso");
    write(&h.join("Meus/snes/media/capa.png"), b"png");

    let config = Config {
        rom_folders: vec![RomFolder {
            path: h.join("Meus"),
            system: None,
        }],
        ..Default::default()
    };
    let source = EmulatorSource {
        env: Some(linux(h)),
        config,
        fs: Fs::real(),
    };
    let scan = source.scan();
    let games = &scan.games;
    assert_eq!(
        titles(games),
        [
            "Chrono Trigger",
            "Final Fantasy VII",
            "Super Metroid",
            "Vagrant Story"
        ]
    );
    assert!(scan.client_installed);
    let notice = scan.notice.unwrap();
    assert!(
        notice.contains('2') && notice.contains("Nintendo 64") && notice.contains("GameCube"),
        "{notice}"
    );

    let ct = find(games, "Chrono Trigger");
    assert_eq!(ct.store, Some(GameStore::Emulators));
    assert_eq!(ct.platform.as_deref(), Some("snes"));
    assert!(ct.launch_target.as_deref().unwrap().ends_with("retroarch"));
    let args = ct.launch_arguments.as_deref().unwrap();
    assert!(
        args.starts_with("-f -L ") && args.contains("snes9x_libretro.so"),
        "{args}"
    );
    assert!(
        ct.cover_uri
            .as_deref()
            .unwrap()
            .ends_with("Chrono%20Trigger.png")
    );
    assert_eq!(ct.size_bytes, Some(3));

    // Da pasta do usuário: sem núcleo escolhido, o primeiro instalado do sistema.
    let sm = find(games, "Super Metroid");
    assert!(
        sm.launch_arguments
            .as_deref()
            .unwrap()
            .contains("snes9x_libretro.so")
    );
    // PlayStation: o avulso preferido (DuckStation), também para o que veio do ES-DE.
    let vs = find(games, "Vagrant Story");
    assert!(
        vs.launch_target
            .as_deref()
            .unwrap()
            .ends_with("duckstation-qt")
    );
    assert!(
        vs.launch_arguments
            .as_deref()
            .unwrap()
            .starts_with("-batch -fullscreen -- ")
    );
    assert!(vs.rom_path.as_deref().unwrap().ends_with(".cue"));
    assert_eq!(vs.platform.as_deref(), Some("ps1"));
    let ff = find(games, "Final Fantasy VII");
    assert!(
        ff.launch_target
            .as_deref()
            .unwrap()
            .ends_with("duckstation-qt")
    );
    assert!(ff.cover_uri.as_deref().unwrap().ends_with("ff7.png"));
    // Ids estáveis e únicos.
    let ids: HashSet<_> = games.iter().map(|g| g.id.clone()).collect();
    assert_eq!(ids.len(), games.len());
    assert_eq!(
        ct.id,
        game_id(&h.join("roms/snes/Chrono Trigger (USA).sfc"))
    );

    // Escolha por sistema: PlayStation pelo RetroArch (com núcleo instalado).
    write(&h.join("cores/swanstation_libretro.so"), b"");
    let mut config = source.config.clone();
    config.by_system.insert("psx".into(), Emulator::RetroArch);
    let source = EmulatorSource { config, ..source };
    let games = source.scan().games;
    let vs = find(&games, "Vagrant Story");
    assert!(vs.launch_target.as_deref().unwrap().ends_with("retroarch"));
    assert!(
        vs.launch_arguments
            .as_deref()
            .unwrap()
            .contains("swanstation_libretro.so")
    );
}

#[test]
fn the_same_file_by_two_paths_is_one_game() {
    // `x/../Meus` faz o papel do `/var` → `/private/var` e do `ROMs` x `roms` do macOS: outro caminho, mesmo arquivo.
    let tmp = tempfile::tempdir().unwrap();
    let h = tmp.path();
    write(&h.join("bin/retroarch"), b"");
    write(
        &h.join(".config/retroarch/retroarch.cfg"),
        b"libretro_directory = \"~/cores\"\n",
    );
    write(&h.join("cores/snes9x_libretro.so"), b"");
    write(&h.join("Meus/snes/Chrono Trigger (USA).sfc"), b"rom");
    fs::create_dir_all(h.join("x")).unwrap();
    let folder = |path: PathBuf| RomFolder { path, system: None };
    let source = EmulatorSource {
        env: Some(linux(h)),
        config: Config {
            rom_folders: vec![folder(h.join("Meus")), folder(h.join("x/../Meus"))],
            ..Default::default()
        },
        fs: Fs::real(),
    };
    assert_eq!(titles(&source.scan().games), ["Chrono Trigger"]);
}

#[test]
fn nothing_installed_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let source = EmulatorSource {
        env: Some(linux(tmp.path())),
        config: Config::default(),
        fs: Fs::real(),
    };
    let scan = source.scan();
    assert!(scan.games.is_empty());
    assert!(!scan.client_installed);
    assert!(scan.notice.is_some());
}

#[test]
fn folders_as_games_and_disc_headers() {
    let tmp = tempfile::tempdir().unwrap();
    let h = tmp.path();
    // PS3: pasta do jogo com EBOOT.BIN e o título no PARAM.SFO; Wii U: pasta com code/*.rpx.
    write(&h.join("ps3/BLUS30443/PS3_GAME/USRDIR/EBOOT.BIN"), b"elf");
    write(&h.join("wiiu/Mario Kart 8 [AMKP01]/code/Turbo.rpx"), b"rpx");
    write(&h.join("wiiu/Mario Kart 8 [AMKP01]/content/x.bin"), b"x");
    let mut iso = vec![0u8; 0x20];
    iso[0x18..0x1C].copy_from_slice(&0x5D1C_9EA3u32.to_be_bytes());
    write(&h.join("dolphin/Wii Sports.iso"), &iso);
    let mut gc = vec![0u8; 0x20];
    gc[0x1C..0x20].copy_from_slice(&0xC233_9F3Du32.to_be_bytes());
    write(&h.join("dolphin/Melee.iso"), &gc);
    let mut rvz = vec![0u8; 0x4C];
    rvz[0x48..0x4C].copy_from_slice(&2u32.to_be_bytes());
    write(&h.join("dolphin/Mario Galaxy.rvz"), &rvz);

    let fs = Fs::real();
    let mut budget = MAX_ENTRIES;
    let ps3 = scan_folder(
        &RomFolder {
            path: h.join("ps3"),
            system: Some("ps3".into()),
        },
        Origin::Folder,
        &fs,
        &mut budget,
    );
    assert_eq!(ps3.len(), 1);
    assert!(ps3[0].path.ends_with("EBOOT.BIN"));
    let wiiu = scan_folder(
        &RomFolder {
            path: h.join("wiiu"),
            system: None,
        },
        Origin::Folder,
        &fs,
        &mut budget,
    );
    assert_eq!(wiiu.len(), 1);
    assert_eq!(wiiu[0].title.as_deref(), Some("Mario Kart 8"));

    let mut out = Vec::new();
    let here: Vec<&'static System> = ["gc", "wii"]
        .iter()
        .filter_map(|s| systems::by_id(s))
        .collect();
    walk(
        &h.join("dolphin"),
        &here,
        false,
        Origin::Emulator(Emulator::Dolphin),
        &Via::Emulator(Emulator::Dolphin),
        &fs,
        &mut budget,
        &mut out,
        0,
    );
    let sys = |t: &str| {
        out.iter()
            .find(|f| f.title.as_deref() == Some(t))
            .unwrap()
            .system
            .unwrap()
            .id
    };
    assert_eq!(sys("Wii Sports"), "wii");
    assert_eq!(sys("Melee"), "gc");
    assert_eq!(sys("Mario Galaxy"), "wii");
}

#[test]
fn m3u_hides_its_discs() {
    let tmp = tempfile::tempdir().unwrap();
    let h = tmp.path();
    write(
        &h.join("psx/FF7.m3u"),
        b"FF7 (Disc 1).chd\nFF7 (Disc 2).chd\n",
    );
    write(&h.join("psx/FF7 (Disc 1).chd"), b"1");
    write(&h.join("psx/FF7 (Disc 2).chd"), b"2");
    write(&h.join("psx/Outro.chd"), b"x");
    let mut budget = MAX_ENTRIES;
    let found = scan_folder(
        &RomFolder {
            path: h.join("psx"),
            system: None,
        },
        Origin::Folder,
        &Fs::real(),
        &mut budget,
    );
    let mut names: Vec<_> = found.iter().map(|f| f.title.clone().unwrap()).collect();
    names.sort();
    assert_eq!(names, ["FF7", "Outro"]);
}

#[test]
fn scan_stops_at_the_entry_budget() {
    let tmp = tempfile::tempdir().unwrap();
    for i in 0..30 {
        write(&tmp.path().join(format!("snes/{i}.sfc")), b"x");
    }
    let mut budget = 10;
    let found = scan_folder(
        &RomFolder {
            path: tmp.path().join("snes"),
            system: None,
        },
        Origin::Folder,
        &Fs::real(),
        &mut budget,
    );
    assert!(found.is_empty());
    assert_eq!(budget, 0);
}

#[test]
fn windows_places_and_chosen_path() {
    let env = Env {
        os: Some(Os::Windows),
        home: PathBuf::from(r"C:\Users\u"),
        vars: vec![
            ("ProgramFiles".into(), PathBuf::from(r"C:\Program Files")),
            ("SystemDrive".into(), PathBuf::from("C:\\")),
            (
                "LOCALAPPDATA".into(),
                PathBuf::from(r"C:\Users\u\AppData\Local"),
            ),
            (
                "APPDATA".into(),
                PathBuf::from(r"C:\Users\u\AppData\Roaming"),
            ),
        ],
        path: Vec::new(),
        steam_root: Some(PathBuf::from(r"D:\Steam")),
        programs: vec![("Cemu 2.0".into(), PathBuf::from(r"E:\Emu\Cemu"))],
    };
    let c = emulator::candidates(Emulator::RetroArch, &env);
    assert!(
        c.contains(
            &Path::new("C:\\")
                .join("RetroArch-Win64")
                .join("retroarch.exe")
        )
    );
    assert!(
        c.contains(
            &PathBuf::from(r"D:\Steam")
                .join("steamapps")
                .join("common")
                .join("RetroArch")
                .join("retroarch.exe")
        )
    );
    assert!(
        c.contains(
            &PathBuf::from(r"C:\Users\u")
                .join("scoop")
                .join("apps")
                .join("retroarch")
                .join("current")
                .join("retroarch.exe")
        )
    );
    let cemu = emulator::candidates(Emulator::Cemu, &env);
    assert!(cemu.contains(&PathBuf::from(r"E:\Emu\Cemu").join("Cemu.exe")));
    let duck = emulator::candidates(Emulator::DuckStation, &env);
    assert!(
        duck.contains(
            &PathBuf::from(r"C:\Users\u\AppData\Local")
                .join("Programs")
                .join("DuckStation")
                .join("duckstation-qt-x64-ReleaseLTCG.exe")
        )
    );
    let none = |_: &Path| false;
    let list = |_: &Path| Vec::new();
    assert_eq!(
        emulator::find(Emulator::Dolphin, &env, None, &none, &list),
        None
    );
    // O caminho escolhido vale só se existir.
    let chosen = PathBuf::from(r"F:\Dolphin").join("Dolphin.exe");
    let yes = |p: &Path| p == chosen.as_path();
    let p = emulator::find(Emulator::Dolphin, &env, Some(&chosen), &yes, &list).unwrap();
    assert_eq!(p.target, chosen);
    assert_eq!(
        emulator::find(Emulator::Dolphin, &env, Some(&chosen), &none, &list),
        None
    );
    // Configuração portátil ao lado do executável.
    let files = standalone::config_files(Emulator::Dolphin, &env, p.dir.as_deref());
    assert_eq!(
        files[0],
        Path::new(r"F:\Dolphin")
            .join("User")
            .join("Config")
            .join("Dolphin.ini")
    );
}

#[test]
fn linux_appimage_and_flatpak() {
    let tmp = tempfile::tempdir().unwrap();
    let h = tmp.path();
    let env = linux(h);
    write(&h.join("Applications/DuckStation-x64.AppImage"), b"");
    let fs = Fs::real();
    let exists = |p: &Path| (fs.exists)(p);
    let list = |p: &Path| (fs.list_dir)(p);
    let p = emulator::find(Emulator::DuckStation, &env, None, &exists, &list).unwrap();
    assert!(p.target.ends_with("DuckStation-x64.AppImage"));
    // Flatpak: precisa do `flatpak` e da pasta do app.
    assert_eq!(
        emulator::find(Emulator::Ppsspp, &env, None, &exists, &list),
        None
    );
    write(&h.join("bin/flatpak"), b"");
    fs::create_dir_all(env.flatpak_home("org.ppsspp.PPSSPP")).unwrap();
    let p = emulator::find(Emulator::Ppsspp, &env, None, &exists, &list).unwrap();
    assert_eq!(p.prefix, ["run", "org.ppsspp.PPSSPP"]);
    let line = command_line(Os::Linux, &p, &Emulator::Ppsspp.args("/r/a b.iso", None));
    assert_eq!(line, "run org.ppsspp.PPSSPP --fullscreen \"/r/a b.iso\"");
}

#[test]
fn quoting_per_os() {
    assert_eq!(quote(Os::Windows, r"C:\Jogos\a.sfc"), r"C:\Jogos\a.sfc");
    assert_eq!(
        quote(Os::Windows, r"C:\Meus Jogos\a.sfc"),
        r#""C:\Meus Jogos\a.sfc""#
    );
    assert_eq!(quote(Os::Windows, r#"a"b"#), r#""a\"b""#);
    assert_eq!(
        quote(Os::Windows, r"C:\pasta com espaço\"),
        r#""C:\pasta com espaço\\""#
    );
    assert_eq!(quote(Os::Linux, "/r/it's $x.sfc"), "\"/r/it's \\$x.sfc\"");
    assert_eq!(quote(Os::Linux, ""), "\"\"");
    let p = Program::exe(PathBuf::from("/bin/retroarch"));
    let line = command_line(
        Os::Linux,
        &p,
        &Emulator::RetroArch.args("/r/x.sfc", Some("/c/s.so")),
    );
    assert_eq!(line, "-f -L /c/s.so /r/x.sfc");
}

#[test]
fn titles_and_systems() {
    assert_eq!(
        clean_title("Super Mario World (USA) [!]"),
        "Super Mario World"
    );
    assert_eq!(
        clean_title("Sonic_the_Hedgehog (Rev 1)"),
        "Sonic the Hedgehog"
    );
    assert_eq!(clean_title("(USA)"), "(USA)");
    assert_eq!(
        systems::by_name("Nintendo - Super Nintendo Entertainment System.lpl")
            .unwrap()
            .id,
        "snes"
    );
    assert_eq!(systems::by_name("genesis").unwrap().id, "megadrive");
    assert_eq!(systems::by_name("PS2").unwrap().id, "ps2");
    assert!(systems::by_name("documentos").is_none());
    assert!(systems::by_extension("iso").count() >= 3);
    for s in systems::SYSTEMS {
        assert!(
            !s.cores.is_empty() || !s.standalone.is_empty(),
            "{} sem como abrir",
            s.id
        );
    }
}

#[test]
fn config_round_trip_and_broken_values() {
    let mut c = Config::default();
    c.rom_folders.push(RomFolder {
        path: PathBuf::from("/roms"),
        system: None,
    });
    c.rom_folders.push(RomFolder {
        path: PathBuf::from("/snes"),
        system: Some("snes".into()),
    });
    c.paths
        .insert(Emulator::Dolphin, PathBuf::from("/d/dolphin"));
    c.by_system.insert("psx".into(), Emulator::DuckStation);
    let saved = c.save();
    let get = |k: &str| saved.iter().find(|(x, _)| *x == k).map(|(_, v)| v.as_str());
    assert_eq!(
        Config::parse(
            get(keys::ROM_FOLDERS),
            get(keys::PATHS),
            get(keys::BY_SYSTEM)
        ),
        c
    );
    let broken = Config::parse(
        Some("{lixo"),
        Some(r#"{"naoexiste":"/x","dolphin":""}"#),
        Some(r#"{"psx":"naoexiste","marte":"dolphin"}"#),
    );
    assert_eq!(broken, Config::default());
    let unknown_system = Config::parse(
        Some(r#"[{"path":"/r","system":"marte"},{"path":""}]"#),
        None,
        None,
    );
    assert_eq!(
        unknown_system.rom_folders,
        [RomFolder {
            path: PathBuf::from("/r"),
            system: None
        }]
    );
}

#[test]
fn rom_added_by_hand_opens_in_the_emulator_of_its_system() {
    let tmp = tempfile::tempdir().unwrap();
    let exe = tmp.path().join("Dolphin.exe");
    write(&exe, b"");
    let rom = tmp.path().join("Melee (USA).iso");
    write(&rom, b"iso");
    let mut config = Config::default();
    // Sem emulador para o sistema: nada.
    let none = command_for(&rom, Some("switch"), &config);
    assert!(none.is_none() || !none.unwrap().1.contains("Melee"));
    config.paths.insert(Emulator::Dolphin, exe.clone());
    // A chave dos metadados ("gamecube") também vale.
    let (target, args, dir) = command_for(&rom, Some("gamecube"), &config).unwrap();
    assert_eq!(PathBuf::from(target), exe);
    assert!(
        args.starts_with("-b -e ") && args.contains("Melee"),
        "{args}"
    );
    assert_eq!(dir.map(PathBuf::from), Some(tmp.path().to_path_buf()));
    assert_eq!(systems::platform_key("psx"), "ps1");
    assert_eq!(systems::by_name("ps1").unwrap().id, "psx");
}

#[test]
fn macos_apps_and_their_configuration() {
    let tmp = tempfile::tempdir().unwrap();
    let h = tmp.path();
    let env = Env {
        os: Some(Os::MacOs),
        home: h.to_path_buf(),
        ..Default::default()
    };
    let bin = h.join("Applications/RetroArch.app/Contents/MacOS/RetroArch");
    write(&bin, b"");
    let fs = Fs::real();
    let exists = |p: &Path| (fs.exists)(p);
    let list = |p: &Path| (fs.list_dir)(p);
    assert_eq!(
        emulator::find(Emulator::RetroArch, &env, None, &exists, &list)
            .unwrap()
            .target,
        bin
    );
    let ra = retroarch::config_dirs(&env, None);
    assert!(
        ra.contains(
            &h.join("Library")
                .join("Application Support")
                .join("RetroArch")
        )
    );
    let pcsx2 = standalone::config_files(Emulator::Pcsx2, &env, None);
    assert_eq!(
        pcsx2,
        [h.join("Library")
            .join("Application Support")
            .join("PCSX2")
            .join("inis")
            .join("PCSX2.ini")]
    );
    assert_eq!(
        retroarch::core_file(Path::new("/c"), "snes9x", Os::MacOs),
        Path::new("/c").join("snes9x_libretro.dylib")
    );
}
