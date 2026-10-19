//! Pacote de tema (`.takotheme` zip ou pasta): inventário, limites e caminhos seguros (ideias de
//! `ThemePackage.cs`). O pacote é dado não confiável: nada é gravado antes de tudo ser validado, e os
//! tamanhos são contados de novo na extração (o tamanho declarado no zip pode mentir).

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use tako_core::i18n::{t, tf};
use zip::ZipArchive;

use crate::Problem;
use crate::manifest::{MANIFEST_FILE, MAX_MANIFEST_BYTES, ThemeManifest};

pub const EXTENSION: &str = "takotheme";
pub const MAX_PATH_LEN: usize = 240;
pub const ALLOWED_EXTENSIONS: [&str; 12] = [
    "slint", "toml", "png", "jpg", "jpeg", "svg", "webp", "ttf", "otf", "txt", "md", "wav",
];

/// Limites de um pacote; os padrões cobrem com folga temas reais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Soma dos tamanhos descompactados.
    pub max_total_bytes: u64,
    pub max_file_bytes: u64,
    /// Entradas (arquivos e pastas).
    pub max_entries: usize,
    /// Tamanho do `.takotheme` em disco.
    pub max_package_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_total_bytes: 200 * 1024 * 1024,
            max_file_bytes: 64 * 1024 * 1024,
            max_entries: 5000,
            max_package_bytes: 200 * 1024 * 1024,
        }
    }
}

enum Source {
    Zip(ZipArchive<File>),
    Dir,
}

/// Arquivo do tema, já com o caminho relativo à raiz do tema.
struct Entry {
    path: String,
    size: u64,
    /// Índice no zip ou caminho real na pasta.
    zip_index: usize,
    file: PathBuf,
}

/// Pacote aberto e validado (tudo menos a compilação do `.slint`).
pub(crate) struct Package {
    source: Source,
    files: Vec<Entry>,
    pub manifest: ThemeManifest,
}

/// Valida um `.takotheme` ou uma pasta de tema sem gravar nada. Não compila o `.slint`
/// (isso é do `tako theme check`, em `tako_ui::theme::check`).
pub fn validate_package(path: &Path) -> Result<ThemeManifest, Vec<Problem>> {
    validate_package_with(path, &Limits::default())
}

pub fn validate_package_with(path: &Path, limits: &Limits) -> Result<ThemeManifest, Vec<Problem>> {
    Package::open(path, limits).map(|p| p.manifest)
}

fn one(code: &'static str, message: String) -> Vec<Problem> {
    vec![Problem::new(code, message)]
}

fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

impl Package {
    pub(crate) fn open(path: &Path, limits: &Limits) -> Result<Package, Vec<Problem>> {
        let shown = path.display().to_string();
        let meta = fs::metadata(path).map_err(|_| {
            one(
                "package.not-found",
                tf("themes.package.not_found", &[("path", &shown)]),
            )
        })?;
        let (source, raw) = if meta.is_dir() {
            let raw = list_dir(path, limits)?;
            (Source::Dir, raw)
        } else {
            if meta.len() > limits.max_package_bytes {
                return Err(one(
                    "package.too-large",
                    tf(
                        "themes.package.too_large",
                        &[("max", &mb(limits.max_package_bytes))],
                    ),
                ));
            }
            let file = File::open(path).map_err(|e| io_problem(&e))?;
            let mut zip = ZipArchive::new(file)
                .map_err(|_| one("package.not-zip", t("themes.package.not_zip")))?;
            let raw = list_zip(&mut zip, limits)?;
            (Source::Zip(zip), raw)
        };
        let files = inventory(raw, limits)?;

        // Manifesto na raiz ou dentro de uma única pasta de topo (zip feito da pasta do tema).
        let Some(prefix) = find_root(&files) else {
            return Err(one(
                "package.missing-manifest",
                t("themes.package.missing_manifest"),
            ));
        };
        let files: Vec<Entry> = files
            .into_iter()
            .map(|mut e| {
                e.path = e.path[prefix.len()..].to_owned();
                e
            })
            .collect();
        let mut source = source;
        let index = files
            .iter()
            .position(|e| e.path == MANIFEST_FILE)
            .ok_or_else(|| {
                one(
                    "package.missing-manifest",
                    t("themes.package.missing_manifest"),
                )
            })?;
        let too_large = || {
            one(
                "manifest.too-large",
                tf(
                    "themes.manifest.too_large",
                    &[("max", &(MAX_MANIFEST_BYTES / 1024).to_string())],
                ),
            )
        };
        if files[index].size > MAX_MANIFEST_BYTES {
            return Err(too_large());
        }
        let mut reader = open_entry(&mut source, &files[index]).map_err(|e| io_problem(&e))?;
        let bytes = read_bounded(&mut reader, MAX_MANIFEST_BYTES)
            .map_err(|e| io_problem(&e))?
            .ok_or_else(too_large)?;
        drop(reader);
        let text = String::from_utf8(bytes).map_err(|_| {
            one(
                "manifest.invalid",
                tf("themes.manifest.invalid", &[("error", "UTF-8")]),
            )
        })?;
        let manifest = ThemeManifest::parse(text.trim_start_matches('\u{feff}'))?;
        if !files.iter().any(|e| e.path == manifest.entry) {
            return Err(one(
                "package.missing-entry",
                tf(
                    "themes.package.missing_entry",
                    &[("entry", &manifest.entry)],
                ),
            ));
        }
        for audio_path in [
            &manifest.audio.music,
            &manifest.audio.move_sound,
            &manifest.audio.accept_sound,
            &manifest.audio.back_sound,
        ]
        .into_iter()
        .flatten()
        {
            if !files.iter().any(|e| e.path == *audio_path) {
                return Err(one(
                    "package.missing-audio",
                    tf("themes.package.missing_audio", &[("path", audio_path)]),
                ));
            }
        }
        if let Some(tokens_entry) = files.iter().find(|e| e.path == "tokens.toml") {
            let mut reader = open_entry(&mut source, tokens_entry).map_err(|e| io_problem(&e))?;
            if let Some(bytes) = read_bounded(&mut reader, MAX_MANIFEST_BYTES).map_err(|e| io_problem(&e))? {
                if let Ok(text) = String::from_utf8(bytes) {
                    if let Ok(tokens_audio) = crate::parse_tokens_audio(&text) {
                        for audio_path in [
                            &tokens_audio.music,
                            &tokens_audio.move_sound,
                            &tokens_audio.accept_sound,
                            &tokens_audio.back_sound,
                        ]
                        .into_iter()
                        .flatten()
                        {
                            if !files.iter().any(|e| e.path == *audio_path) {
                                return Err(one(
                                    "package.missing-audio",
                                    tf("themes.package.missing_audio", &[("path", audio_path)]),
                                ));
                            }
                        }
                    }
                }
            }
        }
        Ok(Package {
            source,
            files,
            manifest,
        })
    }

    /// Extrai para `dir` (que já existe), contando os bytes reais. Devolve (arquivos, bytes).
    pub(crate) fn extract_to(
        &mut self,
        dir: &Path,
        limits: &Limits,
    ) -> Result<(usize, u64), Problem> {
        let mut total = 0u64;
        for entry in &self.files {
            let rel = &entry.path;
            let target = rel.split('/').fold(dir.to_path_buf(), |p, s| p.join(s));
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| extract_problem(&e))?;
            }
            let limit = limits
                .max_file_bytes
                .min(limits.max_total_bytes.saturating_sub(total));
            let mut reader =
                open_entry(&mut self.source, entry).map_err(|e| extract_problem(&e))?;
            let mut out = File::create_new(&target).map_err(|e| extract_problem(&e))?;
            let written = io::copy(&mut (&mut reader).take(limit + 1), &mut out)
                .map_err(|e| extract_problem(&e))?;
            if written > limit {
                return Err(Problem::new(
                    "package.too-large",
                    tf("themes.package.grew", &[("entry", rel)]),
                ));
            }
            total += written;
        }
        Ok((self.files.len(), total))
    }
}

fn open_entry<'a>(source: &'a mut Source, e: &Entry) -> io::Result<Box<dyn Read + 'a>> {
    match source {
        Source::Zip(zip) => Ok(Box::new(
            zip.by_index(e.zip_index).map_err(io::Error::other)?,
        )),
        Source::Dir => Ok(Box::new(File::open(&e.file)?)),
    }
}

fn io_problem(e: &io::Error) -> Vec<Problem> {
    one(
        "package.io",
        tf("themes.package.io", &[("error", &e.to_string())]),
    )
}

fn extract_problem(e: &io::Error) -> Problem {
    Problem::new(
        "package.extract-failed",
        tf(
            "themes.package.extract_failed",
            &[("error", &e.to_string())],
        ),
    )
}

/// Lê até `max` bytes; `None` se passar disso.
fn read_bounded(reader: &mut dyn Read, max: u64) -> io::Result<Option<Vec<u8>>> {
    let mut buf = Vec::new();
    reader.take(max + 1).read_to_end(&mut buf)?;
    Ok((buf.len() as u64 <= max).then_some(buf))
}

/// Entrada crua do pacote: nome como veio, tamanho declarado, onde ler.
struct Raw {
    name: String,
    size: u64,
    zip_index: usize,
    file: PathBuf,
}

fn too_many(limits: &Limits) -> Vec<Problem> {
    one(
        "package.too-many-entries",
        tf(
            "themes.package.too_many_entries",
            &[("max", &limits.max_entries.to_string())],
        ),
    )
}

fn list_zip(zip: &mut ZipArchive<File>, limits: &Limits) -> Result<Vec<Raw>, Vec<Problem>> {
    if zip.len() > limits.max_entries {
        return Err(too_many(limits));
    }
    let corrupt = |e: zip::result::ZipError| {
        one(
            "package.corrupt",
            tf("themes.package.corrupt", &[("error", &e.to_string())]),
        )
    };
    let mut raw = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let f = zip.by_index_raw(i).map_err(corrupt)?;
        let name = f.name().map_err(corrupt)?.into_owned();
        if f.is_symlink() {
            return Err(one(
                "package.symlink",
                tf("themes.package.symlink", &[("entry", &name)]),
            ));
        }
        raw.push(Raw {
            name,
            size: f.size(),
            zip_index: i,
            file: PathBuf::new(),
        });
    }
    Ok(raw)
}

/// Pasta de tema: sem seguir links (um link sairia da pasta do tema).
fn list_dir(root: &Path, limits: &Limits) -> Result<Vec<Raw>, Vec<Problem>> {
    let mut raw = Vec::new();
    let mut stack = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = stack.pop() {
        for item in fs::read_dir(&dir).map_err(|e| io_problem(&e))? {
            let item = item.map_err(|e| io_problem(&e))?;
            let name = item.file_name().to_string_lossy().into_owned();
            let rel = format!("{prefix}{name}");
            let kind = item.file_type().map_err(|e| io_problem(&e))?;
            if raw.len() >= limits.max_entries {
                return Err(too_many(limits));
            }
            if kind.is_symlink() {
                return Err(one(
                    "package.symlink",
                    tf("themes.package.symlink", &[("entry", &rel)]),
                ));
            } else if kind.is_dir() {
                raw.push(Raw {
                    name: format!("{rel}/"),
                    size: 0,
                    zip_index: 0,
                    file: item.path(),
                });
                stack.push((item.path(), format!("{rel}/")));
            } else {
                let size = item.metadata().map_err(|e| io_problem(&e))?.len();
                raw.push(Raw {
                    name: rel,
                    size,
                    zip_index: 0,
                    file: item.path(),
                });
            }
        }
    }
    Ok(raw)
}

/// Caminhos, duplicatas, tamanhos declarados e extensões; junta todos os problemas.
fn inventory(raw: Vec<Raw>, limits: &Limits) -> Result<Vec<Entry>, Vec<Problem>> {
    let mut problems = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut files = Vec::new();
    let mut declared = 0u64;
    for r in raw {
        let path = match normalize_entry(&r.name) {
            Ok(Some(p)) => p,
            Ok(None) => continue, // pasta
            Err(reason) => {
                problems.push(Problem::new(
                    "package.unsafe-path",
                    tf(
                        "themes.package.unsafe_path",
                        &[("entry", &r.name), ("reason", &t(reason))],
                    ),
                ));
                continue;
            }
        };
        if !seen.insert(path.to_lowercase()) {
            problems.push(Problem::new(
                "package.duplicate-entry",
                tf("themes.package.duplicate_entry", &[("entry", &path)]),
            ));
        }
        if r.size > limits.max_file_bytes {
            problems.push(Problem::new(
                "package.file-too-large",
                tf(
                    "themes.package.file_too_large",
                    &[("entry", &path), ("max", &mb(limits.max_file_bytes))],
                ),
            ));
        }
        declared = declared.saturating_add(r.size);
        let ext = path
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .filter(|e| !e.contains('/'));
        if !ext.is_some_and(|e| ALLOWED_EXTENSIONS.contains(&e.as_str())) {
            problems.push(Problem::new(
                "package.blocked-extension",
                tf("themes.package.blocked_extension", &[("entry", &path)]),
            ));
        }
        files.push(Entry {
            path,
            size: r.size,
            zip_index: r.zip_index,
            file: r.file,
        });
    }
    if declared > limits.max_total_bytes {
        problems.push(Problem::new(
            "package.too-large",
            tf(
                "themes.package.too_large",
                &[("max", &mb(limits.max_total_bytes))],
            ),
        ));
    }
    if problems.is_empty() {
        Ok(files)
    } else {
        Err(problems)
    }
}

/// Prefixo ("" ou "Pasta/") onde está o `theme.toml`.
fn find_root(files: &[Entry]) -> Option<String> {
    if files.iter().any(|e| e.path == MANIFEST_FILE) {
        return Some(String::new());
    }
    let top = files.first()?.path.split_once('/')?.0;
    let prefix = format!("{top}/");
    (files.iter().all(|e| e.path.starts_with(&prefix))
        && files
            .iter()
            .any(|e| e.path == format!("{prefix}{MANIFEST_FILE}")))
    .then_some(prefix)
}

/// Port de `ZipEntryPath.TryNormalize`: "a/b/c", `Ok(None)` para pastas, ou a chave do motivo da recusa
/// (absoluto, unidade, `..`, caracteres inválidos, nomes de dispositivo, final em ponto ou espaço).
pub fn normalize_entry(name: &str) -> Result<Option<String>, &'static str> {
    if name.is_empty() {
        return Err("themes.path.empty");
    }
    let path = name.replace('\\', "/");
    if path.starts_with('/') {
        return Err("themes.path.absolute");
    }
    if path.chars().count() > MAX_PATH_LEN {
        return Err("themes.path.too_long");
    }
    let mut segments = Vec::new();
    for s in path.split('/') {
        if s.is_empty() || s == "." {
            continue;
        }
        if s == ".." {
            return Err("themes.path.parent");
        }
        if s.chars().any(|c| c.is_control() || "<>:\"|?*".contains(c)) {
            return Err(if s.contains(':') {
                "themes.path.drive"
            } else {
                "themes.path.invalid_char"
            });
        }
        if s.ends_with('.') || s.ends_with(' ') {
            return Err("themes.path.trailing_dot");
        }
        if crate::manifest::is_device_name(s.split('.').next().unwrap_or(s)) {
            return Err("themes.path.device");
        }
        segments.push(s);
    }
    if segments.is_empty() || path.ends_with('/') {
        return Ok(None);
    }
    Ok(Some(segments.join("/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_safe_entry_names() {
        assert_eq!(
            normalize_entry("ui/main.slint"),
            Ok(Some("ui/main.slint".into()))
        );
        assert_eq!(
            normalize_entry("ui\\main.slint"),
            Ok(Some("ui/main.slint".into()))
        );
        assert_eq!(
            normalize_entry("./ui//main.slint"),
            Ok(Some("ui/main.slint".into()))
        );
        assert_eq!(normalize_entry("ui/"), Ok(None));
    }

    #[test]
    fn refuses_dangerous_names() {
        for (name, reason) in [
            ("../evil.slint", "themes.path.parent"),
            ("ui/../../evil.slint", "themes.path.parent"),
            ("..\\evil.slint", "themes.path.parent"),
            ("/etc/evil.slint", "themes.path.absolute"),
            ("\\\\servidor\\share\\evil.slint", "themes.path.absolute"),
            ("C:/Windows/evil.slint", "themes.path.drive"),
            ("C:evil.slint", "themes.path.drive"),
            ("theme.toml:stream", "themes.path.drive"),
            ("ui/CON.slint", "themes.path.device"),
            ("ui/nome. ", "themes.path.trailing_dot"),
            ("ui/a\u{1}.png", "themes.path.invalid_char"),
            ("", "themes.path.empty"),
        ] {
            assert_eq!(normalize_entry(name), Err(reason), "{name:?}");
        }
        assert_eq!(
            normalize_entry(&"a/".repeat(121)),
            Err("themes.path.too_long")
        );
    }

    #[test]
    fn validates_declared_audio_files_exist_in_package() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(
            dir.join("theme.toml"),
            "id = \"meu.tema\"\nname = \"Tema\"\nversion = \"1.0\"\nauthor = \"Eu\"\napi = 1\nentry = \"main.slint\"\n[audio]\nmusic = \"bgm.wav\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("main.slint"), "").unwrap();
        // Missing bgm.wav
        let err = validate_package(dir).unwrap_err();
        assert_eq!(err[0].code, "package.missing-audio");

        // Present bgm.wav
        std::fs::write(dir.join("bgm.wav"), b"RIFF....WAVE").unwrap();
        let manifest = validate_package(dir).unwrap();
        assert_eq!(manifest.audio.music.as_deref(), Some("bgm.wav"));
    }

    #[test]
    fn validates_declared_audio_files_in_tokens_exist_in_package() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::write(
            dir.join("theme.toml"),
            "id = \"meu.tema\"\nname = \"Tema\"\nversion = \"1.0\"\nauthor = \"Eu\"\napi = 1\nentry = \"main.slint\"\n",
        )
        .unwrap();
        std::fs::write(dir.join("main.slint"), "").unwrap();
        std::fs::write(
            dir.join("tokens.toml"),
            "[audio]\naccept = \"click.wav\"\n",
        )
        .unwrap();

        // Missing click.wav declared in tokens.toml
        let err = validate_package(dir).unwrap_err();
        assert_eq!(err[0].code, "package.missing-audio");

        // Present click.wav
        std::fs::write(dir.join("click.wav"), b"RIFF....WAVE").unwrap();
        let manifest = validate_package(dir).unwrap();
        assert_eq!(manifest.id, "meu.tema");
    }
}
