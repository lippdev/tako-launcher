//! Compila um tema e mostra os diagnósticos: `cargo run -p tako-ui --example theme_check -- <main.slint>`.
fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/theme/main.slint").into());
    let r = spin_on::spin_on(slint_interpreter::Compiler::default().build_from_path(&path));
    for d in r.diagnostics() {
        eprintln!("{d}");
    }
    println!("components: {:?}", r.component_names().collect::<Vec<_>>());
}
