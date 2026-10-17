//! Pedidos externos usam as mesmas ações e confirmações da navegação por controle.

use super::{Shell, details};
use tako_core::links::Link;

impl Shell {
    pub fn open_link(&mut self, link: Link) {
        if self.ctx.scanning {
            self.pending_link = Some(link);
            return;
        }
        self.close_control_center();
        match link {
            Link::Screen(route) => self.open_route(route),
            Link::Game { id, play } => {
                if self.ctx.library.find(&id).is_none() {
                    self.ctx.toast(self.ctx.t("links.missing_game"));
                    return;
                }
                self.open_route("library:all");
                self.ctx.open_details = Some(id.clone());
                self.after_action();
                if play {
                    let nav = details::primary_action(&mut self.ctx, &id);
                    self.apply_nav(nav);
                }
            }
        }
        self.ctx.dirty = true;
    }

    pub(crate) fn apply_pending_link(&mut self) {
        if let Some(link) = self.pending_link.take() {
            self.open_link(link);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    #[test]
    fn links_wait_for_discovery_and_unknown_ids_do_not_launch_anything() {
        let _guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        ctx.library.refresh();
        let id = ctx.library.games()[0].id.clone();
        let mut shell = Shell::new(ctx);
        shell.ctx.scanning = true;
        shell.open_link(Link::Game {
            id: id.clone(),
            play: false,
        });
        assert_eq!(shell.page().kind(), "home");
        shell.ctx.scanning = false;
        shell.apply_pending_link();
        assert_eq!(shell.page().kind(), "details");
        assert!(shell.ctx.effects.is_empty());
        shell.open_link(Link::Game {
            id: "missing".into(),
            play: true,
        });
        assert!(shell.ctx.effects.is_empty());
        assert!(shell.ctx.toast.is_some());
        shell.open_link(Link::Screen("settings"));
        assert_eq!(shell.page().kind(), "settings");
    }
}
