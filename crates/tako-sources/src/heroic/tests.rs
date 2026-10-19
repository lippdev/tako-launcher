//! Fixtures fictícias no formato real dos arquivos do Heroic 2.x (legendary, gogdl, nile, sideload).

#![allow(clippy::unwrap_used)]

use super::*;
use std::fs;

const LEGENDARY_INSTALLED: &str = r#"{
  "Fennec": {
    "app_name": "Fennec", "title": "Jogo Fictício Epic", "install_path": "@DIR@/Fennec",
    "install_size": 1073741824, "executable": "Game.exe", "platform": "Windows",
    "is_dlc": false, "can_run_offline": true, "version": "1.0", "base_urls": [], "install_tags": []
  },
  "FennecDlc": {
    "app_name": "FennecDlc", "title": "DLC", "install_path": "@DIR@/Fennec", "is_dlc": true
  },
  "Gone": { "app_name": "Gone", "title": "Apagado", "install_path": "@DIR@/nao-existe", "is_dlc": false }
}"#;

const LEGENDARY_LIBRARY: &str = r#"{
  "library": [
    { "app_name": "Fennec", "title": "Jogo Fictício Epic", "runner": "legendary",
      "art_square": "https://cdn.example/fennec_tall.jpg", "art_cover": "https://cdn.example/fennec_wide.jpg",
      "is_installed": true, "install": {} },
    { "app_name": "Quail", "title": "Codorna Fictícia", "runner": "legendary", "namespace": "quailns",
      "is_installed": false, "install": { "is_dlc": false, "platform": "Windows" } },
    { "app_name": "QuailDlc", "title": "Codorna - Expansão", "runner": "legendary",
      "is_installed": false, "install": { "is_dlc": true } },
    { "app_name": "Gone", "title": "Apagado", "runner": "legendary", "is_installed": true, "install": null }
  ],
  "__timestamp.library": "2026-01-01T00:00:00.000Z"
}"#;

const GOG_INSTALLED: &str = r#"{
  "installed": [
    { "platform": "linux", "executable": "", "install_path": "@DIR@/Tabuleiro", "install_size": "1.2 GiB",
      "is_dlc": false, "version": "1.0", "appName": "1207658924", "installedWithDLCs": false },
    { "platform": "windows", "install_path": "@DIR@/Tabuleiro", "is_dlc": true, "appName": "1207658925" }
  ]
}"#;

const GOG_LIBRARY: &str = r#"{
  "games": [
    { "app_name": "1207658924", "title": "Tabuleiro Fictício", "runner": "gog",
      "art_square": "https://images.gog.example/tab.jpg", "art_cover": "https://images.gog.example/tab_wide.jpg",
      "is_installed": false, "install": {} },
    { "app_name": "gog-redist", "title": "Galaxy Common Redistributables", "runner": "gog",
      "is_installed": true, "install": { "is_dlc": true } },
    { "app_name": "1207658930", "title": "Xadrez Fictício", "runner": "gog",
      "is_installed": false, "install": { "is_dlc": false } },
    "item quebrado"
  ]
}"#;

const NILE_INSTALLED: &str = r#"[{ "id": "amzn1.adg.product.0000-1111", "version": "v1", "path": "@DIR@/Corrida", "size": 123 }]"#;

const NILE_LIBRARY: &str = r#"{ "library": [
  { "app_name": "amzn1.adg.product.0000-1111", "title": "Corrida Fictícia", "runner": "nile",
    "art_square": "https://m.media.example/icon.png", "is_installed": true, "install": {} },
  { "app_name": "amzn1.adg.product.2222-3333", "title": "Voo Fictício", "runner": "nile",
    "is_installed": false, "install": {} }
] }"#;

const SIDELOAD: &str = r#"{ "games": [
  { "runner": "sideload", "app_name": "aB3dE5fG7hI9", "title": "Emulador Caseiro",
    "install": { "executable": "/opt/emu/emu", "platform": "linux", "is_dlc": false },
    "folder_name": "/opt/emu", "art_cover": "@DIR@/capa.png", "art_square": "@DIR@/capa.png",
    "is_installed": true, "canRunOffline": true },
  { "runner": "sideload", "app_name": "x/../../etc", "title": "Malicioso", "is_installed": true, "install": {} },
  { "runner": "sideload", "app_name": "naoinstalado", "title": "Removido", "is_installed": false, "install": {} }
] }"#;

fn write(root: &Path, rel: &str, text: &str, dir: &Path) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        text.replace("@DIR@", &dir.to_str().unwrap().replace('\\', "/")),
    )
    .unwrap();
}

fn fake_heroic() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("heroic");
    let games = tmp.path().join("Games");
    for d in ["Fennec", "Tabuleiro", "Corrida"] {
        fs::create_dir_all(games.join(d)).unwrap();
    }
    fs::write(games.join("capa.png"), b"png").unwrap();
    write(
        &root,
        "legendaryConfig/legendary/installed.json",
        LEGENDARY_INSTALLED,
        &games,
    );
    write(
        &root,
        "store_cache/legendary_library.json",
        LEGENDARY_LIBRARY,
        &games,
    );
    write(&root, "gog_store/installed.json", GOG_INSTALLED, &games);
    write(&root, "store_cache/gog_library.json", GOG_LIBRARY, &games);
    write(
        &root,
        "nile_config/nile/installed.json",
        NILE_INSTALLED,
        &games,
    );
    write(&root, "store_cache/nile_library.json", NILE_LIBRARY, &games);
    write(&root, "sideload_apps/library.json", SIDELOAD, &games);
    (tmp, root)
}

fn by_id<'a>(games: &'a [Game], id: &str) -> &'a Game {
    games.iter().find(|g| g.id == id).unwrap()
}

#[test]
fn scans_every_runner_and_skips_dlc_missing_and_bad_names() {
    let (_tmp, root) = fake_heroic();
    let games = scan_root(&root);
    let mut ids: Vec<&str> = games
        .iter()
        .filter(|g| g.is_installed)
        .map(|g| g.id.as_str())
        .collect();
    ids.sort();
    assert_eq!(
        ids,
        [
            "heroic:gog:1207658924",
            "heroic:legendary:Fennec",
            "heroic:nile:amzn1.adg.product.0000-1111",
            "heroic:sideload:aB3dE5fG7hI9",
        ]
    );

    let epic = by_id(&games, "heroic:legendary:Fennec");
    assert_eq!(epic.title, "Jogo Fictício Epic");
    assert_eq!(epic.store, Some(GameStore::Heroic));
    assert_eq!(
        epic.launch_target.as_deref(),
        Some("heroic://launch/legendary/Fennec")
    );
    assert_eq!(epic.store_product_id.as_deref(), Some("Fennec"));
    assert!(epic.is_installed);

    // Título do GOG e da Amazon vem do cache de biblioteca.
    assert_eq!(
        by_id(&games, "heroic:gog:1207658924").title,
        "Tabuleiro Fictício"
    );
    assert_eq!(
        by_id(&games, "heroic:nile:amzn1.adg.product.0000-1111").title,
        "Corrida Fictícia"
    );

    let side = by_id(&games, "heroic:sideload:aB3dE5fG7hI9");
    assert_eq!(side.install_path.as_deref(), Some("/opt/emu"));
    assert!(side.cover_uri.as_deref().unwrap().starts_with("file:///"));
    assert!(side.cover_uri.as_deref().unwrap().ends_with("/capa.png"));
}

#[test]
fn uses_art_from_heroic_image_cache_only_when_downloaded() {
    let (_tmp, root) = fake_heroic();
    let cache = root.join("images-cache");
    fs::create_dir_all(&cache).unwrap();
    // Capa da Epic: a URL que a grade pede, com o redimensionamento.
    let url = "https://cdn.example/fennec_tall.jpg?h=400&resize=1&w=300";
    let digest = format!("{:x}", Sha256::digest(url.as_bytes()));
    fs::write(cache.join(&digest), b"jpg").unwrap();

    let games = scan_root(&root);
    let epic = by_id(&games, "heroic:legendary:Fennec");
    assert_eq!(
        epic.cover_uri.as_deref(),
        Some(file_uri(&cache.join(digest)).as_str())
    );
    assert!(epic.has_fallback_art);
    assert_eq!(epic.hero_uri, None);
    assert_eq!(by_id(&games, "heroic:gog:1207658924").cover_uri, None);
}

#[test]
fn broken_files_yield_nothing() {
    for bad in [&b"{"[..], b"", b"[1,2]", b"null", &[0xff, 0xfe, 0x00]] {
        assert!(parse_legendary_installed(bad).is_empty());
        assert!(parse_gog_installed(bad).is_empty());
        assert!(parse_nile_installed(bad).is_empty());
        assert!(parse_library_cache(bad).is_empty());
        assert!(parse_sideload_library(bad).is_empty());
    }
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "gog_store/installed.json",
        "{ quebrado",
        tmp.path(),
    );
    assert!(scan_root(tmp.path()).is_empty());
}

#[test]
fn missing_heroic_reports_notice() {
    let tmp = tempfile::tempdir().unwrap();
    let scan = HeroicSource::new(vec![tmp.path().join("nada")]).scan();
    assert!(!scan.client_installed);
    assert!(scan.notice.is_some());
}

#[test]
fn source_merges_roots_without_duplicates() {
    let (_tmp, root) = fake_heroic();
    let scan = HeroicSource::new(vec![root.clone(), root]).scan();
    assert!(scan.client_installed);
    assert_eq!(scan.games.len(), 8); // 4 instalados + 4 da conta
    assert_eq!(scan.games[0].title, "Apagado");
}

#[test]
fn owned_games_that_are_not_installed_come_from_the_library_cache() {
    let (_tmp, root) = fake_heroic();
    let games = scan_root(&root);
    let mut owned: Vec<(&str, &str)> = games
        .iter()
        .filter(|g| !g.is_installed)
        .map(|g| (g.id.as_str(), g.install_uri.as_deref().unwrap()))
        .collect();
    owned.sort();
    // Sem DLC nem o gog-redist; o "Gone" foi apagado por fora do Heroic e volta a ser instalável.
    assert_eq!(
        owned,
        [
            ("heroic:gog:1207658930", "heroic://launch/gog/1207658930"),
            ("heroic:legendary:Gone", "heroic://launch/legendary/Gone"),
            ("heroic:legendary:Quail", "heroic://launch/legendary/Quail"),
            (
                "heroic:nile:amzn1.adg.product.2222-3333",
                "heroic://launch/nile/amzn1.adg.product.2222-3333"
            ),
        ]
    );
    let quail = by_id(&games, "heroic:legendary:Quail");
    assert_eq!(quail.title, "Codorna Fictícia");
    assert_eq!(quail.launch_target, None);
    assert!(!quail.can_play());
    // O instalado não aparece duas vezes.
    assert_eq!(
        games
            .iter()
            .filter(|g| g.id == "heroic:legendary:Fennec")
            .count(),
        1
    );
}

// Caminhos montados a partir do temporário: `is_absolute` depende do SO (no Windows "/home/u" não é absoluto).
#[test]
fn linux_roots_do_not_repeat_the_default_folder() {
    let home = std::env::temp_dir().join("u");
    let expected = [
        home.join(".config/heroic"),
        home.join(".var/app/com.heroicgameslauncher.hgl/config/heroic"),
    ];
    for xdg in [None, Some(home.join(".config")), Some(PathBuf::from("rel"))] {
        assert_eq!(linux_roots(&home, xdg), expected);
    }
}

#[test]
fn linux_roots_add_the_host_folder_when_xdg_is_the_sandbox() {
    let home = std::env::temp_dir().join("u");
    let sandbox = home.join(".var/app/io.github.lippdev.Tako/config");
    let roots = linux_roots(&home, Some(sandbox.clone()));
    assert_eq!(roots.len(), 3);
    assert_eq!(roots[0], sandbox.join("heroic"));
    assert_eq!(roots[2], home.join(".config/heroic"));
}
