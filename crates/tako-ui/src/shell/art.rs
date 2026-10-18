//! Seletor de imagens (port de `ArtPickerViewModel.cs`, ADR-0023): abas de tipo (Capa, Fundo, Logo, Ícone) com
//! LB/RB, abas das fontes ligadas com "Arquivo do PC" no fim (↑ chega nelas, ←/→ trocam), grade de miniaturas e
//! prévia. A aplica e fixa por tipo, ☰ volta para a automática, Y escolhe outra imagem do PC, X abre os filtros do
//! SteamGridDB (estilo, formato, animadas), B fecha. O SteamGridDB vem em páginas: perto do fim carrega a próxima.

use super::layer::{Flow, Layer, MenuItem, MenuLayer, Pick};
use super::{Cmd, Ctx};
use crate::metadata::{Pending, spawn};
use crate::model::{Hint, Overlay, PanelRow, Tile};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use tako_core::library::{ArtKind, Game, manual};
use tako_core::metadata::MetadataAgent;
use tako_core::metadata::art::{
    ArtCandidate, FILE_SOURCE, kind_label, local_images, offers, pins, source_label,
};
use tako_core::metadata::art_catalog::{ArtCatalog, ArtQuery, known_ids};
use tako_core::metadata::sources::STEAMGRIDDB;
use tako_core::metadata::steamgriddb::{SteamGridDbFilter, style_label, styles};

/// Colunas da grade (o tema desenha miniaturas de largura fixa).
const COLS: usize = 5;
const THUMB: (u32, u32) = (300, 450);
const PREVIEW: (u32, u32) = (780, 780);
/// Página cheia do SteamGridDB: menos que isso = acabou (o C# fazia o mesmo).
const PAGE_SIZE: usize = 50;
/// Carrega a próxima página quando o foco chega a tantas miniaturas do fim.
const MORE_BEFORE_END: usize = 8;

type Pages = Rc<RefCell<Vec<Pending<Vec<ArtCandidate>>>>>;
type Fetch = Rc<dyn Fn(u32) -> Pending<Vec<ArtCandidate>>>;

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
    /// Filtro do SteamGridDB (o painel de filtros muda e recarrega).
    filter: Rc<RefCell<SteamGridDbFilter>>,
    /// Páginas carregadas (ou carregando), na ordem.
    pages: Pages,
    /// Desktop (SeletorDeImagens): clicar escolhe e "Usar" aplica; a tela cheia aplica no A.
    desktop: bool,
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
    let animated = catalog
        .steamgriddb
        .as_ref()
        .is_some_and(|p| p.client().settings().animated);
    let filter = SteamGridDbFilter {
        animated,
        ..SteamGridDbFilter::new(ArtKind::ALL[0])
    };
    let mut p = ArtPicker {
        game,
        catalog,
        ids,
        sources,
        kind: 0,
        source: 0,
        sel: 0,
        on_sources: false,
        filter: Rc::new(RefCell::new(filter)),
        pages: Rc::default(),
        desktop: ctx.desktop,
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

    /// Busca uma página da fonte e do tipo atuais com o filtro do momento (numa thread).
    fn fetch(&self) -> Fetch {
        let (catalog, game, ids) = (self.catalog.clone(), self.game.clone(), self.ids.clone());
        let (source, kind, filter) = (self.source(), self.kind(), self.filter.clone());
        Rc::new(move |page| {
            let (catalog, game, ids) = (catalog.clone(), game.clone(), ids.clone());
            let filter = (source == STEAMGRIDDB).then(|| filter.borrow().clone());
            spawn(move || {
                let q = ArtQuery {
                    game: &game,
                    source,
                    kind,
                    filter,
                    steamgriddb_game: None,
                    page,
                };
                catalog.list(&q, &ids)
            })
        })
    }

    fn load(&mut self) {
        self.sel = 0;
        *self.pages.borrow_mut() = vec![self.fetch()(0)];
    }

    /// As páginas já carregadas, em ordem (para na primeira que ainda carrega).
    fn items(&self) -> Vec<ArtCandidate> {
        self.pages
            .borrow()
            .iter()
            .map_while(Pending::get)
            .flatten()
            .collect()
    }

    /// Perto do fim do SteamGridDB, pede a próxima página (se a última veio cheia).
    fn more(&mut self) {
        if self.source() != STEAMGRIDDB {
            return;
        }
        let n = self.items().len();
        let full = self
            .pages
            .borrow()
            .last()
            .and_then(Pending::get)
            .is_some_and(|v| v.len() >= PAGE_SIZE);
        if full && self.sel + MORE_BEFORE_END >= n {
            let page = self.pages.borrow().len() as u32;
            let next = self.fetch()(page);
            self.pages.borrow_mut().push(next);
        }
    }

    /// X: estilo, formato (capa), animadas; adulto e humor ficam sempre de fora.
    fn filters(&self, ctx: &Ctx) -> MenuLayer {
        let (filter, pages, fetch) = (self.filter.clone(), self.pages.clone(), self.fetch());
        let change = Rc::new(move |f: &dyn Fn(&mut SteamGridDbFilter)| {
            f(&mut filter.borrow_mut());
            *pages.borrow_mut() = vec![fetch(0)];
        });
        let filter = self.filter.clone();
        MenuLayer::panel(ctx.t("art_picker.filters"), move |ctx| {
            let f = filter.borrow().clone();
            let mut v = Vec::new();
            let c = change.clone();
            v.push(
                MenuItem::new(PanelRow::choice(
                    "style",
                    ctx.t("art_picker.style"),
                    style_label(f.style.as_deref()),
                ))
                .on(move |_| {
                    c(&|f| {
                        let all = styles(f.kind);
                        f.style = match f
                            .style
                            .as_deref()
                            .and_then(|s| all.iter().position(|x| *x == s))
                        {
                            None => all.first(),
                            Some(i) => all.get(i + 1),
                        }
                        .map(|s| (*s).to_owned());
                    });
                    Flow::Stay
                })
                .options({
                    let filter = filter.clone();
                    move |_| {
                        // Nenhum e os estilos, na ordem do ciclo.
                        let f = filter.borrow();
                        let all = styles(f.kind);
                        let mut picks = vec![Pick::new(style_label(None))];
                        picks.extend(all.iter().map(|s| Pick::new(style_label(Some(s)))));
                        let cur = f
                            .style
                            .as_deref()
                            .and_then(|s| all.iter().position(|x| *x == s))
                            .map_or(0, |i| i + 1);
                        (picks, cur)
                    }
                }),
            );
            if f.kind == ArtKind::Cover {
                let c = change.clone();
                v.push(
                    MenuItem::new(PanelRow::choice(
                        "shape",
                        ctx.t("art_picker.shape"),
                        ctx.t(if f.horizontal {
                            "art_picker.horizontal"
                        } else {
                            "art_picker.vertical"
                        }),
                    ))
                    .on(move |_| {
                        c(&|f| f.horizontal = !f.horizontal);
                        Flow::Stay
                    })
                    .options({
                        let horizontal = f.horizontal;
                        move |ctx| {
                            let picks = ["art_picker.vertical", "art_picker.horizontal"]
                                .into_iter()
                                .map(|k| Pick::new(ctx.t(k)))
                                .collect();
                            (picks, usize::from(horizontal))
                        }
                    }),
                );
            }
            let c = change.clone();
            v.push(
                MenuItem::new(PanelRow::toggle(
                    "animated",
                    ctx.t("art_picker.animated_filter"),
                    f.animated,
                ))
                .on(move |_| {
                    c(&|f| f.animated = !f.animated);
                    Flow::Stay
                }),
            );
            v.push(MenuItem::new(
                PanelRow::info("safe", ctx.t("art_picker.safe"), "")
                    .desc(ctx.t("art_picker.safe_text")),
            ));
            v
        })
    }

    fn set_kind(&mut self, k: usize) {
        self.kind = k % ArtKind::ALL.len();
        // Tipo novo: estilo e formato voltam ao padrão, "animadas" fica (como no C#).
        let animated = self.filter.borrow().animated;
        *self.filter.borrow_mut() = SteamGridDbFilter {
            animated,
            ..SteamGridDbFilter::new(self.kind())
        };
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
        match self.pages.borrow().first().and_then(Pending::get) {
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

impl ArtPicker {
    /// Fonte (o rótulo do select), Estilo (só no SteamGridDB), do computador, Usar, Voltar à original, Cancelar e
    /// "Atual" (etiqueta da imagem em uso) e "Prévia", nessa ordem de chaves.
    fn desktop_rows(&self, ctx: &Ctx, can_use: bool) -> Vec<PanelRow> {
        vec![
            PanelRow::info("source", ctx.t("art_picker.source"), ""),
            PanelRow::button("style", ctx.t("art_picker.style"))
                .enabled(self.source() == STEAMGRIDDB),
            PanelRow::button("file", ctx.t("art_picker.from_pc")),
            PanelRow {
                style: "primary".into(),
                ..PanelRow::button("use", ctx.t("art_picker.use")).enabled(can_use)
            },
            PanelRow::button("original", ctx.t("art_picker.original")),
            PanelRow::button("cancel", ctx.t("common.cancel")),
            PanelRow::info("current", ctx.t("art_picker.current"), ""),
            PanelRow::info("preview", ctx.t("art_picker.preview"), ""),
        ]
    }
}

impl Layer for ArtPicker {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        let n = self.items().len();
        // Um filtro novo pode ter encurtado a lista.
        self.sel = self.sel.min(n.saturating_sub(1));
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
            Cmd::Favorite if self.source() == STEAMGRIDDB => {
                return Flow::Push(Box::new(self.filters(ctx)));
            }
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
        self.more();
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
        let sel = self.sel.min(items.len().saturating_sub(1));
        let focused = items.get(sel).filter(|_| !self.on_sources);
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
            sel: if self.on_sources { -1 } else { sel as i32 },
            preview: crate::shell::tiles::image(focused.map(|c| c.url.as_str()), PREVIEW),
            preview_text,
            // Desktop: os botões e rótulos do SeletorDeImagens (a tela cheia usa as dicas) e a capa do jogo.
            rows: if ctx.desktop {
                self.desktop_rows(ctx, focused.is_some())
            } else {
                Vec::new()
            },
            image: if ctx.desktop {
                crate::shell::tiles::image(
                    ctx.library
                        .find(&self.game.id)
                        .and_then(|g| g.shown_cover()),
                    (80, 120),
                )
            } else {
                slint::Image::default()
            },
            ..Default::default()
        }
    }

    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        let mut h = vec![
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
        ];
        if self.source() == STEAMGRIDDB {
            h.push(ctx.hint(Cmd::Favorite, &ctx.t("art_picker.filters")));
        }
        h
    }

    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        match zone {
            // Desktop: escolher só marca e mostra na prévia; "Usar" aplica.
            "thumb" if self.desktop => {
                self.on_sources = false;
                self.sel = index;
                self.more();
            }
            "use" => return self.handle(Cmd::Accept, ctx),
            "original" => return self.handle(Cmd::Menu, ctx),
            "file" => return self.handle(Cmd::Search, ctx),
            "style" => return self.handle(Cmd::Favorite, ctx),
            "cancel" | "outside" => return Flow::Close,
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
        // Desktop: a grade rolou até o fim; o hover não troca a escolha.
        if zone == "more" {
            self.more();
            return;
        }
        if zone == "thumb" && !self.desktop {
            self.on_sources = false;
            self.sel = index;
            self.more();
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

    /// SteamGridDB falso: o jogo 5 tem 50 capas na página 0 e 3 na página 1; guarda os caminhos pedidos.
    struct Sgdb(std::sync::Mutex<Vec<String>>);

    impl tako_core::metadata::http::Http for Sgdb {
        fn send(
            &self,
            req: &tako_core::metadata::http::HttpRequest,
        ) -> Result<tako_core::metadata::http::HttpResponse, String> {
            self.0.lock().unwrap().push(req.url.clone());
            let n = if !req.url.contains("/grids/game/5?") {
                None
            } else if req.url.contains("page=1") {
                Some(3)
            } else {
                Some(PAGE_SIZE)
            };
            let body = n.map(|n| {
                let items: Vec<String> = (0..n)
                    .map(|i| format!(r#"{{"id":{i},"score":0,"style":"alternate","width":600,"height":900,"nsfw":false,"humor":false,"mime":"image/png","url":"https://cdn2.steamgriddb.com/grid/{}{i}.png"}}"#, req.url.len()))
                    .collect();
                format!(r#"{{"success":true,"data":[{}]}}"#, items.join(","))
            });
            Ok(tako_core::metadata::http::HttpResponse {
                status: if body.is_some() { 200 } else { 404 },
                body: body.unwrap_or_default().into_bytes(),
                content_type: None,
                retry_after: None,
            })
        }
    }

    struct NoWait;

    impl tako_core::metadata::http::Clock for NoWait {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            chrono::Utc::now()
        }
        fn sleep(&self, _: std::time::Duration) {}
    }

    #[test]
    fn steamgriddb_pages_near_the_end_and_filters_reload() {
        use tako_core::metadata::steamgriddb::{
            SteamGridDbClient, SteamGridDbProvider, SteamGridDbSettings,
        };
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = crate::metadata::test_ctx(dir.path(), true);
        let id = ctx.library.games()[0].id.clone();
        ctx.pinned_ids
            .entry(id.clone())
            .or_default()
            .insert(STEAMGRIDDB.into(), "5".into());
        let http = Arc::new(Sgdb(Default::default()));
        let settings = SteamGridDbSettings {
            enabled: true,
            api_key: "0123456789abcdef0123456789abcdef".into(),
            auto_fill: false,
            animated: true,
        };
        let clock: Arc<dyn tako_core::metadata::http::Clock> = Arc::new(NoWait);
        let catalog = ArtCatalog {
            steamgriddb: Some(Arc::new(SteamGridDbProvider::new(
                SteamGridDbClient::new(
                    http.clone(),
                    Arc::new(move || settings.clone()),
                    clock.clone(),
                    None,
                ),
                clock,
            ))),
            ..crate::metadata::basic_catalog()
        };
        ctx.metadata = Some(Rc::new(crate::metadata::Metadata::offline(
            MetadataAgent::new(Vec::new(), Arc::new(NoWait)),
            catalog,
        )));
        let grids = |h: &Sgdb| -> Vec<String> {
            h.0.lock()
                .unwrap()
                .iter()
                .filter(|u| u.contains("/grids/"))
                .cloned()
                .collect()
        };

        let mut p = picker(&ctx, &id);
        let tab = p
            .render(&ctx)
            .tabs2
            .iter()
            .position(|t| t == "SteamGridDB")
            .unwrap();
        p.click("tab2", tab, &mut ctx);
        assert_eq!(p.render(&ctx).thumbs.len(), PAGE_SIZE);
        assert!(p.hints(&ctx).iter().any(|h| h.label == "Filtros"));
        // "Mostrar animadas" ligado nas configurações já vem no filtro.
        let first = grids(&http);
        assert_eq!(first.len(), 1);
        assert!(first[0].ends_with(
            "grids/game/5?dimensions=600x900,342x482,660x930&types=static,animated&nsfw=false&humor=false"
        ));

        // Longe do fim: nada; a 8 do fim: a próxima página.
        p.hover("thumb", 10);
        assert_eq!(grids(&http).len(), 1);
        p.hover("thumb", PAGE_SIZE - MORE_BEFORE_END);
        assert_eq!(p.render(&ctx).thumbs.len(), PAGE_SIZE + 3);
        assert!(grids(&http)[1].ends_with("&page=1"));
        // A última veio incompleta: acabou.
        p.hover("thumb", PAGE_SIZE + 2);
        assert_eq!(grids(&http).len(), 2);

        // X: filtros; Estilo passa de "Todos" para "Alternativo" e recarrega da página 0.
        let Flow::Push(mut panel) = p.handle(Cmd::Favorite, &mut ctx) else {
            panic!("X abre os filtros");
        };
        let rows = panel.render(&ctx).rows;
        let titles: Vec<&str> = rows.iter().map(|r| r.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Estilo", "Formato", "Animadas", "Adulto e humor: ocultos"]
        );
        assert_eq!(rows[0].value, "Todos");
        panel.handle(Cmd::Accept, &mut ctx);
        assert_eq!(panel.render(&ctx).rows[0].value, "Alternativo");
        assert!(grids(&http)[2].contains("&styles=alternate&"));
        assert!(!grids(&http)[2].contains("page="));
        assert_eq!(p.render(&ctx).thumbs.len(), PAGE_SIZE);

        // Outro tipo: o estilo volta a "Todos", "animadas" fica.
        p.handle(Cmd::NextSection, &mut ctx);
        p.handle(Cmd::PrevSection, &mut ctx);
        let last = grids(&http).pop().unwrap();
        assert!(!last.contains("styles="));
        assert!(last.contains("types=static,animated"));
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
