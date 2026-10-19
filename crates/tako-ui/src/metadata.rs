//! Metadados na interface (port de `MetadataService.cs`): monta as fontes ligadas, aplica o cache ao ler a
//! biblioteca, põe na fila os jogos sem dados e aplica cada resultado na thread da UI.

use crate::shell::{Ctx, Shell};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tako_core::library::{Game, GameLibrary, system_clock};
use tako_core::metadata::MetadataProvider;
use tako_core::metadata::agent::{MetadataAgent, PinnedIds};
use tako_core::metadata::art_catalog::ArtCatalog;
use tako_core::metadata::cache::MetadataCache;
use tako_core::metadata::http::{Clock, FileCache, Http, SystemClock, UreqHttp};
use tako_core::metadata::igdb::{IgdbClient, IgdbProvider};
use tako_core::metadata::rom_hash::RomHashCache;
use tako_core::metadata::screenscraper::{
    ScreenScraperClient, ScreenScraperProvider, ScreenScraperSoftware,
};
use tako_core::metadata::steam_store::SteamStoreProvider;
use tako_core::metadata::steamgriddb::{SteamGridDbClient, SteamGridDbProvider};
use tako_core::metadata::worker::{MetadataUpdated, MetadataWorker, SPACING};

/// Fontes ligadas (busca, ids e imagens) e a fila em segundo plano.
pub struct Metadata {
    /// Para "Fontes e IDs" (buscar, conferir ids); a fila tem o próprio.
    pub agent: Arc<MetadataAgent>,
    pub catalog: Arc<ArtCatalog>,
    pub worker: Option<MetadataWorker>,
    /// Lê os resultados da fila a cada 250 ms (parado quando `Metadata` sai do `Ctx`).
    poll: slint::Timer,
}

impl Metadata {
    /// Sem fila (testes): só busca e imagens.
    pub fn offline(agent: MetadataAgent, catalog: ArtCatalog) -> Metadata {
        Metadata {
            agent: Arc::new(agent),
            catalog: Arc::new(catalog),
            worker: None,
            poll: slint::Timer::default(),
        }
    }
}

pub fn cache(ctx: &Ctx) -> MetadataCache {
    MetadataCache::new(ctx.paths.metadata_cache())
}

/// Imagens, Downloads e Área de Trabalho do usuário ("Arquivo do PC").
// shortcut: nomes em inglês das pastas (o padrão do Windows e do macOS); ler `user-dirs.dirs` se pedirem.
pub fn picture_folders() -> Vec<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    home.map(|h| {
        ["Pictures", "Downloads", "Desktop"]
            .iter()
            .map(|d| h.join(d))
            .collect()
    })
    .unwrap_or_default()
}

/// Catálogo só com Steam e "Arquivo do PC" (sem fontes ligadas, `--simulate`).
pub fn basic_catalog() -> ArtCatalog {
    ArtCatalog {
        igdb: None,
        screenscraper: None,
        steamgriddb: None,
        picture_folders: picture_folders(),
    }
}

/// Liga os metadados: escolhas do banco, cache aplicado a cada leitura da biblioteca e, fora do `--simulate`,
/// a fila com as fontes ligadas. Chamar de novo (ex.: depois de salvar uma chave) refaz tudo.
pub fn start_metadata(shell: &Rc<RefCell<Shell>>) {
    let rx = {
        let mut s = shell.borrow_mut();
        let ctx = &mut s.ctx;
        load_choices(ctx);
        install_enrich(ctx);
        if ctx.simulated() {
            return;
        }
        let (metadata, rx) = build(ctx);
        ctx.metadata = Some(Rc::new(metadata));
        rx
    };
    let weak = Rc::downgrade(shell);
    let mut queued = None;
    let s = shell.borrow();
    let Some(m) = s.ctx.metadata.as_ref() else {
        return;
    };
    m.poll.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(250),
        move || {
            let Some(shell) = weak.upgrade() else { return };
            let Ok(mut s) = shell.try_borrow_mut() else {
                return;
            };
            let ctx = &mut s.ctx;
            let mut any = false;
            while let Ok(u) = rx.try_recv() {
                apply_update(ctx, &u);
                any = true;
            }
            if any {
                crate::images::retry_failed();
            }
            // Biblioteca mudou (leitura das lojas, jogo adicionado): os que faltam entram na fila.
            let version = ctx.library.version();
            if !ctx.scanning && queued != Some(version) {
                queued = Some(version);
                queue(ctx, None, false);
            }
        },
    );
}

/// "Atualizar dados dos jogos" (todos) ou de um jogo só, consultando as fontes de novo.
pub fn refresh(ctx: &mut Ctx, id: Option<&str>) {
    let Some(m) = ctx.metadata.clone() else {
        if id.is_none() {
            ctx.toast(ctx.t("game_meta.unavailable"));
        }
        return;
    };
    queue(ctx, id, true);
    if id.is_none() {
        let names = m.agent.enabled_names();
        ctx.toast(ctx.tf("game_meta.refreshing", &[("sources", &names)]));
    }
}

fn queue(ctx: &Ctx, id: Option<&str>, force: bool) {
    let Some(worker) = ctx.metadata.as_ref().and_then(|m| m.worker.as_ref()) else {
        return;
    };
    let games: Vec<Game> = match id {
        Some(id) => ctx.library.find(id).cloned().into_iter().collect(),
        None => ctx.library.games().to_vec(),
    };
    worker.refresh(games, force, ctx.pinned_ids.clone());
}

/// Metadados novos de um jogo: campos combinados no jogo da biblioteca; as imagens fixadas continuam valendo.
pub fn apply_update(ctx: &mut Ctx, u: &MetadataUpdated) {
    if let Some(g) = ctx
        .library
        .games_mut()
        .iter_mut()
        .find(|g| g.id == u.game_id)
    {
        u.metadata.apply_to(g);
        ctx.dirty = true;
    }
}

/// Imagens e ids fixados, lidos uma vez do banco.
fn load_choices(ctx: &mut Ctx) {
    let Some(c) = &ctx.choices else { return };
    match c.load_all_art() {
        Ok(art) => {
            if let Ok(mut pins) = ctx.art_pins.lock() {
                *pins = art;
            }
        }
        Err(e) => tracing::warn!("imagens fixadas: {e}"),
    }
    match c.load_all_ids() {
        Ok(ids) => {
            ctx.pinned_ids = ids
                .into_iter()
                .map(|(g, ids)| (g, ids.into_iter().collect::<PinnedIds>()))
                .collect();
        }
        Err(e) => tracing::warn!("ids fixados: {e}"),
    }
}

/// A biblioteca aplica o cache e as imagens fixadas a cada leitura: capas e descrições aparecem já ao abrir.
fn install_enrich(ctx: &mut Ctx) {
    let cache = cache(ctx);
    let pins = ctx.art_pins.clone();
    let exe_icons = ctx.paths.cache().join("exe-icons");
    let enrich = move |g: &mut Game| {
        // Jogo adicionado à mão não passa pela leitura das lojas: se o ícone do .exe já foi extraído,
        // confere se o arquivo existe (sem extrair nada na UI).
        if g.store() == tako_core::library::GameStore::Manual {
            crate::sources::fill_exe_icon(g, &exe_icons, tako_platform::exe_icon::cached_icon_file);
        }
        if let Some(m) = cache.read(&g.id) {
            m.apply_to(g);
        }
        g.pinned_art = pins
            .lock()
            .ok()
            .and_then(|p| p.get(&g.id).cloned())
            .unwrap_or_default();
    };
    let library = std::mem::replace(
        &mut ctx.library,
        GameLibrary::in_memory(Vec::new(), system_clock()),
    );
    ctx.library = library.with_enrich(Box::new(enrich));
    ctx.library.refresh();
    ctx.sync_collections();
}

/// Provedores pelas chaves e opções que as Configurações gravam no cofre.
// shortcut: chaves lidas ao ligar; depois de salvar uma chave, chamar `start_metadata` de novo.
fn build(ctx: &Ctx) -> (Metadata, Receiver<MetadataUpdated>) {
    let http: Arc<dyn Http> = Arc::new(UreqHttp::new(Duration::from_secs(20)));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let cache_dir = ctx.paths.cache();
    let week = Duration::from_secs(7 * 24 * 3600);
    let file_cache = |name: &str| -> Option<Box<dyn tako_core::metadata::http::ResponseCache>> {
        Some(Box::new(FileCache::new(cache_dir.join(name), week)))
    };

    // Mesma leitura (em cache) das Configurações › Capas e metadados: chaves e ligado/desligado no cofre.
    let crate::shell::settings::metadata::Providers { igdb, ss, sgdb } =
        crate::shell::settings::metadata::providers(ctx);

    let igdb = Arc::new(IgdbProvider::new(
        IgdbClient::new(
            http.clone(),
            Arc::new(move || igdb.clone()),
            clock.clone(),
            file_cache("igdb"),
        ),
        clock.clone(),
    ));
    let screenscraper = Arc::new(ScreenScraperProvider::new(
        Arc::new(ScreenScraperClient::new(
            http.clone(),
            ScreenScraperSoftware::from_build(),
            Arc::new(move || ss.clone()),
            clock.clone(),
            file_cache("screenscraper"),
        )),
        RomHashCache::new(Some(cache_dir.join("rom-hashes.json"))),
        clock.clone(),
    ));
    let steamgriddb = Arc::new(SteamGridDbProvider::new(
        SteamGridDbClient::new(
            http.clone(),
            Arc::new(move || sgdb.clone()),
            clock.clone(),
            file_cache("steamgriddb"),
        ),
        clock.clone(),
    ));
    let providers: Vec<Arc<dyn MetadataProvider>> = vec![
        Arc::new(SteamStoreProvider::new(http)),
        igdb.clone(),
        screenscraper.clone(),
        steamgriddb.clone(),
    ];
    let (worker, rx) = MetadataWorker::spawn(
        MetadataAgent::new(providers.clone(), clock.clone()),
        cache(ctx),
        SPACING,
    );
    let metadata = Metadata {
        agent: Arc::new(MetadataAgent::new(providers, clock)),
        catalog: Arc::new(ArtCatalog {
            igdb: Some(igdb),
            screenscraper: Some(screenscraper),
            steamgriddb: Some(steamgriddb),
            picture_folders: picture_folders(),
        }),
        worker: Some(worker),
        poll: slint::Timer::default(),
    };
    (metadata, rx)
}

/// Resultado de uma tarefa aberta junto com uma camada (que ainda não tem `&mut Ctx`): a camada lê ao desenhar.
/// Sem janela (testes), roda na hora.
pub struct Pending<R>(Arc<Mutex<Option<R>>>);

impl<R> Clone for Pending<R> {
    fn clone(&self) -> Self {
        Pending(self.0.clone())
    }
}

impl<R: Clone> Pending<R> {
    pub fn get(&self) -> Option<R> {
        self.0.lock().ok().and_then(|r| r.clone())
    }
}

pub fn spawn<R: Send + 'static>(job: impl FnOnce() -> R + Send + 'static) -> Pending<R> {
    let slot = Arc::new(Mutex::new(None));
    if crate::tasks::shell().is_none() {
        *slot.lock().unwrap_or_else(|e| e.into_inner()) = Some(job());
        return Pending(slot);
    }
    let out = slot.clone();
    std::thread::spawn(move || {
        let r = job();
        *out.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
        let _ = slint::invoke_from_event_loop(|| {
            if let Some(shell) = crate::tasks::shell()
                && let Ok(mut s) = shell.try_borrow_mut()
            {
                s.ctx.dirty = true;
            }
        });
    });
    Pending(slot)
}

/// Contexto de teste em português. O idioma é global: os testes de texto rodam um por vez.
#[cfg(test)]
pub(crate) fn test_ctx(
    dir: &std::path::Path,
    simulate: bool,
) -> (std::sync::MutexGuard<'static, ()>, Ctx) {
    let guard = crate::LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = crate::app::build_ctx(
        tako_core::paths::DataPaths::at(dir),
        tako_platform::Platform::simulated(),
        simulate,
    );
    tako_core::i18n::set_lang(tako_core::i18n::Lang::PtBr);
    (guard, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tako_core::library::ArtKind;
    use tako_core::metadata::GameMetadata;

    #[test]
    fn updates_merge_fields_and_keep_pinned_art() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = test_ctx(dir.path(), true);
        let id = ctx.library.games()[0].id.clone();
        ctx.library.games_mut()[0].pinned_art =
            [(ArtKind::Cover, "file:///minha.png".to_string())].into();
        let u = MetadataUpdated {
            game_id: id.clone(),
            metadata: GameMetadata {
                description: Some("Nova descrição".into()),
                developer: Some("Team Cherry".into()),
                cover_url: Some("https://x/capa.jpg".into()),
                ..GameMetadata::new("IGDB")
            },
        };
        apply_update(&mut ctx, &u);
        let g = ctx.library.find(&id).unwrap();
        assert_eq!(g.description, "Nova descrição");
        assert_eq!(g.developer.as_deref(), Some("Team Cherry"));
        assert_eq!(g.cover_uri.as_deref(), Some("https://x/capa.jpg"));
        assert_eq!(g.shown_cover(), Some("file:///minha.png"));
    }

    #[test]
    fn cache_and_pins_are_applied_on_every_library_read() {
        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = test_ctx(dir.path(), true);
        let id = ctx.library.games()[1].id.clone();
        cache(&ctx)
            .write(
                &id,
                &GameMetadata {
                    description: Some("Do cache".into()),
                    ..GameMetadata::new("Steam")
                },
            )
            .unwrap();
        ctx.art_pins.lock().unwrap().insert(
            id.clone(),
            [(ArtKind::Hero, "file:///fundo.png".to_string())].into(),
        );
        install_enrich(&mut ctx);
        ctx.library.refresh();
        let g = ctx.library.find(&id).unwrap();
        assert_eq!(g.description, "Do cache");
        assert_eq!(g.pinned_art[&ArtKind::Hero], "file:///fundo.png");
    }

    #[test]
    fn enrich_does_not_extract_icons() {
        use tako_core::library::ManualGameRecord;

        let dir = tempfile::tempdir().unwrap();
        let (_l, mut ctx) = test_ctx(dir.path(), true);
        let exe = dir.path().join("Jogos").join("manual.exe");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, b"MZfake").unwrap();

        let id = "manual:teste";
        let mut record =
            ManualGameRecord::create("Jogo Teste", &exe.to_string_lossy(), chrono::Utc::now());
        record.id = id.into();
        ctx.library.add_manual(record).unwrap();
        install_enrich(&mut ctx);
        ctx.library.refresh();

        let exe_icons_dir = ctx.paths.cache().join("exe-icons");
        let g = ctx.library.find(id).unwrap();
        // O enrich não deve ter extraído nada: nenhum ícone no jogo e nenhum arquivo criado no cache
        assert_eq!(g.media.get("icon"), None);
        assert!(
            !exe_icons_dir.exists() || std::fs::read_dir(&exe_icons_dir).unwrap().next().is_none()
        );

        // Se o PNG já estiver gerado no cache, o enrich apenas confere a existência e associa
        std::fs::create_dir_all(&exe_icons_dir).unwrap();
        let meta = std::fs::metadata(&exe).unwrap();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        let expected_file = exe_icons_dir.join(tako_platform::exe_icon::cache_name(
            &exe,
            meta.len(),
            modified,
        ));
        std::fs::write(&expected_file, b"PNG fake").unwrap();

        ctx.library.refresh();
        let g = ctx.library.find(id).unwrap();
        assert_eq!(
            g.media.get("icon").map(String::as_str),
            Some(tako_sources::file_uri(&expected_file).as_str())
        );
        assert_eq!(g.icon_origin, Some(tako_core::library::IconOrigin::Exe));
    }
}
