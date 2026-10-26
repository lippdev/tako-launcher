use super::*;
use std::fs;
use tempfile::tempdir;

const SAMPLE_BOTTLE_YML: &str = r#"
Arch: win64
CompatData: ''
Creation_Date: '2026-06-02 19:19:24.348777'
Custom_Path: false
DLL_Overrides: {}
DXVK: dxvk-2.7.1
Environment: Gaming
Environment_Variables: {}
External_Programs:
  ceff4adc-db4e-49e4-ad7c-d9d40c92d7dc:
    arguments: -opengl -SkipBuildPatchPrereq
    executable: EpicGamesLauncher.exe
    folder: /home/user/games/epic
    id: ceff4adc-db4e-49e4-ad7c-d9d40c92d7dc
    name: Epic Games Store
    path: C:\Program Files (x86)\Epic Games\Launcher\Portal\Binaries\Win32\EpicGamesLauncher.exe
    icon: epic.png
  2d0ded70-3edf-4884-9e2a-c22a64c5e9e1:
    arguments: ''
    executable: TOMB2.EXE
    folder: /home/user/games/tomb
    id: 2d0ded70-3edf-4884-9e2a-c22a64c5e9e1
    name: Tomb Raider II
    path: C:\Tomb2\TOMB2.EXE
Name: Jogos Clássicos
Parameters:
  dxvk: true
"#;

#[test]
fn parse_bottle_yml_reads_name_and_programs() {
    let cfg = parse_bottle_yml(SAMPLE_BOTTLE_YML, "fallback_dir");
    assert_eq!(cfg.name, "Jogos Clássicos");
    assert_eq!(cfg.programs.len(), 2);

    let epic = &cfg.programs[0];
    assert_eq!(epic.id, "ceff4adc-db4e-49e4-ad7c-d9d40c92d7dc");
    assert_eq!(epic.name, "Epic Games Store");
    assert_eq!(epic.executable, "EpicGamesLauncher.exe");
    assert_eq!(epic.folder.as_deref(), Some("/home/user/games/epic"));
    assert_eq!(
        epic.arguments.as_deref(),
        Some("-opengl -SkipBuildPatchPrereq")
    );
    assert_eq!(epic.icon.as_deref(), Some("epic.png"));

    let tomb = &cfg.programs[1];
    assert_eq!(tomb.name, "Tomb Raider II");
    assert_eq!(tomb.executable, "TOMB2.EXE");
}

#[test]
fn parse_bottle_yml_falls_back_to_folder_name_when_name_missing() {
    let yml = r#"
Environment: Application
External_Programs:
  prog1:
    name: Bloco de Notas
    executable: notepad.exe
"#;
    let cfg = parse_bottle_yml(yml, "MinhaGarrafa");
    assert_eq!(cfg.name, "MinhaGarrafa");
    assert_eq!(cfg.programs.len(), 1);
    assert_eq!(cfg.programs[0].name, "Bloco de Notas");
}

#[test]
fn parse_bottle_yml_malformed_input_never_panics() {
    for malformed in [
        "",
        ":::",
        "Name",
        "External_Programs:\n    - item 1\n    - item 2",
        "External_Programs:\n  id:\n    name\n    executable",
        "\0\u{ff}\u{fe}",
        "Name: [unclosed bracket",
    ] {
        let cfg = parse_bottle_yml(malformed, "garrafa_quebrada");
        assert!(!cfg.name.is_empty());
    }
}

#[test]
fn scan_bottle_generates_native_and_flatpak_launch_commands() {
    let dir = tempdir().unwrap();
    let bottle_dir = dir.path().join("Jogos");
    fs::create_dir_all(&bottle_dir).unwrap();
    fs::write(bottle_dir.join("bottle.yml"), SAMPLE_BOTTLE_YML).unwrap();

    // 1. Nativo
    let native_games = scan_bottle(&bottle_dir, false).unwrap();
    assert_eq!(native_games.len(), 2);
    let epic_native = &native_games[0];
    assert_eq!(
        epic_native.id,
        "bottles:Jogos_Cl_ssicos:ceff4adc-db4e-49e4-ad7c-d9d40c92d7dc"
    );
    assert_eq!(epic_native.store, Some(GameStore::Bottles));
    assert_eq!(epic_native.title, "Epic Games Store");
    assert_eq!(epic_native.launch_target.as_deref(), Some("bottles-cli"));
    assert_eq!(
        epic_native.launch_arguments.as_deref(),
        Some("run -b \"Jogos Clássicos\" -p \"Epic Games Store\"")
    );

    // 2. Flatpak
    let flatpak_games = scan_bottle(&bottle_dir, true).unwrap();
    assert_eq!(flatpak_games.len(), 2);
    let epic_flatpak = &flatpak_games[0];
    assert_eq!(epic_flatpak.launch_target.as_deref(), Some("flatpak"));
    assert_eq!(
        epic_flatpak.launch_arguments.as_deref(),
        Some(
            "run --command=bottles-cli com.usebottles.bottles run -b \"Jogos Clássicos\" -p \"Epic Games Store\""
        )
    );
}

#[test]
fn scan_bottle_resolves_icons() {
    let dir = tempdir().unwrap();
    let bottle_dir = dir.path().join("GarrafaComIcone");
    let icons_dir = bottle_dir.join("icons");
    fs::create_dir_all(&icons_dir).unwrap();

    let icon_file = icons_dir.join("app.png");
    fs::write(&icon_file, b"fake png data").unwrap();

    let yml = r#"
Name: Garrafa
External_Programs:
  prog1:
    name: App Teste
    executable: app.exe
    icon: app.png
"#;
    fs::write(bottle_dir.join("bottle.yml"), yml).unwrap();

    let games = scan_bottle(&bottle_dir, false).unwrap();
    assert_eq!(games.len(), 1);
    let g = &games[0];
    assert!(g.media.contains_key(media::ICON));
    assert_eq!(g.icon_origin, Some(IconOrigin::Store));
    let icon_uri = g.media.get(media::ICON).unwrap();
    assert!(icon_uri.starts_with("file://"));
    assert!(icon_uri.ends_with("app.png"));
}

#[test]
fn scan_bottle_skips_empty_program_name() {
    let dir = tempdir().unwrap();
    let bottle_dir = dir.path().join("Jogos");
    fs::create_dir_all(&bottle_dir).unwrap();
    let yml = r#"
Name: Garrafa
External_Programs:
  empty_id:
    name: ""
    executable: dummy.exe
  valid_id:
    name: Programa Valido
    executable: app.exe
"#;
    fs::write(bottle_dir.join("bottle.yml"), yml).unwrap();
    let games = scan_bottle(&bottle_dir, false).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].title, "Programa Valido");
    assert_eq!(games[0].id, "bottles:Garrafa:valid_id");
}

#[test]
fn scan_bottle_skips_empty_program_uuid() {
    let dir = tempdir().unwrap();
    let bottle_dir = dir.path().join("Jogos");
    fs::create_dir_all(&bottle_dir).unwrap();
    let yml = r#"
Name: Garrafa
External_Programs:
  "":
    name: Programa Sem UUID
    executable: dummy.exe
  "   ":
    name: Programa Com Espacos no UUID
    executable: dummy2.exe
  valid_uuid:
    name: Programa Valido
    executable: app.exe
"#;
    fs::write(bottle_dir.join("bottle.yml"), yml).unwrap();
    let games = scan_bottle(&bottle_dir, false).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].title, "Programa Valido");
    assert_eq!(games[0].id, "bottles:Garrafa:valid_uuid");
}

#[test]
fn scan_bottle_rejects_icon_outside_icons_dir() {
    let dir = tempdir().unwrap();
    let bottle_dir = dir.path().join("Garrafa");
    let icons_dir = bottle_dir.join("icons");
    fs::create_dir_all(&icons_dir).unwrap();

    let outside_icon = dir.path().join("outside.png");
    fs::write(&outside_icon, b"fake").unwrap();

    let yml = format!(
        r#"
Name: Garrafa
External_Programs:
  prog1:
    name: Teste
    executable: test.exe
    icon: {}
"#,
        outside_icon.display()
    );
    fs::write(bottle_dir.join("bottle.yml"), yml).unwrap();
    let games = scan_bottle(&bottle_dir, false).unwrap();
    assert_eq!(games.len(), 1);
    assert!(!games[0].media.contains_key(media::ICON));
}

#[test]
fn linux_roots_deduplication_and_sandbox() {
    let dir = tempdir().unwrap();
    let home = dir.path().join("usuario");
    let roots = linux_roots(&home, None);
    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].path, home.join(".local/share/bottles"));
    assert!(!roots[0].is_flatpak);
    assert_eq!(
        roots[1].path,
        home.join(".var/app/com.usebottles.bottles/data/bottles")
    );
    assert!(roots[1].is_flatpak);

    // XDG_DATA_HOME da sandbox aponta para outro lugar: raiz do host entra sem repetir
    let sandbox = dir.path().join("sandbox_data");
    let roots = linux_roots(&home, Some(sandbox.clone()));
    assert_eq!(roots.len(), 3);
    assert_eq!(roots[0].path, sandbox.join("bottles"));
    assert_eq!(roots[2].path, home.join(".local/share/bottles"));
}

#[test]
fn bottles_source_reports_missing_notice_when_empty() {
    let dir = tempdir().unwrap();
    let source = BottlesSource::new(vec![BottlesRoot {
        path: dir.path().join("nao_existe"),
        is_flatpak: false,
    }]);

    let scan = source.scan();
    assert!(!scan.client_installed);
    assert!(scan.games.is_empty());
    assert!(scan.notice.is_some());
}
