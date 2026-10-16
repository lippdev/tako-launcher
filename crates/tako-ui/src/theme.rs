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
    Ok(dir)
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
