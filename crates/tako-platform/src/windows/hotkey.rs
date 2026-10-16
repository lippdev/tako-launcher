//! Atalho de teclado global durante o jogo (Central rápida), como o `RegisterHotKey` do Tako C#: uma thread com
//! fila de mensagens registra o atalho e chama `on_press` a cada `WM_HOTKEY`. Soltar o [`Hotkey`] encerra a thread
//! e libera o atalho (Ctrl+Shift+Tab, por exemplo, volta a funcionar nos outros apps).

use std::sync::mpsc;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW, WM_HOTKEY, WM_QUIT,
};

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

/// `modifiers` em bits `MOD_*`; `vk` = tecla virtual. `None` se outro app já usa o atalho.
pub fn register(modifiers: u32, vk: u32, on_press: impl Fn() + Send + 'static) -> Option<Hotkey> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut msg = MSG::default();
        // SAFETY: chamadas Win32 na própria thread; `msg` vive durante o laço. PeekMessage cria a fila antes de
        // devolver o id, para o WM_QUIT do Drop não se perder.
        unsafe {
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
            let mods = HOT_KEY_MODIFIERS(modifiers) | MOD_NOREPEAT;
            if let Err(e) = RegisterHotKey(None, 1, mods, vk) {
                tracing::warn!("atalho de teclado da Central: {e}");
                let _ = tx.send(None);
                return;
            }
            let _ = tx.send(Some(GetCurrentThreadId()));
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_HOTKEY {
                    on_press();
                }
            }
            let _ = UnregisterHotKey(None, 1);
        }
    });
    rx.recv().ok().flatten().map(|thread| Hotkey { thread })
}
