//! Emuladores e pastas de ROMs pela ponte (SRC-001, WEB-13c1): o que a Configurações › Biblioteca mostra e muda, com as
//! mesmas regras e as mesmas chaves do app Slint (`tako-ui/src/shell/settings_emulators.rs`).
//!
//! - **Fonte da verdade.** A lista de pastas mora nas configurações (`EmulatorRomFolders`, JSON); cada chamada lê de
//!   lá, muda e grava as três chaves juntas, como o Slint (`Config::save`). Os dois apps seguem compatíveis.
//! - **Leitura de novo.** Mudar uma pasta muda os jogos do "Emuladores": o núcleo guarda a configuração nova para a
//!   próxima leitura e pede uma, sem esperar por ela.
//! - **Demonstração.** Nada é procurado nem verificado no disco: não há emulador achado e qualquer caminho vale.
//! - **Seletor de pasta.** Escolher pela janela do sistema é coisa do hospedeiro; aqui o caminho chega digitado.

use crate::protocol::{
    BridgeError, Call, EmulatorEntry, EmulatorFolder, EmulatorFolderSystem, EmulatorsView,
    ErrorCode, Reply, RomFolderEntry, RomFolderSystem,
};
use crate::{Core, organize, settings};
use std::path::{Path, PathBuf};
use tako_sources::emulators::emulator::{Emulator, Env};
use tako_sources::emulators::{Config, Fs, Installed, RomFolder, keys, systems};

/// O mesmo teto do `Config::parse`: o que passar disso seria descartado na próxima abertura.
const MAX_FOLDERS: usize = 256;
const MAX_PATH_CHARS: usize = 4096;

/// A configuração gravada, do jeito que o Slint a lê.
pub(crate) fn config(core: &Core) -> Config {
    let get = |key: &str| core.settings.get(key).map(String::as_str);
    Config::parse(
        get(keys::ROM_FOLDERS),
        get(keys::PATHS),
        get(keys::BY_SYSTEM),
    )
}

/// Grava as três chaves e deixa a configuração nova para a próxima leitura das lojas.
fn save(core: &mut Core, config: Config) -> Result<(), BridgeError> {
    for (key, value) in config.save() {
        settings::save(core, key, &value)?;
    }
    *core
        .emulators
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = config;
    Ok(())
}

/// Pede uma leitura das lojas (a que está em andamento já cobre o que estava gravado antes). `true` se começou.
pub(crate) fn rescan(core: &mut Core) -> Result<bool, BridgeError> {
    let disabled = core
        .settings
        .get("DisabledStores")
        .cloned()
        .unwrap_or_default();
    core.sources
        .start(&disabled)
        .map_err(|e| BridgeError::new(ErrorCode::Internal, e.to_string()))
}

/// Atende as quatro chamadas. O `bool` diz se a configuração mudou e a biblioteca precisa ser lida de novo.
pub(crate) fn handle(core: &mut Core, call: Call) -> Result<(Reply, bool), BridgeError> {
    match call {
        Call::EmulatorsList => Ok((Reply::EmulatorsList(view(core)), false)),
        Call::EmulatorsFolderAdd(request) => {
            add(core, request)?;
            Ok((Reply::EmulatorsFolderAdd(view(core)), true))
        }
        Call::EmulatorsFolderRemove(request) => {
            remove(core, request)?;
            Ok((Reply::EmulatorsFolderRemove(view(core)), true))
        }
        Call::EmulatorsFolderSystemSet(request) => {
            let changed = set_system(core, request)?;
            Ok((Reply::EmulatorsFolderSystemSet(view(core)), changed))
        }
        _ => Err(organize::bad("chamada de emuladores inesperada")),
    }
}

fn not_found() -> BridgeError {
    BridgeError::new(ErrorCode::NotFound, "pasta de ROMs não encontrada")
}

/// Caminho digitado: absoluto, sem lixo de controle. Fora da demonstração a pasta precisa existir.
fn parse_path(core: &Core, text: &str) -> Result<PathBuf, BridgeError> {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > MAX_PATH_CHARS || text.contains('\0') {
        return Err(organize::bad("caminho de pasta inválido"));
    }
    let path = PathBuf::from(text);
    if !path.is_absolute() {
        return Err(organize::bad("use o caminho completo da pasta"));
    }
    if !core.simulate && !path.is_dir() {
        return Err(organize::bad("a pasta não existe"));
    }
    Ok(path)
}

/// Os sistemas que uma pasta aceita: os que têm extensões (e o PS3, que é uma pasta de jogo).
fn folder_systems() -> impl Iterator<Item = &'static systems::System> {
    systems::SYSTEMS
        .iter()
        .filter(|s| !s.extensions.is_empty() || s.id == "ps3")
}

fn add(core: &mut Core, request: EmulatorFolder) -> Result<(), BridgeError> {
    let path = parse_path(core, &request.path)?;
    let mut config = config(core);
    // Já está na lista: nada a fazer (o Slint também não repete a pasta).
    if config.rom_folders.iter().any(|f| f.path == path) {
        return Ok(());
    }
    if config.rom_folders.len() >= MAX_FOLDERS {
        return Err(BridgeError::new(
            ErrorCode::Conflict,
            "pastas de ROMs demais",
        ));
    }
    config.rom_folders.push(RomFolder { path, system: None });
    save(core, config)
}

fn remove(core: &mut Core, request: EmulatorFolder) -> Result<(), BridgeError> {
    // Tirar não olha o disco: a pasta pode já ter sumido, e é justamente por isso que se quer tirá-la.
    let path = PathBuf::from(request.path.trim());
    let mut config = config(core);
    let before = config.rom_folders.len();
    config.rom_folders.retain(|f| f.path != path);
    if config.rom_folders.len() == before {
        return Err(not_found());
    }
    save(core, config)
}

/// `true` se o sistema da pasta mudou.
fn set_system(core: &mut Core, request: EmulatorFolderSystem) -> Result<bool, BridgeError> {
    let system = match request.system.as_deref() {
        None => None,
        Some(id) => Some(
            folder_systems()
                .find(|s| s.id == id)
                .map(|s| s.id.to_owned())
                .ok_or_else(|| organize::bad("sistema inválido"))?,
        ),
    };
    let path = PathBuf::from(request.path.trim());
    let mut config = config(core);
    let folder = config
        .rom_folders
        .iter_mut()
        .find(|f| f.path == path)
        .ok_or_else(not_found)?;
    if folder.system == system {
        return Ok(false);
    }
    folder.system = system;
    save(core, config)?;
    Ok(true)
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn view(core: &Core) -> EmulatorsView {
    let config = config(core);
    // A busca olha várias pastas do disco; na demonstração ela nem roda.
    let found = if core.simulate {
        Installed::default()
    } else {
        Installed::detect(&Env::current(), &config, &Fs::real())
    };
    EmulatorsView {
        rom_folders: config
            .rom_folders
            .iter()
            .map(|f| RomFolderEntry {
                path: text(&f.path),
                system: f.system.clone(),
            })
            .collect(),
        emulators: Emulator::ALL
            .into_iter()
            .map(|emulator| {
                let program = found.programs.get(&emulator);
                EmulatorEntry {
                    key: emulator.key().to_owned(),
                    name: emulator.name().to_owned(),
                    found: program.is_some(),
                    path: program.map(|p| text(&p.target)),
                    flatpak: program.and_then(|p| p.flatpak.clone()),
                    chosen: program.is_some() && config.paths.contains_key(&emulator),
                }
            })
            .collect(),
        systems: folder_systems()
            .map(|s| RomFolderSystem {
                id: s.id.to_owned(),
                name: s.name.to_owned(),
            })
            .collect(),
        max_folders: MAX_FOLDERS,
    }
}
