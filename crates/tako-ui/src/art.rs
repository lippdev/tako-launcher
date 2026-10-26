//! Arte dos jogos: o cache de imagens em disco (com limite, MIG-028) resolve URIs para arquivos.

use crate::images::Resolver;
use crate::shell::Ctx;
use std::sync::Arc;
use tako_core::metadata::art_cache::ArtCache;
use tako_core::paths::DataPaths;

pub fn cache(paths: &DataPaths) -> Arc<ArtCache> {
    tako_core::metadata::runtime::art_cache(paths, tako_sources::steam::find_root(), None)
}

pub fn resolver(ctx: &Ctx) -> Resolver {
    let cache = ctx.art.clone();
    Arc::new(move |uri: &str| cache.locate(uri))
}
