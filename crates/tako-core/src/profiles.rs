//! Perfis locais (UX-004): várias pessoas na mesma máquina, cada uma com a própria biblioteca, favoritos, tempo
//! de jogo, coleções e configurações. Cada perfil é uma pasta de dados (`DataPaths::for_profile`); a lista e o
//! perfil em uso ficam em `profiles.toml`, na pasta da instalação. O principal é a pasta de sempre.

use crate::paths::{DataPaths, PROFILES_DIR};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const FILE: &str = "profiles.toml";
/// O perfil que já existia antes dos perfis: usa a própria pasta da instalação.
pub const MAIN: &str = "main";
pub const MAX_PROFILES: usize = 8;
pub const MAX_NAME: usize = 32;
const MAX_FILE: u64 = 64 * 1024;
/// Marcador "já escolheu quem está jogando": a interface reaberta pela troca não pergunta de novo.
const CHOSEN_MARKER: &str = "profile-chosen";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    /// Vazio no principal até alguém dar um nome (a interface mostra "Principal").
    #[serde(default)]
    pub name: String,
}

impl Profile {
    /// Pasta do perfil em `profiles/` (`None` = a da instalação).
    pub fn dir(&self) -> Option<&str> {
        (self.id != MAIN).then_some(self.id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profiles {
    pub active: String,
    /// Perguntar "Quem está jogando?" ao abrir o Tako.
    #[serde(default)]
    pub ask_on_start: bool,
    #[serde(default, rename = "profile")]
    pub list: Vec<Profile>,
}

impl Default for Profiles {
    fn default() -> Self {
        Profiles {
            active: MAIN.into(),
            ask_on_start: false,
            list: vec![Profile {
                id: MAIN.into(),
                name: String::new(),
            }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("nome inválido")]
    InvalidName,
    #[error("nome repetido")]
    DuplicateName,
    #[error("perfis demais")]
    TooMany,
    #[error("perfil desconhecido")]
    Unknown,
    #[error("o perfil principal e o perfil em uso não podem ser apagados")]
    Protected,
}

/// Ids gerados aqui são `p` + hexadecimal; o arquivo pode ter sido editado, então só passa o que vira pasta segura.
fn valid_id(id: &str) -> bool {
    id == MAIN
        || (!id.is_empty()
            && id.len() <= 40
            && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

/// Nome sem espaços nas pontas, sem caracteres de controle, com 1 a `MAX_NAME` caracteres.
pub fn clean_name(name: &str) -> Option<String> {
    let name = name.trim();
    let n = name.chars().count();
    ((1..=MAX_NAME).contains(&n) && !name.chars().any(char::is_control)).then(|| name.to_owned())
}

impl Profiles {
    pub fn file(machine: &Path) -> PathBuf {
        machine.join(FILE)
    }

    /// Lê a lista; arquivo ausente, grande demais ou quebrado vira só o principal. Ids inválidos e repetidos
    /// saem; o principal sempre existe; perfil em uso desconhecido volta ao principal.
    pub fn load(machine: &Path) -> Profiles {
        let path = Self::file(machine);
        let read = std::fs::metadata(&path)
            .ok()
            .filter(|m| m.len() <= MAX_FILE)
            .and_then(|_| std::fs::read_to_string(&path).ok());
        let mut p: Profiles = match read.as_deref().map(toml::from_str) {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                tracing::warn!("{FILE} inválido, usando só o perfil principal: {e}");
                Profiles::default()
            }
            None => Profiles::default(),
        };
        let mut seen = std::collections::HashSet::new();
        p.list
            .retain(|x| valid_id(&x.id) && seen.insert(x.id.clone()));
        for x in &mut p.list {
            x.name = clean_name(&x.name).unwrap_or_default();
        }
        if !p.list.iter().any(|x| x.id == MAIN) {
            p.list.insert(0, Profiles::default().list.remove(0));
        }
        p.list.truncate(MAX_PROFILES);
        if !p.list.iter().any(|x| x.id == p.active) {
            p.active = MAIN.into();
        }
        p
    }

    /// Grava num arquivo temporário e troca: uma queda no meio não deixa a lista pela metade.
    pub fn save(&self, machine: &Path) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(machine).map_err(|e| e.to_string())?;
        let tmp = machine.join(format!("{FILE}.tmp"));
        std::fs::write(&tmp, text)
            .and_then(|()| std::fs::rename(&tmp, Self::file(machine)))
            .map_err(|e| e.to_string())
    }

    pub fn find(&self, id: &str) -> Option<&Profile> {
        self.list.iter().find(|p| p.id == id)
    }

    pub fn active(&self) -> Profile {
        self.find(&self.active)
            .cloned()
            .unwrap_or_else(|| Profiles::default().list.remove(0))
    }

    /// Pastas do perfil em uso.
    pub fn paths(&self, machine: &DataPaths) -> DataPaths {
        machine.for_profile(self.active().dir())
    }

    fn check_name(&self, name: &str, except: Option<&str>) -> Result<String, ProfileError> {
        let name = clean_name(name).ok_or(ProfileError::InvalidName)?;
        let taken = self
            .list
            .iter()
            .any(|p| Some(p.id.as_str()) != except && p.name.to_lowercase() == name.to_lowercase());
        if taken {
            return Err(ProfileError::DuplicateName);
        }
        Ok(name)
    }

    /// Cria um perfil vazio (a pasta nasce quando ele é aberto) e devolve o id.
    pub fn add(&mut self, name: &str) -> Result<String, ProfileError> {
        if self.list.len() >= MAX_PROFILES {
            return Err(ProfileError::TooMany);
        }
        let name = self.check_name(name, None)?;
        let id = format!("p{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
        self.list.push(Profile {
            id: id.clone(),
            name,
        });
        Ok(id)
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), ProfileError> {
        let name = self.check_name(name, Some(id))?;
        let p = self
            .list
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(ProfileError::Unknown)?;
        p.name = name;
        Ok(())
    }

    /// Tira da lista; os dados saem com `delete_data`, depois de gravar a lista.
    pub fn remove(&mut self, id: &str) -> Result<(), ProfileError> {
        if id == MAIN || id == self.active {
            return Err(ProfileError::Protected);
        }
        let before = self.list.len();
        self.list.retain(|p| p.id != id);
        if self.list.len() == before {
            return Err(ProfileError::Unknown);
        }
        Ok(())
    }

    pub fn set_active(&mut self, id: &str) -> Result<(), ProfileError> {
        self.find(id).ok_or(ProfileError::Unknown)?;
        self.active = id.to_owned();
        Ok(())
    }
}

/// Apaga a pasta de um perfil que saiu da lista. Nunca a do principal nem um id que não seja só letras,
/// números e `-` (não sai de `profiles/`).
pub fn delete_data(machine: &Path, id: &str) -> Result<(), String> {
    if id == MAIN || !valid_id(id) {
        return Err(format!("id de perfil recusado: {id:?}"));
    }
    let dir = machine.join(PROFILES_DIR).join(id);
    match std::fs::remove_dir_all(&dir) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

/// A troca de perfil reabre a interface: ela não deve perguntar "Quem está jogando?" de novo.
pub fn mark_chosen(machine: &Path) {
    if let Err(e) = std::fs::write(machine.join(CHOSEN_MARKER), "") {
        tracing::warn!("marcador de perfil escolhido: {e}");
    }
}

/// `true` (e apaga o marcador) se a interface abre logo depois de uma troca de perfil.
pub fn take_chosen(machine: &Path) -> bool {
    std::fs::remove_file(machine.join(CHOSEN_MARKER)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_file_means_only_the_main_profile_in_the_install_folder() {
        let dir = tempfile::tempdir().unwrap();
        let p = Profiles::load(dir.path());
        assert_eq!(p, Profiles::default());
        let machine = DataPaths::at(dir.path());
        assert_eq!(p.paths(&machine), machine);
    }

    #[test]
    fn profiles_are_created_renamed_switched_saved_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let mut p = Profiles::load(dir.path());
        let ana = p.add("  Ana ").unwrap();
        assert_eq!(p.find(&ana).unwrap().name, "Ana");
        assert_eq!(p.add("ana"), Err(ProfileError::DuplicateName));
        assert_eq!(p.add(" "), Err(ProfileError::InvalidName));
        assert_eq!(
            p.add(&"x".repeat(MAX_NAME + 1)),
            Err(ProfileError::InvalidName)
        );
        assert_eq!(p.add("a\u{7}"), Err(ProfileError::InvalidName));
        p.rename(&ana, "Ana Clara").unwrap();
        p.rename(&ana, "ana clara").unwrap();
        assert_eq!(p.rename("nada", "X"), Err(ProfileError::Unknown));
        p.set_active(&ana).unwrap();
        assert_eq!(p.set_active("nada"), Err(ProfileError::Unknown));
        p.ask_on_start = true;
        p.save(dir.path()).unwrap();

        let loaded = Profiles::load(dir.path());
        assert_eq!(loaded, p);
        let machine = DataPaths::at(dir.path());
        let paths = loaded.paths(&machine);
        assert_eq!(paths.root, dir.path().join("profiles").join(&ana));
        assert_eq!(paths.database(), paths.root.join("tako.db"));
        assert_eq!(paths.logs(), machine.logs());
        assert_eq!(paths.art_cache(), machine.art_cache());
        assert_eq!(paths.session_journal(), machine.session_journal());

        assert_eq!(p.remove(&ana), Err(ProfileError::Protected));
        assert_eq!(p.remove(MAIN), Err(ProfileError::Protected));
        p.set_active(MAIN).unwrap();
        std::fs::create_dir_all(paths.root.join("themes")).unwrap();
        p.remove(&ana).unwrap();
        delete_data(dir.path(), &ana).unwrap();
        assert!(!paths.root.exists());
        assert!(delete_data(dir.path(), &ana).is_ok());
        for bad in [MAIN, "..", "../x", "a/b", ""] {
            assert!(delete_data(dir.path(), bad).is_err(), "{bad:?}");
        }
        assert!(dir.path().exists());

        for i in 0..MAX_PROFILES - 1 {
            p.add(&format!("P{i}")).unwrap();
        }
        assert_eq!(p.add("Mais um"), Err(ProfileError::TooMany));
    }

    #[test]
    fn a_broken_or_edited_file_never_points_outside_profiles() {
        let dir = tempfile::tempdir().unwrap();
        let write = |s: &str| std::fs::write(dir.path().join(FILE), s).unwrap();
        write("isto não é toml [[[");
        assert_eq!(Profiles::load(dir.path()), Profiles::default());
        write(
            r#"active = "../fora"
[[profile]]
id = "../fora"
name = "X"
[[profile]]
id = "p1"
name = "Bia"
[[profile]]
id = "p1"
name = "Repetido"
[[profile]]
id = "p2"
name = "\u0007"
"#,
        );
        let p = Profiles::load(dir.path());
        let ids: Vec<&str> = p.list.iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, [MAIN, "p1", "p2"]);
        assert_eq!(p.active, MAIN);
        assert_eq!(p.find("p2").unwrap().name, "");
        write(&"x".repeat(MAX_FILE as usize + 1));
        assert_eq!(Profiles::load(dir.path()), Profiles::default());
    }

    #[test]
    fn the_chosen_marker_is_used_once() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!take_chosen(dir.path()));
        mark_chosen(dir.path());
        assert!(take_chosen(dir.path()));
        assert!(!take_chosen(dir.path()));
    }
}
