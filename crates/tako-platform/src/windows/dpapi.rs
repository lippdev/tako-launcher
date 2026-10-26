//! Decifra os segredos das alphas C# (`DpapiSecretProtector`: DPAPI do usuário atual, sem interface). Só leitura:
//! o Rust guarda segredos no cofre do SO (MIG-030), nunca mais em DPAPI.

use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData,
};

/// `CryptUnprotectData`. `None` quando o dado é de outro usuário ou PC, ou está danificado.
/// Assinatura de `tako_core::persistence::alpha_migration::read_secrets`.
pub fn unprotect(data: &[u8]) -> Option<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(data.len()).ok()?,
        pbData: data.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `input` aponta para `data`, vivo durante a chamada, e a API só o lê; `output` é preenchido pela API.
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    }
    .ok()?;
    if output.pbData.is_null() {
        return Some(Vec::new());
    }
    // SAFETY: em caso de sucesso `pbData` tem `cbData` bytes alocados pela API, copiados antes do `LocalFree`.
    let plain =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    // SAFETY: o bloco foi alocado com `LocalAlloc` pela API e é liberado uma vez só.
    unsafe {
        LocalFree(Some(HLOCAL(output.pbData.cast())));
    }
    Some(plain)
}
