//! Tarefas em segundo plano (rede, disco, lojas): `job` roda numa thread e `done` volta na thread da UI com
//! o resultado e o contexto, e a tela é redesenhada.

use crate::shell::{Ctx, Shell};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Done = Box<dyn FnOnce(&mut Ctx, Box<dyn Any + Send>)>;

thread_local! {
    static SHELL: RefCell<Weak<RefCell<Shell>>> = const { RefCell::new(Weak::new()) };
    static PENDING: RefCell<(u64, HashMap<u64, Done>)> = RefCell::new((0, HashMap::new()));
}

/// Liga as tarefas à `Shell` desta janela (chamado pelo `App`).
pub fn attach(shell: &Rc<RefCell<Shell>>) {
    SHELL.with(|s| *s.borrow_mut() = Rc::downgrade(shell));
}

pub fn shell() -> Option<Rc<RefCell<Shell>>> {
    SHELL.with(|s| s.borrow().upgrade())
}

/// Roda `job` fora da UI; `done` recebe o resultado na UI. Sem janela (testes), roda tudo na hora.
pub fn run<R: Send + 'static>(
    ctx: &mut Ctx,
    job: impl FnOnce() -> R + Send + 'static,
    done: impl FnOnce(&mut Ctx, R) + 'static,
) {
    if shell().is_none() {
        let r = job();
        done(ctx, r);
        return;
    }
    let id = PENDING.with(|p| {
        let mut p = p.borrow_mut();
        p.0 += 1;
        let id = p.0;
        p.1.insert(
            id,
            Box::new(move |ctx: &mut Ctx, any: Box<dyn Any + Send>| {
                if let Ok(r) = any.downcast::<R>() {
                    done(ctx, *r);
                }
            }),
        );
        id
    });
    std::thread::spawn(move || {
        let result: Box<dyn Any + Send> = Box::new(job());
        let _ = slint::invoke_from_event_loop(move || {
            let Some(done) = PENDING.with(|p| p.borrow_mut().1.remove(&id)) else {
                return;
            };
            if let Some(shell) = shell() {
                let mut s = shell.borrow_mut();
                done(&mut s.ctx, result);
                s.after_action(); // um diálogo aberto pelo resultado aparece já, sem esperar uma tecla
                s.ctx.dirty = true;
            }
        });
    });
}
