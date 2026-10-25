//! Bandeja do sistema no Windows (WEB-36, PRD D-10 e D-15).
//!
//! Registra um ícone de notificação na barra de tarefas (`Shell_NotifyIconW`) e gerencia
//! o menu de contexto suspenso (`TrackPopupMenu`).

use crate::tray::{TrayAction, TrayState};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{PCWSTR, w};

const WM_TRAY_CALLBACK: u32 = WM_APP + 42;
const WINDOW_CLASS_NAME: PCWSTR = w!("TakoTrayWindow");

fn encode_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

struct TrayContext {
    state: Arc<Mutex<TrayState>>,
    on_action: Rc<dyn Fn(TrayAction) + Send + 'static>,
    nid: NOTIFYICONDATAW,
    taskbar_created_msg: u32,
    _owned_icon: Option<OwnedIcon>,
    available: Arc<AtomicBool>,
}

/// LoadImageW sem LR_SHARED transfere a posse; LoadIconW devolve um recurso compartilhado.
struct OwnedIcon(HICON);

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        // SAFETY: somente o handle próprio de LoadImageW chega aqui, depois da remoção da bandeja.
        if let Err(error) = unsafe { DestroyIcon(self.0) } {
            tracing::warn!(%error, "liberar ícone da bandeja");
        }
    }
}

pub struct WindowsTray {
    hwnd: isize,
    handle: Option<JoinHandle<()>>,
    available: Arc<AtomicBool>,
}

impl WindowsTray {
    pub fn new(
        state: Arc<Mutex<TrayState>>,
        on_action: impl Fn(TrayAction) + Send + 'static,
    ) -> Option<Self> {
        let (tx, rx) = mpsc::channel();
        let available = Arc::new(AtomicBool::new(false));
        let thread_available = available.clone();
        let handle = std::thread::Builder::new()
            .name("tako-tray".into())
            .spawn(move || {
                run_tray_thread(state, Box::new(on_action), tx, thread_available);
            })
            .ok()?;

        let hwnd = rx.recv().ok()??;
        Some(Self {
            hwnd,
            handle: Some(handle),
            available,
        })
    }

    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Acquire)
    }
}

impl Drop for WindowsTray {
    fn drop(&mut self) {
        if self.hwnd != 0 {
            // SAFETY: posta WM_CLOSE na janela da bandeja para encerrar o loop e limpar os recursos.
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(self.hwnd as *mut _)),
                    WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn run_tray_thread(
    state: Arc<Mutex<TrayState>>,
    on_action: Box<dyn Fn(TrayAction) + Send + 'static>,
    ready_tx: mpsc::Sender<Option<isize>>,
    available: Arc<AtomicBool>,
) {
    // SAFETY: obtém o HINSTANCE do módulo atual para registrar a classe de janela.
    let hinstance = unsafe { GetModuleHandleW(None).unwrap_or_default() };
    let hinst = HINSTANCE(hinstance.0);

    let wnd_class = WNDCLASSW {
        lpfnWndProc: Some(tray_wnd_proc),
        hInstance: hinst,
        lpszClassName: WINDOW_CLASS_NAME,
        ..Default::default()
    };

    // SAFETY: registra a classe de janela da bandeja. Se já estiver registrada, RegisterClassW falha
    // de forma segura e prosseguimos.
    unsafe {
        let _ = RegisterClassW(&wnd_class);
    }

    // SAFETY: mensagem emitida pelo Explorer quando a barra de tarefas reinicia.
    let taskbar_created_msg = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAY_CALLBACK,
        uID: 1,
        ..Default::default()
    };

    let tip = encode_wide("Tako");
    let tip_len = tip.len().min(nid.szTip.len() - 1);
    nid.szTip[..tip_len].copy_from_slice(&tip[..tip_len]);

    // Ícone da aplicação (resource 1 no binário oficial ou IDI_APPLICATION de reserva).
    // SAFETY: busca o ícone pequeno do sistema para a bandeja.
    let (icon, owned_icon) = unsafe {
        let cx = GetSystemMetrics(SM_CXSMICON);
        let cy = GetSystemMetrics(SM_CYSMICON);
        let loaded = LoadImageW(
            Some(hinst),
            PCWSTR(1 as _),
            IMAGE_ICON,
            cx,
            cy,
            LR_DEFAULTCOLOR,
        )
        .map(|h| HICON(h.0));
        match loaded {
            Ok(icon) => (icon, Some(OwnedIcon(icon))),
            Err(_) => (LoadIconW(None, IDI_APPLICATION).unwrap_or_default(), None),
        }
    };
    nid.hIcon = icon;

    let mut ctx = Box::new(TrayContext {
        state,
        on_action: Rc::from(on_action),
        nid,
        taskbar_created_msg,
        _owned_icon: owned_icon,
        available,
    });
    let ctx_ptr = &raw mut *ctx;

    // SAFETY: cria a janela oculta da bandeja com o contexto no lpParam.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WINDOW_CLASS_NAME,
            w!("TakoTray"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinst),
            Some(ctx_ptr as *const _),
        )
    };

    let Ok(hwnd) = hwnd else {
        let _ = ready_tx.send(None);
        return;
    };

    let _ = ready_tx.send(Some(hwnd.0 as isize));

    // SAFETY: loop de mensagens padrão do Win32.
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        // SAFETY: lparam aponta para CREATESTRUCTW durante WM_NCCREATE.
        let cs = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
        // SAFETY: associa o ponteiro de contexto à janela via GWLP_USERDATA.
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
        }
        return LRESULT(1);
    }

    // SAFETY: lê o ponteiro de contexto armazenado no GWLP_USERDATA.
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut TrayContext;
    if ptr.is_null() {
        // SAFETY: sem ponteiro de contexto, passa a mensagem ao procedimento padrão.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // SAFETY: o Box pertence à thread e vive até o fim do loop (inclusive WM_NCDESTROY).
    // Só copiamos dados neste escopo: nenhuma referência ao contexto atravessa APIs reentrantes.
    let (state, on_action, mut nid, taskbar_created_msg, available) = unsafe {
        let ctx = &*ptr;
        (
            ctx.state.clone(),
            ctx.on_action.clone(),
            ctx.nid,
            ctx.taskbar_created_msg,
            ctx.available.clone(),
        )
    };

    match msg {
        WM_CREATE => {
            nid.hWnd = hwnd;
            // SAFETY: acesso pontual na thread dona, sem chamada reentrante durante a escrita.
            unsafe { (*ptr).nid = nid };
            // SAFETY: adiciona o ícone na bandeja do Windows.
            let added = register_icon(&available, || unsafe {
                Shell_NotifyIconW(NIM_ADD, &nid).as_bool()
            });
            LRESULT(if added { 0 } else { -1 })
        }
        WM_TRAY_CALLBACK => {
            let event = lparam.0 as u32;
            match event {
                // Clique com botão esquerdo ou duplo clique reabre o Tako onde estava.
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    on_action(TrayAction::Open);
                }
                // Clique com botão direito abre o menu de contexto da bandeja.
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    let snapshot = state.lock().map(|s| s.clone()).unwrap_or_default();
                    show_context_menu(hwnd, snapshot, on_action);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd = wparam.0 & 0xffff;
            match cmd {
                1 => on_action(TrayAction::Open),
                2 => on_action(TrayAction::Central),
                99 => on_action(TrayAction::Quit),
                id @ 100..=120 => {
                    let idx = id - 100;
                    let state = state.lock().map(|s| s.clone()).unwrap_or_default();
                    if let Some(game) = state.recent_games.get(idx) {
                        on_action(TrayAction::Launch(game.id.clone()));
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            available.store(false, Ordering::Release);
            // SAFETY: remove o ícone da bandeja ao fechar a janela.
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            // SAFETY: desassocia o contexto; o Box continua vivo na thread até o loop terminar.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        m if m == taskbar_created_msg && taskbar_created_msg != 0 => {
            // Se o Explorer reiniciou, re-adiciona o ícone.
            // SAFETY: chamada Shell_NotifyIconW em recuperação do Explorer.
            register_icon(&available, || unsafe {
                Shell_NotifyIconW(NIM_ADD, &nid).as_bool()
            });
            LRESULT(0)
        }
        // SAFETY: repassa mensagens não tratadas ao DefWindowProcW.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn register_icon(available: &AtomicBool, add: impl FnOnce() -> bool) -> bool {
    let added = add();
    available.store(added, Ordering::Release);
    if !added {
        tracing::warn!("Windows não adicionou o ícone da bandeja; fechar encerra o aplicativo");
    }
    added
}

fn show_context_menu(hwnd: HWND, state: TrayState, on_action: Rc<dyn Fn(TrayAction) + Send>) {
    let mut pt = POINT::default();
    // SAFETY: lê a posição do cursor e traz a janela oculta para o foco antes de TrackPopupMenu.
    unsafe {
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
    }

    // SAFETY: cria o menu popup de contexto da bandeja.
    let Ok(hmenu) = (unsafe { CreatePopupMenu() }) else {
        return;
    };

    let lang = state.lang;

    // 1. Abrir o Tako
    let open_text = encode_wide(&tako_core::i18n::t_in(lang, "tray.open"));
    // SAFETY: adiciona o item "Abrir o Tako" no menu.
    unsafe {
        let _ = AppendMenuW(hmenu, MF_STRING, 1, PCWSTR(open_text.as_ptr()));
    }

    // 2. Jogos recentes (se houver, até 5)
    if !state.recent_games.is_empty() {
        // SAFETY: adiciona separador para os jogos recentes.
        unsafe {
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        }
        for (idx, game) in state.recent_games.iter().enumerate().take(5) {
            let title = encode_wide(&game.title);
            // SAFETY: adiciona item de jogo recente.
            unsafe {
                let _ = AppendMenuW(hmenu, MF_STRING, 100 + idx, PCWSTR(title.as_ptr()));
            }
        }
    }

    // 3. Central de Controle (se houver jogo aberto)
    if state.game_active {
        // SAFETY: adiciona separador antes da Central de Controle.
        unsafe {
            let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        }
        let central_text = encode_wide(&tako_core::i18n::t_in(lang, "tray.central"));
        // SAFETY: adiciona "Central de Controle" no menu.
        unsafe {
            let _ = AppendMenuW(hmenu, MF_STRING, 2, PCWSTR(central_text.as_ptr()));
        }
    }

    // 4. Separador antes de Sair
    // SAFETY: adiciona separador antes de Sair.
    unsafe {
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
    }

    // 5. Sair do Tako
    let quit_text = encode_wide(&tako_core::i18n::t_in(lang, "tray.quit"));
    // SAFETY: adiciona "Sair do Tako" no menu.
    unsafe {
        let _ = AppendMenuW(hmenu, MF_STRING, 99, PCWSTR(quit_text.as_ptr()));
    }

    // SAFETY: exibe o menu e aguarda a seleção do usuário.
    let cmd = unsafe {
        TrackPopupMenu(
            hmenu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            pt.x,
            pt.y,
            Some(0),
            hwnd,
            None,
        )
    };

    // SAFETY: envia WM_NULL necessário para fechamento do menu e destrói o HMENU.
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(hmenu);
    }

    match cmd.0 as usize {
        1 => on_action(TrayAction::Open),
        2 => on_action(TrayAction::Central),
        99 => on_action(TrayAction::Quit),
        id @ 100..=120 => {
            let idx = id - 100;
            if let Some(game) = state.recent_games.get(idx) {
                on_action(TrayAction::Launch(game.id.clone()));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_icon_registration_is_unavailable_including_after_explorer_restart() {
        let available = AtomicBool::new(false);
        assert!(!register_icon(&available, || false));
        assert!(!available.load(Ordering::Acquire));
        assert!(register_icon(&available, || true));
        assert!(available.load(Ordering::Acquire));
        assert!(!register_icon(&available, || false));
        assert!(!available.load(Ordering::Acquire));
    }

    #[test]
    fn owned_icon_is_destroyed_without_destroying_the_shared_fallback() {
        // SAFETY: LoadIconW é compartilhado; só a cópia de CopyIcon é entregue ao guardião de posse.
        unsafe {
            let shared = LoadIconW(None, IDI_APPLICATION).unwrap();
            let owned = CopyIcon(shared).unwrap();
            drop(OwnedIcon(owned));
            assert!(
                DestroyIcon(owned).is_err(),
                "o guardião não liberou o ícone próprio"
            );
            // A origem compartilhada continua válida depois da limpeza.
            drop(OwnedIcon(CopyIcon(shared).unwrap()));
        }
    }

    #[test]
    fn repeated_popup_menus_dispatch_reentrant_commands_and_close_cleanly() {
        let (tx, actions) = mpsc::channel();
        let tray = WindowsTray::new(Arc::new(Mutex::new(TrayState::default())), move |action| {
            let _ = tx.send(action);
        })
        .unwrap();
        assert!(tray.is_available());
        for _ in 0..5 {
            // SAFETY: a janela pertence a este teste. O menu bombeia WM_COMMAND antes de ser cancelado.
            unsafe {
                let hwnd = Some(HWND(tray.hwnd as *mut _));
                PostMessageW(
                    hwnd,
                    WM_TRAY_CALLBACK,
                    WPARAM(0),
                    LPARAM(WM_RBUTTONUP as isize),
                )
                .unwrap();
                PostMessageW(hwnd, WM_COMMAND, WPARAM(1), LPARAM(0)).unwrap();
            }
            let action = actions.recv_timeout(std::time::Duration::from_secs(2));
            // SAFETY: cancela apenas o menu da nossa janela, inclusive se a asserção abaixo falhar.
            unsafe {
                PostMessageW(
                    Some(HWND(tray.hwnd as *mut _)),
                    WM_CANCELMODE,
                    WPARAM(0),
                    LPARAM(0),
                )
                .unwrap();
            }
            assert_eq!(action.unwrap(), TrayAction::Open);
        }
        drop(tray);
    }

    #[test]
    fn tray_creation_and_teardown() {
        let state = Arc::new(Mutex::new(TrayState::default()));
        let tray = WindowsTray::new(state, |_| {});
        assert!(tray.is_some());
        drop(tray);
    }
}
