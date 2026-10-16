//! Compila um tema (com `@tako` = o tema padrão) e mostra os diagnósticos: `cargo run -p tako-ui --example theme_check -- <main.slint>`.
fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/theme/main.slint").into());
    let mut compiler = slint_interpreter::Compiler::default();
    compiler.set_library_paths(std::collections::HashMap::from([(
        "tako".to_owned(),
        concat!(env!("CARGO_MANIFEST_DIR"), "/theme").into(),
    )]));
    let r = spin_on::spin_on(compiler.build_from_path(&path));
    for d in r.diagnostics() {
        eprintln!("{d}");
    }
    println!("components: {:?}", r.component_names().collect::<Vec<_>>());
}
