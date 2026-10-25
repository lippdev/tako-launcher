//! Montagem das fontes compartilhada pelo Slint e pelo núcleo web. Segredos ficam nos clientes Rust.

use super::MetadataProvider;
use super::agent::MetadataAgent;
use super::art_cache::ArtCache;
use super::art_catalog::ArtCatalog;
use super::http::{Clock, FileCache, Http, ResponseCache, SystemClock, UreqHttp};
use super::igdb::{IgdbClient, IgdbProvider, IgdbSettings};
use super::rom_hash::RomHashCache;
use super::screenscraper::{
    ScreenScraperClient, ScreenScraperProvider, ScreenScraperSettings, ScreenScraperSoftware,
};
use super::steam_art::SteamArtResolver;
use super::steam_store::SteamStoreProvider;
use super::steamgriddb::{SteamGridDbClient, SteamGridDbProvider, SteamGridDbSettings};
use crate::paths::DataPaths;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub struct Sources {
    pub providers: Vec<Arc<dyn MetadataProvider>>,
    pub clock: Arc<dyn Clock>,
    pub catalog: ArtCatalog,
    pub screenscraper: Arc<ScreenScraperClient>,
}

impl Sources {
    pub fn agent(&self) -> MetadataAgent {
        MetadataAgent::new(self.providers.clone(), self.clock.clone())
    }
}

/// Mesmas chaves, cache de respostas de uma semana, clientes e ordem do app Slint.
pub fn sources(
    paths: &DataPaths,
    igdb: IgdbSettings,
    ss: ScreenScraperSettings,
    sgdb: SteamGridDbSettings,
    picture_folders: Vec<PathBuf>,
) -> Sources {
    let http: Arc<dyn Http> = Arc::new(UreqHttp::new(Duration::from_secs(20)));
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let cache_dir = paths.cache();
    let file_cache = |name: &str| -> Option<Box<dyn ResponseCache>> {
        Some(Box::new(FileCache::new(
            cache_dir.join(name),
            Duration::from_secs(7 * 24 * 3600),
        )))
    };
    let igdb = Arc::new(IgdbProvider::new(
        IgdbClient::new(
            http.clone(),
            Arc::new(move || igdb.clone()),
            clock.clone(),
            file_cache("igdb"),
        ),
        clock.clone(),
    ));
    let screenscraper = Arc::new(ScreenScraperClient::new(
        http.clone(),
        ScreenScraperSoftware::from_build(),
        Arc::new(move || ss.clone()),
        clock.clone(),
        file_cache("screenscraper"),
    ));
    let ss_provider = Arc::new(ScreenScraperProvider::new(
        screenscraper.clone(),
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
    Sources {
        providers: vec![
            Arc::new(SteamStoreProvider::new(http)),
            igdb.clone(),
            ss_provider.clone(),
            steamgriddb.clone(),
        ],
        clock,
        catalog: ArtCatalog {
            igdb: Some(igdb),
            screenscraper: Some(ss_provider),
            steamgriddb: Some(steamgriddb),
            picture_folders,
        },
        screenscraper,
    }
}

/// Tempo-limite de arte do Slint (8 s), cache de 1 GiB e cache local da Steam antes da CDN.
pub fn art_cache(
    paths: &DataPaths,
    steam_root: Option<PathBuf>,
    screenscraper: Option<Arc<ScreenScraperClient>>,
) -> Arc<ArtCache> {
    let http = Arc::new(UreqHttp::new(Duration::from_secs(8)));
    Arc::new(ArtCache::new(
        paths.art_cache(),
        http.clone(),
        SteamArtResolver::new(http, steam_root),
        screenscraper,
    ))
}
