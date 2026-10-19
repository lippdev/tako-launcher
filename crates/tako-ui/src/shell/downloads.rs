//! Downloads do desktop (DL-001, ADR-0028, PainelDownloads): Configurações › Downloads (ligar com o aviso legal,
//! fontes do usuário, limite de envio), "Baixar de uma fonte" no jogo, a pílula da barra e o painel lateral com os
//! downloads e as instalações das lojas.
//!
//! Desligado por padrão. O motor só existe com a função ligada e o aviso aceito. Ao terminar, só "Abrir pasta":
//! o Tako nunca executa nem aponta um executável do conteúdo baixado (CONTRIBUTING §6.7).

use super::layer::{Flow, Layer, MenuItem, MenuLayer, Pick};
use super::{Cmd, Ctx, Effect};
use crate::model::{Hint, Overlay, PanelRow};
use crate::shell::keyboard::KeyboardLayer;
use std::collections::HashSet;
use std::path::PathBuf;
use tako_core::downloads::{
    Download, DownloadState, Downloads, NewDownload, source, source::Source, suggest,
};
use tako_core::library::{GameStore, InstallProgress};
use tako_core::metadata::art::format_bytes;
use tako_core::persistence::{DownloadStore, SourceRow};
use tako_core::presentation::game_text;

pub const ENABLED: &str = "DownloadsEnabled";
pub const ACCEPTED: &str = "DownloadsLegalAccepted";
/// Limite de envio em KiB/s; 0 = sem limite.
pub const UPLOAD: &str = "DownloadsUploadLimitKiB";
const UPLOAD_DEFAULT: i64 = 256;
const UPLOAD_CHOICES: [i64; 4] = [64, 256, 1024, 0];
/// Fontes por perfil (cada uma pode ter até 20 000 itens em memória).
pub const MAX_SOURCES: usize = 20;
/// Sugestões da fonte mostradas no jogo.
const SUGGESTIONS: usize = 8;

/// Estado dos downloads no contexto.
#[derive(Default)]
pub struct State {
    pub store: Option<DownloadStore>,
    pub manager: Option<Downloads>,
    /// Fontes salvas e o conteúdo do cache (lidas na primeira vez que alguém precisa).
    pub sources: Vec<(SourceRow, Source)>,
    pub(crate) sources_loaded: bool,
    /// Jogos em que o Tako pôs o progresso do download (`installing`), para tirar quando acabar.
    pub(crate) linked: HashSet<String>,
    /// Motor de mentira (`--simulate` e testes): nada sai da máquina.
    pub simulated: bool,
    /// Falha na inicialização do motor (só tenta de novo por ação do usuário).
    pub engine_failed: bool,
}

pub fn enabled(ctx: &Ctx) -> bool {
    ctx.settings.bool(ENABLED, false) && ctx.settings.bool(ACCEPTED, false)
}

/// `<Downloads do SO>/Tako`.
pub fn root() -> PathBuf {
    tako_platform::downloads::downloads_dir().join("Tako")
}

fn sources_dir(ctx: &Ctx) -> PathBuf {
    ctx.paths.root.join("download-sources")
}

fn upload_limit(ctx: &Ctx) -> i64 {
    ctx.settings
        .get(UPLOAD)
        .and_then(|v| v.parse().ok())
        .unwrap_or(UPLOAD_DEFAULT)
}

/// O gerenciador, criado na primeira vez com a função ligada (o motor não fala com ninguém até o 1º download).
pub fn manager(ctx: &mut Ctx) -> Option<&mut Downloads> {
    if !enabled(ctx) || ctx.downloads.engine_failed {
        return None;
    }
    if ctx.downloads.manager.is_none() {
        let saved = ctx
            .downloads
            .store
            .as_ref()
            .and_then(|s| s.downloads().ok())
            .unwrap_or_default();
        let engine: Box<dyn tako_core::downloads::Downloader> = if ctx.downloads.simulated {
            Box::new(tako_core::downloads::fake::MemoryDownloader::default())
        } else {
            let limit = upload_limit(ctx);
            match tako_downloads::TorrentEngine::new(tako_downloads::EngineOptions {
                upload_limit: (limit > 0).then(|| (limit * 1024).min(u32::MAX as i64) as u32),
            }) {
                Ok(e) => Box::new(e),
                Err(e) => {
                    tracing::error!("motor de downloads: {e}");
                    ctx.downloads.engine_failed = true;
                    return None;
                }
            }
        };
        ctx.downloads.manager = Some(Downloads::new(engine, root(), saved));
    }
    ctx.downloads.manager.as_mut()
}

fn save(ctx: &Ctx) {
    if let (Some(store), Some(m)) = (&ctx.downloads.store, &ctx.downloads.manager)
        && let Err(e) = store.save_downloads(m.list())
    {
        tracing::warn!("downloads: {e}");
    }
}

fn load_sources(ctx: &mut Ctx) {
    if ctx.downloads.sources_loaded {
        return;
    }
    ctx.downloads.sources_loaded = true;
    let dir = sources_dir(ctx);
    let store = ctx.downloads.store.clone();
    // O cache pode ter alguns MB por fonte: lido e validado fora da thread da UI.
    crate::tasks::run(
        ctx,
        move || {
            let rows = store.and_then(|s| s.sources().ok()).unwrap_or_default();
            rows.into_iter()
                .map(|r| {
                    let s = source::read_cache(&dir, &r.url).unwrap_or(Source {
                        name: r.name.clone(),
                        items: Vec::new(),
                        dropped: 0,
                    });
                    (r, s)
                })
                .collect::<Vec<_>>()
        },
        |ctx, loaded| {
            // Uma fonte adicionada enquanto lia já está na lista e fica como está.
            for (r, s) in loaded {
                if !ctx.downloads.sources.iter().any(|(x, _)| x.url == r.url) {
                    ctx.downloads.sources.push((r, s));
                }
            }
            ctx.dirty = true;
        },
    );
}

/// Lê o motor (a cada segundo, pelo app) e põe o progresso nos jogos vinculados. `true` = redesenhar.
pub fn poll(ctx: &mut Ctx) -> bool {
    if enabled(ctx) {
        load_sources(ctx);
    }
    let Some(m) = manager(ctx) else {
        return false;
    };
    let states: Vec<_> = m.list().iter().map(|d| d.state.clone()).collect();
    let changed = m.poll();
    let state_changed = states != m.list().iter().map(|d| d.state.clone()).collect::<Vec<_>>();
    let finished: Vec<String> = m
        .list()
        .iter()
        .zip(states.iter())
        .filter(|(d, before)| {
            d.state == DownloadState::Completed && **before != DownloadState::Completed
        })
        .map(|(d, _)| d.title.clone())
        .collect();
    if state_changed {
        save(ctx);
    }
    for title in finished {
        ctx.toast(ctx.tf("downloads.finished", &[("title", &title)]));
    }
    sync_games(ctx);
    changed
}

/// "Baixando · N%" no jogo vinculado (o mesmo `installing` das lojas, SRC-005); some quando o download acaba.
fn sync_games(ctx: &mut Ctx) {
    let open: Vec<(String, Option<f32>)> = ctx
        .downloads
        .manager
        .as_ref()
        .map(|m| {
            m.list()
                .iter()
                .filter(|d| !matches!(d.state, DownloadState::Completed | DownloadState::Failed(_)))
                .filter_map(|d| d.game_id.clone().map(|g| (g, d.fraction())))
                .collect()
        })
        .unwrap_or_default();
    let now: HashSet<String> = open.iter().map(|(g, _)| g.clone()).collect();
    let gone: Vec<String> = ctx.downloads.linked.difference(&now).cloned().collect();
    for g in ctx.library.games_mut() {
        if let Some((_, f)) = open.iter().find(|(id, _)| *id == g.id) {
            let p = Some(InstallProgress { fraction: *f });
            if g.installing != p {
                g.installing = p;
                ctx.dirty = true;
            }
        } else if gone.contains(&g.id) {
            g.installing = None;
            ctx.dirty = true;
        }
    }
    ctx.downloads.linked = now;
}

/// Jogos instalando por uma loja (SRC-005), fora os que o Tako está baixando.
fn store_installs(ctx: &Ctx) -> Vec<&tako_core::library::Game> {
    ctx.library
        .games()
        .iter()
        .filter(|g| g.installing.is_some() && !ctx.downloads.linked.contains(&g.id))
        .collect()
}

/// Texto da pílula da barra ("42%", ou vazio sem nada ativo). Soma downloads e instalações das lojas.
pub fn pill(ctx: &Ctx) -> Option<String> {
    let (mut done, mut total, mut any) = (0f64, 0f64, false);
    if let Some(m) = &ctx.downloads.manager {
        for d in m.list().iter().filter(|d| {
            matches!(
                d.state,
                DownloadState::Downloading | DownloadState::Paused | DownloadState::Queued
            )
        }) {
            any = true;
            if let Some(f) = d.fraction() {
                done += f as f64;
                total += 1.;
            }
        }
    }
    for g in store_installs(ctx) {
        any = true;
        if let Some(f) = g.installing.and_then(|p| p.fraction) {
            done += f as f64;
            total += 1.;
        }
    }
    any.then(|| {
        if total > 0. {
            format!("{}%", (done / total * 100.).round() as i32)
        } else {
            String::new()
        }
    })
}

fn speed(bps: u64) -> String {
    format!("{}/s", format_bytes(bps))
}

fn remaining(ctx: &Ctx, d: &Download) -> String {
    match d.total {
        Some(t) if d.speed > 0 && t > d.done => {
            let mins = ((t - d.done) / d.speed).div_ceil(60).max(1);
            let time = match (mins / 60, mins % 60) {
                (0, m) => format!("{m} min"),
                (h, 0) => format!("{h} h"),
                (h, m) if h < 10 => format!("{h} h {m} min"),
                (h, _) => format!("{h} h"),
            };
            ctx.tf("downloads.left", &[("time", &time)])
        }
        _ => String::new(),
    }
}

fn line(ctx: &Ctx, d: &Download) -> String {
    let mut parts = Vec::new();
    match (&d.state, d.total) {
        (DownloadState::Failed(e), _) => parts.push(ctx.tf("downloads.failed", &[("error", e)])),
        (_, None) if d.state == DownloadState::Downloading => {
            parts.push(ctx.t("downloads.metadata"))
        }
        (state, total) => {
            if *state == DownloadState::Paused {
                parts.push(ctx.t("downloads.paused"));
            }
            if let (Some(f), Some(t)) = (d.fraction(), total) {
                parts.push(format!("{}%", (f * 100.).round() as i32));
                parts.push(ctx.tf(
                    "downloads.of",
                    &[("done", &format_bytes(d.done)), ("total", &format_bytes(t))],
                ));
            }
            if *state == DownloadState::Queued {
                parts.push(ctx.t("downloads.waiting"));
            }
            let left = remaining(ctx, d);
            if !left.is_empty() && *state == DownloadState::Downloading {
                parts.push(left);
            }
        }
    }
    parts.push(ctx.tf("downloads.from", &[("source", &d.source)]));
    parts.join(" · ")
}

fn thumb(ctx: &Ctx, game_id: Option<&str>) -> slint::Image {
    let cover = game_id
        .and_then(|g| ctx.library.find(g))
        .and_then(|g| g.shown_cover());
    super::tiles::image(cover, super::desktop::THUMB)
}

fn download_row(ctx: &Ctx, d: &Download, style: &str) -> MenuItem {
    let id = d.id;
    let mut row = PanelRow::new(format!("dl-{id}"), "download", d.title.clone())
        .desc(line(ctx, d))
        .value(d.fraction().map(|f| f.to_string()).unwrap_or_default());
    row.style = style.into();
    row.image = thumb(ctx, d.game_id.as_deref());
    // Ação 0: pausar/retomar/baixar agora; ação 1: cancelar (com confirmação).
    let item = MenuItem::new(row).on(move |ctx| {
        if let Some(m) = manager(ctx) {
            let running = m
                .get(id)
                .is_some_and(|d| d.state == DownloadState::Downloading);
            let _ = if running { m.pause(id) } else { m.resume(id) };
        }
        save(ctx);
        sync_games(ctx);
        ctx.dirty = true;
        Flow::Stay
    });
    let mut item = item;
    item.alt = Some(std::rc::Rc::new(move |ctx: &mut Ctx| {
        Flow::Push(Box::new(confirm_cancel(ctx, id)))
    }));
    item
}

fn confirm_cancel(ctx: &Ctx, id: u64) -> MenuLayer {
    let title = ctx
        .downloads
        .manager
        .as_ref()
        .and_then(|m| m.get(id))
        .map(|d| d.title.clone())
        .unwrap_or_default();
    let cancel = move |delete: bool| {
        move |ctx: &mut Ctx| {
            if let Some(m) = manager(ctx) {
                let _ = m.cancel(id, delete);
            }
            save(ctx);
            sync_games(ctx);
            Flow::Close
        }
    };
    MenuLayer::dialog(ctx.t("downloads.cancel_title"), move |ctx| {
        vec![
            MenuItem::new(PanelRow::button("keep", ctx.t("downloads.cancel_keep")).link())
                .on(|_| Flow::Close),
            MenuItem::new(PanelRow::button(
                "partial",
                ctx.t("downloads.cancel_keep_files"),
            ))
            .on(cancel(false)),
            MenuItem::new(PanelRow::button("delete", ctx.t("downloads.cancel_delete")).danger())
                .on(cancel(true)),
        ]
    })
    .overline(title.to_uppercase())
    .text(ctx.t("downloads.cancel_text"))
}

/// Endereço que mostra a instalação no cliente da loja (o Tako não controla o download dela).
fn store_client(g: &tako_core::library::Game) -> Option<String> {
    match g.store() {
        GameStore::Steam => Some("steam://open/downloads".into()),
        _ => g.install_uri.clone(),
    }
}

fn store_row(ctx: &Ctx, g: &tako_core::library::Game) -> MenuItem {
    let lang = ctx.lang();
    let store = game_text::store_name(lang, g.store());
    let fraction = g.installing.and_then(|p| p.fraction);
    let desc = match fraction {
        Some(f) => format!(
            "{}% · {}",
            (f * 100.).round() as i32,
            ctx.tf("downloads.by_store", &[("store", &store)])
        ),
        None => ctx.tf("downloads.installing_in", &[("store", &store)]),
    };
    let mut row = PanelRow::new(format!("store-{}", g.id), "download", g.title.clone())
        .desc(desc)
        .value(fraction.map(|f| f.to_string()).unwrap_or_default())
        .icon(super::desktop::store_icon(g.store()));
    row.style = "store".into();
    row.image = super::tiles::image(g.shown_cover(), super::desktop::THUMB);
    let uri = store_client(g);
    let open = move |ctx: &mut Ctx| {
        if let Some(u) = &uri {
            ctx.effect(Effect::OpenUrl(u.clone()));
        }
        Flow::Stay
    };
    let mut item = MenuItem::new(row).on(open.clone());
    item.alt = Some(std::rc::Rc::new(open));
    item
}

fn completed_row(ctx: &Ctx, d: &Download) -> MenuItem {
    let (id, folder) = (d.id, d.folder.clone());
    let mut row = PanelRow::new(format!("dl-{id}"), "download", d.title.clone())
        .desc(ctx.tf("downloads.done_in", &[("source", &d.source)]));
    row.style = "completed".into();
    row.image = thumb(ctx, d.game_id.as_deref());
    // Só abre a pasta no gerenciador de arquivos: nada de dentro dela é aberto ou executado.
    let mut item = MenuItem::new(row).on(move |ctx| {
        let inside_root = match folder.strip_prefix(&root()) {
            Ok(rel) => {
                let mut comps = rel.components();
                match comps.next() {
                    Some(std::path::Component::Normal(first)) if !first.is_empty() => rel
                        .components()
                        .all(|c| matches!(c, std::path::Component::Normal(_))),
                    _ => false,
                }
            }
            Err(_) => false,
        };
        if inside_root && folder.is_dir() {
            if let Err(e) = ctx
                .platform
                .processes
                .open_folder(&folder.to_string_lossy())
            {
                ctx.toast(e.to_string());
            }
        } else {
            ctx.toast(ctx.t("downloads.folder_missing"));
        }
        Flow::Stay
    });
    item.alt = Some(std::rc::Rc::new(move |ctx: &mut Ctx| {
        if let Some(m) = manager(ctx) {
            let _ = m.cancel(id, false);
        }
        save(ctx);
        sync_games(ctx);
        ctx.dirty = true;
        Flow::Stay
    }));
    item
}

fn clear_completed(ctx: &mut Ctx) -> Flow {
    let ids = ctx
        .downloads
        .manager
        .as_ref()
        .map(|m| {
            m.list()
                .iter()
                .filter(|d| d.state == DownloadState::Completed)
                .map(|d| d.id)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(m) = manager(ctx) {
        for id in ids {
            let _ = m.cancel(id, false);
        }
    }
    save(ctx);
    sync_games(ctx);
    ctx.dirty = true;
    Flow::Stay
}

fn panel_rows(ctx: &Ctx) -> Vec<MenuItem> {
    let list: Vec<Download> = ctx
        .downloads
        .manager
        .as_ref()
        .map(|m| m.list().to_vec())
        .unwrap_or_default();
    let mut v = Vec::new();
    let active: Vec<&Download> = list
        .iter()
        .filter(|d| {
            matches!(
                d.state,
                DownloadState::Downloading | DownloadState::Paused | DownloadState::Failed(_)
            )
        })
        .collect();
    let stores = store_installs(ctx);
    if !active.is_empty() || !stores.is_empty() {
        v.push(MenuItem::new(PanelRow::header(
            ctx.t("downloads.group_active"),
        )));
        for d in active {
            let style = match d.state {
                DownloadState::Downloading => "active",
                DownloadState::Paused => "paused",
                _ => "failed",
            };
            v.push(download_row(ctx, d, style));
        }
        v.extend(stores.into_iter().map(|g| store_row(ctx, g)));
    }
    let queued: Vec<&Download> = list
        .iter()
        .filter(|d| d.state == DownloadState::Queued)
        .collect();
    if !queued.is_empty() {
        v.push(MenuItem::new(PanelRow::header(
            ctx.t("downloads.group_queued"),
        )));
        v.extend(queued.into_iter().map(|d| download_row(ctx, d, "queued")));
    }
    let done: Vec<&Download> = list
        .iter()
        .filter(|d| d.state == DownloadState::Completed)
        .collect();
    if !done.is_empty() {
        v.push(MenuItem::new(PanelRow::header(
            ctx.t("downloads.group_done"),
        )));
        v.extend(done.into_iter().map(|d| completed_row(ctx, d)));
        v.push(
            MenuItem::new(PanelRow::action(
                "clear-completed",
                ctx.t("downloads.clear_completed"),
            ))
            .on(clear_completed),
        );
    }
    if v.is_empty() {
        v.push(MenuItem::new(
            PanelRow::info("empty", ctx.t("downloads.empty"), "")
                .desc(ctx.t("downloads.empty_desc")),
        ));
    }
    v.push(MenuItem::new(PanelRow::header(String::new())));
    v.push(
        MenuItem::new(
            PanelRow::action("sources", ctx.t("downloads.sources"))
                .desc(ctx.tf(
                    "downloads.sources_desc",
                    &[("count", &ctx.downloads.sources.len().to_string())],
                ))
                .value(ctx.t("downloads.manage")),
        )
        .on(|ctx| {
            ctx.open_route = Some("settings:downloads".into());
            Flow::CloseAll
        }),
    );
    v
}

/// O PainelLateral de 480 (PainelDownloads), com o resumo atualizado embaixo do título.
pub struct Panel(MenuLayer);

pub fn panel(ctx: &mut Ctx) -> Panel {
    load_sources(ctx);
    let _ = manager(ctx);
    Panel(MenuLayer::panel(ctx.t("downloads.title"), panel_rows).wide())
}

fn summary(ctx: &Ctx) -> String {
    let s = ctx
        .downloads
        .manager
        .as_ref()
        .map(Downloads::summary)
        .unwrap_or_default();
    let downloading = s.downloading + store_installs(ctx).len();
    let mut parts = vec![ctx.tf("downloads.n_active", &[("n", &downloading.to_string())])];
    if s.queued > 0 {
        parts.push(ctx.tf("downloads.n_queued", &[("n", &s.queued.to_string())]));
    }
    if s.speed > 0 {
        parts.push(speed(s.speed));
    }
    parts.join(" · ")
}

impl Layer for Panel {
    fn handle(&mut self, cmd: Cmd, ctx: &mut Ctx) -> Flow {
        self.0.handle(cmd, ctx)
    }
    fn render(&self, ctx: &Ctx) -> Overlay {
        Overlay {
            overline: summary(ctx),
            ..self.0.render(ctx)
        }
    }
    fn hints(&self, ctx: &Ctx) -> Vec<Hint> {
        self.0.hints(ctx)
    }
    fn click(&mut self, zone: &str, index: usize, ctx: &mut Ctx) -> Flow {
        self.0.click(zone, index, ctx)
    }
    fn hover(&mut self, zone: &str, index: usize) {
        self.0.hover(zone, index)
    }
}

/// Abrir o painel pela pílula da barra.
pub fn open(ctx: &mut Ctx) {
    let p = panel(ctx);
    ctx.pending_layer = Some(Box::new(p));
}

/// "Baixar de uma fonte" no jogo: os itens das fontes parecidos com o título, só como sugestão; o usuário
/// escolhe o item, e o download fica vinculado a este jogo.
pub fn for_game(ctx: &mut Ctx, game_id: &str) -> MenuLayer {
    load_sources(ctx);
    let title = ctx
        .library
        .find(game_id)
        .map(|g| g.title.clone())
        .unwrap_or_default();
    // Calculadas uma vez ao abrir (uma fonte pode ter 20 000 itens).
    let sources = ctx.downloads.sources.iter().map(|(r, s)| {
        let name = if s.name.is_empty() { &r.name } else { &s.name };
        (name.as_str(), s)
    });
    let found: Vec<(NewDownload, String)> = suggest(&title, sources, SUGGESTIONS)
        .into_iter()
        .map(|(source_name, item)| {
            let desc = [item.file_size.as_str(), source_name]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            let new = NewDownload {
                title: item.title.clone(),
                game_id: Some(game_id.to_owned()),
                source: source_name.to_owned(),
                uri: item.uris[0].clone(),
            };
            (new, desc)
        })
        .collect();
    MenuLayer::panel(ctx.t("downloads.pick_title"), move |ctx| {
        let mut v: Vec<MenuItem> = found
            .iter()
            .map(|(new, desc)| {
                let new = new.clone();
                MenuItem::new(PanelRow::action("item", new.title.clone()).desc(desc.clone()))
                    .on(move |ctx| Flow::Push(Box::new(confirm_start(ctx, new.clone()))))
            })
            .collect();
        if v.is_empty() {
            v.push(MenuItem::new(
                PanelRow::info("none", ctx.t("downloads.no_match"), "")
                    .desc(ctx.t("downloads.no_match_desc")),
            ));
        }
        v
    })
    .overline(title)
    .text(ctx.t("downloads.pick_text"))
}

fn confirm_start(ctx: &Ctx, new: NewDownload) -> MenuLayer {
    let game = new
        .game_id
        .as_deref()
        .and_then(|g| ctx.library.find(g))
        .map(|g| g.title.clone())
        .unwrap_or_default();
    let folder = root()
        .join(tako_core::downloads::paths::folder_name(&new.title))
        .display()
        .to_string();
    let text = ctx.tf(
        "downloads.start_text",
        &[("item", &new.title), ("game", &game), ("folder", &folder)],
    );
    MenuLayer::dialog(ctx.t("downloads.start_title"), move |ctx| {
        let new = new.clone();
        vec![
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
            MenuItem::new(PanelRow::button("start", ctx.t("downloads.start"))).on(move |ctx| {
                let result = match manager(ctx) {
                    Some(m) => m.add(new.clone()),
                    None => return Flow::CloseAll,
                };
                match result {
                    Ok(_) => {
                        save(ctx);
                        sync_games(ctx);
                        ctx.toast(ctx.tf("downloads.started", &[("title", &new.title)]));
                    }
                    Err(e) => ctx.toast(e.to_string()),
                }
                Flow::CloseAll
            }),
        ]
    })
    .text(text)
}

/// Linha do ⋯ do jogo: baixar de uma fonte (função ligada, jogo não instalado) ou ver o download dele.
pub fn game_action(ctx: &Ctx, game_id: &str) -> Option<MenuItem> {
    if !enabled(ctx) || !ctx.desktop {
        return None;
    }
    let g = ctx.library.find(game_id)?;
    let busy = ctx
        .downloads
        .manager
        .as_ref()
        .is_some_and(|m| m.for_game(game_id).is_some());
    if busy {
        return Some(
            MenuItem::new(PanelRow::button("download", ctx.t("downloads.see")).icon("download"))
                .on(|ctx| Flow::Replace(Box::new(panel(ctx)))),
        );
    }
    if g.is_installed {
        return None;
    }
    let id = game_id.to_owned();
    Some(
        MenuItem::new(
            PanelRow::button("download", ctx.t("downloads.from_source")).icon("download"),
        )
        .on(move |ctx| Flow::Replace(Box::new(for_game(ctx, &id)))),
    )
}

// ---- Configurações › Downloads ----

pub fn settings_rows(ctx: &Ctx) -> Vec<MenuItem> {
    let on = enabled(ctx);
    let mut v = vec![
        MenuItem::new(
            PanelRow::toggle("enabled", ctx.t("downloads.enable"), on)
                .desc(ctx.t("downloads.enable_desc")),
        )
        .on(move |ctx| {
            ctx.downloads.engine_failed = false;
            if enabled(ctx) {
                save(ctx);
                // Solta o motor: os downloads param e voltam pausados quando a função for ligada de novo.
                ctx.downloads.manager = None;
                ctx.settings.set_bool(ENABLED, false);
                sync_games(ctx);
                return Flow::Stay;
            }
            if ctx.settings.bool(ACCEPTED, false) {
                ctx.settings.set_bool(ENABLED, true);
                return Flow::Stay;
            }
            Flow::Push(Box::new(legal_dialog(ctx)))
        }),
        MenuItem::new(
            PanelRow::info("privacy", ctx.t("downloads.privacy"), "")
                .desc(ctx.t("downloads.privacy_desc")),
        ),
    ];
    if !on {
        return v;
    }
    v.push(MenuItem::new(
        PanelRow::info(
            "folder",
            ctx.t("downloads.folder"),
            root().display().to_string(),
        )
        .desc(ctx.t("downloads.folder_desc")),
    ));
    let label = |kib: i64, ctx: &Ctx| {
        if kib == 0 {
            ctx.t("downloads.upload_none")
        } else {
            format!("{}/s", format_bytes(kib as u64 * 1024))
        }
    };
    let current = upload_limit(ctx);
    v.push(
        MenuItem::new(
            PanelRow::choice("upload", ctx.t("downloads.upload"), label(current, ctx))
                .desc(ctx.t("downloads.upload_desc")),
        )
        .on(|ctx| {
            let now = upload_limit(ctx);
            let i = UPLOAD_CHOICES.iter().position(|c| *c == now).unwrap_or(0);
            set_upload(ctx, UPLOAD_CHOICES[(i + 1) % UPLOAD_CHOICES.len()]);
            Flow::Stay
        })
        .options(move |ctx| {
            let cur = UPLOAD_CHOICES
                .iter()
                .position(|c| *c == upload_limit(ctx))
                .unwrap_or(0);
            let picks = UPLOAD_CHOICES
                .iter()
                .map(|&c| {
                    Pick::new(label(c, ctx)).set(move |ctx| {
                        set_upload(ctx, c);
                        Flow::Stay
                    })
                })
                .collect();
            (picks, cur)
        }),
    );
    v.push(MenuItem::new(PanelRow::header(ctx.t("downloads.sources"))));
    for (row, src) in &ctx.downloads.sources {
        let url = row.url.clone();
        let lang = ctx.lang();
        let when = game_text::date(
            lang,
            row.updated_at.with_timezone(&chrono::Local).date_naive(),
        );
        v.push(
            MenuItem::new(PanelRow::action("source", row.name.clone()).desc(ctx.tf(
                "downloads.source_desc",
                &[("count", &src.items.len().to_string()), ("date", &when)],
            )))
            .on(move |ctx| Flow::Push(Box::new(source_panel(ctx, &url)))),
        );
    }
    v.push(
        MenuItem::new(
            PanelRow::action("add-source", ctx.t("downloads.add_source"))
                .desc(ctx.t("downloads.add_source_desc")),
        )
        .on(|ctx| Flow::Push(Box::new(add_source(ctx)))),
    );
    v.push(MenuItem::new(
        PanelRow::info("tool", ctx.t("downloads.tool"), "").desc(ctx.t("downloads.tool_desc")),
    ));
    v
}

fn set_upload(ctx: &mut Ctx, kib: i64) {
    ctx.settings.set(UPLOAD, &kib.to_string());
    // O limite vale ao criar o motor: sem nada baixando, recria agora.
    let idle = ctx
        .downloads
        .manager
        .as_ref()
        .is_none_or(|m| m.summary().downloading == 0);
    if idle {
        save(ctx);
        ctx.downloads.manager = None;
    }
}

/// Primeiro uso: aviso legal e de privacidade, com confirmação gravada nas configurações.
fn legal_dialog(ctx: &Ctx) -> MenuLayer {
    MenuLayer::dialog(ctx.t("downloads.legal_title"), |ctx| {
        vec![
            MenuItem::new(PanelRow::button("cancel", ctx.t("common.cancel"))).on(|_| Flow::Close),
            MenuItem::new(PanelRow::button("accept", ctx.t("downloads.legal_accept"))).on(|ctx| {
                ctx.settings.set_bool(ACCEPTED, true);
                ctx.settings.set_bool(ENABLED, true);
                ctx.dirty = true;
                Flow::Close
            }),
        ]
    })
    .text(ctx.t("downloads.legal_text"))
}

/// Cabe mais uma fonte (atualizar uma que já existe sempre cabe).
fn can_add(ctx: &Ctx, url: &str) -> bool {
    ctx.downloads.sources.len() < MAX_SOURCES
        || ctx.downloads.sources.iter().any(|(r, _)| r.url == url)
}

fn add_source(ctx: &Ctx) -> KeyboardLayer {
    KeyboardLayer::new(ctx, ctx.t("downloads.add_source"), "", |ctx, text| {
        let url = text.trim().to_owned();
        if !url.to_ascii_lowercase().starts_with("https://") {
            ctx.toast(ctx.t("downloads.https_only"));
            return Flow::Stay;
        }
        if !can_add(ctx, &url) {
            ctx.toast(ctx.tf(
                "downloads.too_many_sources",
                &[("max", &MAX_SOURCES.to_string())],
            ));
            return Flow::Close;
        }
        fetch_source(ctx, url);
        Flow::Close
    })
    .hint(ctx.t("downloads.add_source_hint"))
}

/// Baixa a fonte fora da UI, valida, guarda o cache e a linha no banco.
fn fetch_source(ctx: &mut Ctx, url: String) {
    ctx.toast(ctx.t("downloads.source_loading"));
    let dir = sources_dir(ctx);
    let u = url.clone();
    crate::tasks::run(
        ctx,
        move || {
            let bytes = tako_downloads::fetch_https(&u, source::MAX_SOURCE_BYTES as u64)
                .map_err(|e| e.to_string())?;
            let parsed = source::parse(&bytes).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            std::fs::write(source::cache_file(&dir, &u), &bytes).map_err(|e| e.to_string())?;
            Ok::<_, String>(parsed)
        },
        move |ctx, result| match result {
            Ok(parsed) => {
                if !can_add(ctx, &url) {
                    let _ = std::fs::remove_file(source::cache_file(&sources_dir(ctx), &url));
                    ctx.toast(ctx.tf(
                        "downloads.too_many_sources",
                        &[("max", &MAX_SOURCES.to_string())],
                    ));
                    return;
                }
                let name = if parsed.name.is_empty() {
                    host(&url)
                } else {
                    parsed.name.clone()
                };
                let row = SourceRow {
                    url: url.clone(),
                    name,
                    items: parsed.items.len(),
                    updated_at: chrono::Utc::now(),
                };
                if let Some(store) = &ctx.downloads.store
                    && let Err(e) = store.save_source(&row)
                {
                    tracing::warn!("fonte de download: {e}");
                }
                ctx.downloads.sources.retain(|(r, _)| r.url != url);
                let count = parsed.items.len();
                ctx.downloads.sources.push((row, parsed));
                ctx.toast(ctx.tf("downloads.source_added", &[("count", &count.to_string())]));
            }
            Err(e) => ctx.toast(ctx.tf("downloads.source_failed", &[("error", &e)])),
        },
    );
}

fn host(url: &str) -> String {
    url.split("://")
        .nth(1)
        .and_then(|r| r.split(['/', '?', '#']).next())
        .unwrap_or(url)
        .to_owned()
}

fn source_panel(ctx: &Ctx, url: &str) -> MenuLayer {
    let name = ctx
        .downloads
        .sources
        .iter()
        .find(|(r, _)| r.url == url)
        .map(|(r, _)| r.name.clone())
        .unwrap_or_default();
    let (a, b) = (url.to_owned(), url.to_owned());
    MenuLayer::panel(name, move |ctx| {
        let (a, b) = (a.clone(), b.clone());
        vec![
            MenuItem::new(PanelRow::info("url", ctx.t("downloads.address"), host(&a))),
            MenuItem::new(PanelRow::action(
                "refresh",
                ctx.t("downloads.refresh_source"),
            ))
            .on(move |ctx| {
                fetch_source(ctx, a.clone());
                Flow::Close
            }),
            MenuItem::new(PanelRow::action("remove", ctx.t("downloads.remove_source")).danger())
                .on(move |ctx| {
                    let _ = std::fs::remove_file(source::cache_file(&sources_dir(ctx), &b));
                    if let Some(store) = &ctx.downloads.store {
                        let _ = store.remove_source(&b);
                    }
                    ctx.downloads.sources.retain(|(r, _)| r.url != b);
                    ctx.toast(ctx.t("downloads.source_removed"));
                    Flow::Close
                }),
        ]
    })
}

/// Dados de demonstração (capturas de tela e testes): três downloads de fontes fictícias, um em cada grupo,
/// vinculados a jogos da biblioteca, com o motor em memória.
#[cfg(test)]
pub(crate) fn demo(ctx: &mut Ctx) -> tako_core::downloads::fake::MemoryDownloader {
    use tako_core::downloads::fake::MemoryDownloader;
    ctx.settings.set_bool(ACCEPTED, true);
    ctx.settings.set_bool(ENABLED, true);
    let engine = MemoryDownloader::default();
    let mut m = Downloads::new(Box::new(engine.clone()), root(), Vec::new());
    let games: Vec<String> = ctx
        .library
        .games()
        .iter()
        .filter(|g| g.installing.is_none())
        .map(|g| g.id.clone())
        .take(3)
        .collect();
    for (k, game) in games.iter().enumerate() {
        let title = ctx
            .library
            .find(game)
            .map(|g| g.title.clone())
            .unwrap_or_default();
        let _ = m.add(NewDownload {
            title,
            game_id: Some(game.clone()),
            source: "Minha fonte".into(),
            uri: format!("magnet:?xt=urn:btih:{:040x}", k + 1),
        });
    }
    // O 1º termina, o 2º baixa a 42% e o 3º espera na fila.
    engine.set(1, 2_000_000_000, Some(2_000_000_000), true);
    m.poll();
    engine.set(2, 4_100_000_000, Some(9_800_000_000), false);
    m.poll();
    ctx.downloads.manager = Some(m);
    ctx.downloads.sources_loaded = true;
    sync_games(ctx);
    engine
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> (tempfile::TempDir, Ctx) {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = crate::app::build_ctx(
            tako_core::paths::DataPaths::at(dir.path()),
            tako_platform::Platform::simulated(),
            true,
        );
        ctx.found
            .lock()
            .unwrap()
            .extend(crate::demo::not_installed());
        ctx.library.refresh();
        ctx.desktop = true;
        (dir, ctx)
    }

    fn keys(rows: &[MenuItem]) -> Vec<(String, String)> {
        rows.iter()
            .map(|i| (i.row.kind.clone(), i.row.style.clone()))
            .collect()
    }

    #[test]
    fn off_by_default_and_turning_on_asks_for_the_legal_notice_first() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = ctx();
        assert!(!enabled(&ctx));
        assert!(manager(&mut ctx).is_none());
        let rows = settings_rows(&ctx);
        // Desligada: só ligar e o aviso de privacidade; nada de fontes.
        assert_eq!(rows.len(), 2);
        let flow = rows[0].accept.as_ref().unwrap()(&mut ctx);
        assert!(matches!(flow, Flow::Push(_)));
        assert!(!enabled(&ctx), "ligar só depois de aceitar o aviso");
        let legal = legal_dialog(&ctx);
        let accept = legal.items(&ctx).into_iter().last().unwrap();
        accept.accept.unwrap()(&mut ctx);
        assert!(enabled(&ctx));
        assert!(
            settings_rows(&ctx)
                .iter()
                .any(|i| i.row.key == "add-source")
        );
    }

    #[test]
    fn panel_groups_downloads_and_store_installs_and_the_pill_sums_them() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_d, mut ctx) = ctx();
        assert_eq!(pill(&ctx), Some("42%".into()), "só a instalação da loja");
        demo(&mut ctx);
        let rows = panel_rows(&ctx);
        let k = keys(&rows);
        assert_eq!(k[0], ("header".into(), String::new()));
        assert!(k.contains(&("download".into(), "active".into())));
        assert!(k.contains(&("download".into(), "store".into())));
        assert!(k.contains(&("download".into(), "queued".into())));
        let store = rows.iter().find(|i| i.row.style == "store").unwrap();
        assert!(store.row.description.contains("pela Steam"));
        let done = rows.iter().find(|i| i.row.style == "completed").unwrap();
        assert_eq!(done.row.kind, "download");
        assert!(rows.iter().any(|i| i.row.key == "clear-completed"));
        assert_eq!(rows.last().unwrap().row.key, "sources");
        // (42% da loja + 42% do torrent + fila sem tamanho) / 2.
        assert_eq!(pill(&ctx), Some("42%".into()));
        let ov = panel(&mut ctx).render(&ctx);
        assert!(ov.overline.starts_with("2 baixando · 1 na fila"));
    }

    #[test]
    fn linked_game_shows_progress_until_the_download_finishes() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = ctx();
        let engine = demo(&mut ctx);
        let m = ctx.downloads.manager.as_ref().unwrap();
        let active = m.list()[1].clone();
        let game = active.game_id.clone().unwrap();
        let progress = |ctx: &Ctx| ctx.library.find(&game).unwrap().installing;
        assert_eq!(
            progress(&ctx)
                .and_then(|p| p.fraction)
                .map(|f| (f * 100.).round()),
            Some(42.)
        );
        engine.set(2, 9_800_000_000, Some(9_800_000_000), true);
        assert!(poll(&mut ctx));
        assert_eq!(progress(&ctx), None);
        assert!(ctx.toast.is_some());
        // O da fila começa sozinho.
        let m = ctx.downloads.manager.as_ref().unwrap();
        assert_eq!(m.list()[2].state, DownloadState::Downloading);
    }

    #[test]
    fn the_game_menu_offers_a_download_only_when_it_makes_sense() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = ctx();
        let missing = ctx
            .library
            .games()
            .iter()
            .find(|g| !g.is_installed && g.installing.is_none())
            .unwrap()
            .id
            .clone();
        let installed = ctx
            .library
            .games()
            .iter()
            .find(|g| g.is_installed)
            .unwrap()
            .id
            .clone();
        assert!(game_action(&ctx, &missing).is_none(), "desligado");
        ctx.settings.set_bool(ACCEPTED, true);
        ctx.settings.set_bool(ENABLED, true);
        assert_eq!(game_action(&ctx, &missing).unwrap().row.key, "download");
        assert!(game_action(&ctx, &installed).is_none());
    }

    #[test]
    fn cancel_asks_and_unlinks_the_game() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = ctx();
        demo(&mut ctx);
        let rows = panel_rows(&ctx);
        let active = rows.iter().find(|i| i.row.style == "active").unwrap();
        // Cancelar abre a confirmação; pausar não.
        assert!(matches!(
            active.alt.as_ref().unwrap()(&mut ctx),
            Flow::Push(_)
        ));
        let id = ctx.downloads.manager.as_ref().unwrap().list()[1].id;
        let game = ctx.downloads.manager.as_ref().unwrap().list()[1]
            .game_id
            .clone()
            .unwrap();
        assert!(matches!(
            active.accept.as_ref().unwrap()(&mut ctx),
            Flow::Stay
        ));
        assert_eq!(
            ctx.downloads
                .manager
                .as_ref()
                .unwrap()
                .get(id)
                .unwrap()
                .state,
            DownloadState::Paused
        );
        let dialog = confirm_cancel(&ctx, id);
        let delete = dialog
            .items(&ctx)
            .into_iter()
            .find(|i| i.row.key == "delete")
            .unwrap();
        delete.accept.unwrap()(&mut ctx);
        assert!(ctx.downloads.manager.as_ref().unwrap().get(id).is_none());
        assert_eq!(ctx.library.find(&game).unwrap().installing, None);
    }

    #[test]
    fn sources_are_limited_but_a_refresh_always_fits() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let (_d, mut ctx) = ctx();
        for i in 0..MAX_SOURCES {
            ctx.downloads.sources.push((
                SourceRow {
                    url: format!("https://f{i}.invalid/a.json"),
                    name: format!("F{i}"),
                    items: 0,
                    updated_at: chrono::Utc::now(),
                },
                Source {
                    name: String::new(),
                    items: Vec::new(),
                    dropped: 0,
                },
            ));
        }
        assert!(!can_add(&ctx, "https://nova.invalid/a.json"));
        assert!(can_add(&ctx, "https://f3.invalid/a.json"));
    }

    #[test]
    fn host_never_shows_the_path() {
        assert_eq!(
            host("https://exemplo.invalid/a/b.json?x"),
            "exemplo.invalid"
        );
    }

    #[test]
    fn remove_completed_and_clear_completed() {
        let _g = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
        let (_d, mut ctx) = ctx();
        let engine = demo(&mut ctx);
        let rows = panel_rows(&ctx);
        let done = rows.iter().find(|i| i.row.style == "completed").unwrap();
        assert_eq!(done.row.kind, "download");
        assert!(
            done.alt.is_some(),
            "linha concluída tem ação secundária para remover da lista"
        );

        // Remover da lista usando o botão secundário (alt)
        let alt = done.alt.clone().unwrap();
        let flow = alt(&mut ctx);
        assert!(matches!(flow, Flow::Stay));
        assert_eq!(ctx.downloads.manager.as_ref().unwrap().get(1), None);
        assert!(
            !engine.removed().contains(&(1, true)),
            "não apaga os arquivos ao remover da lista"
        );

        // Limpar concluídos
        demo(&mut ctx);
        let rows_before = panel_rows(&ctx);
        let clear_item = rows_before
            .iter()
            .find(|i| i.row.key == "clear-completed")
            .unwrap();
        let accept = clear_item.accept.clone().unwrap();
        let flow = accept(&mut ctx);
        assert!(matches!(flow, Flow::Stay));
        let rows_after = panel_rows(&ctx);
        assert!(!rows_after.iter().any(|i| i.row.style == "completed"));
        assert!(!rows_after.iter().any(|i| i.row.key == "clear-completed"));
    }
}
