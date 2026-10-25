//! Bandeja do sistema no Windows (WEB-36, PRD D-10 e D-15).
//!
//! Registra um ícone de notificação na barra de tarefas (`Shell_NotifyIconW`) e gerencia
//! o menu de contexto suspenso (`TrackPopupMenu`).

use crate::tray::{TrayAction, TrayState};
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
    on_action: Box<dyn Fn(TrayAction) + Send + 'static>,
    nid: NOTIFYICONDATAW,
    taskbar_created_msg: u32,
}

pub struct WindowsTray {
    hwnd: isize,
    handle: Option<JoinHandle<()>>,
}

impl WindowsTray {
    pub fn new(
        state: Arc<Mutex<TrayState>>,
        on_action: impl Fn(TrayAction) + Send + 'static,
    ) -> Option<Self> {
        let (tx, rx) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name("tako-tray".into())
            .spawn(move || {
                run_tray_thread(state, Box::new(on_action), tx);
            })
            .ok()?;

        let hwnd = rx.recv().ok()??;
        Some(Self {
            hwnd,
            handle: Some(handle),
        })
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
    let icon = unsafe {
        let cx = GetSystemMetrics(SM_CXSMICON);
        let cy = GetSystemMetrics(SM_CYSMICON);
        LoadImageW(
            Some(hinst),
            PCWSTR(1 as _),
            IMAGE_ICON,
            cx,
            cy,
            LR_DEFAULTCOLOR,
        )
        .map(|h| HICON(h.0))
        .or_else(|_| LoadIconW(None, IDI_APPLICATION))
        .unwrap_or_default()
    };
    nid.hIcon = icon;

    let ctx = Box::new(TrayContext {
        state,
        on_action,
        nid,
        taskbar_created_msg,
    });
    let ctx_ptr = Box::into_raw(ctx);

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
        // SAFETY: recupera o Box se a janela não foi criada para evitar vazamento de memória.
        unsafe {
            let _ = Box::from_raw(ctx_ptr);
        }
        let _ = ready_tx.send(None);
        return;
    };

    let _ = ready_tx.send(Some(hwnd.0 as isize));

    // SAFETY: loop de mensagens padrão do Win32.
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
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
    // SAFETY: ptr é um ponteiro válido para TrayContext alocado via Box::into_raw.
    let ctx = unsafe { &mut *ptr };

    match msg {
        WM_CREATE => {
            ctx.nid.hWnd = hwnd;
            // SAFETY: adiciona o ícone na bandeja do Windows.
            unsafe {
                let _ = Shell_NotifyIconW(NIM_ADD, &ctx.nid);
            }
            LRESULT(0)
        }
        WM_TRAY_CALLBACK => {
            let event = lparam.0 as u32;
            match event {
                // Clique com botão esquerdo ou duplo clique reabre o Tako onde estava.
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    (ctx.on_action)(TrayAction::Open);
                }
                // Clique com botão direito abre o menu de contexto da bandeja.
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    show_context_menu(hwnd, ctx);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd = wparam.0 & 0xffff;
            match cmd {
                1 => (ctx.on_action)(TrayAction::Open),
                2 => (ctx.on_action)(TrayAction::Central),
                99 => (ctx.on_action)(TrayAction::Quit),
                id @ 100..=120 => {
                    let idx = id - 100;
                    let state = ctx.state.lock().map(|s| s.clone()).unwrap_or_default();
                    if let Some(game) = state.recent_games.get(idx) {
                        (ctx.on_action)(TrayAction::Launch(game.id.clone()));
                    }
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: remove o ícone da bandeja ao fechar a janela.
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &ctx.nid);
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_NCDESTROY => {
            // SAFETY: limpa o GWLP_USERDATA e libera a memória do TrayContext.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(ptr);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        m if m == ctx.taskbar_created_msg && ctx.taskbar_created_msg != 0 => {
            // Se o Explorer reiniciou, re-adiciona o ícone.
            // SAFETY: chamada Shell_NotifyIconW em recuperação do Explorer.
            unsafe {
                let _ = Shell_NotifyIconW(NIM_ADD, &ctx.nid);
            }
            LRESULT(0)
        }
        // SAFETY: repassa mensagens não tratadas ao DefWindowProcW.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn show_context_menu(hwnd: HWND, ctx: &mut TrayContext) {
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

    let state = ctx.state.lock().map(|s| s.clone()).unwrap_or_default();
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
        1 => (ctx.on_action)(TrayAction::Open),
        2 => (ctx.on_action)(TrayAction::Central),
        99 => (ctx.on_action)(TrayAction::Quit),
        id @ 100..=120 => {
            let idx = id - 100;
            if let Some(game) = state.recent_games.get(idx) {
                (ctx.on_action)(TrayAction::Launch(game.id.clone()));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_creation_and_teardown() {
        let state = Arc::new(Mutex::new(TrayState::default()));
        let tray = WindowsTray::new(state, |_| {});
        assert!(tray.is_some());
        drop(tray);
    }
}
