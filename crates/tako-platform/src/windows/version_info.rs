//! Nome do produto e descrição do recurso de versão de um `.exe`, como o `FileVersionInfo` do Tako C#.

use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows::core::PCWSTR;

fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain([0]).collect()
}

/// Valor bruto de `sub` no bloco de versão (ponteiro e tamanho, que apontam para dentro de `block`).
fn query(block: &[u8], sub: &str) -> Option<(*const u8, usize)> {
    let sub = wide(sub);
    let mut ptr = std::ptr::null_mut();
    let mut len = 0u32;
    // SAFETY: `block` é o bloco preenchido por GetFileVersionInfoW e vive mais que o ponteiro devolvido, que
    // aponta para dentro dele; `sub` termina em NUL; `ptr` e `len` são locais válidos.
    let ok = unsafe {
        VerQueryValueW(
            block.as_ptr().cast(),
            PCWSTR(sub.as_ptr()),
            &mut ptr,
            &mut len,
        )
    };
    (ok.as_bool() && !ptr.is_null() && len > 0)
        .then_some((ptr.cast_const().cast::<u8>(), len as usize))
}

/// `[ProductName, FileDescription]` do `.exe`; `None` onde o arquivo não tem o valor.
pub fn names(path: &Path) -> [Option<String>; 2] {
    let file = wide(path.as_os_str());
    // SAFETY: `file` termina em NUL; sem handle.
    let size = unsafe { GetFileVersionInfoSizeW(PCWSTR(file.as_ptr()), None) };
    if size == 0 {
        return [None, None];
    }
    let mut block = vec![0u8; size as usize];
    // SAFETY: `block` tem os `size` bytes que a chamada escreve; `file` termina em NUL.
    let read = unsafe {
        GetFileVersionInfoW(PCWSTR(file.as_ptr()), None, size, block.as_mut_ptr().cast())
    };
    if read.is_err() {
        return [None, None];
    }
    // Idiomas declarados primeiro; depois os do inglês que o .NET também tenta.
    let mut langs: Vec<String> = query(&block, r"\VarFileInfo\Translation")
        .map(|(p, len)| {
            // SAFETY: `p` aponta para `len` bytes dentro de `block`: pares de u16 (idioma, página de código).
            let bytes = unsafe { std::slice::from_raw_parts(p, len) };
            bytes
                .chunks_exact(4)
                .map(|c| {
                    let lang = u16::from_le_bytes([c[0], c[1]]);
                    let cp = u16::from_le_bytes([c[2], c[3]]);
                    format!("{lang:04X}{cp:04X}")
                })
                .collect()
        })
        .unwrap_or_default();
    langs.extend(["040904B0", "040904E4", "04090000"].map(String::from));
    let value = |key: &str| {
        langs.iter().find_map(|lang| {
            let (p, len) = query(&block, &format!(r"\StringFileInfo\{lang}\{key}"))?;
            // SAFETY: para textos, `len` conta os u16 (com o NUL) a partir de `p`, dentro de `block`; o recurso
            // de versão é alinhado em 4 bytes, então `p` serve como `*const u16`.
            let chars = unsafe { std::slice::from_raw_parts(p.cast::<u16>(), len) };
            Some(
                String::from_utf16_lossy(chars)
                    .trim_end_matches('\0')
                    .to_owned(),
            )
        })
    };
    [value("ProductName"), value("FileDescription")]
}
