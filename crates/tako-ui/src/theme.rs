//! Carrega o tema: o padrão (embutido) ou o escolhido pelo usuário, com volta ao padrão se falhar (PRD §7).

use slint_interpreter::{Compiler, ComponentDefinition};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tako_core::i18n::tf;

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

/// Destaque personalizado do tema (sobreposição salva, rascunho em teste ou prévia ao vivo).
pub fn custom_theme_accent(ctx: &crate::shell::Ctx) -> Option<tako_core::presentation::Rgb> {
    if let Some(acc) = ctx
        .theme_preview
        .as_ref()
        .and_then(|p| p.draft_override.as_ref())
        .and_then(|ov| ov.accent.as_ref())
    {
        return tako_core::presentation::Rgb::parse(acc);
    }
    if let Some(ref d) = ctx.theme_draft {
        if let Some(ref acc) = d.accent {
            return tako_core::presentation::Rgb::parse(acc);
        }
    }
    let selected_id = tako_themes::selected(&ctx.paths.themes());
    let theme_id = selected_id.as_deref();
    if let Some(text) = tako_themes::overrides::load_override(&ctx.paths.themes(), theme_id) {
        let ov = tako_themes::overrides::ThemeTokensOverride::from_toml_str(&text);
        if let Some(acc) = ov.accent {
            return tako_core::presentation::Rgb::parse(&acc);
        }
    }
    None
}

/// Carrega imagem de fundo com limites defensivos (32 MB, 16384 px, 512 MB de alocação).
/// Em caso de erro, devolve None para cair no padrão do tema sem travar ou entrar em loop de falha.
pub fn load_safe_image(path: &Path) -> Option<slint::Image> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > tako_themes::overrides::MAX_IMAGE_BYTES {
        return None;
    }
    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(tako_themes::overrides::MAX_IMAGE_DIMENSION);
    limits.max_image_height = Some(tako_themes::overrides::MAX_IMAGE_DIMENSION);
    limits.max_alloc = Some(512 * 1024 * 1024);
    let mut reader = reader;
    reader.limits(limits);
    let img = reader.decode().ok()?;
    let rgba = img.to_rgba8();
    let buf =
        slint::SharedPixelBuffer::clone_from_slice(rgba.as_raw(), rgba.width(), rgba.height());
    Some(crate::images::shared(buf))
}

/// `tokens.toml` opcional na pasta do tema ou na sobreposição: troca cores, fontes e fundo do global `Ui` (THEME-003).
pub fn apply_tokens(ui: &slint_interpreter::ComponentInstance, dir: Option<&Path>) {
    apply_tokens_with_override(ui, dir, None, None);
}

/// Aplica tokens combinando o `tokens.toml` original do tema e a sobreposição (THEME-003).
pub fn apply_tokens_with_override(
    ui: &slint_interpreter::ComponentInstance,
    theme_dir: Option<&Path>,
    override_text: Option<&str>,
    override_dir: Option<&Path>,
) {
    let mut table: toml::Table = theme_dir
        .and_then(|d| std::fs::read_to_string(d.join("tokens.toml")).ok())
        .and_then(|t| t.parse().ok())
        .unwrap_or_default();

    let override_table: Option<toml::Table> = if let Some(text) = override_text {
        text.parse().ok()
    } else if let Some(d) = override_dir {
        std::fs::read_to_string(d.join("tokens.toml"))
            .ok()
            .and_then(|t| t.parse().ok())
    } else {
        None
    };

    if let Some(ov) = override_table {
        for (k, v) in ov {
            table.insert(k, v);
        }
    }

    if table.is_empty() {
        crate::bridge::set_global(
            ui,
            "Ui",
            "has-custom-background",
            slint_interpreter::Value::Bool(false),
        );
        return;
    }

    let mut has_custom_bg = false;

    for (name, value) in table {
        if name == "font"
            || name == "font-title"
            || name == "font-text"
            || name == "font-heading"
            || name == "font-desk"
            || name == "font-wordmark"
        {
            if let Some(f) = value.as_str() {
                if name == "font" {
                    for target in [
                        "font-title",
                        "font-text",
                        "font-heading",
                        "font-desk",
                        "font-wordmark",
                    ] {
                        crate::bridge::set_global(
                            ui,
                            "Ui",
                            target,
                            slint_interpreter::Value::String(f.into()),
                        );
                    }
                } else {
                    crate::bridge::set_global(
                        ui,
                        "Ui",
                        &name,
                        slint_interpreter::Value::String(f.into()),
                    );
                }
            }
        } else if name == "custom-background" || name == "background-image" {
            if let Some(bg_path) = value.as_str().filter(|s| !s.trim().is_empty()) {
                if !tako_themes::overrides::is_safe_background_path(bg_path) {
                    tracing::warn!("tokens.toml: custom-background inseguro ignorado: {bg_path}");
                    continue;
                }
                let resolved = if let Some(od) = override_dir {
                    let candidate = od.join(bg_path);
                    if candidate.is_file() {
                        Some(candidate)
                    } else {
                        theme_dir.map(|td| td.join(bg_path))
                    }
                } else {
                    theme_dir.map(|td| td.join(bg_path))
                };

                let img_file = resolved.unwrap_or_else(|| PathBuf::from(bg_path));
                if img_file.is_file() {
                    match load_safe_image(&img_file) {
                        Some(img) => {
                            crate::bridge::set_global(
                                ui,
                                "Ui",
                                "custom-background",
                                slint_interpreter::Value::Image(img),
                            );
                            crate::bridge::set_global(
                                ui,
                                "Ui",
                                "has-custom-background",
                                slint_interpreter::Value::Bool(true),
                            );
                            has_custom_bg = true;
                        }
                        None => {
                            tracing::warn!(
                                "falha ao carregar imagem de fundo {}: limites excedidos ou arquivo inválido",
                                img_file.display()
                            );
                        }
                    }
                }
            }
        } else {
            if name == "canvas" {
                if let Some(c_str) = value.as_str() {
                    if !tako_themes::overrides::validate_canvas(c_str) {
                        tracing::warn!(
                            "tokens.toml: canvas ignorado por contraste insuficiente (< 4.5:1)"
                        );
                        continue;
                    }
                }
            }
            match value.as_str().and_then(parse_color) {
                Some(c) => {
                    crate::bridge::set_global(
                        ui,
                        "Ui",
                        &name,
                        slint_interpreter::Value::Brush(slint::Brush::SolidColor(c)),
                    );
                    if name == "canvas" {
                        crate::bridge::set_global(
                            ui,
                            "Ui",
                            "desk-canvas",
                            slint_interpreter::Value::Brush(slint::Brush::SolidColor(c)),
                        );
                    }
                    if name == "accent" {
                        let deep = slint::Color::from_argb_u8(
                            c.alpha(),
                            (c.red() as f32 * 0.55) as u8,
                            (c.green() as f32 * 0.55) as u8,
                            (c.blue() as f32 * 0.55) as u8,
                        );
                        crate::bridge::set_global(
                            ui,
                            "Ui",
                            "accent-deep",
                            slint_interpreter::Value::Brush(slint::Brush::SolidColor(deep)),
                        );
                    }
                }
                None => tracing::warn!("tokens.toml: cor inválida em {name}"),
            }
        }
    }

    if !has_custom_bg {
        crate::bridge::set_global(
            ui,
            "Ui",
            "has-custom-background",
            slint_interpreter::Value::Bool(false),
        );
    }
}

pub fn parse_color(s: &str) -> Option<slint::Color> {
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
        let problems = token_problems(&text, &colors);
        if !problems.is_empty() {
            return Err(problems);
        }
    }
    Ok(manifest)
}

fn token_problems(text: &str, colors: &[String]) -> Vec<String> {
    let table: toml::Table = match text.parse() {
        Ok(t) => t,
        Err(e) => return vec![tf("themes.check.tokens_invalid", &[("error", e.message())])],
    };
    table
        .iter()
        .filter_map(|(name, value)| {
            if name == "font" || name.starts_with("font-") {
                if value.as_str().is_none() {
                    Some(tf("themes.check.token_invalid", &[("name", name)]))
                } else {
                    None
                }
            } else if name == "custom-background" || name == "background-image" {
                let Some(s) = value.as_str() else {
                    return Some(tf("themes.check.token_invalid", &[("name", name)]));
                };
                if !tako_themes::overrides::is_safe_background_path(s) {
                    Some(tf("themes.check.token_invalid", &[("name", name)]))
                } else {
                    None
                }
            } else if !colors.iter().any(|c| c == name) {
                Some(tf("themes.check.token_unknown", &[("name", name)]))
            } else if name == "canvas" {
                let Some(s) = value.as_str() else {
                    return Some(tf("themes.check.token_invalid", &[("name", name)]));
                };
                if !tako_themes::overrides::validate_canvas(s) {
                    Some(tf("themes.check.token_invalid", &[("name", name)]))
                } else {
                    None
                }
            } else if value.as_str().and_then(parse_color).is_none() {
                Some(tf("themes.check.token_invalid", &[("name", name)]))
            } else {
                None
            }
        })
        .collect()
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

    #[test]
    fn custom_background_and_canvas_validate_tokens() {
        let colors = vec!["canvas".to_string(), "accent".to_string()];

        // Caminhos inseguros são rejeitados
        let problems = token_problems("custom-background = \"/etc/passwd\"\n", &colors);
        assert!(!problems.is_empty(), "deveria rejeitar caminho absoluto");

        let problems = token_problems("custom-background = \"../secret.png\"\n", &colors);
        assert!(!problems.is_empty(), "deveria rejeitar caminho com ..");

        // Caminho seguro é aceito
        let problems = token_problems("custom-background = \"bg.png\"\n", &colors);
        assert!(
            problems.is_empty(),
            "deveria aceitar caminho relativo seguro"
        );

        // Canvas com contraste insuficiente (< 4.5:1 com branco) é rejeitado
        let problems = token_problems("canvas = \"#FFFFFF\"\n", &colors);
        assert!(
            !problems.is_empty(),
            "deveria rejeitar canvas sem contraste"
        );

        // Canvas escuro com bom contraste é aceito
        let problems = token_problems("canvas = \"#0E1116\"\n", &colors);
        assert!(
            problems.is_empty(),
            "deveria aceitar canvas com bom contraste"
        );
    }

    #[test]
    fn load_safe_image_checks_limits() {
        let tmp = tempfile::tempdir().unwrap();
        let valid_path = tmp.path().join("valid.png");
        let img = image::RgbaImage::new(32, 32);
        img.save(&valid_path).unwrap();

        // Imagem válida carrega
        let loaded = load_safe_image(&valid_path);
        assert!(loaded.is_some());

        // Arquivo corrupto/inválido retorna None sem entrar em loop de falha
        let bad_path = tmp.path().join("corrupt.png");
        std::fs::write(&bad_path, b"not an image").unwrap();
        assert!(load_safe_image(&bad_path).is_none());

        // Arquivo inexistente retorna None
        let missing = tmp.path().join("nonexistent.png");
        assert!(load_safe_image(&missing).is_none());
    }
}
