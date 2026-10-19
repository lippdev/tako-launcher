//! Temas do Tako (PRD §7, MIG-031): pacote `.takotheme` (zip ou pasta com `theme.toml`), validação,
//! instalação atômica em `<dados>/themes/<id>/`, tema escolhido e volta ao padrão em loop de falha.
//! A compilação do `.slint` fica no `tako-ui` (`tako theme check`).

mod manifest;
pub mod overrides;
mod package;
pub mod remote;

use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tako_core::i18n::tf;

pub use manifest::{
    DEFAULT_ENTRY, MANIFEST_FILE, MAX_MANIFEST_BYTES, MAX_NAV_SOUND_DURATION_SECS,
    MAX_THEME_MUSIC_BYTES, THEME_API_VERSION, ThemeAudio, ThemeManifest, is_safe_id,
    parse_audio_table, validate_audio_path, wav_duration, wav_duration_from_reader,
};
pub use package::{
    ALLOWED_EXTENSIONS, EXTENSION, Limits, normalize_entry, validate_package, validate_package_with,
};

/// Tema embutido no `tako-ui`; nenhum pacote pode usar este id.
pub const DEFAULT_THEME_ID: &str = "default";
/// Temas incluídos no app, cujo diretório de instalação é mantido pelo Tako.
pub const BUILTIN_EXTRA_THEME_IDS: [&str; 2] = ["tako-alto-contraste", "tako-fita"];
const SELECTED_FILE: &str = ".selected";
const LOADING_FILE: &str = ".loading";
const FAULTS_FILE: &str = ".faults";
/// Falhas do tema escolhido dentro da janela que fazem voltar ao padrão.
pub const MAX_FAULTS: usize = 3;
pub const FAULT_WINDOW: Duration = Duration::from_secs(10);

/// Motivo de recusa: `code` estável (para testes e logs), `message` já traduzida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub code: &'static str,
    pub message: String,
}

impl Problem {
    pub fn new(code: &'static str, message: String) -> Problem {
        Problem { code, message }
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Pasta de tema com o resultado da validação (um tema quebrado aparece na lista com os problemas).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledTheme {
    pub dir: PathBuf,
    pub manifest: Result<ThemeManifest, Vec<Problem>>,
}

impl InstalledTheme {
    pub fn id(&self) -> String {
        match &self.manifest {
            Ok(m) => m.id.clone(),
            Err(_) => self
                .dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }

    /// Temas em `themes_root`: subpastas com `theme.toml` (ocultas e temporárias ficam de fora).
    pub fn scan(themes_root: &Path) -> Vec<InstalledTheme> {
        let Ok(dir) = fs::read_dir(themes_root) else {
            return Vec::new();
        };
        let mut themes: Vec<InstalledTheme> = dir
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .map(|e| e.path())
            .filter(|p| p.join(MANIFEST_FILE).is_file())
            .map(|dir| InstalledTheme {
                manifest: validate_package(&dir),
                dir,
            })
            .collect();
        themes.sort_by_cached_key(|t| match &t.manifest {
            Ok(m) => m.name.to_lowercase(),
            Err(_) => t.id().to_lowercase(),
        });
        themes
    }
}

fn reserved(id: &str) -> bool {
    id.eq_ignore_ascii_case(DEFAULT_THEME_ID)
}

fn is_builtin_extra(id: &str) -> bool {
    BUILTIN_EXTRA_THEME_IDS
        .iter()
        .any(|builtin| id.eq_ignore_ascii_case(builtin))
}

/// Instala o pacote em `<themes_root>/<id>/`, trocando uma versão anterior do mesmo tema só no fim.
pub fn install(package: &Path, themes_root: &Path) -> Result<InstalledTheme, Vec<Problem>> {
    install_with(package, themes_root, &Limits::default())
}

pub fn install_with(
    package: &Path,
    themes_root: &Path,
    limits: &Limits,
) -> Result<InstalledTheme, Vec<Problem>> {
    let mut pkg = package::Package::open(package, limits)?;
    let id = pkg.manifest.id.clone();
    if reserved(&id) || is_builtin_extra(&id) {
        return Err(vec![Problem::new(
            "package.reserved-id",
            tf("themes.package.reserved_id", &[("id", &id)]),
        )]);
    }
    let fail = |e: io::Error| {
        vec![Problem::new(
            "package.extract-failed",
            tf(
                "themes.package.extract_failed",
                &[("error", &e.to_string())],
            ),
        )]
    };
    fs::create_dir_all(themes_root).map_err(fail)?;
    // Mesmo volume: a troca final é um rename. A pasta temporária some sozinha se algo falhar.
    let staging = tempfile::Builder::new()
        .prefix(".import-")
        .tempdir_in(themes_root)
        .map_err(fail)?;
    pkg.extract_to(staging.path(), limits)
        .map_err(|p| vec![p])?;
    let dest = themes_root.join(&id);
    let old = themes_root.join(format!(".old-{id}"));
    let _ = fs::remove_dir_all(&old);
    let replacing = dest.exists();
    if replacing {
        fs::rename(&dest, &old).map_err(fail)?;
    }
    if let Err(e) = fs::rename(staging.path(), &dest) {
        if replacing {
            let _ = fs::rename(&old, &dest);
        }
        return Err(fail(e));
    }
    let _ = fs::remove_dir_all(&old);
    tracing::info!(id, dir = %dest.display(), "tema instalado");
    Ok(InstalledTheme {
        dir: dest,
        manifest: Ok(pkg.manifest),
    })
}

/// Valida e extrai um pacote para um diretório temporário novo, sem aplicar regras de instalação.
/// Usado por `theme check`, que precisa compilar até temas embutidos sem registrá-los no catálogo.
pub fn extract_for_check(package: &Path, output: &Path) -> Result<ThemeManifest, Vec<Problem>> {
    let limits = Limits::default();
    let mut pkg = package::Package::open(package, &limits)?;
    fs::create_dir(output).map_err(|e| {
        vec![Problem::new(
            "package.extract-failed",
            tf(
                "themes.package.extract_failed",
                &[("error", &e.to_string())],
            ),
        )]
    })?;
    if let Err(problem) = pkg.extract_to(output, &limits) {
        let _ = fs::remove_dir_all(output);
        return Err(vec![problem]);
    }
    Ok(pkg.manifest)
}

/// Remove um tema instalado (e a escolha, se era ele). O id seguro impede sair da pasta de temas.
pub fn uninstall(themes_root: &Path, id: &str) -> io::Result<()> {
    if !is_safe_id(id) || reserved(id) || is_builtin_extra(id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, id.to_owned()));
    }
    let dir = themes_root.join(id);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    let _ = overrides::reset_override(themes_root, Some(id));
    if selected(themes_root).is_some_and(|s| s.eq_ignore_ascii_case(id)) {
        select(themes_root, None)?;
    }
    remote::forget(themes_root, id);
    Ok(())
}

/// Tema escolhido (`themes/.selected`); `None` é o padrão.
pub fn selected(themes_root: &Path) -> Option<String> {
    let mut text = String::new();
    fs::File::open(themes_root.join(SELECTED_FILE))
        .ok()?
        .take(256)
        .read_to_string(&mut text)
        .ok()?;
    let id = text.trim();
    (is_safe_id(id) && !reserved(id)).then(|| id.to_owned())
}

pub fn select(themes_root: &Path, id: Option<&str>) -> io::Result<()> {
    let file = themes_root.join(SELECTED_FILE);
    match id.filter(|i| !reserved(i)) {
        Some(id) if is_safe_id(id) => {
            fs::create_dir_all(themes_root)?;
            fs::write(file, id)
        }
        Some(id) => Err(io::Error::new(io::ErrorKind::InvalidInput, id.to_owned())),
        None => match fs::remove_file(file) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        },
    }
}

/// Tema a carregar agora (`None` = padrão). Marca a carga em disco: se o processo cair antes de
/// [`end_load`], a próxima abertura conta uma falha, então um tema que derruba o Tako não vira loop.
pub fn begin_load(themes_root: &Path, now: SystemTime) -> Option<String> {
    let marker = themes_root.join(LOADING_FILE);
    if marker.exists() {
        record_fault(themes_root, now);
    }
    let Some(id) = selected(themes_root) else {
        end_load(themes_root);
        return None;
    };
    let _ = fs::write(marker, &id);
    Some(id)
}

/// Fim da carga (ou saída limpa, para proteger também o uso).
pub fn end_load(themes_root: &Path) {
    let _ = fs::remove_file(themes_root.join(LOADING_FILE));
}

/// Registra uma falha do tema escolhido. `true` quando estourou [`MAX_FAULTS`] em [`FAULT_WINDOW`]:
/// a escolha volta ao padrão.
pub fn record_fault(themes_root: &Path, now: SystemTime) -> bool {
    let millis = |t: SystemTime| {
        t.duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    };
    let now_ms = millis(now);
    let window = FAULT_WINDOW.as_millis() as u64;
    let file = themes_root.join(FAULTS_FILE);
    let mut faults: Vec<u64> = fs::read_to_string(&file)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .filter(|t: &u64| now_ms.saturating_sub(*t) < window && *t <= now_ms)
        .collect();
    faults.push(now_ms);
    if faults.len() >= MAX_FAULTS {
        tracing::warn!(theme = ?selected(themes_root), "tema falhou {MAX_FAULTS} vezes; voltando ao padrão");
        let _ = select(themes_root, None);
        let _ = fs::remove_file(&file);
        return true;
    }
    let text: Vec<String> = faults.iter().map(u64::to_string).collect();
    let _ = fs::create_dir_all(themes_root);
    let _ = fs::write(&file, text.join("\n"));
    false
}

/// Caminhos resolvidos e validados dos arquivos de áudio do tema em uso.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThemeAudioPaths {
    pub music: Option<PathBuf>,
    pub move_sound: Option<PathBuf>,
    pub accept_sound: Option<PathBuf>,
    pub back_sound: Option<PathBuf>,
}

/// Extrai e valida declarações de áudio do `tokens.toml`.
pub fn parse_tokens_audio(text: &str) -> Result<ThemeAudio, Vec<Problem>> {
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| {
        vec![Problem::new(
            "manifest.invalid",
            tf("themes.manifest.invalid", &[("error", e.message())]),
        )]
    })?;
    let mut problems = Vec::new();
    let audio = parse_audio_table(&table, &mut problems);
    if problems.is_empty() {
        Ok(audio)
    } else {
        Err(problems)
    }
}

/// Resolve os arquivos de áudio do tema a partir da pasta instalada.
/// Lê `theme.toml` e `tokens.toml` (que pode sobrescrever), validando que os arquivos existem
/// e não saem da pasta do tema.
pub fn resolve_theme_audio(theme_dir: &Path) -> ThemeAudioPaths {
    let manifest_audio = fs::read_to_string(theme_dir.join(MANIFEST_FILE))
        .ok()
        .and_then(|text| ThemeManifest::parse(&text).ok())
        .map(|manifest| manifest.audio)
        .unwrap_or_default();
    let tokens_audio = fs::read_to_string(theme_dir.join("tokens.toml"))
        .ok()
        .and_then(|text| parse_tokens_audio(&text).ok())
        .unwrap_or_default();
    let resolve_file = |rel_opt: Option<String>, is_music: bool| -> Option<PathBuf> {
        let rel = rel_opt?;
        let clean = validate_audio_path(&rel).ok()?;
        let full = theme_dir.join(clean);
        if !full.is_file() {
            return None;
        }
        let canon_dir = theme_dir.canonicalize().ok()?;
        let canon_full = full.canonicalize().ok()?;
        if !canon_full.starts_with(&canon_dir) {
            return None;
        }
        if is_music {
            let size = full.metadata().ok()?.len();
            if size > MAX_THEME_MUSIC_BYTES || size == 0 {
                return None;
            }
        } else {
            let dur = wav_duration(&full)?;
            if dur > MAX_NAV_SOUND_DURATION_SECS || dur <= 0.0 {
                return None;
            }
        }
        Some(full)
    };

    let resolve_merged = |token_rel: Option<String>,
                          manifest_rel: Option<String>,
                          is_music: bool|
     -> Option<PathBuf> {
        token_rel
            .and_then(|t| resolve_file(Some(t), is_music))
            .or_else(|| manifest_rel.and_then(|m| resolve_file(Some(m), is_music)))
    };

    ThemeAudioPaths {
        music: resolve_merged(tokens_audio.music, manifest_audio.music, true),
        move_sound: resolve_merged(tokens_audio.move_sound, manifest_audio.move_sound, false),
        accept_sound: resolve_merged(
            tokens_audio.accept_sound,
            manifest_audio.accept_sound,
            false,
        ),
        back_sound: resolve_merged(tokens_audio.back_sound, manifest_audio.back_sound, false),
    }
}

#[cfg(test)]
mod tests;
