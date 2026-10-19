//! Bottles (Linux, SRC-002): garrafas de Wine lidas de `bottle.yml` sem o Bottles aberto.
//!
//! Suporta a instalação nativa e Flatpak (`com.usebottles.bottles`).
//! Cada garrafa contém uma configuração `bottle.yml` com programas registrados em `External_Programs`.
//!
//! A abertura é feita via linha de comando:
//! - Nativo: `bottles-cli run -b <garrafa> -p <programa>`
//! - Flatpak: `flatpak run --command=bottles-cli com.usebottles.bottles run -b <garrafa> -p <programa>`
//!
//! Ícones podem ser arquivos absolutos ou relativos na subpasta `icons/` da garrafa.

use crate::{LibraryScan, LibrarySource, MAX_STORE_FILE, file_uri, read_limited};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore, IconOrigin, media};

#[cfg(test)]
mod tests;

const ACCENT: &str = "#597AC8";

/// Garrafa e ambiente configurado.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BottleConfig {
    pub name: String,
    pub programs: Vec<BottleProgram>,
}

/// Programa registrado em uma garrafa (`External_Programs`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BottleProgram {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub path: Option<String>,
    pub folder: Option<String>,
    pub arguments: Option<String>,
    pub icon: Option<String>,
}

/// Uma raiz de dados do Bottles (nativa ou Flatpak).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BottlesRoot {
    pub path: PathBuf,
    pub is_flatpak: bool,
}

/// Limpa aspas simples ou duplas de uma string YAML.
fn unquote(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"') && s.len() >= 2)
        || (s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2)
    {
        s[1..s.len() - 1].replace("\\\"", "\"").replace("\\'", "'")
    } else {
        s.to_owned()
    }
}

/// Escapa e envolve em aspas um argumento para linha de comando.
fn quote_arg(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Transforma o nome em um slug seguro para o id do jogo.
fn valid_slug(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() { "bottle".into() } else { out }
}

/// Parser seguro e sem pânico para o formato YAML simplificado de `bottle.yml`.
pub fn parse_bottle_yml(content: &str, folder_name: &str) -> BottleConfig {
    let mut config = BottleConfig {
        name: String::new(),
        programs: Vec::new(),
    };

    let mut in_external_programs = false;
    let mut current_prog: Option<BottleProgram> = None;

    for line in content.lines() {
        // Ignora comentários e linhas vazias
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let indent = line.len() - line.trim_start().len();

        if indent == 0 {
            // Se mudou de seção no nível zero, fecha o programa atual
            if let Some(p) = current_prog
                .take()
                .filter(|p| !p.name.is_empty() || !p.executable.is_empty())
            {
                config.programs.push(p);
            }
            in_external_programs = false;

            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim();
                let val = v.trim();
                if key.eq_ignore_ascii_case("Name") {
                    config.name = unquote(val);
                } else if key.eq_ignore_ascii_case("External_Programs") {
                    in_external_programs = true;
                }
            }
            continue;
        }

        if in_external_programs {
            // Nível de entrada de programa (indent 2 ou 3)
            if (2..=3).contains(&indent) {
                if let Some(p) = current_prog
                    .take()
                    .filter(|p| !p.name.is_empty() || !p.executable.is_empty())
                {
                    config.programs.push(p);
                }
                let mut id = trimmed;
                if let Some(stripped) = id.strip_suffix(':') {
                    id = stripped.trim();
                }
                current_prog = Some(BottleProgram {
                    id: unquote(id),
                    ..Default::default()
                });
                continue;
            }

            // Propriedades do programa (indent >= 4)
            if indent >= 4
                && let Some(prog) = current_prog.as_mut()
                && let Some((k, v)) = trimmed.split_once(':')
            {
                let key = k.trim();
                let val = unquote(v.trim());
                if key.eq_ignore_ascii_case("name") {
                    prog.name = val;
                } else if key.eq_ignore_ascii_case("executable") {
                    prog.executable = val;
                } else if key.eq_ignore_ascii_case("path") {
                    prog.path = Some(val);
                } else if key.eq_ignore_ascii_case("folder") {
                    prog.folder = Some(val);
                } else if key.eq_ignore_ascii_case("arguments") {
                    prog.arguments = (!val.is_empty()).then_some(val);
                } else if key.eq_ignore_ascii_case("icon") {
                    prog.icon = (!val.is_empty()).then_some(val);
                } else if key.eq_ignore_ascii_case("id") && prog.id.is_empty() {
                    prog.id = val;
                }
            }
        }
    }

    if let Some(p) = current_prog
        .take()
        .filter(|p| !p.name.is_empty() || !p.executable.is_empty())
    {
        config.programs.push(p);
    }

    if config.name.is_empty() {
        config.name = folder_name.to_owned();
    }

    config
}

/// Encontra o ícone de um programa da garrafa, se existir.
fn resolve_icon(bottle_dir: &Path, icon_val: Option<&str>) -> Option<String> {
    let icon_val = icon_val?.trim();
    if icon_val.is_empty() || icon_val.starts_with("com.usebottles.") {
        return None;
    }

    // Restringe o ícone estritamente à subpasta icons/ da garrafa
    let p = Path::new(icon_val);
    if p.is_absolute()
        || p.components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }

    let icons_dir = bottle_dir.join("icons");
    let rel = icons_dir.join(icon_val);
    if rel.is_file() {
        return Some(file_uri(&rel));
    }

    // Com extensão .png caso tenha vindo sem
    if !icon_val.ends_with(".png") {
        let with_ext = icons_dir.join(format!("{icon_val}.png"));
        if with_ext.is_file() {
            return Some(file_uri(&with_ext));
        }
    }

    None
}

/// Lê uma garrafa específica e seus programas registrados.
pub fn scan_bottle(bottle_dir: &Path, is_flatpak: bool) -> Result<Vec<Game>, String> {
    let yml_path = bottle_dir.join("bottle.yml");
    if !yml_path.is_file() {
        return Ok(Vec::new());
    }

    let folder_name = bottle_dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "bottle".into());

    let bytes = read_limited(&yml_path, MAX_STORE_FILE)
        .map_err(|e| format!("não foi possível ler {}: {e}", yml_path.display()))?;
    let content = String::from_utf8_lossy(&bytes);
    let config = parse_bottle_yml(&content, &folder_name);

    let bottle_slug = valid_slug(&config.name);
    let mut games = Vec::new();

    for prog in config.programs {
        let prog_name = prog.name.trim();
        let prog_uuid = prog.id.trim();
        // Não passe -p "" quando prog.name estiver vazio e pule o programa se o UUID for vazio
        if prog_name.is_empty() || prog_uuid.is_empty() {
            continue;
        }

        let prog_title = prog_name.to_owned();
        // O id do Bottles deve usar a chave UUID de External_Programs (prog.id)
        let id = format!("bottles:{bottle_slug}:{prog_uuid}");

        let (target, args) = if is_flatpak {
            (
                "flatpak".to_owned(),
                format!(
                    "run --command=bottles-cli com.usebottles.bottles run -b {} -p {}",
                    quote_arg(&config.name),
                    quote_arg(prog_name)
                ),
            )
        } else {
            (
                "bottles-cli".to_owned(),
                format!(
                    "run -b {} -p {}",
                    quote_arg(&config.name),
                    quote_arg(prog_name)
                ),
            )
        };

        let icon_uri = resolve_icon(bottle_dir, prog.icon.as_deref());

        let mut game = Game {
            id,
            title: prog_title,
            store: Some(GameStore::Bottles),
            accent: ACCENT.to_owned(),
            is_installed: true,
            launch_target: Some(target),
            launch_arguments: Some(args),
            install_path: prog.folder.filter(|f| !f.trim().is_empty()),
            store_product_id: Some(format!("{}:{}", config.name, prog.name)),
            ..Default::default()
        };

        if let Some(uri) = icon_uri {
            game.media.insert(media::ICON.to_owned(), uri);
            game.icon_origin = Some(IconOrigin::Store);
        }

        games.push(game);
    }

    Ok(games)
}

/// Varre uma raiz de garrafas (a pasta que contém `bottles/` ou garrafas diretas).
pub fn scan_root(root: &BottlesRoot) -> Result<Vec<Game>, String> {
    let mut games = Vec::new();
    let bottles_sub = root.path.join("bottles");
    let target_dir = if bottles_sub.is_dir() {
        &bottles_sub
    } else {
        &root.path
    };

    let entries = std::fs::read_dir(target_dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() && p.join("bottle.yml").is_file() {
            match scan_bottle(&p, root.is_flatpak) {
                Ok(list) => games.extend(list),
                Err(e) => tracing::warn!("garrafa ilegível em {}: {e}", p.display()),
            }
        }
    }

    Ok(games)
}

/// Diretórios de garrafas no Linux: padrão XDG e Flatpak `com.usebottles.bottles`.
#[cfg(any(unix, test))]
pub fn linux_roots(home: &Path, xdg_data: Option<PathBuf>) -> Vec<BottlesRoot> {
    let data = xdg_data
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home.join(".local/share"));
    let mut roots = vec![
        BottlesRoot {
            path: data.join("bottles"),
            is_flatpak: false,
        },
        BottlesRoot {
            path: home.join(".var/app/com.usebottles.bottles/data/bottles"),
            is_flatpak: true,
        },
    ];
    let host = home.join(".local/share/bottles");
    if !roots.iter().any(|r| r.path == host) {
        roots.push(BottlesRoot {
            path: host,
            is_flatpak: false,
        });
    }
    roots
}

/// Fonte de biblioteca para o Bottles.
pub struct BottlesSource {
    roots: Vec<BottlesRoot>,
}

impl BottlesSource {
    pub fn new(roots: Vec<BottlesRoot>) -> Self {
        BottlesSource { roots }
    }

    #[cfg(unix)]
    pub fn detect() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        BottlesSource::new(linux_roots(
            &home,
            std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        ))
    }
}

impl LibrarySource for BottlesSource {
    fn store(&self) -> GameStore {
        GameStore::Bottles
    }

    fn scan(&self) -> LibraryScan {
        let valid_roots: Vec<&BottlesRoot> =
            self.roots.iter().filter(|r| r.path.is_dir()).collect();
        if valid_roots.is_empty() {
            return LibraryScan::missing(t("sources_unix.bottles_missing"));
        }
        let mut seen = HashSet::new();
        let mut games = Vec::new();
        for root in valid_roots {
            match scan_root(root) {
                Ok(list) => games.extend(list.into_iter().filter(|g| seen.insert(g.id.clone()))),
                Err(e) => {
                    tracing::warn!("pasta de garrafas ilegível em {}: {e}", root.path.display())
                }
            }
        }
        games.sort_by_cached_key(|g| g.title.to_lowercase());
        LibraryScan {
            games,
            client_installed: true,
            notice: None,
        }
    }
}
