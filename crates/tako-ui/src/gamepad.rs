//! Controles pelo SDL3: a leitura mora em `tako-platform` (`gamepad`), para o núcleo do `tako --web` usar a mesma;
//! as regras (família, estado somado, avisos) em `tako_core::input::pad`. Este módulo só mantém os nomes de sempre.

pub use tako_platform::gamepad::*;
