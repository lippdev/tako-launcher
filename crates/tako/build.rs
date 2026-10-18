//! No Windows, embute o ícone do Tako no `Tako.exe`: a barra de tarefas, os atalhos e o Explorer leem o ícone
//! do executável, não o da janela.

fn main() {
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=../../packaging/tako.ico");
        winresource::WindowsResource::new()
            .set_icon("../../packaging/tako.ico")
            .compile()
            .expect("embutir o ícone no executável");
    }
}
