//! Lista os catálogos `locales/<idioma>/*.toml` (um por módulo, para PRs paralelas não brigarem pelo
//! mesmo arquivo); o `i18n` lê e junta todos.

use std::{env, fs, path::Path};

fn main() {
    let out = env::var("OUT_DIR").expect("OUT_DIR");
    println!("cargo:rerun-if-changed=locales");
    for lang in ["pt-BR", "en-US"] {
        let dir = Path::new("locales").join(lang);
        let mut files: Vec<_> = fs::read_dir(&dir)
            .map(|d| {
                d.filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x == "toml"))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        // Cada arquivo é um documento TOML próprio (dois módulos podem ter a mesma tabela, como `[library]`).
        let manifest = env::var("CARGO_MANIFEST_DIR").expect("dir");
        let mut code = String::from("&[\n");
        for f in files {
            println!("cargo:rerun-if-changed={}", f.display());
            let abs = Path::new(&manifest).join(&f);
            code.push_str(&format!(
                "    ({:?}, include_str!({:?})),\n",
                f.display().to_string(),
                abs.display().to_string()
            ));
        }
        code.push_str("]\n");
        fs::write(Path::new(&out).join(format!("{lang}.rs")), code).expect("gravar catálogo");
    }
}
