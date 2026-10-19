//! Ícone do `.exe` de um jogo em PNG no cache do Tako (UX-005), para jogos sem ícone de loja (adicionados à
//! mão e lojas que só dão o caminho do executável). A extração é do Windows (`windows/exe_icon.rs`); nos outros
//! SOs não há ícone embutido no executável e [`icon_file`] devolve `None`. O arquivo é dado externo: tamanho
//! limitado e qualquer falha é "sem ícone".

use std::path::{Path, PathBuf};

/// Lado pedido ao Windows: o ícone aparece com até 56 px (28 px × 2 em alta densidade), então 128 sobra para
/// reduzir bem sem encher o cache.
pub const SIDE: u32 = 128;
/// Maior lado aceito de uma extração.
const MAX_SIDE: u32 = 256;

/// Pixels de um ícone como o `GetDIBits` entrega: BGRA, de cima para baixo.
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    /// Máscara AND (também 32 bpp): pixel claro = transparente. Só vale para ícones sem canal alfa.
    pub mask: Option<Vec<u8>>,
}

static FAILED: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<PathBuf>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

/// PNG do ícone do `exe` no cache `dir` se já existir no disco.
/// Não tenta extrair nem chama APIs de plataforma (seguro para rodar na thread da UI).
pub fn cached_icon_file(exe: &Path, dir: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(exe).ok().filter(|m| m.is_file())?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let file = dir.join(cache_name(exe, meta.len(), modified));
    if file.is_file() { Some(file) } else { None }
}

/// PNG do ícone do `exe` no cache `dir` (criado se preciso), pronto para virar `file:///`. Reaproveita o que já
/// foi extraído: o nome muda quando o executável muda (caminho, tamanho ou data). Falhas são guardadas num
/// `HashSet` para não tentar extrair de novo.
/// shortcut: o que sai de uso (exe trocado ou removido) fica no cache; limpar junto do cache de artes se pesar.
pub fn icon_file(exe: &Path, dir: &Path) -> Option<PathBuf> {
    icon_file_with(exe, dir, &FAILED, extract)
}

#[cfg(target_os = "windows")]
fn extract(exe: &Path) -> Option<Pixels> {
    crate::windows::exe_icon::extract(exe, SIDE)
}

#[cfg(not(target_os = "windows"))]
fn extract(_: &Path) -> Option<Pixels> {
    None
}

fn icon_file_with(
    exe: &Path,
    dir: &Path,
    failed: &std::sync::Mutex<std::collections::HashSet<PathBuf>>,
    extract: impl Fn(&Path) -> Option<Pixels>,
) -> Option<PathBuf> {
    if let Ok(f) = failed.lock()
        && f.contains(exe)
    {
        return None;
    }
    let record_failure = || {
        if let Ok(mut f) = failed.lock() {
            f.insert(exe.to_path_buf());
        }
    };
    let meta = match std::fs::metadata(exe).ok().filter(|m| m.is_file()) {
        Some(m) => m,
        None => {
            record_failure();
            return None;
        }
    };
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    let file = dir.join(cache_name(exe, meta.len(), modified));
    if file.is_file() {
        return Some(file);
    }
    let pixels = extract(exe);
    let png = pixels.as_ref().and_then(to_png);
    let Some(png) = png else {
        record_failure();
        return None;
    };
    if std::fs::create_dir_all(dir).is_err() {
        record_failure();
        return None;
    }
    // Grava e renomeia: leitor nunca vê PNG pela metade (a leitura da biblioteca e a interface podem pedir juntas).
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let tmp = file.with_extension(format!(
        "{}.tmp",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let saved = std::fs::write(&tmp, png).and_then(|()| std::fs::rename(&tmp, &file));
    if let Err(e) = saved {
        tracing::warn!("ícone de {}: {e}", exe.display());
        let _ = std::fs::remove_file(&tmp);
        record_failure();
        return None;
    }
    Some(file)
}

/// `<hash do caminho, tamanho e data>.png` (FNV-1a de 64 bits; o caminho sem diferenciar maiúsculas, como o NTFS).
pub fn cache_name(exe: &Path, len: u64, modified: u64) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let path = exe.to_string_lossy().to_lowercase();
    for b in path
        .bytes()
        .chain(len.to_le_bytes())
        .chain(modified.to_le_bytes())
    {
        hash = (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}.png")
}

/// PNG RGBA dos pixels; `None` se as medidas não batem. Ícone antigo (sem alfa) usa a máscara; sem nenhum pixel
/// visível não há ícone.
fn to_png(p: &Pixels) -> Option<Vec<u8>> {
    let valid = |n: u32| (1..=MAX_SIDE).contains(&n);
    if !valid(p.width) || !valid(p.height) {
        return None;
    }
    let len = p.width as usize * p.height as usize * 4;
    if p.bgra.len() != len || p.mask.as_ref().is_some_and(|m| m.len() != len) {
        return None;
    }
    let has_alpha = p.bgra.chunks_exact(4).any(|px| px[3] != 0);
    let mut rgba = Vec::with_capacity(len);
    for (i, px) in p.bgra.chunks_exact(4).enumerate() {
        let alpha = if has_alpha {
            px[3]
        } else if p
            .mask
            .as_ref()
            .is_some_and(|m| m[i * 4..i * 4 + 3] != [0, 0, 0])
        {
            0
        } else {
            255
        };
        rgba.extend([px[2], px[1], px[0], alpha]);
    }
    if rgba.chunks_exact(4).all(|px| px[3] == 0) {
        return None;
    }
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_raw(p.width, p.height, rgba)?
        .write_to(&mut out, image::ImageFormat::Png)
        .ok()?;
    Some(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn square(side: u32, bgra: [u8; 4]) -> Pixels {
        Pixels {
            width: side,
            height: side,
            bgra: bgra.repeat((side * side) as usize),
            mask: None,
        }
    }

    fn decode(png: &[u8]) -> image::RgbaImage {
        image::load_from_memory(png).unwrap().to_rgba8()
    }

    #[test]
    fn bgra_with_alpha_becomes_rgba_png() {
        let png = to_png(&square(4, [10, 20, 30, 200])).unwrap();
        assert_eq!(decode(&png).get_pixel(1, 1).0, [30, 20, 10, 200]);
    }

    #[test]
    fn old_icons_without_alpha_use_the_and_mask() {
        let mut p = square(2, [1, 2, 3, 0]);
        // Só o primeiro pixel é transparente na máscara (claro); os outros ficam opacos.
        p.mask = Some([[255, 255, 255, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]].concat());
        let img = decode(&to_png(&p).unwrap());
        assert_eq!(img.get_pixel(0, 0).0[3], 0);
        assert_eq!(img.get_pixel(1, 0).0, [3, 2, 1, 255]);
        // Sem máscara e sem alfa: opaco.
        p.mask = None;
        assert_eq!(decode(&to_png(&p).unwrap()).get_pixel(0, 0).0[3], 255);
    }

    #[test]
    fn broken_pixels_are_no_icon() {
        let ok = square(4, [1, 2, 3, 255]);
        assert!(
            to_png(&Pixels {
                width: 0,
                ..square(4, [1, 2, 3, 255])
            })
            .is_none()
        );
        assert!(
            to_png(&Pixels {
                width: 257,
                height: 1,
                bgra: vec![0; 257 * 4],
                mask: None
            })
            .is_none()
        );
        assert!(
            to_png(&Pixels {
                bgra: ok.bgra[..10].to_vec(),
                ..square(4, [0; 4])
            })
            .is_none()
        );
        assert!(
            to_png(&Pixels {
                mask: Some(vec![0; 3]),
                ..square(4, [1, 2, 3, 255])
            })
            .is_none()
        );
        // Tudo transparente (alfa e máscara): nada a mostrar.
        let all_clear = Pixels {
            mask: Some([255u8; 4].repeat(16)),
            ..square(4, [1, 2, 3, 0])
        };
        assert!(to_png(&all_clear).is_none());
    }

    #[test]
    fn extracts_once_then_reuses_the_cached_png() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Jogo.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let cache = dir.path().join("cache").join("exe-icons");
        let calls = Cell::new(0);
        let fake = |_: &Path| {
            calls.set(calls.get() + 1);
            Some(square(8, [0, 0, 255, 255]))
        };
        let failed = std::sync::Mutex::new(std::collections::HashSet::new());
        let file = icon_file_with(&exe, &cache, &failed, fake).unwrap();
        assert_eq!(
            decode(&std::fs::read(&file).unwrap()).get_pixel(0, 0).0,
            [255, 0, 0, 255]
        );
        assert_eq!(
            icon_file_with(&exe, &cache, &failed, fake),
            Some(file.clone())
        );
        assert_eq!(calls.get(), 1);
        // Executável mudou (tamanho): outro nome, extrai de novo.
        std::fs::write(&exe, b"MZ mudou").unwrap();
        assert_ne!(icon_file_with(&exe, &cache, &failed, fake), Some(file));
        assert_eq!(calls.get(), 2);
        // Nenhum .tmp sobra.
        assert!(
            std::fs::read_dir(&cache)
                .unwrap()
                .flatten()
                .all(|e| e.path().extension().is_some_and(|x| x == "png"))
        );
    }

    #[test]
    fn missing_or_unreadable_executables_are_no_icon() {
        let dir = tempfile::tempdir().unwrap();
        let failed = std::sync::Mutex::new(std::collections::HashSet::new());
        let none = |_: &Path| -> Option<Pixels> { None };
        assert!(
            icon_file_with(
                &dir.path().join("nao-existe.exe"),
                dir.path(),
                &failed,
                none
            )
            .is_none()
        );
        // Pasta no lugar do arquivo.
        assert!(icon_file_with(dir.path(), dir.path(), &failed, none).is_none());
        let exe = dir.path().join("sem-icone.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        assert!(icon_file_with(&exe, &dir.path().join("c"), &failed, none).is_none());
        assert!(!dir.path().join("c").exists());
    }

    #[test]
    fn failed_extraction_is_cached_in_hashset_and_not_retried() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("SemIcone.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let cache = dir.path().join("cache").join("exe-icons");
        let failed = std::sync::Mutex::new(std::collections::HashSet::new());
        let calls = Cell::new(0);
        let failing = |_: &Path| {
            calls.set(calls.get() + 1);
            None
        };
        assert!(icon_file_with(&exe, &cache, &failed, failing).is_none());
        assert_eq!(calls.get(), 1);
        // Na segunda chamada, não tenta extrair de novo: o HashSet guarda a falha.
        assert!(icon_file_with(&exe, &cache, &failed, failing).is_none());
        assert_eq!(calls.get(), 1);
        assert!(failed.lock().unwrap().contains(&exe));
    }

    #[test]
    fn cached_icon_file_only_returns_existing_file_without_extracting() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("Jogo.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let cache = dir.path().join("cache").join("exe-icons");
        assert!(cached_icon_file(&exe, &cache).is_none());
        let failed = std::sync::Mutex::new(std::collections::HashSet::new());
        let fake = |_: &Path| Some(square(8, [0, 0, 255, 255]));
        let file = icon_file_with(&exe, &cache, &failed, fake).unwrap();
        assert_eq!(cached_icon_file(&exe, &cache), Some(file));
    }

    #[test]
    fn cache_name_follows_path_size_and_date() {
        let (a, b) = (Path::new(r"C:\Jogos\A.exe"), Path::new(r"c:\jogos\a.exe"));
        assert_eq!(cache_name(a, 1, 2), cache_name(b, 1, 2));
        assert_ne!(cache_name(a, 1, 2), cache_name(a, 3, 2));
        assert_ne!(cache_name(a, 1, 2), cache_name(a, 1, 3));
        assert_ne!(
            cache_name(a, 1, 2),
            cache_name(Path::new(r"C:\Jogos\B.exe"), 1, 2)
        );
    }
}
