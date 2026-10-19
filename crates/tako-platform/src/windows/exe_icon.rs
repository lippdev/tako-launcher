//! Ícone embutido num `.exe` (Windows): `PrivateExtractIconsW` extrai e escala o ícone para o tamanho
//! pedido e `GetDIBits` lê os pixels. O executável é dado externo: tamanho do caminho e do
//! bitmap limitados, e qualquer falha é "sem ícone".

use crate::exe_icon::Pixels;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits,
    GetObjectW, HBITMAP, ReleaseDC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, GetIconInfo, HICON, ICONINFO, PrivateExtractIconsW,
};

/// Ícone de `exe` escalado pela API para o tamanho `side`; `None` sem ícone, ou caminho com mais de 259 caracteres.
pub fn extract(exe: &Path, side: u32) -> Option<Pixels> {
    // A API recebe o caminho num vetor fixo de MAX_PATH (260) caracteres.
    let mut name = [0u16; 260];
    let wide: Vec<u16> = exe.as_os_str().encode_wide().collect();
    if wide.is_empty() || wide.len() >= name.len() {
        return None;
    }
    name[..wide.len()].copy_from_slice(&wide);
    let mut icon = [HICON::default()];
    let side = i32::try_from(side).ok()?;
    // SAFETY: `name` termina em NUL (inicia zerado e o caminho é menor que o vetor); `icon` é um vetor de um
    // elemento que a chamada preenche; sem id de ícone (ponteiro nulo).
    let found = unsafe { PrivateExtractIconsW(&name, 0, side, side, Some(&mut icon), None, 0) };
    if found == 0 || found == u32::MAX || icon[0].is_invalid() {
        return None;
    }
    let pixels = pixels_of(icon[0]);
    // SAFETY: o ícone foi criado por `PrivateExtractIconsW` para nós e não é usado depois daqui.
    let _ = unsafe { DestroyIcon(icon[0]) };
    pixels
}

fn pixels_of(icon: HICON) -> Option<Pixels> {
    let mut info = ICONINFO::default();
    // SAFETY: `info` é um ICONINFO local válido para a escrita.
    unsafe { GetIconInfo(icon, &mut info) }.ok()?;
    let pixels = read_bitmaps(&info);
    for bitmap in [info.hbmColor, info.hbmMask] {
        if !bitmap.is_invalid() {
            // SAFETY: `GetIconInfo` cria uma cópia de cada bitmap e quem chama é o dono: liberar uma vez.
            let _ = unsafe { DeleteObject(bitmap.into()) };
        }
    }
    pixels
}

fn read_bitmaps(info: &ICONINFO) -> Option<Pixels> {
    // Ícone monocromático não tem bitmap colorido: fica de fora.
    if info.hbmColor.is_invalid() {
        return None;
    }
    let mut bm = BITMAP::default();
    // SAFETY: `bm` tem o tamanho de BITMAP, que é o que se informa à chamada.
    let got = unsafe {
        GetObjectW(
            info.hbmColor.into(),
            std::mem::size_of::<BITMAP>() as i32,
            Some((&raw mut bm).cast()),
        )
    };
    let valid = |n: i32| (1..=256).contains(&n);
    if got == 0 || !valid(bm.bmWidth) || !valid(bm.bmHeight) {
        return None;
    }
    // SAFETY: tela inteira (janela nula); liberada abaixo com o mesmo identificador.
    let dc = unsafe { GetDC(None) };
    if dc.is_invalid() {
        return None;
    }
    let (w, h) = (bm.bmWidth, bm.bmHeight);
    let read = |bitmap: HBITMAP| -> Option<Vec<u8>> {
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // de cima para baixo
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; w as usize * h as usize * 4];
        // SAFETY: `buf` tem os `w`×`h`×4 bytes que o pedido de 32 bits em `bi` escreve; `bi` e `dc` são válidos
        // e o bitmap não está selecionado em nenhum DC.
        let lines = unsafe {
            GetDIBits(
                dc,
                bitmap,
                0,
                h as u32,
                Some(buf.as_mut_ptr().cast()),
                &mut bi,
                DIB_RGB_COLORS,
            )
        };
        (lines == h).then_some(buf)
    };
    let bgra = read(info.hbmColor);
    let mask = (!info.hbmMask.is_invalid())
        .then(|| read(info.hbmMask))
        .flatten();
    // SAFETY: `dc` veio de `GetDC(None)` acima e não é mais usado.
    unsafe { ReleaseDC(None, dc) };
    Some(Pixels {
        width: w as u32,
        height: h as u32,
        bgra: bgra?,
        mask,
    })
}
