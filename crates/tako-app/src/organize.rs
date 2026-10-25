//! Organização da biblioteca: valida o contrato e delega as regras aos serviços do núcleo.

use crate::{Core, protocol::*, settings};
use tako_core::library::{self, CollectionError, CollectionInfo, LibraryError, collections, smart};

pub(crate) fn bad(message: &str) -> BridgeError {
    BridgeError::new(ErrorCode::BadRequest, message)
}
pub(crate) fn internal(error: impl std::fmt::Display) -> BridgeError {
    tracing::error!("persistência da organização: {error}");
    BridgeError::new(ErrorCode::Internal, "não foi possível gravar")
}
fn missing() -> BridgeError {
    BridgeError::new(ErrorCode::NotFound, "alvo não encontrado")
}
fn collection_error(error: CollectionError) -> BridgeError {
    match error {
        CollectionError::Name(_) => bad("nome de coleção inválido"),
        CollectionError::NotFound(_) => missing(),
        CollectionError::Store(error) => internal(error),
    }
}
fn library_error(error: LibraryError) -> BridgeError {
    match error {
        LibraryError::NotFound(_) => missing(),
        error => internal(error),
    }
}
fn id(value: &str) -> Result<(), BridgeError> {
    if value.trim().is_empty() || value.chars().count() > 128 || value.contains('\0') {
        Err(bad("id inválido"))
    } else {
        Ok(())
    }
}
fn games(core: &Core, ids: &[String]) -> Result<(), BridgeError> {
    if ids.is_empty() || ids.len() > 200 {
        return Err(bad("lista de jogos inválida"));
    }
    for value in ids {
        id(value)?;
        if core.library.find(value).is_none() {
            return Err(missing());
        }
    }
    Ok(())
}
pub(crate) fn smart_collections(core: &Core) -> Vec<smart::SmartCollection> {
    smart::parse(core.setting("SmartCollections").unwrap_or_default())
}
pub(crate) fn check_collection(core: &Core, value: Option<&str>) -> Result<(), BridgeError> {
    if let Some(value) = value {
        id(value)?;
        if core.collections.find(value).is_none()
            && !smart_collections(core).iter().any(|c| c.id == value)
        {
            return Err(missing());
        }
    }
    Ok(())
}
fn manual(c: &CollectionInfo) -> Collection {
    Collection {
        id: c.id.clone(),
        name: c.name.clone(),
        game_ids: c.game_ids.iter().cloned().collect(),
        criteria: None,
    }
}
fn automatic(c: &smart::SmartCollection) -> Collection {
    Collection {
        id: c.id.clone(),
        name: c.name.clone(),
        game_ids: Vec::new(),
        criteria: Some(criteria_from(c)),
    }
}
fn save_smart(core: &mut Core, list: &[smart::SmartCollection]) -> Result<(), BridgeError> {
    let json = smart::to_json(list);
    if json.len() > 256 * 1024 || list.len() > 256 {
        return Err(bad("coleções demais"));
    }
    settings::save(core, "SmartCollections", &json)
}

pub(crate) fn handle(core: &mut Core, call: Call) -> Result<(Reply, bool), BridgeError> {
    let reply = match call {
        Call::SettingsGet(request) => {
            return Ok((Reply::SettingsGet(settings::get(core, request)?), false));
        }
        Call::SettingsSet(request) => {
            settings::set(core, request)?;
            return Ok((Reply::SettingsSet, false));
        }
        Call::CollectionsList => {
            let mut list: Vec<_> = core.collections.collections().iter().map(manual).collect();
            list.extend(smart_collections(core).iter().map(automatic));
            return Ok((Reply::CollectionsList(list), false));
        }
        Call::CollectionsCreate(request) => {
            let c = core
                .collections
                .create(&request.name, None)
                .map_err(collection_error)?;
            Reply::CollectionsCreate(manual(&c))
        }
        Call::CollectionsRename(request) => {
            check_collection(core, Some(&request.collection_id))?;
            let mut list = smart_collections(core);
            if let Some(index) = list.iter().position(|c| c.id == request.collection_id) {
                if smart::name_problem(&request.name, &list, Some(&request.collection_id)).is_some()
                {
                    return Err(bad("nome inválido"));
                }
                list[index].name = collections::clean_name(&request.name);
                save_smart(core, &list)?;
            } else {
                core.collections
                    .rename(&request.collection_id, &request.name)
                    .map_err(collection_error)?;
            }
            Reply::CollectionsRename
        }
        Call::CollectionsDelete(request) => {
            check_collection(core, Some(&request.collection_id))?;
            if core.collections.find(&request.collection_id).is_some() {
                core.collections
                    .delete(&request.collection_id)
                    .map_err(collection_error)?;
            } else {
                let mut list = smart_collections(core);
                list.retain(|c| c.id != request.collection_id);
                save_smart(core, &list)?;
            }
            Reply::CollectionsDelete
        }
        Call::CollectionsAddGames(request) => {
            membership(core, request, true)?;
            Reply::CollectionsAddGames
        }
        Call::CollectionsRemoveGames(request) => {
            membership(core, request, false)?;
            Reply::CollectionsRemoveGames
        }
        Call::CollectionsSmartSet(request) => {
            let (criteria, filter) = criteria_to(&request.criteria)?;
            let mut list = smart_collections(core);
            let value = if let Some(value) = request.collection_id {
                id(&value)?;
                let index = list
                    .iter()
                    .position(|c| c.id == value)
                    .ok_or_else(missing)?;
                if smart::name_problem(&request.name, &list, Some(&value)).is_some() {
                    return Err(bad("nome inválido"));
                }
                list[index] = smart::SmartCollection {
                    id: value.clone(),
                    name: collections::clean_name(&request.name),
                    criteria,
                    filter,
                };
                value
            } else {
                smart::create(&mut list, &request.name, criteria, filter)
                    .map_err(|_| bad("nome inválido"))?
            };
            let result = automatic(list.iter().find(|c| c.id == value).ok_or_else(missing)?);
            save_smart(core, &list)?;
            Reply::CollectionsSmartSet(result)
        }
        Call::TagsList => {
            return Ok((
                Reply::TagsList(
                    library::tags::all(core.library.games())
                        .into_iter()
                        .map(|t| Tag {
                            name: t.name,
                            count: t.count,
                        })
                        .collect(),
                ),
                false,
            ));
        }
        Call::GameTagsSet(request) => {
            games(core, &request.game_ids)?;
            if request.tags.is_empty()
                || request.tags.len() > 64
                || request
                    .tags
                    .iter()
                    .any(|t| t.contains('\0') || library::tags::name_problem(t).is_some())
            {
                return Err(bad("tags inválidas"));
            }
            for tag in request.tags {
                core.library
                    .set_tag(&request.game_ids, &tag, request.on)
                    .map_err(library_error)?;
            }
            Reply::GameTagsSet
        }
        Call::GameStatusSet(request) => {
            games(core, &request.game_ids)?;
            for value in request.game_ids {
                core.library
                    .set_status(&value, request.status.map(Into::into))
                    .map_err(library_error)?;
            }
            Reply::GameStatusSet
        }
        Call::GameFavoriteSet(request) => {
            games(core, &request.game_ids)?;
            for value in request.game_ids {
                if core
                    .library
                    .find(&value)
                    .is_some_and(|g| g.is_favorite != request.favorite)
                {
                    core.library
                        .toggle_favorite(&value)
                        .map_err(library_error)?;
                }
            }
            Reply::GameFavoriteSet
        }
        _ => return Err(bad("chamada fora da organização")),
    };
    Ok((reply, true))
}

fn membership(core: &mut Core, request: CollectionGames, on: bool) -> Result<(), BridgeError> {
    id(&request.collection_id)?;
    games(core, &request.game_ids)?;
    let c = core
        .collections
        .find(&request.collection_id)
        .ok_or_else(missing)?;
    let ids: std::collections::BTreeSet<_> = request
        .game_ids
        .into_iter()
        .filter(|id| c.contains(id) != on)
        .collect();
    for value in ids {
        core.collections
            .toggle(&request.collection_id, &value)
            .map_err(collection_error)?;
    }
    Ok(())
}

pub(crate) fn criteria_to(
    request: &SmartCriteria,
) -> Result<(library::LibraryCriteria, library::LibraryFilter), BridgeError> {
    request.library.validate()?;
    if request.library.offset != 0
        || request.genres.len() > 64
        || request
            .genres
            .iter()
            .any(|g| g.trim().is_empty() || g.chars().count() > 128)
    {
        return Err(bad("critérios inválidos"));
    }
    if request.filter_store.is_some() && request.library.filter != LibraryTab::All {
        return Err(bad("filtro de loja exige a aba todos"));
    }
    let (mut criteria, filter) = crate::library::criteria(request.library.clone());
    criteria.genres = request.genres.iter().cloned().collect();
    criteria.played_only = request.played_only;
    criteria.broken_only = request.broken_only;
    criteria.favorites_only = request.favorites_only;
    if request.collections.len() > 64 {
        return Err(bad("coleções demais"));
    }
    for value in &request.collections {
        id(value)?;
    }
    criteria
        .collections
        .extend(request.collections.iter().cloned());
    criteria.sources = request
        .library
        .stores
        .iter()
        .map(|s| library::GameStore::from(*s).key().to_owned())
        .collect();
    criteria.play_time = match request.play_time {
        PlayTime::Any => library::criteria::PlayTimeFilter::Any,
        PlayTime::Unplayed => library::criteria::PlayTimeFilter::Unplayed,
        PlayTime::UnderHour => library::criteria::PlayTimeFilter::UnderHour,
        PlayTime::OneToTenHours => library::criteria::PlayTimeFilter::OneToTenHours,
        PlayTime::OverTenHours => library::criteria::PlayTimeFilter::OverTenHours,
    };
    Ok((
        criteria,
        request
            .filter_store
            .map(|s| library::LibraryFilter::Store(s.into()))
            .unwrap_or(filter),
    ))
}

fn criteria_from(c: &smart::SmartCollection) -> SmartCriteria {
    let r = &c.criteria;
    SmartCriteria {
        library: LibraryListRequest {
            filter: match c.filter {
                library::LibraryFilter::Favorites => LibraryTab::Favorites,
                library::LibraryFilter::Installed => LibraryTab::Installed,
                _ => LibraryTab::All,
            },
            search: r.search.clone(),
            sort: crate::library::wire_sort(r.sort),
            reversed: r.reversed,
            collection: r.collections.iter().next().cloned(),
            tags: r.tags.iter().cloned().collect(),
            statuses: r.statuses.iter().copied().map(Into::into).collect(),
            no_status: r.no_status,
            stores: r
                .sources
                .iter()
                .filter_map(|s| library::GameStore::from_key(s).map(Into::into))
                .collect(),
            installed: match r.install {
                library::InstallFilter::Any => None,
                library::InstallFilter::Installed => Some(true),
                library::InstallFilter::NotInstalled => Some(false),
            },
            ..Default::default()
        },
        favorites_only: r.favorites_only,
        collections: r.collections.iter().cloned().collect(),
        filter_store: match c.filter {
            library::LibraryFilter::Store(store) => Some(store.into()),
            _ => None,
        },
        genres: r.genres.iter().cloned().collect(),
        played_only: r.played_only,
        broken_only: r.broken_only,
        play_time: match r.play_time {
            library::criteria::PlayTimeFilter::Any => PlayTime::Any,
            library::criteria::PlayTimeFilter::Unplayed => PlayTime::Unplayed,
            library::criteria::PlayTimeFilter::UnderHour => PlayTime::UnderHour,
            library::criteria::PlayTimeFilter::OneToTenHours => PlayTime::OneToTenHours,
            library::criteria::PlayTimeFilter::OverTenHours => PlayTime::OverTenHours,
        },
    }
}

/// A CollectionLibrary já mantém os itens; a demonstração só dispensa as gravações.
pub(crate) struct MemoryCollections;
impl library::CollectionStore for MemoryCollections {
    fn load_all(&mut self) -> Result<Vec<CollectionInfo>, String> {
        Ok(Vec::new())
    }
    fn insert(&mut self, _: &CollectionInfo) -> Result<(), String> {
        Ok(())
    }
    fn rename(&mut self, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn delete(&mut self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn add_game(
        &mut self,
        _: &str,
        _: &str,
        _: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        Ok(())
    }
    fn remove_game(&mut self, _: &str, _: &str) -> Result<(), String> {
        Ok(())
    }
}
