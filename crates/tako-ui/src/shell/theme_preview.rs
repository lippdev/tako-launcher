//! Prévia de tema (THEME-002): escolher um tema na lista o põe na tela sem salvar. "Aplicar" salva a escolha;
//! "Voltar" (B), o tempo esgotar ou a camada sumir devolvem o tema que valia. Nada vai para o disco antes de
//! confirmar, então uma queda durante a prévia reabre o Tako no tema de antes.
//!
//! A camada não desenha nada: o tema inteiro precisa aparecer, sem diálogo nem véu por cima. Quem fala é a barra de
//! dicas (Aplicar / Voltar com a contagem), que todo tema já mostra.

use super::layer::{Flow, Layer};
use super::settings::{BUILTIN_THEME, themes_panel};
use super::{Cmd, Ctx, Effect, MenuLayer};
use crate::model::{Hint, Overlay};
use std::time::{Duration, Instant};

/// Quanto tempo a prévia espera por uma resposta antes de voltar sozinha.
pub const PREVIEW_TIME: Duration = Duration::from_secs(15);

pub struct ThemePreview {
    /// O tema em prévia (`None` = o padrão).
    pub id: Option<String>,
    pub until: Instant,
}

struct PreviewLayer {
    name: String,
}

/// Põe `id` (ou o padrão) na tela como prévia e troca a lista de temas pela camada da prévia.
pub fn start(ctx: &mut Ctx, id: Option<&str>, name: &str) -> Flow {
    ctx.theme_preview = Some(ThemePreview {
        id: id.map(str::to_owned),
        until: Instant::now() + PREVIEW_TIME,
    });
    ctx.effect(Effect::ApplyTheme(id.unwrap_or(BUILTIN_THEME).to_owned()));
    let text = ctx.tf("settings.appearance.preview_started", &[("name", name)]);
    ctx.toast(text);
    Flow::Replace(Box::new(PreviewLayer {
        name: name.to_owned(),
    }))
}

/// A lista de temas com o foco no tema que estava em prévia.
pub fn panel(ctx: &Ctx, id: Option<&str>) -> MenuLayer {
    let mut panel = themes_panel(ctx);
    let key = id.unwrap_or(BUILTIN_THEME);
    panel.focus = panel
        .items(ctx)
        .iter()
        .position(|i| i.row.key == key)
        .unwrap_or(usize::MAX);
    panel
}

/// A tela já trocou para a prévia: recarrega o tema que vale (o escolhido, ou o padrão).
pub fn reload_selected(ctx: &mut Ctx) {
    let id = tako_themes::selected(&ctx.paths.themes()).unwrap_or_else(|| BUILTIN_THEME.to_owned());
    ctx.effect(Effect::ApplyTheme(id));
}

impl Layer for PreviewLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let Some(p) = ctx.theme_preview.take() else {
            return Flow::Close;
        };
        match cmd {
            Cmd::Accept => match tako_themes::select(&ctx.paths.themes(), p.id.as_deref()) {
                // A tela já mostra esse tema: não precisa recarregar.
                Ok(()) => {
                    let text = ctx.tf(
                        "settings.appearance.preview_applied",
                        &[("name", &self.name)],
                    );
                    ctx.toast(text);
                }
                Err(e) => {
                    ctx.toast(e.to_string());
                    reload_selected(ctx);
                }
            },
            Cmd::Back => reload_selected(ctx),
            _ => {
                ctx.theme_preview = Some(p);
                return Flow::Stay;
            }
        }
        Flow::Replace(Box::new(panel(ctx, p.id.as_deref())))
    }

    fn render(&self, _ctx: &Ctx) -> Overlay {
        // `kind` vazio: o tema não desenha nada para esta camada.
        Overlay::default()
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let secs = ctx.theme_preview.as_ref().map_or(0, |p| {
            p.until.saturating_duration_since(Instant::now()).as_secs() + 1
        });
        vec![
            ctx.hint(Cmd::Accept, &ctx.t("settings.appearance.preview_apply")),
            ctx.hint(
                Cmd::Back,
                &ctx.tf(
                    "settings.appearance.preview_back",
                    &[("secs", &secs.to_string())],
                ),
            ),
        ]
    }

    fn live(&self) -> bool {
        true
    }

    fn theme_preview(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LANG_LOCK as LOCK;
    use crate::shell::Shell;
    use tako_core::i18n::{self, Lang};
    use tako_core::paths::DataPaths;
    use tako_platform::Platform;

    const THEME: &str = "meu-tema";

    fn setup() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = crate::app::build_ctx(DataPaths::at(dir.path()), Platform::simulated(), true);
        i18n::set_lang(Lang::PtBr);
        let theme = ctx.paths.themes().join(THEME);
        std::fs::create_dir_all(&theme).unwrap();
        std::fs::write(
            theme.join("theme.toml"),
            format!(
                "id = \"{THEME}\"\nname = \"Meu tema\"\nversion = \"1.0.0\"\nauthor = \"Eu\"\napi = 1\nentry = \"main.slint\"\n"
            ),
        )
        .unwrap();
        std::fs::write(
            theme.join("main.slint"),
            "export { App } from \"@tako/main.slint\";",
        )
        .unwrap();
        (dir, ctx)
    }

    fn reloads(ctx: &Ctx) -> usize {
        ctx.effects
            .iter()
            .filter(|e| matches!(e, Effect::ApplyTheme(_)))
            .count()
    }

    #[test]
    fn previewing_shows_the_theme_without_saving_it() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = setup();
        let flow = start(&mut ctx, Some(THEME), "Meu tema");
        assert!(matches!(flow, Flow::Replace(_)));
        assert_eq!(
            ctx.theme_preview.as_ref().unwrap().id.as_deref(),
            Some(THEME)
        );
        assert_eq!(tako_themes::selected(&ctx.paths.themes()), None);
        assert_eq!(reloads(&ctx), 1);
        assert!(ctx.toast.unwrap().0.contains("Meu tema"));
    }

    #[test]
    fn apply_saves_the_choice_and_back_restores_the_previous_theme() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = setup();
        // Voltar (B): nada salvo, o tema anterior é recarregado, a lista volta com o foco no tema.
        let Flow::Replace(mut layer) = start(&mut ctx, Some(THEME), "Meu tema") else {
            panic!("a prévia troca a lista");
        };
        ctx.effects.clear();
        assert!(matches!(layer.handle(Cmd::Down, &mut ctx), Flow::Stay));
        assert!(ctx.theme_preview.is_some());
        assert!(matches!(
            layer.handle(Cmd::Back, &mut ctx),
            Flow::Replace(_)
        ));
        assert!(ctx.theme_preview.is_none());
        assert_eq!(tako_themes::selected(&ctx.paths.themes()), None);
        assert_eq!(reloads(&ctx), 1);
        // Aplicar (A): salva e não recarrega, a tela já mostra o tema.
        let Flow::Replace(mut layer) = start(&mut ctx, Some(THEME), "Meu tema") else {
            panic!();
        };
        ctx.effects.clear();
        assert!(matches!(
            layer.handle(Cmd::Accept, &mut ctx),
            Flow::Replace(_)
        ));
        assert!(ctx.theme_preview.is_none());
        assert_eq!(
            tako_themes::selected(&ctx.paths.themes()).as_deref(),
            Some(THEME)
        );
        assert_eq!(reloads(&ctx), 0);
        // O padrão também entra em prévia quando outro tema está escolhido, e voltar recarrega o escolhido.
        let Flow::Replace(mut layer) = start(&mut ctx, None, "Tako (padrão)") else {
            panic!();
        };
        ctx.effects.clear();
        layer.handle(Cmd::Back, &mut ctx);
        assert!(matches!(ctx.effects.last(), Some(Effect::ApplyTheme(id)) if id == THEME));
    }

    #[test]
    fn the_preview_goes_back_alone_on_timeout_or_when_its_layer_disappears() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, ctx) = setup();
        let mut shell = Shell::new(ctx);
        let flow = start(&mut shell.ctx, Some(THEME), "Meu tema");
        let Flow::Replace(layer) = flow else { panic!() };
        shell.push_layer(layer);
        shell.ctx.effects.clear();
        // Dentro do prazo e com a camada: nada muda.
        shell.tick_theme_preview(Instant::now());
        assert!(shell.ctx.theme_preview.is_some());
        assert_eq!(reloads(&shell.ctx), 0);
        // Prazo vencido: volta ao anterior e a lista de temas reaparece.
        shell.tick_theme_preview(Instant::now() + PREVIEW_TIME);
        assert!(shell.ctx.theme_preview.is_none());
        assert_eq!(reloads(&shell.ctx), 1);
        assert!(shell.has_layers() && !shell.hints().is_empty());
        // Camada fechada por outro caminho (ex.: fechar tudo): a prévia não fica órfã.
        let Flow::Replace(layer) = start(&mut shell.ctx, Some(THEME), "Meu tema") else {
            panic!()
        };
        shell.push_layer(layer);
        shell.close_layers();
        shell.ctx.effects.clear();
        shell.tick_theme_preview(Instant::now());
        assert!(shell.ctx.theme_preview.is_none());
        assert_eq!(reloads(&shell.ctx), 1);
        assert!(!shell.has_layers());
    }

    #[test]
    fn a_broken_theme_in_preview_falls_back_to_the_default_and_is_reported() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = setup();
        let builtin = crate::theme::install_builtin(&ctx.paths.themes()).unwrap();
        start(&mut ctx, Some(THEME), "Meu tema");
        let (_, dir) = crate::app::load_theme(&ctx, &builtin).unwrap();
        assert_eq!(
            dir,
            Some(ctx.paths.themes().join(THEME)),
            "tema bom carrega"
        );
        // O `.slint` quebra: o carregamento cai no padrão (sem pânico) e o app sabe que a prévia falhou.
        std::fs::write(
            ctx.paths.themes().join(THEME).join("main.slint"),
            "export component App inherits Window { x: ; }",
        )
        .unwrap();
        let (_, dir) = crate::app::load_theme(&ctx, &builtin).unwrap();
        assert_eq!(dir, None);
        assert!(ctx.theme_preview_wants_installed());
        // Prévia do padrão nunca "falha" por isso.
        ctx.theme_preview = Some(ThemePreview {
            id: None,
            until: Instant::now(),
        });
        assert!(!ctx.theme_preview_wants_installed());
        // O aviso e a saída da prévia.
        let mut shell = Shell::new(ctx);
        shell.ctx.theme_preview = Some(ThemePreview {
            id: Some(THEME.into()),
            until: Instant::now() + PREVIEW_TIME,
        });
        shell.push_layer(Box::new(PreviewLayer { name: "x".into() }));
        shell.ctx.effects.clear();
        shell.fail_theme_preview();
        assert!(shell.ctx.theme_preview.is_none());
        assert!(shell.ctx.toast.as_ref().unwrap().0.contains("não carregou"));
        assert_eq!(reloads(&shell.ctx), 0);
    }

    #[test]
    fn the_hints_count_down_and_the_texts_exist_in_both_languages() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = setup();
        let Flow::Replace(layer) = start(&mut ctx, Some(THEME), "Meu tema") else {
            panic!()
        };
        let hints = layer.hints(&ctx);
        assert_eq!(hints[0].label, "Aplicar tema");
        assert!(hints[1].label.contains("15 s"), "{}", hints[1].label);
        i18n::set_lang(Lang::EnUs);
        assert_eq!(layer.hints(&ctx)[0].label, "Apply theme");
        i18n::set_lang(Lang::PtBr);
    }
}
