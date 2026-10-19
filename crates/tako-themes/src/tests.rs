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

fn dummy_wav(duration_secs: f64) -> Vec<u8> {
    let sample_rate: u32 = 44100;
    let channels: u16 = 1;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let num_samples = (sample_rate as f64 * duration_secs) as u32;
    let data_bytes = num_samples * block_align as u32;
    let riff_size = 36 + data_bytes;

    let mut v = Vec::with_capacity(44 + data_bytes as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&riff_size.to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&sample_rate.to_le_bytes());
    v.extend_from_slice(&byte_rate.to_le_bytes());
    v.extend_from_slice(&block_align.to_le_bytes());
    v.extend_from_slice(&bits_per_sample.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_bytes.to_le_bytes());
    v.resize(44 + data_bytes as usize, 0);
    v
}

#[test]
fn resolves_theme_audio_and_tokens_override_with_path_safety() {
    let w = Work::new();
    let theme_dir = w.root().join("tema-audio");
    fs::create_dir_all(&theme_dir).unwrap();
    fs::write(
        theme_dir.join("theme.toml"),
        format!("{TOML}\n[audio]\nmusic = \"bgm.wav\"\nmove = \"mover.wav\"\n"),
    )
    .unwrap();
    fs::write(theme_dir.join("main.slint"), SLINT).unwrap();
    fs::write(theme_dir.join("bgm.wav"), dummy_wav(1.0)).unwrap();
    fs::write(theme_dir.join("mover.wav"), dummy_wav(0.5)).unwrap();

    // Default from theme.toml
    let audio = resolve_theme_audio(&theme_dir);
    assert_eq!(audio.music, Some(theme_dir.join("bgm.wav")));
    assert_eq!(audio.move_sound, Some(theme_dir.join("mover.wav")));
    assert_eq!(audio.accept_sound, None);
    assert_eq!(audio.back_sound, None);

    // Override with tokens.toml
    fs::write(
        theme_dir.join("tokens.toml"),
        "[audio]\nmusic = \"tokens_bgm.wav\"\naccept = \"confirm.wav\"\n",
    )
    .unwrap();
    fs::write(theme_dir.join("tokens_bgm.wav"), dummy_wav(1.0)).unwrap();
    fs::write(theme_dir.join("confirm.wav"), dummy_wav(0.5)).unwrap();

    let audio = resolve_theme_audio(&theme_dir);
    assert_eq!(audio.music, Some(theme_dir.join("tokens_bgm.wav")));
    assert_eq!(audio.move_sound, Some(theme_dir.join("mover.wav")));
    assert_eq!(audio.accept_sound, Some(theme_dir.join("confirm.wav")));

    // Path traversal attempt in tokens.toml is ignored and safely rejected (file outside actually exists)
    let outside_path = w.root().join("outside.wav");
    fs::write(&outside_path, dummy_wav(0.5)).unwrap();
    fs::write(
        theme_dir.join("tokens.toml"),
        "[audio]\nmusic = \"../outside.wav\"\n",
    )
    .unwrap();
    let audio = resolve_theme_audio(&theme_dir);
    assert_eq!(audio.music, Some(theme_dir.join("bgm.wav")));

    // Absolute path is rejected
    let abs_str = outside_path.to_string_lossy();
    fs::write(
        theme_dir.join("tokens.toml"),
        format!("[audio]\nmusic = \"{abs_str}\"\n"),
    )
    .unwrap();
    let audio = resolve_theme_audio(&theme_dir);
    assert_eq!(audio.music, Some(theme_dir.join("bgm.wav")));

    // Symlink pointing outside is rejected
    #[cfg(unix)]
    {
        let symlink_path = theme_dir.join("symlink_outside.wav");
        let _ = std::os::unix::fs::symlink(&outside_path, &symlink_path);
        fs::write(
            theme_dir.join("tokens.toml"),
            "[audio]\nmove = \"symlink_outside.wav\"\n",
        )
        .unwrap();
        let audio = resolve_theme_audio(&theme_dir);
        assert_eq!(audio.move_sound, Some(theme_dir.join("mover.wav")));
    }

    // Navigation sound duration > 2.0s is rejected
    fs::write(theme_dir.join("long_move.wav"), dummy_wav(2.5)).unwrap();
    fs::write(
        theme_dir.join("tokens.toml"),
        "[audio]\nmove = \"long_move.wav\"\n",
    )
    .unwrap();
    let audio = resolve_theme_audio(&theme_dir);
    assert_eq!(audio.move_sound, Some(theme_dir.join("mover.wav")));

    // Navigation sound duration <= 2.0s is accepted
    fs::write(theme_dir.join("short_move.wav"), dummy_wav(1.8)).unwrap();
    fs::write(
        theme_dir.join("tokens.toml"),
        "[audio]\nmove = \"short_move.wav\"\n",
    )
    .unwrap();
    let audio = resolve_theme_audio(&theme_dir);
    assert_eq!(audio.move_sound, Some(theme_dir.join("short_move.wav")));

    // Music > 32 MB is rejected
    let big_music = theme_dir.join("big_music.wav");
    let file = fs::File::create(&big_music).unwrap();
    file.set_len(crate::MAX_THEME_MUSIC_BYTES + 1024).unwrap();
    drop(file);
    fs::write(
        theme_dir.join("tokens.toml"),
        "[audio]\nmusic = \"big_music.wav\"\n",
    )
    .unwrap();
    let audio = resolve_theme_audio(&theme_dir);
    assert_eq!(audio.music, Some(theme_dir.join("bgm.wav")));
}

#[test]
fn test_theme_name_validation_and_id_derivation() {
    use crate::create::{ThemeNameProblem, derive_theme_id, validate_theme_name};

    assert_eq!(validate_theme_name(""), Err(ThemeNameProblem::Empty));
    assert_eq!(validate_theme_name("   "), Err(ThemeNameProblem::Empty));
    let long_name = "a".repeat(51);
    assert_eq!(
        validate_theme_name(&long_name),
        Err(ThemeNameProblem::TooLong(50))
    );
    assert_eq!(
        validate_theme_name("Meu/Tema"),
        Err(ThemeNameProblem::InvalidCharacters)
    );
    assert_eq!(
        validate_theme_name("Meu\\Tema"),
        Err(ThemeNameProblem::InvalidCharacters)
    );
    assert_eq!(
        validate_theme_name("Tema:Novo"),
        Err(ThemeNameProblem::InvalidCharacters)
    );
    assert_eq!(
        validate_theme_name("Tema..Novo"),
        Err(ThemeNameProblem::InvalidCharacters)
    );
    assert!(validate_theme_name("Meu Tema 2026").is_ok());
    assert!(validate_theme_name("Tema Alto-Contraste").is_ok());

    assert_eq!(derive_theme_id("Meu Tema 2026!"), "meu-tema-2026");
    assert_eq!(
        derive_theme_id("Tema com Acentos (São Paulo)"),
        "tema-com-acentos-sao-paulo"
    );
    assert_eq!(derive_theme_id("..."), "meu-tema");
    assert_ne!(derive_theme_id("default"), "default");
}

#[test]
fn test_create_theme_from_default_and_installed_and_reject_duplicate() {
    use crate::create::{
        CreateThemeError, CreateThemeOptions, create_theme, is_tokens_only_theme, uses_default_ui,
    };
    use crate::overrides::ThemeTokensOverride;

    let tmp = tempfile::tempdir().unwrap();
    let themes_root = tmp.path().join("themes");

    let mut tokens = ThemeTokensOverride {
        accent: Some("#6C5CE7".into()),
        font: Some("Poppins".into()),
        canvas: Some("#0E1116".into()),
        ..ThemeTokensOverride::default()
    };

    let opts = CreateThemeOptions {
        name: "Meu Tema Púrpura",
        custom_id: None,
        author: Some("Criador"),
        version: Some("1.0.0"),
        description: Some("Tema criado para testes"),
        tokens: &tokens,
        background_image_source: None,
        overwrite: false,
    };

    let installed = create_theme(&themes_root, &opts).unwrap();
    assert_eq!(installed.id(), "meu-tema-purpura");
    assert!(installed.dir.join("theme.toml").is_file());
    assert!(installed.dir.join("main.slint").is_file());
    assert!(installed.dir.join("tokens.toml").is_file());
    assert!(uses_default_ui(&installed.dir));
    assert!(is_tokens_only_theme(&installed.dir));

    let toml_str = fs::read_to_string(installed.dir.join("tokens.toml")).unwrap();
    assert!(toml_str.contains("accent = \"#6C5CE7\""));
    assert!(toml_str.contains("font = \"Poppins\""));

    // Tentar criar de novo sem overwrite deve falhar com AlreadyExists
    let err = create_theme(&themes_root, &opts).unwrap_err();
    assert!(matches!(err, CreateThemeError::AlreadyExists(id) if id == "meu-tema-purpura"));

    // Com overwrite = true, deve substituir com sucesso
    tokens.accent = Some("#0B7A3B".into());
    let opts_overwrite = CreateThemeOptions {
        name: "Meu Tema Púrpura",
        custom_id: None,
        author: Some("Criador"),
        version: Some("1.0.1"),
        description: Some("Tema atualizado"),
        tokens: &tokens,
        background_image_source: None,
        overwrite: true,
    };
    let updated = create_theme(&themes_root, &opts_overwrite).unwrap();
    let updated_toml = fs::read_to_string(updated.dir.join("tokens.toml")).unwrap();
    assert!(updated_toml.contains("accent = \"#0B7A3B\""));

    // Criar um novo tema a partir do tema instalado como base
    let base_tokens = ThemeTokensOverride::from_toml_str(&updated_toml);
    let mut derived_tokens = base_tokens.clone();
    derived_tokens.accent = Some("#E3161E".into());

    let opts_derived = CreateThemeOptions {
        name: "Tema Derivado",
        custom_id: None,
        author: Some("Outro"),
        version: Some("1.0.0"),
        description: None,
        tokens: &derived_tokens,
        background_image_source: None,
        overwrite: false,
    };
    let derived = create_theme(&themes_root, &opts_derived).unwrap();
    assert_eq!(derived.id(), "tema-derivado");
    let derived_toml = fs::read_to_string(derived.dir.join("tokens.toml")).unwrap();
    assert!(derived_toml.contains("accent = \"#E3161E\""));
    assert!(derived_toml.contains("font = \"Poppins\""));
}

#[test]
fn test_export_and_install_round_trip_with_background_image() {
    use crate::create::{CreateThemeOptions, create_theme};
    use crate::export::export_theme;
    use crate::install;
    use crate::overrides::ThemeTokensOverride;

    let tmp = tempfile::tempdir().unwrap();
    let themes_root = tmp.path().join("themes");
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    // Cria uma imagem PNG de teste
    let img = image::RgbaImage::new(32, 32);
    let img_path = src_dir.join("meu-wallpaper.png");
    img.save(&img_path).unwrap();

    let tokens = ThemeTokensOverride {
        accent: Some("#2B3A55".into()),
        ..ThemeTokensOverride::default()
    };

    let opts = CreateThemeOptions {
        name: "Tema com Wallpaper",
        custom_id: None,
        author: Some("Artista"),
        version: Some("1.0.0"),
        description: Some("Tema com imagem"),
        tokens: &tokens,
        background_image_source: Some(&img_path),
        overwrite: false,
    };

    let created = create_theme(&themes_root, &opts).unwrap();
    assert!(created.dir.join("background.png").is_file());
    let tokens_content = fs::read_to_string(created.dir.join("tokens.toml")).unwrap();
    assert!(tokens_content.contains("custom-background = \"background.png\""));

    // Exporta para .takotheme
    let export_dir = tmp.path().join("exported");
    fs::create_dir_all(&export_dir).unwrap();
    let pkg_path = export_dir.join("tema-com-wallpaper.takotheme");
    export_theme(&created.dir, &pkg_path).unwrap();
    assert!(pkg_path.is_file());

    // Instala em outra pasta themes_root_2
    let themes_root_2 = tmp.path().join("themes_dest");
    let installed = install(&pkg_path, &themes_root_2).unwrap();
    assert_eq!(installed.id(), "tema-com-wallpaper");
    assert!(installed.dir.join("background.png").is_file());
    assert!(installed.dir.join("tokens.toml").is_file());

    let installed_tokens = fs::read_to_string(installed.dir.join("tokens.toml")).unwrap();
    assert_eq!(installed_tokens, tokens_content);

    // Valida o pacote instalado
    let manifest = crate::package::validate_package(&installed.dir).unwrap();
    assert_eq!(manifest.name, "Tema com Wallpaper");
    assert_eq!(manifest.author, "Artista");
}

#[test]
fn test_reserved_theme_ids_and_derivation() {
    use crate::create::{
        CreateThemeError, CreateThemeOptions, create_theme, derive_theme_id, is_reserved_theme_id,
        unique_theme_id,
    };
    use crate::overrides::ThemeTokensOverride;

    assert!(is_reserved_theme_id("default"));
    assert!(is_reserved_theme_id("tako-default"));
    assert!(is_reserved_theme_id("tako-fita"));
    assert!(is_reserved_theme_id("tako-alto-contraste"));
    assert!(!is_reserved_theme_id("meu-tema"));

    // Derivação deve desviar de ids reservados
    assert_eq!(derive_theme_id("default"), "default-tema");
    assert_eq!(derive_theme_id("Tako Fita"), "tako-fita-tema");
    assert_eq!(
        derive_theme_id("Tako Alto Contraste"),
        "tako-alto-contraste-tema"
    );

    let tmp = tempfile::tempdir().unwrap();
    let themes_root = tmp.path().join("themes");
    fs::create_dir_all(&themes_root).unwrap();

    let tokens = ThemeTokensOverride::default();
    let opts_reserved = CreateThemeOptions {
        name: "Fita Oficial",
        custom_id: Some("tako-fita"),
        author: None,
        version: None,
        description: None,
        tokens: &tokens,
        background_image_source: None,
        overwrite: true,
    };
    let err = create_theme(&themes_root, &opts_reserved).unwrap_err();
    assert!(matches!(err, CreateThemeError::Reserved(id) if id == "tako-fita"));

    // unique_theme_id nunca devolve id reservado
    let u1 = unique_theme_id(&themes_root, "Tako Fita");
    assert_ne!(u1, "tako-fita");
    assert_eq!(u1, "tako-fita-tema");
}

#[test]
fn test_cannot_overwrite_third_party_or_non_tako_theme() {
    use crate::create::{
        CreateThemeError, CreateThemeOptions, create_theme, is_tako_created_theme,
    };
    use crate::overrides::ThemeTokensOverride;

    let tmp = tempfile::tempdir().unwrap();
    let themes_root = tmp.path().join("themes");
    let third_party = themes_root.join("tema-terceiro");
    fs::create_dir_all(&third_party).unwrap();

    // Manifesto sem a marca created-by-tako
    let manifest = "id = \"tema-terceiro\"\nname = \"Terceiro\"\nversion = \"1.0\"\nauthor = \"Autor Externo\"\napi = 1\nentry = \"main.slint\"\n";
    fs::write(third_party.join("theme.toml"), manifest).unwrap();
    fs::write(
        third_party.join("main.slint"),
        "export { App, Ui, Hint, PanelRow } from \"@tako/main.slint\";\n",
    )
    .unwrap();

    assert!(!is_tako_created_theme(&third_party));

    let tokens = ThemeTokensOverride::default();
    let opts = CreateThemeOptions {
        name: "Tema Terceiro",
        custom_id: None,
        author: Some("Outro"),
        version: Some("2.0.0"),
        description: None,
        tokens: &tokens,
        background_image_source: None,
        overwrite: true,
    };
    let err = create_theme(&themes_root, &opts).unwrap_err();
    assert!(matches!(err, CreateThemeError::CannotOverwrite(id) if id == "tema-terceiro"));
}

#[test]
fn test_overwrite_resets_old_overrides_directory() {
    use crate::create::{CreateThemeOptions, create_theme};
    use crate::overrides::{ThemeTokensOverride, save_override};

    let tmp = tempfile::tempdir().unwrap();
    let themes_root = tmp.path().join("themes");

    let tokens = ThemeTokensOverride {
        accent: Some("#6C5CE7".into()),
        ..Default::default()
    };
    let opts = CreateThemeOptions {
        name: "Tema Sobrescrito",
        custom_id: None,
        author: None,
        version: None,
        description: None,
        tokens: &tokens,
        background_image_source: None,
        overwrite: false,
    };
    let created = create_theme(&themes_root, &opts).unwrap();
    let id = created.id();

    // Cria sobreposições antigas para este tema
    save_override(&themes_root, Some(&id), "accent = \"#123456\"\n").unwrap();
    assert!(themes_root.join(".overrides").join(&id).is_dir());

    // Sobrescreve o tema
    let opts_overwrite = CreateThemeOptions {
        overwrite: true,
        ..opts
    };
    create_theme(&themes_root, &opts_overwrite).unwrap();

    // A pasta .overrides/<id> deve ter sido limpa pelo reset_override
    assert!(!themes_root.join(".overrides").join(&id).exists());
}

#[test]
fn test_uses_default_ui_rejects_fita_and_accepts_reexport() {
    use crate::create::uses_default_ui;

    let tmp = tempfile::tempdir().unwrap();
    let fita_dir = tmp.path().join("fita");
    fs::create_dir_all(&fita_dir).unwrap();
    fs::write(
        fita_dir.join("theme.toml"),
        "id = \"fita\"\nname = \"Fita\"\nversion = \"1.0\"\nauthor = \"Tako\"\napi = 1\nentry = \"main.slint\"\n",
    ).unwrap();
    // Simula o main.slint do Fita: importa do @tako/main.slint mas define seu próprio App
    fs::write(
        fita_dir.join("main.slint"),
        "import { HomeRow } from \"@tako/main.slint\";\nexport component App inherits Window {}\n",
    )
    .unwrap();

    assert!(!uses_default_ui(&fita_dir));

    let alto_dir = tmp.path().join("alto");
    fs::create_dir_all(&alto_dir).unwrap();
    fs::write(
        alto_dir.join("theme.toml"),
        "id = \"alto\"\nname = \"Alto\"\nversion = \"1.0\"\nauthor = \"Tako\"\napi = 1\nentry = \"main.slint\"\n",
    ).unwrap();
    fs::write(
        alto_dir.join("main.slint"),
        "export { App, Ui, Hint, PanelRow } from \"@tako/main.slint\";\n",
    )
    .unwrap();

    assert!(uses_default_ui(&alto_dir));
}

#[test]
fn test_old_dir_preserved_if_not_matching() {
    use crate::create::{CreateThemeOptions, create_theme};
    use crate::overrides::ThemeTokensOverride;

    let tmp = tempfile::tempdir().unwrap();
    let themes_root = tmp.path().join("themes");
    fs::create_dir_all(&themes_root).unwrap();

    // Cria uma pasta .old-meu-tema que NÃO é cópia antiga deste id (ex: pasta de outro tema ou corrompida)
    let old_dir = themes_root.join(".old-meu-tema");
    fs::create_dir_all(&old_dir).unwrap();
    fs::write(old_dir.join("arquivo_importante.txt"), "dados").unwrap();

    let tokens = ThemeTokensOverride::default();
    let opts = CreateThemeOptions {
        name: "Meu Tema",
        custom_id: None,
        author: None,
        version: None,
        description: None,
        tokens: &tokens,
        background_image_source: None,
        overwrite: false,
    };
    create_theme(&themes_root, &opts).unwrap();

    // A pasta com dados desconhecidos não deve ter sido destruída, mas sim renomeada com sufixo
    assert!(themes_root.join(".old-meu-tema-bak-1").is_dir());
    assert_eq!(
        fs::read_to_string(themes_root.join(".old-meu-tema-bak-1/arquivo_importante.txt")).unwrap(),
        "dados"
    );
}

#[test]
fn test_background_source_name_is_zeroed_in_created_theme() {
    use crate::create::{CreateThemeOptions, create_theme};
    use crate::overrides::ThemeTokensOverride;

    let tmp = tempfile::tempdir().unwrap();
    let themes_root = tmp.path().join("themes");

    let tokens = ThemeTokensOverride {
        background_source_name: Some("minha_foto_privada.png".into()),
        ..Default::default()
    };

    let opts = CreateThemeOptions {
        name: "Tema Sem Vazamento",
        custom_id: None,
        author: None,
        version: None,
        description: None,
        tokens: &tokens,
        background_image_source: None,
        overwrite: false,
    };
    let created = create_theme(&themes_root, &opts).unwrap();
    let tokens_text = fs::read_to_string(created.dir.join("tokens.toml")).unwrap();
    assert!(!tokens_text.contains("custom-background-name"));
    assert!(!tokens_text.contains("minha_foto_privada.png"));
}
