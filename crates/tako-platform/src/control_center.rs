//! Atalho da Central web: RegisterHotKey, sem capturar o teclado inteiro (WEB-22).

use tako_core::input::shortcut::KeyShortcut;

/// O retorno mantém a inscrição até o fim da sessão. Falhas não impedem o atalho do controle.
pub fn register(shortcut: &KeyShortcut, on_press: impl Fn() + Send + 'static) -> Option<Hotkey> {
    #[cfg(windows)]
    {
        native::register(shortcut, on_press)
    }
    #[cfg(not(windows))]
    {
        let _ = (shortcut, on_press);
        None
    }
}

pub struct Hotkey {
    #[cfg(windows)]
    thread: u32,
    #[cfg(windows)]
    join: Option<std::thread::JoinHandle<()>>,
}

impl Drop for Hotkey {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            // SAFETY: fila criada pela própria thread antes de publicar seu id.
            let posted = unsafe {
                windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                    self.thread,
                    windows::Win32::UI::WindowsAndMessaging::WM_QUIT,
                    Default::default(),
                    Default::default(),
                )
            };
            if posted.is_ok()
                && let Some(join) = self.join.take()
                && join.join().is_err()
            {
                tracing::warn!("thread do atalho da Central encerrou com pânico");
            }
        }
    }
}

/// Traz uma janela visível dos processos indicados para a frente, sem injetar entrada no jogo.
pub fn focus_processes(pids: &[u32]) {
    #[cfg(windows)]
    native::focus_processes(pids);
    #[cfg(not(windows))]
    let _ = pids;
}

#[cfg(windows)]
mod native {
    use super::*;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetMessageW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, MSG,
        PM_NOREMOVE, PeekMessageW, SW_RESTORE, SetForegroundWindow, ShowWindowAsync, WM_HOTKEY,
    };
    use windows_core::BOOL;

    pub(super) fn register(
        shortcut: &KeyShortcut,
        on_press: impl Fn() + Send + 'static,
    ) -> Option<Hotkey> {
        let vk = shortcut.virtual_key()?;
        let modifiers = HOT_KEY_MODIFIERS(u32::from(shortcut.modifiers.0)) | MOD_NOREPEAT;
        let (tx, rx) = std::sync::mpsc::channel();
        let join = std::thread::spawn(move || {
            let mut msg = MSG::default();
            // SAFETY: inscrição e remoção na mesma thread; msg permanece válida durante o laço.
            unsafe {
                let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
                if let Err(error) = RegisterHotKey(None, 22, modifiers, vk) {
                    tracing::warn!(%error, "não foi possível registrar o atalho da Central");
                    let _ = tx.send(None);
                    return;
                }
                if tx.send(Some(GetCurrentThreadId())).is_ok() {
                    while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                        if msg.message == WM_HOTKEY && msg.wParam.0 == 22 {
                            on_press();
                        }
                    }
                }
                if let Err(error) = UnregisterHotKey(None, 22) {
                    tracing::warn!(%error, "remover atalho da Central");
                }
            }
        });
        match rx.recv().ok().flatten() {
            Some(thread) => Some(Hotkey {
                thread,
                join: Some(join),
            }),
            None => {
                let _ = join.join();
                None
            }
        }
    }

    struct Search<'a> {
        pids: &'a [u32],
        window: Option<HWND>,
    }

    unsafe extern "system" fn find(window: HWND, data: LPARAM) -> BOOL {
        // SAFETY: EnumWindows é síncrono; data aponta para Search durante toda a enumeração.
        let search = unsafe { &mut *(data.0 as *mut Search<'_>) };
        let mut pid = 0;
        // SAFETY: handle fornecido pelo Windows e saída local válida.
        unsafe {
            GetWindowThreadProcessId(window, Some(&mut pid));
            if search.window.is_none()
                && search.pids.contains(&pid)
                && IsWindowVisible(window).as_bool()
            {
                search.window = Some(window);
            }
        }
        BOOL(1)
    }

    pub(super) fn focus_processes(pids: &[u32]) {
        let mut search = Search { pids, window: None };
        // SAFETY: Search vive até a enumeração terminar; apenas restaura e ativa janela do PID indicado.
        unsafe {
            if let Err(error) = EnumWindows(Some(find), LPARAM((&raw mut search) as isize)) {
                tracing::warn!(%error, "procurar janela da sessão");
            }
            if let Some(window) = search.window {
                focus_window(window);
            }
        }
    }

    fn focus_window(window: HWND) {
        // SAFETY: handle encontrado por EnumWindows; janela encerrada nesse intervalo só faz a API falhar.
        unsafe {
            if IsIconic(window).as_bool() {
                // A fila do jogo pode estar travada: restaurar só posta o pedido, sem esperar por ela.
                let _ = ShowWindowAsync(window, SW_RESTORE);
            }
            if !SetForegroundWindow(window).as_bool() {
                tracing::debug!("Windows recusou ativar janela da sessão");
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use windows::Win32::UI::WindowsAndMessaging::*;
        use windows::core::w;

        #[test]
        fn focus_does_not_wait_for_an_unresponsive_window_thread() {
            let (ready, window) = std::sync::mpsc::channel();
            let (release, blocked) = std::sync::mpsc::channel();
            let owner = std::thread::spawn(move || {
                // SAFETY: janela e recursos pertencem apenas a esta thread do teste.
                unsafe {
                    let hwnd = CreateWindowExW(
                        WINDOW_EX_STYLE::default(),
                        w!("STATIC"),
                        w!("Tako focus test"),
                        WS_OVERLAPPEDWINDOW,
                        0,
                        0,
                        100,
                        100,
                        None,
                        None,
                        None,
                        None,
                    )
                    .unwrap();
                    let _ = ShowWindow(hwnd, SW_MINIMIZE);
                    assert!(
                        IsIconic(hwnd).as_bool(),
                        "janela de teste não foi minimizada"
                    );
                    ready.send(hwnd.0 as isize).unwrap();
                    // Não bombeia mensagens: simula um jogo que parou de responder.
                    let _ = blocked.recv();
                    DestroyWindow(hwnd).unwrap();
                }
            });
            let hwnd = window
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            let (done, finished) = std::sync::mpsc::channel();
            let focus = std::thread::spawn(move || {
                focus_window(HWND(hwnd as *mut _));
                let _ = done.send(());
            });
            let result = finished.recv_timeout(std::time::Duration::from_secs(1));
            // Sempre solta a thread dona antes de afirmar, inclusive na regressão com ShowWindow síncrono.
            release.send(()).unwrap();
            owner.join().unwrap();
            focus.join().unwrap();
            assert!(
                result.is_ok(),
                "restaurar esperou pela fila travada da janela"
            );
        }
    }
}
