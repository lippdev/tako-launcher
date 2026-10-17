//! Atalho de teclado global durante o jogo (Central rápida). Um gancho de teclado de baixo nível
//! (`WH_KEYBOARD_LL`) numa thread com fila de mensagens vê a tecla antes do jogo e chama `on_press`.
//! O `RegisterHotKey` do Tako C# não serve: jogos que leem o teclado por raw input com
//! `RIDEV_NOHOTKEYS` (ex.: Resident Evil 2) desligam os atalhos globais enquanto estão na frente.
//! O atalho é engolido (o jogo não o recebe) e soltar o [`Hotkey`] encerra a thread e remove o gancho.

use std::cell::RefCell;
use std::sync::mpsc;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, HC_ACTION, KBDLLHOOKSTRUCT, MSG, PM_NOREMOVE, PeekMessageW,
    PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN,
    WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

// Bits de `modifiers`, os mesmos `MOD_*` do `RegisterHotKey` (e do `ModifierKeys` do WPF).
const MOD_ALT: u32 = 1;
const MOD_CONTROL: u32 = 2;
const MOD_SHIFT: u32 = 4;
const MOD_WIN: u32 = 8;

pub struct Hotkey {
    thread: u32,
}

impl Drop for Hotkey {
    fn drop(&mut self) {
        // SAFETY: só posta uma mensagem na fila da thread do atalho; falha se ela já saiu, o que não importa.
        let _ = unsafe {
            PostThreadMessageW(self.thread, WM_QUIT, Default::default(), Default::default())
        };
    }
}

struct Wanted {
    modifiers: u32,
    vk: u32,
    on_press: Box<dyn Fn()>,
    /// A tecla do atalho está apertada (repetição do teclado não dispara de novo).
    down: bool,
}

thread_local! {
    // O gancho roda na thread que o instalou (a do laço de mensagens abaixo).
    static WANTED: RefCell<Option<Wanted>> = const { RefCell::new(None) };
}

fn pressed(vk: u16) -> bool {
    // SAFETY: só lê o estado de uma tecla.
    (unsafe { GetAsyncKeyState(vk as i32) } as u16 & 0x8000) != 0
}

fn modifiers_now() -> u32 {
    let mut m = 0;
    if pressed(VK_MENU.0) {
        m |= MOD_ALT;
    }
    if pressed(VK_CONTROL.0) {
        m |= MOD_CONTROL;
    }
    if pressed(VK_SHIFT.0) {
        m |= MOD_SHIFT;
    }
    if pressed(VK_LWIN.0) || pressed(VK_RWIN.0) {
        m |= MOD_WIN;
    }
    m
}

/// `true` = engolir a tecla.
fn on_key(vk: u32, down: bool) -> bool {
    WANTED.with_borrow_mut(|w| {
        let Some(w) = w.as_mut() else { return false };
        if vk != w.vk {
            return false;
        }
        if !down {
            // Solta a tecla do atalho: engole só se a descida também foi engolida.
            return std::mem::take(&mut w.down);
        }
        if w.down {
            return true;
        }
        if modifiers_now() != w.modifiers {
            return false;
        }
        w.down = true;
        (w.on_press)();
        true
    })
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: em `HC_ACTION` o `lparam` de um gancho WH_KEYBOARD_LL aponta para um KBDLLHOOKSTRUCT válido.
        let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let msg = wparam.0 as u32;
        let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        let up = msg == WM_KEYUP || msg == WM_SYSKEYUP;
        if (down || up) && on_key(kb.vkCode, down) {
            return LRESULT(1);
        }
    }
    // SAFETY: repassa o evento ao próximo gancho com os mesmos argumentos.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// `modifiers` em bits `MOD_*`; `vk` = tecla virtual. `None` se o gancho não pôde ser instalado.
pub fn register(modifiers: u32, vk: u32, on_press: impl Fn() + Send + 'static) -> Option<Hotkey> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        WANTED.set(Some(Wanted {
            modifiers,
            vk,
            on_press: Box::new(on_press),
            down: false,
        }));
        let mut msg = MSG::default();
        // SAFETY: chamadas Win32 na própria thread; `msg` vive durante o laço. PeekMessage cria a fila antes de
        // devolver o id, para o WM_QUIT do Drop não se perder. O gancho de baixo nível só é chamado enquanto esta
        // thread processa mensagens (GetMessageW).
        unsafe {
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
            let handle = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), None, 0) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!("atalho de teclado da Central: {e}");
                    let _ = tx.send(None);
                    return;
                }
            };
            let _ = tx.send(Some(GetCurrentThreadId()));
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
            let _ = UnhookWindowsHookEx(handle);
        }
    });
    rx.recv().ok().flatten().map(|thread| Hotkey { thread })
}
