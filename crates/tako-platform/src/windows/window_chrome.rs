//! Barra única do modo desktop (ADR-0027): a janela sem moldura responde ao Windows como se a barra de cima fosse a
//! barra de título, para arrastar, duplo clique, menu de sistema e layouts de encaixe do Maximizar.
//!
//! A troca do procedimento da janela usa só a user32 (`SetWindowLongPtrW` + `CallWindowProcW`): as funções de
//! subclasse da comctl32 só existem pelo nome na versão 6, que exige manifesto, e o binário de teste não tem.

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::{
        Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute},
        Gdi::ScreenToClient,
    },
    UI::{
        Input::KeyboardAndMouse::{TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TrackMouseEvent},
        WindowsAndMessaging::*,
    },
};
use windows::core::w;

/// O que está sob o ponteiro na barra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeHit {
    Client,
    Caption,
    Minimize,
    Maximize,
    Close,
}

/// Quem desenha a barra (a interface): diz o que está sob o ponto e recebe hover/pressionado dos botões nativos.
pub trait ChromeHost {
    /// `x`, `y` e `width` em pixels físicos da área cliente.
    fn hit(&self, x: i32, y: i32, width: i32) -> ChromeHit;
    /// 0 = nenhum, 1 = Maximizar, 2 = Fechar, 3 = Minimizar.
    fn set_hover(&self, button: u8);
    fn set_pressed(&self, button: u8);
    /// O botão foi clicado (apertado e solto em cima dele).
    fn activate(&self, button: u8);
}

fn button(hit: u32) -> u8 {
    match hit {
        HTMAXBUTTON => 1,
        HTCLOSE => 2,
        HTMINBUTTON => 3,
        _ => 0,
    }
}

struct State {
    previous: isize,
    host: Box<dyn ChromeHost>,
    pressed: std::cell::Cell<u8>,
}

const PROP: windows::core::PCWSTR = w!("TakoWindowChrome");

/// Instala a barra na janela `hwnd` (o valor do `HWND`). Chamar de novo na mesma janela não faz nada.
pub fn install(hwnd: isize, host: Box<dyn ChromeHost>) {
    let hwnd = HWND(hwnd as *mut _);
    // SAFETY: `hwnd` é a janela viva do Tako e tudo roda na thread dela; a propriedade guarda o `State` até
    // WM_NCDESTROY, quando o procedimento anterior volta e o `State` é liberado.
    unsafe {
        if !GetPropW(hwnd, PROP).is_invalid() {
            return;
        }
        // O DWM aplica os cantos de 8 e os remove ao maximizar; Windows antigos ignoram o atributo.
        let corner = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            std::ptr::from_ref(&corner).cast(),
            std::mem::size_of_val(&corner) as u32,
        );
        let previous = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
        if previous == 0 {
            tracing::warn!("não foi possível instalar o hit-test da barra desktop");
            return;
        }
        // O anterior fica guardado antes da troca: a janela pode receber mensagens logo depois dela.
        let state = Box::into_raw(Box::new(State {
            previous,
            host,
            pressed: std::cell::Cell::new(0),
        }));
        if SetPropW(
            hwnd,
            PROP,
            Some(windows::Win32::Foundation::HANDLE(state.cast())),
        )
        .is_err()
        {
            drop(Box::from_raw(state));
            tracing::warn!("não foi possível instalar o hit-test da barra desktop");
            return;
        }
        if SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize) == 0 {
            let _ = RemovePropW(hwnd, PROP);
            drop(Box::from_raw(state));
            tracing::warn!("não foi possível instalar o hit-test da barra desktop");
        }
    }
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: a propriedade só existe enquanto o `State` vive (ver `install`); tudo na thread da janela.
    unsafe {
        let raw = GetPropW(hwnd, PROP).0 as *mut State;
        if raw.is_null() {
            return DefWindowProcW(hwnd, msg, wp, lp);
        }
        let state = &*raw;
        // `previous` veio de GetWindowLongPtr, então é um WNDPROC válido.
        let previous: WNDPROC = std::mem::transmute::<isize, WNDPROC>(state.previous);
        let call = |msg, wp, lp| CallWindowProcW(previous, hwnd, msg, wp, lp);
        if msg == WM_NCDESTROY {
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, state.previous);
            let _ = RemovePropW(hwnd, PROP);
            let result = call(msg, wp, lp);
            drop(Box::from_raw(raw));
            return result;
        }
        if msg == WM_NCHITTEST {
            let mut p = POINT {
                x: lp.0 as i16 as i32,
                y: (lp.0 >> 16) as i16 as i32,
            };
            let mut rect = RECT::default();
            if ScreenToClient(hwnd, &mut p).as_bool() && GetClientRect(hwnd, &mut rect).is_ok() {
                let code = match state.host.hit(p.x, p.y, rect.right) {
                    ChromeHit::Caption => HTCAPTION,
                    ChromeHit::Minimize => HTMINBUTTON,
                    ChromeHit::Maximize => HTMAXBUTTON,
                    ChromeHit::Close => HTCLOSE,
                    ChromeHit::Client => return call(msg, wp, lp),
                };
                return LRESULT(code as isize);
            }
        }
        // Sem moldura do sistema, o DefWindowProc desenha botões antigos e às vezes ignora o clique: o Tako trata
        // o apertar e o soltar, e o Windows segue mostrando os layouts de encaixe ao parar sobre o Maximizar.
        let hit = button(wp.0 as u32);
        if (msg == WM_NCLBUTTONDOWN || msg == WM_NCLBUTTONDBLCLK) && hit != 0 {
            state.pressed.set(hit);
            state.host.set_pressed(hit);
            return LRESULT(0);
        }
        if msg == WM_NCLBUTTONUP && state.pressed.get() != 0 {
            let pressed = state.pressed.replace(0);
            state.host.set_pressed(0);
            if hit == pressed {
                state.host.activate(hit);
            }
            return LRESULT(0);
        }
        if msg == WM_NCMOUSEMOVE {
            state.host.set_hover(hit);
            let pressed = state.pressed.get();
            if pressed != 0 {
                state
                    .host
                    .set_pressed(if hit == pressed { pressed } else { 0 });
            }
            let mut track = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE | TME_NONCLIENT,
                hwndTrack: hwnd,
                ..Default::default()
            };
            let _ = TrackMouseEvent(&mut track);
        }
        if msg == WM_NCMOUSELEAVE || msg == WM_MOUSEMOVE {
            state.host.set_hover(0);
            if state.pressed.replace(0) != 0 {
                state.host.set_pressed(0);
            }
        }
        // O Windows cuida de duplo clique, menu de sistema, snap e comandos dos botões não cliente.
        call(msg, wp, lp)
    }
}
