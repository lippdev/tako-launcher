//! Carrega o tema: o padrão (embutido) ou o escolhido pelo usuário, com volta ao padrão se falhar (PRD §7).

use slint_interpreter::{Compiler, ComponentDefinition};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tako_core::i18n::tf;

fn is_audio_key(key: &str) -> bool {
    matches!(
        key,
        "audio"
            | "music"
            | "musica"
            | "move"
            | "mover"
            | "accept"
            | "confirmar"
            | "back"
            | "voltar"
    )
}

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/default_theme.rs"));
}

/// Nome da biblioteca que outros temas importam: `import { RowView } from "@tako/widgets.slint";`.
pub const LIBRARY: &str = "tako";

/// Grava o tema padrão em `<themes>/.builtin-<versão>` (uma vez por versão) e devolve a pasta.
pub fn install_builtin(themes_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = themes_dir.join(format!(".builtin-{}", crate::version()));
    if dir.join("main.slint").exists() && !cfg!(debug_assertions) {
        return Ok(dir);
    }
    let tmp = themes_dir.join(format!(".builtin-tmp-{}", std::process::id()));
    for (name, bytes) in embedded::FILES {
        let path = tmp.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)?;
    }
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::rename(&tmp, &dir)?;
    install_extras(themes_dir);
    Ok(dir)
}

/// Temas que vêm com o Tako (`theme/extras/<id>/`) vão para `<themes>/<id>/`, como um tema instalado, e são
/// regravados quando a versão embutida muda.
fn install_extras(themes_dir: &Path) {
    for (name, bytes) in embedded::FILES {
        let Some(rel) = name.strip_prefix("extras/") else {
            continue;
        };
        let path = themes_dir.join(rel);
        if std::fs::read(&path).is_ok_and(|old| old == *bytes) {
            continue;
        }
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&path, bytes));
        if let Err(e) = written {
            tracing::warn!("tema embutido {rel}: {e}");
        }
    }
}

/// `tokens.toml` opcional na pasta do tema: troca cores do global `Ui` (`canvas = "#000000"`, com ou sem alfa
/// `#RRGGBBAA`). Assim um tema reaproveita o padrão inteiro (`export { App } from "@tako/main.slint"`) e só
/// muda a paleta. Chave ou cor inválida é ignorada com log.
pub fn apply_tokens(ui: &slint_interpreter::ComponentInstance, dir: Option<&Path>) {
    let Some(text) = dir.and_then(|d| std::fs::read_to_string(d.join("tokens.toml")).ok()) else {
        return;
    };
    let table: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("tokens.toml: {e}");
            return;
        }
    };
    for (name, value) in table {
        if is_audio_key(&name) {
            continue;
        }
        match value.as_str().and_then(parse_color) {
            Some(c) => crate::bridge::set_global(
                ui,
                "Ui",
                &name,
                slint_interpreter::Value::Brush(slint::Brush::SolidColor(c)),
            ),
            None => tracing::warn!("tokens.toml: cor inválida em {name}"),
        }
    }
}

fn parse_color(s: &str) -> Option<slint::Color> {
    let hex = s.strip_prefix('#')?;
    let n = u32::from_str_radix(hex, 16).ok()?;
    let [a, r, g, b] = match hex.len() {
        6 => [255, (n >> 16) as u8, (n >> 8) as u8, n as u8],
        8 => [n as u8, (n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8],
        _ => return None,
    };
    Some(slint::Color::from_argb_u8(a, r, g, b))
}

#[derive(Debug)]
pub struct ThemeError {
    pub messages: Vec<String>,
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.messages.join("\n"))
    }
}

/// Compila `entry` com a biblioteca `@tako` apontando para o tema padrão. O tema precisa exportar `App`.
pub fn compile(entry: &Path, builtin: &Path) -> Result<ComponentDefinition, ThemeError> {
    compile_named(entry, builtin, "App")
}

/// Janela do modo desktop (ADR-0027): `desktop.slint` do tema padrão, fora da API de tema (temas valem só para a
/// tela cheia).
pub fn compile_desktop(builtin: &Path) -> Result<ComponentDefinition, ThemeError> {
    compile_named(&builtin.join("desktop.slint"), builtin, "Desktop")
}

/// Janela de abertura (TelaCarregamento): `splash.slint`, pequena para compilar rápido antes do resto.
pub fn compile_splash(builtin: &Path) -> Result<ComponentDefinition, ThemeError> {
    compile_named(&builtin.join("splash.slint"), builtin, "Splash")
}

fn compile_named(
    entry: &Path,
    builtin: &Path,
    component: &str,
) -> Result<ComponentDefinition, ThemeError> {
    let mut compiler = Compiler::default();
    compiler.set_library_paths(HashMap::from([(LIBRARY.to_owned(), builtin.to_path_buf())]));
    let result = spin_on::spin_on(compiler.build_from_path(entry));
    let errors: Vec<String> = result
        .diagnostics()
        .filter(|d| d.level() == slint_interpreter::DiagnosticLevel::Error)
        .map(|d| d.to_string())
        .collect();
    if !errors.is_empty() {
        return Err(ThemeError { messages: errors });
    }
    result.component(component).ok_or_else(|| ThemeError {
        messages: vec![format!("o tema não exporta o componente {component}")],
    })
}

/// `tako theme check <pasta ou .takotheme>` (THEME-001): as mesmas regras de instalar (manifesto, limites, caminhos,
/// tipos de arquivo) e a compilação do `.slint` com a biblioteca `@tako`, numa pasta temporária: nada é instalado.
/// Devolve o manifesto ou as mensagens de problema.
pub fn check(package: &Path) -> Result<tako_themes::ThemeManifest, Vec<String>> {
    let tmp = tempfile::tempdir().map_err(|e| vec![e.to_string()])?;
    let builtin = install_builtin(tmp.path()).map_err(|e| vec![e.to_string()])?;
    let installed = tako_themes::install(package, &tmp.path().join("check"))
        .map_err(|p| p.into_iter().map(|p| p.message).collect::<Vec<_>>())?;
    let manifest = installed
        .manifest
        .map_err(|p| p.into_iter().map(|p| p.message).collect::<Vec<_>>())?;
    let def = compile(&installed.dir.join(&manifest.entry), &builtin).map_err(|e| e.messages)?;
    // `tokens.toml`: o Tako ignora a chave ou a cor inválida ao carregar; aqui o autor fica sabendo.
    if let Ok(text) = std::fs::read_to_string(installed.dir.join("tokens.toml")) {
        let colors: Vec<String> = def
            .global_properties("Ui")
            .map(|p| {
                p.filter(|(_, t)| *t == slint_interpreter::ValueType::Brush)
                    .map(|(n, _)| n)
                    .collect()
            })
            .unwrap_or_default();
        let problems = token_problems_with_dir(&text, &colors, Some(&installed.dir));
        if !problems.is_empty() {
            return Err(problems);
        }
    }
    Ok(manifest)
}

fn token_problems_with_dir(text: &str, colors: &[String], dir: Option<&Path>) -> Vec<String> {
    let table: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => return vec![tf("themes.check.tokens_invalid", &[("error", e.message())])],
    };
    let mut problems = Vec::new();
    match tako_themes::parse_tokens_audio(text) {
        Ok(audio) => {
            if let Some(dir) = dir {
                if let Some(ref m) = audio.music {
                    let p = dir.join(m);
                    if !p.exists() {
                        problems.push(tf("themes.package.missing_audio", &[("path", m)]));
                    } else if let Ok(meta) = p.metadata() {
                        if meta.len() == 0 {
                            problems.push(tf("themes.package.audio_invalid", &[("path", m)]));
                        } else if meta.len() > tako_themes::MAX_THEME_MUSIC_BYTES {
                            problems.push(tf(
                                "themes.package.audio_too_large",
                                &[
                                    ("path", m),
                                    (
                                        "max",
                                        &format!("{} MB", tako_themes::MAX_THEME_MUSIC_BYTES / (1024 * 1024)),
                                    ),
                                ],
                            ));
                        } else if tako_themes::wav_duration(&p).is_none() {
                            problems.push(tf("themes.package.audio_invalid", &[("path", m)]));
                        }
                    }
                }
                for sound in [&audio.move_sound, &audio.accept_sound, &audio.back_sound]
                    .into_iter()
                    .flatten()
                {
                    let p = dir.join(sound);
                    if !p.exists() {
                        problems.push(tf("themes.package.missing_audio", &[("path", sound)]));
                    } else {
                        match tako_themes::wav_duration(&p) {
                            None => problems.push(tf("themes.package.audio_invalid", &[("path", sound)])),
                            Some(dur) if dur <= 0.0 => {
                                problems.push(tf("themes.package.audio_invalid", &[("path", sound)]));
                            }
                            Some(dur) if dur > tako_themes::MAX_NAV_SOUND_DURATION_SECS => {
                                problems.push(tf(
                                    "themes.package.sound_too_long",
                                    &[
                                        ("path", sound),
                                        ("duration", &format!("{dur:.1}s")),
                                        ("max", "2s"),
                                    ],
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        Err(probs) => {
            problems.extend(probs.into_iter().map(|p| p.message));
        }
    }
    for (name, value) in &table {
        if is_audio_key(name) {
            continue;
        }
        if !colors.iter().any(|c| c == name) {
            problems.push(tf("themes.check.token_unknown", &[("name", name)]));
        } else if value.as_str().and_then(parse_color).is_none() {
            problems.push(tf("themes.check.token_invalid", &[("name", name)]));
        }
    }
    problems
}

pub fn token_problems(text: &str, colors: &[String]) -> Vec<String> {
    token_problems_with_dir(text, colors, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_theme_compiles() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = install_builtin(tmp.path()).unwrap();
        let def = compile(&dir.join("main.slint"), &dir).unwrap_or_else(|e| panic!("{e}"));
        let props: Vec<String> = def.properties().map(|(n, _)| n).collect();
        for p in ["page", "library-tiles", "overlays", "hints", "sidebar"] {
            assert!(props.iter().any(|x| x == p), "falta a propriedade {p}");
        }
    }

    #[test]
    fn desktop_window_compiles_with_the_properties_the_tako_sets() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = install_builtin(tmp.path()).unwrap();
        let def = compile_desktop(&dir).unwrap_or_else(|e| panic!("{e}"));
        let props: Vec<String> = def.properties().map(|(n, _)| n).collect();
        for p in [
            "page",
            "overlays",
            "desk-games",
            "desk-game",
            "desk-actions",
            "settings-rows",
            "desk-profile-name",
            "desk-profile-initial",
            "window-drag-start",
            "window-drag-end",
        ] {
            assert!(props.iter().any(|x| x == p), "falta a propriedade {p}");
        }
        let callbacks: Vec<String> = def.callbacks().collect();
        for c in ["clicked", "edited", "scrolled", "key", "window-action"] {
            assert!(callbacks.iter().any(|x| x == c), "falta o callback {c}");
        }
    }

    #[test]
    fn bundled_alternative_theme_installs_and_compiles() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = install_builtin(tmp.path()).unwrap();
        let alt = tmp.path().join("tako-alto-contraste");
        let def = compile(&alt.join("main.slint"), &dir).unwrap_or_else(|e| panic!("{e}"));
        assert!(def.properties().any(|(n, _)| n == "library-tiles"));
        // Os tokens do tema são todos cores válidas.
        let tokens: toml::Table = std::fs::read_to_string(alt.join("tokens.toml"))
            .unwrap()
            .parse()
            .unwrap();
        assert!(
            tokens
                .values()
                .all(|v| v.as_str().and_then(parse_color).is_some())
        );
        assert_eq!(
            parse_color("#11223344"),
            Some(slint::Color::from_argb_u8(0x44, 0x11, 0x22, 0x33))
        );
        assert_eq!(parse_color("red"), None);
        assert_eq!(
            tako_themes::InstalledTheme::scan(tmp.path())
                .iter()
                .filter(|t| t.id() == "tako-alto-contraste")
                .count(),
            1
        );
    }

    #[test]
    fn the_public_theme_docs_cover_every_property_and_callback_of_app() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = install_builtin(tmp.path()).unwrap();
        let def = compile(&dir.join("main.slint"), &dir).unwrap();
        for doc in [
            include_str!("../../../docs/themes/README.md"),
            include_str!("../../../docs/themes/README.en-US.md"),
        ] {
            let names = def.properties().map(|(n, _)| n).chain(def.callbacks());
            for name in names {
                assert!(
                    doc.contains(&format!("`{name}`"))
                        || doc.contains(&format!("`{name}("))
                        || doc.contains(&format!(", `{name}")),
                    "falta {name} na documentação"
                );
            }
        }
    }

    #[test]
    fn theme_check_validates_without_installing() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let tmp = tempfile::tempdir().unwrap();
        install_builtin(tmp.path()).unwrap();
        let alt = tmp.path().join("tako-alto-contraste");
        let m = check(&alt).unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(m.id, "tako-alto-contraste");
        // Token que não existe e cor quebrada.
        std::fs::write(
            alt.join("tokens.toml"),
            "canvas = \"preto\"
nada = \"#000000\"
",
        )
        .unwrap();
        let e = check(&alt).unwrap_err();
        assert_eq!(e.len(), 2, "{e:?}");
        assert!(e.iter().any(|p| p.contains("nada")), "{e:?}");
        // `.slint` que não compila.
        std::fs::remove_file(alt.join("tokens.toml")).unwrap();
        std::fs::write(
            alt.join("main.slint"),
            "export component App inherits Window { x: ; }",
        )
        .unwrap();
        assert!(check(&alt).is_err());
        // Manifesto faltando.
        std::fs::remove_file(alt.join("theme.toml")).unwrap();
        assert!(check(&alt).is_err());
    }


    #[test]
    fn theme_check_validates_audio_limits() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let tmp = tempfile::tempdir().unwrap();
        install_builtin(tmp.path()).unwrap();
        let alt = tmp.path().join("tako-alto-contraste");

        let sample_rate = 44100u32;
        let channels = 1u16;
        let bits_per_sample = 16u16;
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
        let block_align = channels * (bits_per_sample / 8);
        let data_len = (2.5 * byte_rate as f64).round() as u32;
        let riff_size = 36 + data_len;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&riff_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.resize(44 + data_len as usize, 0);

        std::fs::write(alt.join("long_click.wav"), &wav).unwrap();
        std::fs::write(
            alt.join("tokens.toml"),
            "confirmar = \"long_click.wav\"\n",
        )
        .unwrap();
        let e = check(&alt).unwrap_err();
        assert!(e.iter().any(|msg| msg.contains("2s")), "{e:?}");

        std::fs::write(alt.join("bad.wav"), b"NOT_WAV").unwrap();
        std::fs::write(
            alt.join("tokens.toml"),
            "confirmar = \"bad.wav\"\n",
        )
        .unwrap();
        let e2 = check(&alt).unwrap_err();
        assert!(e2.iter().any(|msg| msg.contains("WAV") || msg.contains("corrompido")), "{e2:?}");
    }
    #[test]
    fn broken_theme_reports_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = install_builtin(tmp.path()).unwrap();
        let bad = tmp.path().join("bad.slint");
        std::fs::write(
            &bad,
            "export component App inherits Window { nao-existe: 1; }",
        )
        .unwrap();
        assert!(compile(&bad, &dir).is_err());
    }
}
