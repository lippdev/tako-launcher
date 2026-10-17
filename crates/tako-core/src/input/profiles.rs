//! Perfis de botões por controle (INPUT-001): cada perfil tem os botões das ações do Tako e a combinação da
//! Central; cada controle (pelo GUID do SDL) usa um perfil, e o controle sem perfil próprio usa o padrão.
//!
//! O perfil padrão continua nas chaves de antes (`ControllerButtonMap` e `ControlCenterChord`, a mesma das
//! alphas C#); os outros perfis e a escolha de cada controle ficam em `ControllerProfiles` (JSON). Valor
//! quebrado volta ao padrão, campo por campo.
//!
//! Os botões de cada controle são traduzidos para os do mapa padrão antes de juntar os controles
//! ([`Profiles::read`]): o intérprete e as dicas só conhecem o padrão.

use super::shortcut::chord;
use super::{ButtonMap, GamepadButtons as B};
use serde::{Deserialize, Serialize};

/// Chaves das configurações.
pub mod keys {
    pub const BUTTONS: &str = "ControllerButtonMap";
    pub const CHORD: &str = "ControlCenterChord";
    pub const PROFILES: &str = "ControllerProfiles";
}

/// Id do perfil padrão (não se renomeia nem se exclui).
pub const DEFAULT_ID: u32 = 0;
pub const MAX_PROFILES: usize = 16;
pub const MAX_CONTROLLERS: usize = 32;
pub const MAX_NAME: usize = 40;
/// Limite do JSON gravado (dado do usuário: não confiável).
const MAX_JSON: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub id: u32,
    /// Vazio no perfil padrão (a interface mostra "Padrão" no idioma dela).
    pub name: String,
    pub buttons: ButtonMap,
    /// Combinação segurada que abre a Central.
    pub chord: B,
}

impl Profile {
    fn standard(buttons: ButtonMap, chord: B) -> Profile {
        Profile {
            id: DEFAULT_ID,
            name: String::new(),
            buttons,
            chord,
        }
    }

    pub fn is_default(&self) -> bool {
        self.id == DEFAULT_ID
    }

    /// O que este controle aperta agora, nos botões do padrão.
    pub fn read(&self, physical: B) -> PadInput {
        // Com os botões "livres" da combinação (View, Guia) apertados, os outros dela não viram comando: o Y de
        // View + Y não abre a busca.
        let free = self.chord & chord::FREE;
        let held = if !free.is_empty() && physical.contains(free) {
            self.chord & !free
        } else {
            B::NONE
        };
        PadInput {
            buttons: self.buttons.to_default(physical),
            ignore: self.buttons.to_default(held),
            central: !self.chord.is_empty() && physical.contains(self.chord),
        }
    }
}

/// Um controle já visto, com o perfil escolhido para ele.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controller {
    pub guid: String,
    /// Só para mostrar.
    pub name: String,
    pub profile: u32,
}

/// Leitura de um ou mais controles, já nos botões do padrão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PadInput {
    pub buttons: B,
    /// Botões que não viram comando agora (vão no `ignore` do intérprete).
    pub ignore: B,
    /// Algum controle segura a combinação da Central do seu perfil.
    pub central: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profiles {
    /// O primeiro é o padrão.
    profiles: Vec<Profile>,
    controllers: Vec<Controller>,
}

impl Default for Profiles {
    fn default() -> Self {
        Profiles {
            profiles: vec![Profile::standard(ButtonMap::default(), chord::DEFAULT)],
            controllers: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Stored {
    #[serde(default)]
    profiles: Vec<StoredProfile>,
    #[serde(default)]
    controllers: Vec<StoredController>,
}

#[derive(Serialize, Deserialize)]
struct StoredProfile {
    id: u32,
    name: String,
    #[serde(default)]
    buttons: String,
    #[serde(default)]
    chord: u32,
}

#[derive(Serialize, Deserialize)]
struct StoredController {
    guid: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    profile: u32,
}

/// Nome aparado e cortado no limite; vazio = inválido.
pub fn clean_name(name: &str) -> Option<String> {
    let name: String = name
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME)
        .collect();
    let name = name.trim().to_owned();
    (!name.is_empty()).then_some(name)
}

/// GUID do SDL: 32 dígitos hexadecimais.
fn valid_guid(guid: &str) -> bool {
    guid.len() == 32 && guid.bytes().all(|b| b.is_ascii_hexdigit())
}

impl Profiles {
    /// Lê das configurações (`get` devolve o valor gravado de uma chave).
    pub fn load(get: impl Fn(&str) -> Option<String>) -> Profiles {
        let buttons = ButtonMap::parse(get(keys::BUTTONS).as_deref());
        let chord = chord::parse(get(keys::CHORD).as_deref());
        let stored: Stored = get(keys::PROFILES)
            .filter(|s| s.len() <= MAX_JSON)
            .and_then(|s| {
                serde_json::from_str(&s)
                    .map_err(|e| tracing::warn!("perfis de controle inválidos: {e}"))
                    .ok()
            })
            .unwrap_or_default();
        let mut p = Profiles {
            profiles: vec![Profile::standard(buttons, chord)],
            controllers: Vec::new(),
        };
        for s in stored.profiles {
            if p.profiles.len() > MAX_PROFILES - 1 || s.id == DEFAULT_ID || p.get(s.id).is_some() {
                continue;
            }
            let Some(name) = clean_name(&s.name) else {
                continue;
            };
            p.profiles.push(Profile {
                id: s.id,
                name,
                buttons: ButtonMap::parse(Some(&s.buttons)),
                chord: chord::parse(Some(&s.chord.to_string())),
            });
        }
        for c in stored.controllers {
            let guid = c.guid.to_ascii_lowercase();
            if p.controllers.len() >= MAX_CONTROLLERS
                || !valid_guid(&guid)
                || p.controllers.iter().any(|x| x.guid == guid)
            {
                continue;
            }
            let profile = if p.get(c.profile).is_some() {
                c.profile
            } else {
                DEFAULT_ID
            };
            p.controllers.push(Controller {
                guid,
                name: clean_name(&c.name).unwrap_or_default(),
                profile,
            });
        }
        p
    }

    /// Valores a gravar, chave por chave.
    pub fn save(&self) -> [(&'static str, String); 3] {
        let d = self.standard();
        let stored = Stored {
            profiles: self.profiles[1..]
                .iter()
                .map(|p| StoredProfile {
                    id: p.id,
                    name: p.name.clone(),
                    buttons: p.buttons.serialize(),
                    chord: p.chord.0,
                })
                .collect(),
            controllers: self
                .controllers
                .iter()
                .map(|c| StoredController {
                    guid: c.guid.clone(),
                    name: c.name.clone(),
                    profile: c.profile,
                })
                .collect(),
        };
        [
            (keys::BUTTONS, d.buttons.serialize()),
            (keys::CHORD, chord::serialize(d.chord)),
            (
                keys::PROFILES,
                serde_json::to_string(&stored).unwrap_or_default(),
            ),
        ]
    }

    pub fn standard(&self) -> &Profile {
        &self.profiles[0]
    }

    pub fn all(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn controllers(&self) -> &[Controller] {
        &self.controllers
    }

    pub fn get(&self, id: u32) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }

    /// Perfil do controle; sem perfil próprio (ou sem controle), o padrão.
    pub fn of(&self, guid: &str) -> &Profile {
        let guid = guid.to_ascii_lowercase();
        self.controllers
            .iter()
            .find(|c| c.guid == guid)
            .and_then(|c| self.get(c.profile))
            .unwrap_or(self.standard())
    }

    /// Novo perfil com os botões de `from`. `None` = no limite ou nome vazio.
    pub fn create(&mut self, name: &str, from: u32) -> Option<u32> {
        let name = clean_name(name)?;
        if self.profiles.len() >= MAX_PROFILES {
            return None;
        }
        let base = self.get(from).unwrap_or(self.standard()).clone();
        let id = self.profiles.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        self.profiles.push(Profile { id, name, ..base });
        Some(id)
    }

    /// O padrão não muda de nome.
    pub fn rename(&mut self, id: u32, name: &str) -> bool {
        match (clean_name(name), self.get_mut(id)) {
            (Some(name), Some(p)) if !p.is_default() => {
                p.name = name;
                true
            }
            _ => false,
        }
    }

    /// Os controles que usavam o perfil voltam ao padrão. O padrão não se exclui.
    pub fn delete(&mut self, id: u32) -> bool {
        if id == DEFAULT_ID || self.get(id).is_none() {
            return false;
        }
        self.profiles.retain(|p| p.id != id);
        for c in &mut self.controllers {
            if c.profile == id {
                c.profile = DEFAULT_ID;
            }
        }
        true
    }

    /// Escolhe o perfil do controle (e guarda o nome para mostrar).
    pub fn assign(&mut self, guid: &str, name: &str, profile: u32) {
        let guid = guid.to_ascii_lowercase();
        if !valid_guid(&guid) || self.get(profile).is_none() {
            return;
        }
        let name = clean_name(name).unwrap_or_default();
        if let Some(c) = self.controllers.iter_mut().find(|c| c.guid == guid) {
            c.profile = profile;
            if !name.is_empty() {
                c.name = name;
            }
            return;
        }
        if self.controllers.len() >= MAX_CONTROLLERS {
            // Esquece o mais antigo que usa o padrão (é o mesmo que não ter entrada).
            match self
                .controllers
                .iter()
                .position(|c| c.profile == DEFAULT_ID)
            {
                Some(i) => drop(self.controllers.remove(i)),
                None => return,
            }
        }
        self.controllers.push(Controller {
            guid,
            name,
            profile,
        });
    }

    /// Junta os controles conectados (`(guid, botões)`), cada um traduzido pelo seu perfil.
    pub fn read<'a>(&self, pads: impl IntoIterator<Item = (&'a str, B)>) -> PadInput {
        pads.into_iter()
            .map(|(guid, b)| self.of(guid).read(b))
            .fold(PadInput::default(), |a, p| PadInput {
                buttons: a.buttons | p.buttons,
                ignore: a.ignore | p.ignore,
                central: a.central || p.central,
            })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::AppCommand;
    use super::*;
    use std::collections::HashMap;

    const PAD1: &str = "030000005e0400000b0b000000000000";
    const PAD2: &str = "050000004c050000e60c000000000000";

    fn load(values: &[(&str, &str)]) -> Profiles {
        let m: HashMap<String, String> = values
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Profiles::load(|k| m.get(k).cloned())
    }

    fn roundtrip(p: &Profiles) -> Profiles {
        let saved = p.save();
        Profiles::load(|k| saved.iter().find(|(x, _)| *x == k).map(|(_, v)| v.clone()))
    }

    #[test]
    fn each_controller_uses_its_profile_and_the_rest_the_default() {
        let mut p = Profiles::default();
        let arcade = p.create("Arcade", DEFAULT_ID).unwrap();
        let profile = p.get_mut(arcade).unwrap();
        profile.buttons.assign(AppCommand::Accept, B::B);
        profile.chord = B::GUIDE | B::A;
        p.assign(PAD1, "Xbox Wireless", arcade);
        assert_eq!(p.of(PAD1).id, arcade);
        assert_eq!(p.of(&PAD1.to_uppercase()).id, arcade);
        assert!(p.of(PAD2).is_default());
        assert!(p.of("").is_default());

        // B do controle 1 aceita; o B do controle 2 continua voltando.
        assert_eq!(p.read([(PAD1, B::B)]).buttons, B::A);
        assert_eq!(p.read([(PAD2, B::B)]).buttons, B::B);
        // Cada um com a sua combinação da Central.
        assert!(p.read([(PAD1, B::GUIDE | B::A)]).central);
        assert!(!p.read([(PAD1, B::VIEW | B::Y)]).central);
        assert!(p.read([(PAD2, B::VIEW | B::Y)]).central);
        // Metade da combinação em cada controle não abre.
        assert!(!p.read([(PAD2, B::VIEW), (PAD1, B::Y)]).central);
        // O A da combinação do controle 1 é o "voltar" dele (B no padrão): não vira comando.
        assert_eq!(p.read([(PAD1, B::GUIDE | B::A)]).ignore, B::B);
        assert_eq!(p.read([(PAD2, B::VIEW | B::Y)]).ignore, B::Y);
        assert_eq!(p.read([(PAD2, B::Y)]).ignore, B::NONE);

        let back = roundtrip(&p);
        assert_eq!(back, p);
    }

    #[test]
    fn profiles_are_created_renamed_and_deleted() {
        let mut p = Profiles::default();
        assert_eq!(p.create("   ", DEFAULT_ID), None);
        let a = p.create(" Sofá ", DEFAULT_ID).unwrap();
        assert_eq!(p.get(a).unwrap().name, "Sofá");
        let b = p.create(&"x".repeat(100), a).unwrap();
        assert_eq!(p.get(b).unwrap().name.chars().count(), MAX_NAME);
        assert!(p.rename(a, "Sala"));
        assert!(!p.rename(a, ""));
        assert!(!p.rename(DEFAULT_ID, "Outro"));
        p.assign(PAD1, "DualSense", a);
        assert!(!p.delete(DEFAULT_ID));
        assert!(p.delete(a));
        assert!(p.get(a).is_none());
        // O controle volta ao padrão, mas continua lembrado.
        assert!(p.of(PAD1).is_default());
        assert_eq!(p.controllers()[0].name, "DualSense");
        // Ids não se repetem enquanto o maior existir.
        let c = p.create("Novo", DEFAULT_ID).unwrap();
        assert!(c > b);
        while p.create("mais", DEFAULT_ID).is_some() {}
        assert_eq!(p.all().len(), MAX_PROFILES);
        // GUID inválido ou perfil que não existe não entra.
        p.assign("lixo", "X", c);
        p.assign(PAD2, "X", 999);
        assert_eq!(p.controllers().len(), 1);
    }

    #[test]
    fn broken_values_fall_back_to_the_default() {
        assert_eq!(load(&[]), Profiles::default());
        assert_eq!(load(&[(keys::PROFILES, "{lixo")]), Profiles::default());
        assert_eq!(
            load(&[(keys::PROFILES, &"x".repeat(MAX_JSON + 1))]),
            Profiles::default()
        );
        let p = load(&[
            (keys::BUTTONS, "1,2,3"),
            (keys::CHORD, "nada"),
            (
                keys::PROFILES,
                &format!(
                    r#"{{"profiles":[
                        {{"id":0,"name":"padrão falso"}},
                        {{"id":3,"name":"Bom","buttons":"8192,4096,16384,32768,256,512,16","chord":1056}},
                        {{"id":3,"name":"repetido"}},
                        {{"id":4,"name":"  "}},
                        {{"id":5,"name":"Quebrado","buttons":"lixo","chord":99999999}}
                    ],"controllers":[
                        {{"guid":"{PAD1}","name":"Um","profile":3}},
                        {{"guid":"{PAD1}","name":"repetido","profile":5}},
                        {{"guid":"curto","profile":3}},
                        {{"guid":"{PAD2}","profile":42}}
                    ]}}"#
                ),
            ),
        ]);
        assert_eq!(p.standard().buttons, ButtonMap::default());
        assert_eq!(p.standard().chord, chord::DEFAULT);
        let names: Vec<_> = p.all().iter().map(|x| x.name.as_str()).collect();
        assert_eq!(names, ["", "Bom", "Quebrado"]);
        assert_eq!(p.get(3).unwrap().chord, B::VIEW | B::GUIDE);
        assert_eq!(
            p.get(3).unwrap().buttons.button(AppCommand::Accept),
            Some(B::B)
        );
        assert_eq!(p.get(5).unwrap().buttons, ButtonMap::default());
        assert_eq!(p.get(5).unwrap().chord, chord::DEFAULT);
        assert_eq!(p.controllers().len(), 2);
        assert_eq!(p.of(PAD1).id, 3);
        assert!(p.of(PAD2).is_default());
    }
}
