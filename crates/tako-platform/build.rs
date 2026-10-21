use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=TAKO_FSE_PAYLOAD_DIR");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    let payload = env::var_os("TAKO_FSE_PAYLOAD_DIR").map(PathBuf::from);
    let mut source = String::new();
    for (constant, filename) in [
        ("INSTALLED", "Tako-GamingHome.msix"),
        ("PORTABLE", "Tako-GamingHome-Portable.msix"),
        ("CERTIFICATE", "Tako-GamingHome.cer"),
    ] {
        let bytes = payload.as_ref().map_or_else(Vec::new, |dir| {
            let path = dir.join(filename);
            println!("cargo:rerun-if-changed={}", path.display());
            fs::read(path).expect("recurso FSE solicitado deve existir")
        });
        fs::write(out.join(filename), bytes).expect("gravar recurso FSE");
        source.push_str(&format!(
            "const {constant}: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{filename}\"));\n"
        ));
    }
    fs::write(out.join("fse_payload.rs"), source).expect("gerar recursos FSE");
}
