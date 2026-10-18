//! Espaço livre no disco onde um caminho está (SRC-005): o destaque do jogo não instalado diz quanto cabe antes
//! de mandar a loja instalar.

use std::path::Path;
use tako_core::devices::FreeSpace;

/// Bytes livres para o usuário no disco de `path` (que precisa existir) e o nome do disco: a letra no Windows
/// (`C:`), o ponto de montagem no Linux e no macOS. `None` se o SO não responder.
pub fn free_space(path: &Path) -> Option<FreeSpace> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::path::Component;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        use windows::core::PCWSTR;
        let drive = path.components().find_map(|c| match c {
            Component::Prefix(p) => Some(p.as_os_str().to_string_lossy().into_owned()),
            _ => None,
        })?;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        let mut free = 0u64;
        // SAFETY: `wide` termina em NUL e vive durante a chamada; `free` é um local válido para escrita e os
        // outros dois ponteiros opcionais ficam nulos.
        unsafe { GetDiskFreeSpaceExW(PCWSTR(wide.as_ptr()), Some(&mut free), None, None) }.ok()?;
        Some(FreeSpace {
            drive: drive.trim_start_matches(r"\\?\").to_owned(),
            bytes: free,
        })
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::MetadataExt;
        let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
        let mut st = std::mem::MaybeUninit::<libc::statvfs>::zeroed();
        // SAFETY: `c_path` termina em NUL; `st` é um espaço válido do tamanho de `statvfs`.
        if unsafe { libc::statvfs(c_path.as_ptr(), st.as_mut_ptr()) } != 0 {
            return None;
        }
        // SAFETY: a chamada acima deu certo, então a estrutura foi preenchida.
        let st = unsafe { st.assume_init() };
        // Os tipos dos campos mudam entre Linux e macOS.
        #[allow(clippy::unnecessary_cast)]
        let bytes = (st.f_bavail as u64).saturating_mul(st.f_frsize as u64);
        // O ponto de montagem é o ancestral mais alto no mesmo dispositivo.
        let dev = std::fs::metadata(path).ok()?.dev();
        let mount = path
            .canonicalize()
            .ok()?
            .ancestors()
            .take_while(|a| std::fs::metadata(a).is_ok_and(|m| m.dev() == dev))
            .last()?
            .to_string_lossy()
            .into_owned();
        Some(FreeSpace {
            drive: mount,
            bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_free_bytes_and_the_disk_of_an_existing_folder() {
        let space = free_space(&std::env::temp_dir()).unwrap();
        assert!(space.bytes > 0 && !space.drive.is_empty());
        #[cfg(windows)]
        assert!(space.drive.ends_with(':'));
        #[cfg(unix)]
        assert!(space.drive.starts_with('/'));
    }

    #[test]
    fn missing_or_invalid_path_is_none_not_a_panic() {
        assert!(free_space(Path::new("/nao/existe/mesmo")).is_none());
        assert!(free_space(Path::new("")).is_none());
        assert!(free_space(Path::new("a\0b")).is_none());
    }
}
