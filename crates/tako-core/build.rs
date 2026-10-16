//! Junta os catálogos `locales/<idioma>/*.toml` num só arquivo por idioma (cada módulo tem o seu
//! arquivo, para PRs paralelas não brigarem pelo mesmo catálogo).

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
        let mut all = String::new();
        for f in files {
            println!("cargo:rerun-if-changed={}", f.display());
            all.push_str(&fs::read_to_string(&f).expect("ler catálogo"));
            all.push('\n');
        }
        fs::write(Path::new(&out).join(format!("{lang}.toml")), all).expect("gravar catálogo");
    }
}
