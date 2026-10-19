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
    /// Sobreposição temporária em teste (THEME-003). Se confirmado, grava em disco; se voltar, descarta.
    pub draft_override: Option<tako_themes::overrides::ThemeTokensOverride>,
    pub create_base: Option<Option<String>>,
}

/// Um tema pode chamar `hint-clicked(0)` no `init` e confirmar a própria prévia: Aplicar só vale depois disso.
const ACCEPT_AFTER: Duration = Duration::from_millis(500);

struct PreviewLayer {
    name: String,
}

/// Põe `id` (ou o padrão) na tela como prévia e troca a lista de temas pela camada da prévia.
pub fn start(ctx: &mut Ctx, id: Option<&str>, name: &str) -> Flow {
    ctx.theme_preview = Some(ThemePreview {
        id: id.map(str::to_owned),
        until: Instant::now() + PREVIEW_TIME,
        draft_override: None,
        create_base: None,
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

/// Inicia a prévia ao vivo de uma sobreposição de tema (THEME-003).
pub fn start_override(
    ctx: &mut Ctx,
    theme_id: Option<&str>,
    draft: tako_themes::overrides::ThemeTokensOverride,
) -> Flow {
    ctx.theme_preview = Some(ThemePreview {
        id: theme_id.map(str::to_owned),
        until: Instant::now() + PREVIEW_TIME,
        draft_override: Some(draft),
        create_base: None,
    });
    ctx.effect(Effect::ApplyTheme(
        theme_id.unwrap_or(BUILTIN_THEME).to_owned(),
    ));
    let text = ctx.t("settings.appearance.theme_edit_preview_started");
    ctx.toast(text);
    Flow::Replace(Box::new(OverridePreviewLayer {
        theme_id: theme_id.map(str::to_owned),
    }))
}

struct OverridePreviewLayer {
    #[allow(dead_code)]
    theme_id: Option<String>,
}

impl Layer for OverridePreviewLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let Some(p) = ctx.theme_preview.take() else {
            return Flow::Close;
        };
        match cmd {
            Cmd::Accept
                if p.until.saturating_duration_since(Instant::now())
                    > PREVIEW_TIME - ACCEPT_AFTER =>
            {
                ctx.theme_preview = Some(p);
                return Flow::Stay;
            }
            Cmd::Accept => {
                if let Some(mut draft) = p.draft_override {
                    if let Some(ref bg_draft) = draft.background_image {
                        match tako_themes::overrides::promote_background_draft(
                            &ctx.paths.themes(),
                            p.id.as_deref(),
                            bg_draft,
                        ) {
                            Ok(final_name) => {
                                draft.background_image = Some(final_name);
                            }
                            Err(e) => {
                                tracing::error!("promover rascunho de fundo: {e}");
                                ctx.toast(ctx.t("settings.appearance.save_override_error"));
                                reload_selected(ctx);
                                return Flow::Replace(Box::new(super::settings::edit_theme_panel(
                                    ctx,
                                )));
                            }
                        }
                    } else {
                        let _ = tako_themes::overrides::clean_background_orphans(
                            &ctx.paths.themes(),
                            p.id.as_deref(),
                            None,
                        );
                    }
                    let toml_str = draft.to_toml_string();
                    match tako_themes::overrides::save_override(
                        &ctx.paths.themes(),
                        p.id.as_deref(),
                        &toml_str,
                    ) {
                        Ok(_) => {
                            let text = ctx.t("settings.appearance.theme_customized_applied");
                            ctx.toast(text);
                            ctx.theme_draft = None;
                            ctx.reload_theme_cache();
                        }
                        Err(e) => {
                            tracing::error!("salvar sobreposição: {e}");
                            ctx.toast(ctx.t("settings.appearance.save_override_error"));
                            reload_selected(ctx);
                        }
                    }
                }
            }
            Cmd::Back => {
                reload_selected(ctx);
            }
            _ => {
                ctx.theme_preview = Some(p);
                return Flow::Stay;
            }
        }
        Flow::Replace(Box::new(super::settings::edit_theme_panel(ctx)))
    }

    fn render(&self, _ctx: &Ctx) -> Overlay {
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

/// Inicia a prévia ao vivo durante a criação de um novo tema (THEME-006).
pub fn start_override_create(
    ctx: &mut Ctx,
    base_id: Option<&str>,
    draft: tako_themes::overrides::ThemeTokensOverride,
) -> Flow {
    ctx.theme_preview = Some(ThemePreview {
        id: base_id.map(str::to_owned),
        until: Instant::now() + PREVIEW_TIME,
        draft_override: Some(draft),
        create_base: Some(base_id.map(str::to_owned)),
    });
    ctx.effect(Effect::ApplyTheme(
        base_id.unwrap_or(BUILTIN_THEME).to_owned(),
    ));
    let text = ctx.t("settings.appearance.theme_edit_preview_started");
    ctx.toast(text);
    Flow::Replace(Box::new(OverridePreviewCreateLayer {
        base_id: base_id.map(str::to_owned),
    }))
}

struct OverridePreviewCreateLayer {
    base_id: Option<String>,
}

impl Layer for OverridePreviewCreateLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let Some(p) = ctx.theme_preview.take() else {
            return Flow::Close;
        };
        match cmd {
            Cmd::Accept
                if p.until.saturating_duration_since(Instant::now())
                    > PREVIEW_TIME - ACCEPT_AFTER =>
            {
                ctx.theme_preview = Some(p);
                return Flow::Stay;
            }
            Cmd::Accept => {
                reload_selected(ctx);
                return Flow::Replace(Box::new(super::settings::prompt_theme_name(
                    ctx,
                    self.base_id.clone(),
                    true,
                )));
            }
            Cmd::Back => {
                reload_selected(ctx);
            }
            _ => {
                ctx.theme_preview = Some(p);
                return Flow::Stay;
            }
        }
        Flow::Replace(Box::new(super::settings::create_theme_editor_panel(
            ctx,
            self.base_id.clone(),
        )))
    }

    fn render(&self, _ctx: &Ctx) -> Overlay {
        Overlay::default()
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let secs = ctx.theme_preview.as_ref().map_or(0, |p| {
            p.until.saturating_duration_since(Instant::now()).as_secs() + 1
        });
        vec![
            ctx.hint(Cmd::Accept, &ctx.t("settings.appearance.save_as_new_theme")),
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

impl Layer for PreviewLayer {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let Some(p) = ctx.theme_preview.take() else {
            return Flow::Close;
        };
        match cmd {
            Cmd::Accept
                if p.until.saturating_duration_since(Instant::now())
                    > PREVIEW_TIME - ACCEPT_AFTER =>
            {
                ctx.theme_preview = Some(p);
                return Flow::Stay;
            }
            Cmd::Accept => match tako_themes::select(&ctx.paths.themes(), p.id.as_deref()) {
                // A tela já mostra esse tema: não precisa recarregar.
                Ok(()) => {
                    ctx.theme_draft = None;
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

    /// Passa o tempo em que Aplicar é ignorado.
    fn settle(ctx: &mut Ctx) {
        ctx.theme_preview.as_mut().unwrap().until -= ACCEPT_AFTER;
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
        settle(&mut ctx);
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
            draft_override: None,
            create_base: None,
        });
        assert!(!ctx.theme_preview_wants_installed());
        // O aviso e a saída da prévia.
        let mut shell = Shell::new(ctx);
        shell.ctx.theme_preview = Some(ThemePreview {
            id: Some(THEME.into()),
            until: Instant::now() + PREVIEW_TIME,
            draft_override: None,
            create_base: None,
        });
        shell.push_layer(Box::new(PreviewLayer { name: "x".into() }));
        shell.ctx.effects.clear();
        shell.fail_theme_preview();
        assert!(shell.ctx.theme_preview.is_none());
        assert!(shell.ctx.toast.as_ref().unwrap().0.contains("não carregou"));
        assert_eq!(reloads(&shell.ctx), 0);
    }

    #[test]
    fn previewing_theme_override_applies_or_reverts() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = setup();
        let draft = tako_themes::overrides::ThemeTokensOverride {
            accent: Some("#E3161E".into()),
            font: Some("Poppins".into()),
            canvas: Some("#0B1A33".into()),
            background_image: None,
            ..Default::default()
        };

        // 1. Inicia a prévia ao vivo de sobreposição
        let Flow::Replace(mut layer) = start_override(&mut ctx, Some(THEME), draft.clone()) else {
            panic!("esperava Replace");
        };
        assert!(ctx.theme_preview.is_some());
        assert_eq!(
            ctx.theme_preview.as_ref().unwrap().draft_override,
            Some(draft.clone())
        );
        assert_eq!(reloads(&ctx), 1);

        // Voltar descarta a sobreposição sem salvar nada no disco
        let Flow::Replace(_) = layer.handle(Cmd::Back, &mut ctx) else {
            panic!();
        };
        assert!(ctx.theme_preview.is_none());
        assert!(!tako_themes::overrides::has_override(
            &ctx.paths.themes(),
            Some(THEME)
        ));

        // 2. Prévia e aceitar (Aplicar): salva em disco
        let Flow::Replace(mut layer) = start_override(&mut ctx, Some(THEME), draft.clone()) else {
            panic!();
        };
        settle(&mut ctx);
        let Flow::Replace(_) = layer.handle(Cmd::Accept, &mut ctx) else {
            panic!();
        };
        assert!(ctx.theme_preview.is_none());
        assert!(tako_themes::overrides::has_override(
            &ctx.paths.themes(),
            Some(THEME)
        ));
        let loaded =
            tako_themes::overrides::load_override(&ctx.paths.themes(), Some(THEME)).unwrap();
        assert!(loaded.contains("#E3161E"));
        assert!(loaded.contains("Poppins"));
    }

    #[test]
    fn builtin_extra_themes_can_preview_overrides() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = setup();
        let draft = tako_themes::overrides::ThemeTokensOverride {
            accent: Some("#E3161E".into()),
            ..Default::default()
        };

        for id in tako_themes::BUILTIN_EXTRA_THEME_IDS {
            ctx.effects.clear();
            let flow = start_override(&mut ctx, Some(id), draft.clone());
            assert!(matches!(flow, Flow::Replace(_)));
            assert_eq!(ctx.theme_preview.as_ref().unwrap().id.as_deref(), Some(id));
            assert_eq!(
                ctx.theme_preview.as_ref().unwrap().draft_override,
                Some(draft.clone())
            );
            assert!(matches!(ctx.effects.last(), Some(Effect::ApplyTheme(theme)) if theme == id));
            ctx.theme_preview = None;
        }
    }

    #[test]
    fn previewing_theme_override_timeout_returns_to_edit_panel_and_preserves_draft() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, ctx) = setup();
        let mut shell = Shell::new(ctx);

        let ov_dir = tako_themes::overrides::override_dir(&shell.ctx.paths.themes(), Some(THEME));
        std::fs::create_dir_all(&ov_dir).unwrap();
        let prev_img = ov_dir.join("background.png");
        std::fs::write(&prev_img, b"original").unwrap();

        let draft_img = ov_dir.join("background.draft.jpg");
        std::fs::write(&draft_img, b"draft").unwrap();

        let draft = tako_themes::overrides::ThemeTokensOverride {
            accent: Some("#E3161E".into()),
            background_image: Some("background.draft.jpg".into()),
            ..Default::default()
        };

        let Flow::Replace(layer) = start_override(&mut shell.ctx, Some(THEME), draft) else {
            panic!("esperava Replace");
        };
        shell.push_layer(layer);

        // Dispara o timeout da prévia
        shell.tick_theme_preview(Instant::now() + PREVIEW_TIME);

        // A prévia acabou
        assert!(shell.ctx.theme_preview.is_none());

        // O rascunho temporário NÃO é apagado ao expirar (o painel ainda usa)
        assert!(
            draft_img.is_file(),
            "background.draft.jpg deve continuar existindo para o painel"
        );

        // A imagem anterior original permaneceu intacta
        assert!(
            prev_img.is_file(),
            "background.png original deveria existir"
        );
        assert_eq!(std::fs::read(&prev_img).unwrap(), b"original");

        // A camada ativa após o timeout é o painel de edição
        assert!(shell.has_layers());
    }

    #[test]
    fn preview_back_save_does_not_lose_previous_background() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = setup();
        let _ = tako_themes::select(&ctx.paths.themes(), Some(THEME));

        let ov_dir = tako_themes::overrides::override_dir(&ctx.paths.themes(), Some(THEME));
        std::fs::create_dir_all(&ov_dir).unwrap();
        let prev_img = ov_dir.join("background.png");
        std::fs::write(&prev_img, b"original").unwrap();

        let draft_img = ov_dir.join("background.draft.jpg");
        std::fs::write(&draft_img, b"draft").unwrap();

        let draft = tako_themes::overrides::ThemeTokensOverride {
            accent: Some("#E3161E".into()),
            background_image: Some("background.draft.jpg".into()),
            ..Default::default()
        };
        ctx.theme_draft = Some(draft.clone());

        // Inicia a prévia ao vivo
        let Flow::Replace(mut layer) = start_override(&mut ctx, Some(THEME), draft) else {
            panic!("esperava Replace");
        };
        assert!(ctx.theme_preview.is_some());

        // Cancela com Back na prévia: volta ao painel de edição
        let Flow::Replace(mut edit_panel) = layer.handle(Cmd::Back, &mut ctx) else {
            panic!("esperava Replace para o painel de edição");
        };
        assert!(ctx.theme_preview.is_none());

        // O rascunho NÃO pode ter sido apagado pelo Back
        assert!(
            draft_img.is_file(),
            "background.draft.jpg não deve ser apagado no Back da prévia"
        );

        // Ao salvar no painel de edição, o rascunho é promovido para background.jpg
        let ov = edit_panel.render(&ctx);
        let save_idx = ov
            .rows
            .iter()
            .position(|r| r.key == "save")
            .expect("linha salvar");
        let flow = edit_panel.click("row", save_idx, &mut ctx);
        assert!(matches!(flow, Flow::Close));

        // Novo fundo salvo promovido com sucesso
        let new_img = ov_dir.join("background.jpg");
        assert!(new_img.is_file(), "background.jpg deve ter sido promovido");
        assert!(!draft_img.exists(), "rascunho deve ser consumido");
        assert!(!prev_img.exists(), "fundo anterior foi limpo");

        let loaded =
            tako_themes::overrides::load_override(&ctx.paths.themes(), Some(THEME)).unwrap();
        assert!(loaded.contains("background.jpg"));
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

    #[test]
    fn accept_is_ignored_right_after_the_preview_starts() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = setup();
        let Flow::Replace(mut layer) = start(&mut ctx, Some(THEME), "Meu tema") else {
            panic!()
        };
        // Um tema que chama `hint-clicked(0)` no `init` não confirma a própria prévia.
        assert!(matches!(layer.handle(Cmd::Accept, &mut ctx), Flow::Stay));
        assert!(ctx.theme_preview.is_some());
        assert_eq!(tako_themes::selected(&ctx.paths.themes()), None);
        settle(&mut ctx);
        assert!(matches!(
            layer.handle(Cmd::Accept, &mut ctx),
            Flow::Replace(_)
        ));
        assert_eq!(
            tako_themes::selected(&ctx.paths.themes()).as_deref(),
            Some(THEME)
        );
    }

    #[test]
    fn failing_without_a_preview_stays_quiet_and_entering_the_desktop_drops_it() {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, ctx) = setup();
        let mut shell = Shell::new(ctx);
        shell.fail_theme_preview();
        assert!(shell.ctx.toast.is_none());
        // O desktop não mostra temas: a prévia some junto com as camadas, sem pedir tema nem avisar.
        let Flow::Replace(layer) = start(&mut shell.ctx, Some(THEME), "Meu tema") else {
            panic!()
        };
        shell.push_layer(layer);
        shell.ctx.toast = None;
        shell.ctx.effects.clear();
        shell.enter_desktop();
        shell.tick_theme_preview(Instant::now());
        assert!(shell.ctx.theme_preview.is_none());
        assert_eq!(reloads(&shell.ctx), 0);
        assert!(shell.ctx.toast.is_none());
    }
}
