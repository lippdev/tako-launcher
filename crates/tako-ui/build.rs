//! Embute os arquivos do tema padrão (`theme/`) no binário; na primeira execução eles vão para a pasta
//! de dados, de onde o intérprete do Slint os carrega (o mesmo caminho de qualquer tema).

use std::{env, fs, path::Path};

fn main() {
    let out = env::var("OUT_DIR").expect("OUT_DIR");
    println!("cargo:rerun-if-changed=theme");
    let mut files: Vec<String> = Vec::new();
    collect(Path::new("theme"), Path::new(""), &mut files);
    files.sort();
    let mut code = String::from("pub const FILES: &[(&str, &[u8])] = &[\n");
    for f in &files {
        let abs = Path::new(&env::var("CARGO_MANIFEST_DIR").expect("dir"))
            .join("theme")
            .join(f);
        code.push_str(&format!(
            "    ({f:?}, include_bytes!({:?})),\n",
            abs.display().to_string()
        ));
    }
    code.push_str("];\n");
    fs::write(Path::new(&out).join("default_theme.rs"), code).expect("gravar");
}

fn collect(dir: &Path, rel: &Path, out: &mut Vec<String>) {
    for e in fs::read_dir(dir).expect("tema").flatten() {
        let p = e.path();
        let r = rel.join(e.file_name());
        if p.is_dir() {
            collect(&p, &r, out);
        } else {
            out.push(r.to_string_lossy().replace('\\', "/"));
        }
    }
}
