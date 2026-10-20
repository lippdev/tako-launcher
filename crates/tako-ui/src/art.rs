//! Arte dos jogos: o cache de imagens em disco (com limite, MIG-028) resolve URIs para arquivos.

use crate::images::Resolver;
use crate::shell::Ctx;
use std::sync::Arc;
use std::time::Duration;
use tako_core::metadata::art_cache::ArtCache;
use tako_core::metadata::http::UreqHttp;
use tako_core::metadata::steam_art::SteamArtResolver;
use tako_core::paths::DataPaths;

pub fn cache(paths: &DataPaths) -> Arc<ArtCache> {
    // Arte é para a tela de agora: uma CDN lenta não pode prender o worker (cada arte tenta até três endereços).
    let http = Arc::new(UreqHttp::new(Duration::from_secs(8)));
    let steam = SteamArtResolver::new(http.clone(), tako_sources::steam::find_root());
    Arc::new(ArtCache::new(paths.art_cache(), http, steam, None))
}

pub fn resolver(ctx: &Ctx) -> Resolver {
    let cache = ctx.art.clone();
    Arc::new(move |uri: &str| cache.locate(uri))
}
