//! Ideias de `ThemePackageTests.cs` adaptadas ao `.takotheme`.

use std::io::Write;

use zip::write::SimpleFileOptions;

use super::*;

const TOML: &str = "id = \"Autor_TemaTeste\"\nname = \"Tema de teste\"\nversion = \"1.0\"\nauthor = \"Autor\"\napi = 1\n";
const SLINT: &str = "export component Main inherits Window {}";

struct Work {
    dir: tempfile::TempDir,
}

impl Work {
    fn new() -> Work {
        Work {
            dir: tempfile::tempdir().unwrap(),
        }
    }
    fn root(&self) -> PathBuf {
        self.dir.path().join("themes")
    }
    fn zip(&self, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = self.dir.path().join("tema.takotheme");
        let mut z = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        for (name, content) in entries {
            z.start_file(*name, SimpleFileOptions::default()).unwrap();
            z.write_all(content).unwrap();
        }
        z.finish().unwrap();
        path
    }
    fn install(&self, entries: &[(&str, &[u8])]) -> Result<InstalledTheme, Vec<Problem>> {
        install(&self.zip(entries), &self.root())
    }
    fn install_with(
        &self,
        entries: &[(&str, &[u8])],
        limits: Limits,
    ) -> Result<InstalledTheme, Vec<Problem>> {
        install_with(&self.zip(entries), &self.root(), &limits)
    }
    /// Nada fora da pasta de temas e nenhuma pasta temporária sobrando.
    fn assert_nothing_leaked(&self, themes: &[&str]) {
        let mut top: Vec<_> = fs::read_dir(self.dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n != "tema.takotheme")
            .collect();
        top.sort();
        if themes.is_empty() && top.is_empty() {
            return;
        }
        assert_eq!(top, ["themes"]);
        let mut inside: Vec<_> = fs::read_dir(self.root())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n != ".selected")
            .collect();
        inside.sort();
        assert_eq!(inside, themes);
    }
}

fn codes(r: Result<InstalledTheme, Vec<Problem>>) -> Vec<&'static str> {
    r.err().unwrap_or_default().iter().map(|p| p.code).collect()
}

#[test]
fn installs_a_valid_package_into_a_folder_named_after_the_id() {
    let w = Work::new();
    let t = w
        .install(&[
            ("theme.toml", TOML.as_bytes()),
            ("main.slint", SLINT.as_bytes()),
            ("ui/home.slint", SLINT.as_bytes()),
            ("img/bg.png", b"png"),
        ])
        .unwrap();
    assert_eq!(t.dir, w.root().join("Autor_TemaTeste"));
    assert!(t.dir.join("ui").join("home.slint").is_file());
    assert_eq!(t.manifest.as_ref().unwrap().name, "Tema de teste");
    w.assert_nothing_leaked(&["Autor_TemaTeste"]);
    let scanned = InstalledTheme::scan(&w.root());
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].id(), "Autor_TemaTeste");
    assert!(scanned[0].manifest.is_ok());
}

#[test]
fn accepts_a_package_wrapped_in_a_single_folder() {
    let w = Work::new();
    let t = w
        .install(&[
            ("MeuTema/theme.toml", TOML.as_bytes()),
            ("MeuTema/main.slint", SLINT.as_bytes()),
        ])
        .unwrap();
    assert!(t.dir.join("main.slint").is_file());
    assert!(t.dir.join("theme.toml").is_file());
}

#[test]
fn reinstalling_replaces_the_previous_version() {
    let w = Work::new();
    w.install(&[
        ("theme.toml", TOML.as_bytes()),
        ("main.slint", SLINT.as_bytes()),
        ("old.png", b"x"),
    ])
    .unwrap();
    let t = w
        .install(&[
            ("theme.toml", TOML.replace("1.0", "2.0").as_bytes()),
            ("main.slint", SLINT.as_bytes()),
            ("new.png", b"x"),
        ])
        .unwrap();
    assert!(!t.dir.join("old.png").exists());
    assert!(t.dir.join("new.png").exists());
    assert_eq!(t.manifest.unwrap().version, "2.0");
    w.assert_nothing_leaked(&["Autor_TemaTeste"]);
}

#[test]
fn refuses_zip_slip_and_dangerous_names_before_writing_anything() {
    for name in [
        "../evil.png",
        "ui/../../evil.png",
        "..\\evil.png",
        "/etc/evil.png",
        "\\\\servidor\\share\\evil.png",
        "C:/Windows/evil.png",
        "C:evil.png",
        "theme.toml:stream",
        "ui/CON.png",
        "ui/nome. ",
    ] {
        let w = Work::new();
        let r = w.install(&[
            ("theme.toml", TOML.as_bytes()),
            ("main.slint", SLINT.as_bytes()),
            (name, b"x"),
        ]);
        assert!(codes(r).contains(&"package.unsafe-path"), "{name}");
        assert!(!w.root().exists(), "{name}");
        assert!(!w.dir.path().join("evil.png").exists());
    }
}

#[test]
fn refuses_files_outside_the_allowed_types() {
    let w = Work::new();
    let r = w.install(&[
        ("theme.toml", TOML.as_bytes()),
        ("main.slint", SLINT.as_bytes()),
        ("tools/run.exe", b"MZ"),
        ("setup.ps1", b"rm -r"),
        ("README", b"sem extensao"),
    ]);
    assert_eq!(
        codes(r),
        [
            "package.blocked-extension",
            "package.blocked-extension",
            "package.blocked-extension"
        ]
    );
    assert!(!w.root().exists());
}

#[test]
fn refuses_unsupported_api_versions_and_bad_manifests() {
    let w = Work::new();
    let main = ("main.slint", SLINT.as_bytes());
    let api2 = TOML.replace("api = 1", "api = 2");
    assert_eq!(
        codes(w.install(&[("theme.toml", api2.as_bytes()), main])),
        ["manifest.unsupported-api"]
    );
    assert_eq!(
        codes(w.install(&[("theme.toml", b"id = "), main])),
        ["manifest.invalid"]
    );
    assert_eq!(
        codes(w.install(&[("theme.toml", &[0xff, 0xfe]), main])),
        ["manifest.invalid"]
    );
    let big = format!("{TOML}description = \"{}\"\n", "a".repeat(70 * 1024));
    assert_eq!(
        codes(w.install(&[("theme.toml", big.as_bytes()), main])),
        ["manifest.too-large"]
    );
    // Arquivo principal ausente.
    assert_eq!(
        codes(w.install(&[("theme.toml", TOML.as_bytes())])),
        ["package.missing-entry"]
    );
    assert!(!w.root().exists());
}

#[test]
fn refuses_packages_without_manifest() {
    let w = Work::new();
    assert_eq!(
        codes(w.install(&[
            ("main.slint", SLINT.as_bytes()),
            ("Outro/theme.toml", TOML.as_bytes()),
            ("Mais/x.slint", SLINT.as_bytes()),
        ])),
        ["package.missing-manifest"]
    );
}

#[test]
fn refuses_files_that_are_not_zip_or_missing() {
    let w = Work::new();
    let path = w.dir.path().join("x.takotheme");
    fs::write(&path, "isto não é um zip").unwrap();
    assert_eq!(
        validate_package(&path).unwrap_err()[0].code,
        "package.not-zip"
    );
    assert_eq!(
        validate_package(&w.dir.path().join("nao-existe.takotheme")).unwrap_err()[0].code,
        "package.not-found"
    );
}

#[test]
fn refuses_reserved_ids() {
    let w = Work::new();
    let toml = TOML.replace("Autor_TemaTeste", "Default");
    assert_eq!(
        codes(w.install(&[
            ("theme.toml", toml.as_bytes()),
            ("main.slint", SLINT.as_bytes())
        ])),
        ["package.reserved-id"]
    );
}

#[test]
fn refuses_too_many_entries() {
    let w = Work::new();
    let names: Vec<String> = (0..20).map(|i| format!("ui/{i}.slint")).collect();
    let mut entries: Vec<(&str, &[u8])> = vec![
        ("theme.toml", TOML.as_bytes()),
        ("main.slint", SLINT.as_bytes()),
    ];
    entries.extend(names.iter().map(|n| (n.as_str(), SLINT.as_bytes())));
    let limits = Limits {
        max_entries: 10,
        ..Limits::default()
    };
    assert_eq!(
        codes(w.install_with(&entries, limits)),
        ["package.too-many-entries"]
    );
}

#[test]
fn refuses_packages_larger_than_the_limits() {
    let w = Work::new();
    let big = vec![b'a'; 4096];
    let entries: [(&str, &[u8]); 4] = [
        ("theme.toml", TOML.as_bytes()),
        ("main.slint", SLINT.as_bytes()),
        ("a.txt", &big),
        ("b.txt", &big),
    ];
    let total = Limits {
        max_total_bytes: 6000,
        ..Limits::default()
    };
    assert_eq!(
        codes(w.install_with(&entries, total)),
        ["package.too-large"]
    );
    let package = Limits {
        max_package_bytes: 100,
        ..Limits::default()
    };
    assert_eq!(
        codes(w.install_with(&entries, package)),
        ["package.too-large"]
    );
    assert!(!w.root().join("Autor_TemaTeste").exists());
}

#[test]
fn refuses_highly_compressed_files_over_the_file_limit() {
    // "Zip bomb" pequeno: 1 MB de zeros comprime para poucos KB.
    let w = Work::new();
    let zeros = vec![0u8; 1024 * 1024];
    let path = w.zip(&[
        ("theme.toml", TOML.as_bytes()),
        ("main.slint", SLINT.as_bytes()),
        ("img/bomb.png", &zeros),
    ]);
    assert!(fs::metadata(&path).unwrap().len() < 64 * 1024);
    let limits = Limits {
        max_file_bytes: 512 * 1024,
        ..Limits::default()
    };
    assert_eq!(
        codes(install_with(&path, &w.root(), &limits)),
        ["package.file-too-large"]
    );
}

#[test]
fn refuses_duplicate_entries() {
    let w = Work::new();
    assert_eq!(
        codes(w.install(&[
            ("theme.toml", TOML.as_bytes()),
            ("main.slint", SLINT.as_bytes()),
            ("ui/Main.slint", SLINT.as_bytes()),
            ("ui/main.slint", SLINT.as_bytes()),
        ])),
        ["package.duplicate-entry"]
    );
}

#[test]
fn validates_and_installs_a_theme_folder() {
    let w = Work::new();
    let src = w.dir.path().join("src");
    fs::create_dir_all(src.join("ui")).unwrap();
    fs::write(src.join("theme.toml"), TOML).unwrap();
    fs::write(src.join("main.slint"), SLINT).unwrap();
    fs::write(src.join("ui").join("card.slint"), SLINT).unwrap();
    assert_eq!(validate_package(&src).unwrap().id, "Autor_TemaTeste");
    let t = install(&src, &w.root()).unwrap();
    assert!(t.dir.join("ui").join("card.slint").is_file());

    fs::write(src.join("hack.exe"), "MZ").unwrap();
    assert_eq!(
        validate_package(&src).unwrap_err()[0].code,
        "package.blocked-extension"
    );
}

#[cfg(unix)]
#[test]
fn refuses_symlinks_in_a_theme_folder() {
    let w = Work::new();
    let src = w.dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("theme.toml"), TOML).unwrap();
    fs::write(src.join("main.slint"), SLINT).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", src.join("x.txt")).unwrap();
    assert_eq!(
        validate_package(&src).unwrap_err()[0].code,
        "package.symlink"
    );
}

#[test]
fn uninstall_only_takes_safe_ids_and_clears_the_selection() {
    let w = Work::new();
    w.install(&[
        ("theme.toml", TOML.as_bytes()),
        ("main.slint", SLINT.as_bytes()),
    ])
    .unwrap();
    select(&w.root(), Some("Autor_TemaTeste")).unwrap();
    assert!(uninstall(&w.root(), "..").is_err());
    assert!(uninstall(&w.root(), "../themes").is_err());
    assert!(uninstall(&w.root(), "default").is_err());
    overrides::save_override(&w.root(), Some("Autor_TemaTeste"), "accent = \"#112233\"\n").unwrap();
    assert!(overrides::has_override(&w.root(), Some("Autor_TemaTeste")));

    uninstall(&w.root(), "Autor_TemaTeste").unwrap();
    assert!(InstalledTheme::scan(&w.root()).is_empty());
    assert_eq!(selected(&w.root()), None);
    assert!(!overrides::has_override(&w.root(), Some("Autor_TemaTeste")));
}

#[test]
fn scan_lists_broken_themes_with_problems_and_skips_hidden_folders() {
    let w = Work::new();
    let root = w.root();
    fs::create_dir_all(root.join("quebrado")).unwrap();
    fs::write(root.join("quebrado").join("theme.toml"), "api = 9").unwrap();
    fs::create_dir_all(root.join(".import-x")).unwrap();
    fs::write(root.join(".import-x").join("theme.toml"), TOML).unwrap();
    fs::create_dir_all(root.join("sem-manifesto")).unwrap();
    let list = InstalledTheme::scan(&root);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id(), "quebrado");
    assert!(list[0].manifest.is_err());
    assert!(InstalledTheme::scan(&w.dir.path().join("nada")).is_empty());
}

#[test]
fn selection_is_stored_and_validated() {
    let w = Work::new();
    let root = w.root();
    assert_eq!(selected(&root), None);
    select(&root, Some("autor.tema")).unwrap();
    assert_eq!(selected(&root).as_deref(), Some("autor.tema"));
    assert!(select(&root, Some("../x")).is_err());
    select(&root, Some("default")).unwrap();
    assert_eq!(selected(&root), None);
    fs::write(root.join(".selected"), "../../etc").unwrap();
    assert_eq!(selected(&root), None);
}

#[test]
fn crash_loop_falls_back_to_the_default_theme() {
    let w = Work::new();
    let root = w.root();
    select(&root, Some("autor.tema")).unwrap();
    let t0 = UNIX_EPOCH + Duration::from_secs(1_000_000);
    let at = |s: u64| t0 + Duration::from_secs(s);

    // Carga que termina: nada conta.
    assert_eq!(begin_load(&root, at(0)).as_deref(), Some("autor.tema"));
    end_load(&root);
    assert_eq!(begin_load(&root, at(1)).as_deref(), Some("autor.tema"));
    // O processo "caiu" (sem end_load) duas vezes seguidas: duas falhas, ainda no tema.
    assert_eq!(begin_load(&root, at(2)).as_deref(), Some("autor.tema"));
    assert_eq!(begin_load(&root, at(3)).as_deref(), Some("autor.tema"));
    // Terceira falha dentro de 10 s: volta ao padrão e a escolha é apagada.
    assert_eq!(begin_load(&root, at(4)), None);
    assert_eq!(selected(&root), None);
    assert_eq!(begin_load(&root, at(5)), None);

    // Falhas espaçadas não somam.
    select(&root, Some("autor.tema")).unwrap();
    assert!(!record_fault(&root, at(100)));
    assert!(!record_fault(&root, at(111)));
    assert!(!record_fault(&root, at(122)));
    assert!(!record_fault(&root, at(123)));
    assert!(record_fault(&root, at(124)));
}

#[test]
fn theme_from_a_repository_remembers_where_it_came_from() {
    let w = Work::new();
    let repo = remote::Repo::parse("autor/tema").unwrap();
    let pkg = w.zip(&[
        ("theme.toml", TOML.as_bytes()),
        ("main.slint", SLINT.as_bytes()),
    ]);
    let theme =
        remote::install_downloaded(&pkg, &w.root(), &repo, "v1.0", &Limits::default()).unwrap();
    assert_eq!(theme.id(), "Autor_TemaTeste");
    let sources = remote::sources(&w.root());
    assert_eq!(
        sources.get("Autor_TemaTeste"),
        Some(&remote::Source {
            repo: "https://github.com/autor/tema".into(),
            tag: "v1.0".into(),
        })
    );
    // O arquivo de origens não aparece como tema.
    assert_eq!(InstalledTheme::scan(&w.root()).len(), 1);
    // Pacote ruim não instala nem grava origem.
    let bad = w.zip(&[("leiame.txt", b"x")]);
    assert!(remote::install_downloaded(&bad, &w.root(), &repo, "v2", &Limits::default()).is_err());
    assert_eq!(remote::sources(&w.root())["Autor_TemaTeste"].tag, "v1.0");
    // Remover o tema esquece a origem.
    uninstall(&w.root(), "Autor_TemaTeste").unwrap();
    assert!(remote::sources(&w.root()).is_empty());
}
