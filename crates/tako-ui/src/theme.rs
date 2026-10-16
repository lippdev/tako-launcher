//! Carrega o tema: o padrão (embutido) ou o escolhido pelo usuário, com volta ao padrão se falhar (PRD §7).

use slint_interpreter::{Compiler, ComponentDefinition};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/default_theme.rs"));
}

/// Nome da biblioteca que outros temas importam: `import { RowView } from "@tako/widgets.slint";`.
pub const LIBRARY: &str = "tako";

/// Grava o tema padrão em `<themes>/.builtin-<versão>` (uma vez por versão) e devolve a pasta.
pub fn install_builtin(themes_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = themes_dir.join(format!(".builtin-{}", env!("CARGO_PKG_VERSION")));
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
    result.component("App").ok_or_else(|| ThemeError {
        messages: vec!["o tema não exporta o componente App".into()],
    })
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
