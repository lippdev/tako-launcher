//! Coleções automáticas (LIB-003): uma combinação de busca, filtros e ordenação salva com um nome. Os jogos não
//! ficam guardados; a coleção mostra o que os critérios acham na hora, então se atualiza sozinha.
//!
//! Ficam em JSON numa configuração (`SmartCollections`), sem tabela própria: são poucas e pequenas.

use super::collections::{MAX_NAME_LENGTH, NameProblem, clean_name};
use super::criteria::LibraryCriteria;
use super::query::title_eq;
use super::{GameStore, LibraryFilter};
use crate::{i18n::Lang, presentation::game_text};
use serde::{Deserialize, Serialize};

/// Configuração lida acima disto é lixo (centenas de coleções cabem em poucos KB).
const MAX_JSON: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmartCollection {
    pub id: String,
    pub name: String,
    pub criteria: LibraryCriteria,
    #[serde(default)]
    pub filter: LibraryFilter,
}

impl SmartCollection {
    /// Fontes são persistidas como identificadores; trocar o idioma não muda a coleção.
    pub fn criteria_in(&self, lang: Lang) -> LibraryCriteria {
        let mut criteria = self.criteria.clone();
        criteria.sources = criteria
            .sources
            .iter()
            .map(|s| {
                GameStore::from_key(s)
                    .map(|store| game_text::store_name(lang, store))
                    .unwrap_or_else(|| s.clone())
            })
            .collect();
        criteria
    }
}

/// Lê a configuração. Texto quebrado ou grande demais vira lista vazia; uma coleção ilegível (de uma versão mais
/// nova, por exemplo) é pulada sem perder as outras.
pub fn parse(json: &str) -> Vec<SmartCollection> {
    if json.trim().is_empty() {
        return Vec::new();
    }
    if json.len() > MAX_JSON {
        tracing::warn!("coleções automáticas: configuração grande demais, ignorada");
        return Vec::new();
    }
    match serde_json::from_str::<Vec<serde_json::Value>>(json) {
        Ok(items) => items
            .into_iter()
            .filter_map(|v| match serde_json::from_value::<SmartCollection>(v) {
                Ok(c) if !c.id.is_empty() && name_problem(&c.name, &[], None).is_none() => Some(c),
                _ => {
                    tracing::warn!("coleção automática inválida ignorada");
                    None
                }
            })
            .collect(),
        Err(e) => {
            tracing::warn!("coleções automáticas ilegíveis: {e}");
            Vec::new()
        }
    }
}

pub fn to_json(list: &[SmartCollection]) -> String {
    serde_json::to_string(list).unwrap_or_else(|_| "[]".into())
}

/// Problema do nome, ou `None` quando serve. `except_id` é a coleção sendo renomeada.
pub fn name_problem(
    name: &str,
    list: &[SmartCollection],
    except_id: Option<&str>,
) -> Option<NameProblem> {
    let clean = clean_name(name);
    if clean.is_empty() {
        Some(NameProblem::Empty)
    } else if clean.chars().count() > MAX_NAME_LENGTH {
        Some(NameProblem::TooLong)
    } else if list
        .iter()
        .any(|c| Some(c.id.as_str()) != except_id && title_eq(&c.name, &clean))
    {
        Some(NameProblem::Duplicate)
    } else {
        None
    }
}

/// Salva os critérios com um nome novo e devolve o id.
pub fn create(
    list: &mut Vec<SmartCollection>,
    name: &str,
    criteria: LibraryCriteria,
    filter: LibraryFilter,
) -> Result<String, NameProblem> {
    if let Some(p) = name_problem(name, list, None) {
        return Err(p);
    }
    let id = uuid::Uuid::new_v4().simple().to_string();
    let mut criteria = criteria;
    criteria.sources = criteria
        .sources
        .iter()
        .map(|s| {
            GameStore::ALL
                .into_iter()
                .find(|store| {
                    [Lang::PtBr, Lang::EnUs]
                        .into_iter()
                        .any(|lang| game_text::store_name(lang, *store).eq_ignore_ascii_case(s))
                })
                .map(|store| store.key().to_owned())
                .unwrap_or_else(|| s.clone())
        })
        .collect();
    list.push(SmartCollection {
        id: id.clone(),
        name: clean_name(name),
        criteria,
        filter,
    });
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{InstallFilter, LibrarySort, PlayStatus};

    #[test]
    fn saved_route_preserves_intersection_and_sources_survive_language_changes() {
        use crate::library::{Game, query};
        let mut list = Vec::new();
        let criteria = LibraryCriteria::default()
            .toggle_source(&game_text::store_name(Lang::PtBr, GameStore::Manual));
        create(&mut list, "Adicionados", criteria, LibraryFilter::All).unwrap();
        let saved = parse(&to_json(&list)).remove(0);
        assert!(
            saved
                .criteria_in(Lang::EnUs)
                .has_source(&game_text::store_name(Lang::EnUs, GameStore::Manual))
        );

        let criteria = LibraryCriteria::default().toggle_source("Epic Games");
        create(
            &mut list,
            "Interseção",
            criteria,
            LibraryFilter::Store(GameStore::Steam),
        )
        .unwrap();
        let saved = parse(&to_json(&list)).remove(1);
        let games = vec![
            Game {
                store: Some(GameStore::Steam),
                ..Default::default()
            },
            Game {
                store: Some(GameStore::Epic),
                ..Default::default()
            },
        ];
        assert!(query::apply(&games, saved.filter, &saved.criteria_in(Lang::PtBr)).is_empty());
    }

    #[test]
    fn saves_and_reads_back_the_criteria() {
        let criteria = LibraryCriteria {
            install: InstallFilter::Installed,
            sort: LibrarySort::PlayTime,
            ..Default::default()
        }
        .toggle_genre("RPG")
        .toggle_status(PlayStatus::Playing);
        let mut list = Vec::new();
        let id = create(
            &mut list,
            "  RPGs  jogando ",
            criteria.clone(),
            LibraryFilter::All,
        )
        .unwrap();
        assert_eq!(list[0].name, "RPGs jogando");
        assert_eq!(
            create(
                &mut list,
                "rpgs JOGANDO",
                LibraryCriteria::default(),
                LibraryFilter::All
            ),
            Err(NameProblem::Duplicate)
        );
        assert_eq!(
            create(
                &mut list,
                " ",
                LibraryCriteria::default(),
                LibraryFilter::All
            ),
            Err(NameProblem::Empty)
        );

        let back = parse(&to_json(&list));
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].id, id);
        assert_eq!(back[0].criteria, criteria);
    }

    #[test]
    fn broken_settings_do_not_lose_the_good_collections() {
        assert!(parse("").is_empty());
        assert!(parse("{nao é json").is_empty());
        assert!(parse(&format!("[\"{}\"]", "x".repeat(MAX_JSON))).is_empty());
        // Uma coleção com um status desconhecido e outra sem campos novos: só a primeira some.
        let json = r#"[
            {"id":"a","name":"Nova","criteria":{"statuses":["Futuro"]}},
            {"id":"b","name":"Velha","criteria":{"favorites_only":true}}
        ]"#;
        let list = parse(json);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Velha");
        assert!(list[0].criteria.favorites_only);
    }
}
