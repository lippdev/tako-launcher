//! Seletor de imagens (port de `ArtPickerViewModel.cs`, ADR-0023): abas de tipo (Capa, Fundo, Logo, Ícone) com
//! LB/RB, abas das fontes ligadas com "Arquivo do PC" no fim (↑ chega nelas, ←/→ trocam), grade de miniaturas e
//! prévia. A aplica e fixa por tipo, ☰ volta para a automática, Y escolhe outra imagem do PC, B fecha.

use super::layer::{Flow, Layer};
use super::{Cmd, Ctx};
use crate::metadata::{Pending, spawn};
use crate::model::{Hint, Overlay, Tile};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tako_core::library::{ArtKind, Game, manual};
use tako_core::metadata::MetadataAgent;
use tako_core::metadata::art::{
    ArtCandidate, FILE_SOURCE, kind_label, local_images, offers, pins, source_label,
};
use tako_core::metadata::art_catalog::{ArtCatalog, ArtQuery, known_ids};

/// Colunas da grade (o tema desenha miniaturas de largura fixa).
const COLS: usize = 5;
const THUMB: (u32, u32) = (300, 450);
const PREVIEW: (u32, u32) = (780, 780);

pub struct ArtPicker {
    game: Game,
    catalog: Arc<ArtCatalog>,
    ids: BTreeMap<String, String>,
    sources: Vec<&'static str>,
    kind: usize,
    source: usize,
    sel: usize,
    /// Foco nas abas de fonte (acima da grade).
    on_sources: bool,
    items: Pending<Vec<ArtCandidate>>,
}

pub fn picker(ctx: &Ctx, id: &str) -> Box<dyn Layer> {
    let game = ctx.library.find(id).cloned().unwrap_or_default();
    let catalog = ctx
        .metadata
        .as_ref()
        .map(|m| m.catalog.clone())
        .unwrap_or_else(|| Arc::new(crate::metadata::basic_catalog()));
    let cached = MetadataAgent::normalize(crate::metadata::cache(ctx).read(id));
    let pinned = ctx.pinned_ids.get(id).cloned().unwrap_or_default();
    let ids = known_ids(&game, cached.as_ref(), &pinned);
    let sources = catalog.sources(&game, &ids);
    let mut p = ArtPicker {
        game,
        catalog,
        ids,
        sources,
        kind: 0,
        source: 0,
        sel: 0,
        on_sources: false,
        items: spawn(Vec::new),
    };
    p.source = p.first_source();
    p.load();
    Box::new(p)
}

impl ArtPicker {
    fn kind(&self) -> ArtKind {
        ArtKind::ALL[self.kind]
    }

    fn source(&self) -> &'static str {
        self.sources
            .get(self.source)
            .copied()
            .unwrap_or(FILE_SOURCE)
    }

    fn first_source(&self) -> usize {
        let kind = self.kind();
        self.sources
            .iter()
            .position(|s| offers(s, kind))
            .unwrap_or(0)
    }

    fn load(&mut self) {
        self.sel = 0;
        let (catalog, game, ids) = (self.catalog.clone(), self.game.clone(), self.ids.clone());
        let (source, kind) = (self.source(), self.kind());
        self.items = spawn(move || {
            let q = ArtQuery {
                game: &game,
                source,
                kind,
                filter: None,
                steamgriddb_game: None,
                page: 0,
            };
            catalog.list(&q, &ids)
        });
    }

    fn items(&self) -> Vec<ArtCandidate> {
        self.items.get().unwrap_or_default()
    }

    fn set_kind(&mut self, k: usize) {
        self.kind = k % ArtKind::ALL.len();
        if !offers(self.source(), self.kind()) {
            self.source = self.first_source();
        }
        self.load();
    }

    fn set_source(&mut self, s: usize) {
        if s < self.sources.len() && s != self.source {
            self.source = s;
            self.load();
        }
    }

    fn status(&self, ctx: &Ctx) -> String {
        let kind = kind_label(self.kind()).to_lowercase();
        let source = source_label(self.source());
        match self.items.get() {
            None => ctx.tf(
                "art_picker.loading",
                &[("kind", &kind), ("source", &source)],
            ),
            Some(v) if v.is_empty() => match self.source() {
                FILE_SOURCE => ctx.t("art_picker.empty_file"),
                tako_core::metadata::sources::STEAM => ctx.t("art_picker.empty_steam"),
                _ => ctx.tf("art_picker.empty", &[("kind", &kind), ("source", &source)]),
            },
            Some(_) => String::new(),
        }
    }

    fn browse(&self, ctx: &mut Ctx) {
        let (id, kind) = (self.game.id.clone(), self.kind());
        let title = ctx.t("art_picker.choose_image");
        super::add_game::pick(
            ctx,
            move || {
                rfd::FileDialog::new()
                    .set_title(&title)
                    .add_filter("img", &local_images::EXTENSIONS)
                    .pick_file()
            },
            move |ctx, path| {
                let c = ArtCandidate::new(FILE_SOURCE, kind, path.to_string_lossy());
                apply(ctx, &id, &c);
            },
        );
    }
}

/// Copia a imagem do PC para `art-choices/`: apagar ou mover o original (ou limpar o cache) não some com ela.
pub fn copy_chosen(dir: &Path, path: &Path) -> Option<PathBuf> {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    std::fs::metadata(path).ok()?.modified().ok()?.hash(&mut h);
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let target = dir.join(format!("{:016x}{ext}", h.finish()));
    std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::copy(path, &target))
        .map_err(|e| tracing::warn!("imagem escolhida não copiada: {e}"))
        .ok()?;
    Some(target)
}

/// Fixa a imagem do tipo (`None` = volta para a automática): no banco, no que a biblioteca reaplica e no jogo.
pub fn set_pin(ctx: &mut Ctx, id: &str, kind: ArtKind, url: Option<&str>) -> bool {
    if let Some(c) = &ctx.choices
        && let Err(e) = c.set_art(id, kind, url)
    {
        tracing::warn!("imagem fixada de {id}: {e}");
        ctx.toast(ctx.t("art_picker.save_failed"));
        return false;
    }
    if let Ok(mut all) = ctx.art_pins.lock() {
        let next = pins::with(&all.get(id).cloned().unwrap_or_default(), kind, url);
        if next.is_empty() {
            all.remove(id);
        } else {
            all.insert(id.to_owned(), next);
        }
    }
    if let Some(g) = ctx.library.games_mut().iter_mut().find(|g| g.id == id) {
        g.pinned_art = pins::with(&g.pinned_art, kind, url);
    }
    ctx.dirty = true;
    true
}

/// A: aplica e fixa. Imagem do PC é copiada antes (numa thread).
pub fn apply(ctx: &mut Ctx, id: &str, c: &ArtCandidate) {
    let Some(kind) = c.kind else { return };
    let title = ctx
        .library
        .find(id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let toast = ctx.tf(
        "art_picker.applied",
        &[
            ("kind", &kind_label(kind)),
            ("game", &title),
            ("source", &source_label(&c.source)),
        ],
    );
    if c.source != FILE_SOURCE {
        if set_pin(ctx, id, kind, Some(&c.url)) {
            ctx.toast(toast);
        }
        return;
    }
    let (dir, path, id) = (
        ctx.paths.art_choices(),
        PathBuf::from(&c.url),
        id.to_owned(),
    );
    crate::tasks::run(
        ctx,
        move || copy_chosen(&dir, &path).unwrap_or(path),
        move |ctx, file| {
            let uri = manual::file_uri(&file.to_string_lossy());
            if set_pin(ctx, &id, kind, Some(&uri)) {
                ctx.toast(toast);
            }
        },
    );
}

impl Layer for ArtPicker {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let n = self.items().len();
        match cmd {
            Cmd::PrevSection => self.set_kind(self.kind + ArtKind::ALL.len() - 1),
            Cmd::NextSection => self.set_kind(self.kind + 1),
            Cmd::Left if self.on_sources => self.set_source(self.source.saturating_sub(1)),
            Cmd::Right if self.on_sources => self.set_source(self.source + 1),
            Cmd::Left if !self.sel.is_multiple_of(COLS) => self.sel -= 1,
            Cmd::Right if self.sel + 1 < n && !(self.sel + 1).is_multiple_of(COLS) => self.sel += 1,
            Cmd::Up if !self.on_sources && self.sel >= COLS => self.sel -= COLS,
            Cmd::Up => self.on_sources = true,
            Cmd::Down if self.on_sources => self.on_sources = false,
            Cmd::Down if self.sel + COLS < n => self.sel += COLS,
            Cmd::Down if self.sel / COLS < n.saturating_sub(1) / COLS => self.sel = n - 1,
            Cmd::Accept if self.on_sources && self.source() == FILE_SOURCE => self.browse(ctx),
            Cmd::Accept if self.on_sources => self.on_sources = false,
            Cmd::Accept => {
                if let Some(c) = self.items().get(self.sel) {
                    apply(ctx, &self.game.id, c);
                }
            }
            Cmd::Search => self.browse(ctx),
            Cmd::Menu => {
                let kind = self.kind();
                let game = ctx.library.find(&self.game.id);
                let pinned = game.is_some_and(|g| g.pinned_art.contains_key(&kind));
                let title = game.map(|g| g.title.clone()).unwrap_or_default();
                let args = [("kind", kind_label(kind)), ("game", title)];
                let args: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
                if !pinned {
                    ctx.toast(ctx.tf("art_picker.already_automatic", &args));
                } else if set_pin(ctx, &self.game.id, kind, None) {
                    ctx.toast(ctx.tf("art_picker.reset_done", &args));
                }
            }
            Cmd::Back => return Flow::Close,
            _ => {}
        }
        Flow::Stay
    }

    fn render(&self, ctx: &Ctx) -> Overlay {
        let kind = self.kind();
        let items = self.items();
        let shown = ctx
            .library
            .find(&self.game.id)
            .and_then(|g| pins::shown(g, kind).map(str::to_owned));
        let thumbs = items
            .iter()
            .map(|c| Tile {
                id: c.url.clone(),
                title: c.caption(),
                cover: crate::shell::tiles::image(Some(c.preview()), THUMB),
                favorite: shown.as_deref() == Some(c.url.as_str()),
                installed: true,
                ..Default::default()
            })
            .collect();
        let focused = items.get(self.sel).filter(|_| !self.on_sources);
        let preview_text = focused
            .map(|c| {
                let mut t = c.caption();
                if c.is_animated {
                    t.push_str(&format!(" · {}", ctx.t("art_picker.animated")));
                }
                t
            })
            .unwrap_or_default();
        Overlay {
            kind: "art".into(),
            title: ctx.tf("art_picker.title", &[("game", &self.game.title)]),
            text: self.status(ctx),
            tabs: ArtKind::ALL.iter().map(|k| kind_label(*k)).collect(),
            tab: self.kind as i32,
            tabs2: self.sources.iter().map(|s| source_label(s)).collect(),
            tab2: self.source as i32,
            thumbs,
            thumb_cols: COLS as i32,
            sel: if self.on_sources { -1 } else { self.sel as i32 },
            preview: crate::shell::tiles::image(focused.map(|c| c.url.as_str()), PREVIEW),
            preview_text,
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        vec![
            ctx.hint(Cmd::Accept, &ctx.t("art_picker.apply")),
            ctx.hint(Cmd::Back, &ctx.t("hint.close")),
            ctx.hint_raw(
                &format!(
                    "{}/{}",
                    ctx.glyph(Cmd::PrevSection),
                    ctx.glyph(Cmd::NextSection)
                ),
                &ctx.t("art_picker.kind"),
            ),
            ctx.hint(Cmd::Search, &ctx.t("art_picker.other_file")),
            ctx.hint(Cmd::Menu, &ctx.t("art_picker.automatic")),
        ]
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        match zone {
            "tab" => self.set_kind(index),
            "tab2" => {
                self.set_source(index);
                if self.source() == FILE_SOURCE && self.items().is_empty() {
                    self.browse(ctx);
                }
            }
            "thumb" => {
                self.on_sources = false;
                self.sel = index;
                return self.handle(Cmd::Accept, ctx);
            }
            _ => {}
        }
        Flow::Stay
    }

    fn hover(&mut self, zone: &str, index: usize) {
        if zone == "thumb" {
            self.on_sources = false;
            self.sel = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_art_is_listed_applied_pinned_and_reset() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        let id = ctx.library.games()[0].id.clone();
        ctx.library.games_mut()[0].steam_app_id = Some(367520);
        let mut p = picker(&ctx, &id);
        let o = p.render(&ctx);
        assert_eq!(o.kind, "art");
        assert_eq!(o.tabs, vec!["Capa", "Fundo", "Logo", "Ícone"]);
        assert_eq!(o.tabs2, vec!["Steam".to_string(), "Arquivo do PC".into()]);
        assert_eq!(o.thumbs.len(), 1);
        assert!(!o.thumbs[0].favorite);

        p.handle(Cmd::Accept, &mut ctx);
        let g = ctx.library.find(&id).unwrap();
        assert_eq!(g.pinned_art[&ArtKind::Cover], "steam-art://367520/cover");
        assert!(p.render(&ctx).thumbs[0].favorite);
        assert_eq!(ctx.art_pins.lock().unwrap()[&id].len(), 1);

        // RB: Fundo.
        p.handle(Cmd::NextSection, &mut ctx);
        assert_eq!(p.render(&ctx).tab, 1);
        // ☰ em Capa volta para a automática.
        p.handle(Cmd::PrevSection, &mut ctx);
        p.handle(Cmd::Menu, &mut ctx);
        assert!(ctx.library.find(&id).unwrap().pinned_art.is_empty());
        assert!(ctx.art_pins.lock().unwrap().get(&id).is_none());
    }

    #[test]
    fn chosen_files_are_copied() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("Capa.PNG");
        std::fs::write(&src, b"x").unwrap();
        let out = copy_chosen(&dir.path().join("art-choices"), &src).unwrap();
        assert!(out.is_file());
        assert_eq!(out.extension().unwrap(), "png");
        assert!(copy_chosen(dir.path(), &dir.path().join("nao-existe.png")).is_none());
    }
}
