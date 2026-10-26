//! Port de `Metadata/Art/*`: imagens que o usuário pode escolher (ADR-0023), artes fixadas, ids
//! colados por fonte, imagens do PC e tamanhos para a tela.

use super::{STEAM_ART_SCHEME, sources};
use crate::i18n::{self, Lang, t, tf};
use crate::library::{ArtKind, Game, media};
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

/// Fonte "Arquivo do PC" do seletor.
pub const FILE_SOURCE: &str = "File";

/// Ordem das abas de fonte no seletor.
pub const SOURCE_ORDER: [&str; 5] = [
    sources::STEAM,
    sources::IGDB,
    sources::SCREENSCRAPER,
    sources::STEAMGRIDDB,
    FILE_SOURCE,
];

/// "Capa", "Fundo", "Logo", "Ícone".
pub fn kind_label(kind: ArtKind) -> String {
    t(&format!("metadata.art.kind.{}", kind.key()))
}

/// Nome da fonte para a tela (só "Arquivo do PC" é traduzido).
pub fn source_label(source: &str) -> String {
    if source == FILE_SOURCE {
        t("metadata.art.source_file")
    } else {
        source.to_owned()
    }
}

/// Tipos que cada fonte costuma ter (a aba some quando a fonte não tem o tipo).
pub fn offers(source: &str, kind: ArtKind) -> bool {
    match source {
        sources::IGDB => matches!(kind, ArtKind::Cover | ArtKind::Hero),
        sources::SCREENSCRAPER => matches!(kind, ArtKind::Cover | ArtKind::Hero | ArtKind::Logo),
        _ => true,
    }
}

/// Uma imagem que o usuário pode escolher para o jogo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArtCandidate {
    pub source: String,
    pub kind: Option<ArtKind>,
    pub url: String,
    /// Imagem menor para a grade; `None` = a própria `url`.
    pub thumbnail: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// "Caixa 2D · EUA", "Arte oficial".
    pub label: Option<String>,
    /// Quem fez a imagem (SteamGridDB pede o crédito).
    pub author: Option<String>,
    pub style: Option<String>,
    pub is_animated: bool,
    pub is_nsfw: bool,
    pub is_humor: bool,
    pub score: i64,
}

impl ArtCandidate {
    pub fn new(source: &str, kind: ArtKind, url: impl Into<String>) -> Self {
        ArtCandidate {
            source: source.into(),
            kind: Some(kind),
            url: url.into(),
            ..Default::default()
        }
    }

    pub fn preview(&self) -> &str {
        self.thumbnail.as_deref().unwrap_or(&self.url)
    }

    /// "600 × 900".
    pub fn size_text(&self) -> Option<String> {
        Some(format!("{} × {}", self.width?, self.height?))
    }

    /// "Caixa 2D · EUA · 600 × 900 · por Fulano".
    pub fn caption(&self) -> String {
        let author = self
            .author
            .as_ref()
            .map(|a| tf("metadata.art.by", &[("author", a)]));
        [self.label.clone(), self.size_text(), author]
            .into_iter()
            .flatten()
            .filter(|p| !p.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// Imagens da Steam para o seletor: `steam-art://`, resolvidos ao baixar.
pub fn steam_candidates(app_id: u32) -> Vec<ArtCandidate> {
    let art = |kind: &str| format!("{STEAM_ART_SCHEME}://{app_id}/{kind}");
    let c = |kind, key: &str, label: &str, size: Option<(u32, u32)>| ArtCandidate {
        label: Some(t(label)),
        width: size.map(|s| s.0),
        height: size.map(|s| s.1),
        ..ArtCandidate::new(sources::STEAM, kind, art(key))
    };
    vec![
        c(
            ArtKind::Cover,
            "cover",
            "metadata.art.steam_cover",
            Some((600, 900)),
        ),
        c(
            ArtKind::Hero,
            "hero",
            "metadata.art.steam_hero",
            Some((3840, 1240)),
        ),
        c(ArtKind::Logo, "logo", "metadata.art.steam_logo", None),
        c(ArtKind::Icon, "icon", "metadata.art.steam_icon", None),
    ]
}

/// Artes fixadas pelo usuário: aplicar, trocar e voltar à automática.
pub mod pins {
    use super::*;

    /// Novo conjunto com a imagem do tipo fixada (ou solta, com `url` `None`).
    pub fn with(
        current: &BTreeMap<ArtKind, String>,
        kind: ArtKind,
        url: Option<&str>,
    ) -> BTreeMap<ArtKind, String> {
        let mut next = current.clone();
        match url {
            Some(u) => next.insert(kind, u.to_owned()),
            None => next.remove(&kind),
        };
        next
    }

    /// Endereço automático do tipo (o que aparece sem escolha do usuário).
    pub fn automatic(game: &Game, kind: ArtKind) -> Option<&str> {
        match kind {
            ArtKind::Cover => game.cover_uri.as_deref(),
            ArtKind::Hero => game.hero_uri.as_deref(),
            ArtKind::Logo => game.media.get(media::LOGO).map(String::as_str),
            ArtKind::Icon => game.media.get(media::ICON).map(String::as_str),
        }
    }

    /// O fixado ou o automático.
    pub fn shown(game: &Game, kind: ArtKind) -> Option<&str> {
        game.pinned_art
            .get(&kind)
            .map(String::as_str)
            .or_else(|| automatic(game, kind))
    }
}

/// Tamanhos de arquivo para a tela ("12,4 MB" em português, "12.4 MB" em inglês).
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    let decimal = |v: f64, unit: &str| {
        let s = format!("{v:.1} {unit}");
        if i18n::lang() == Lang::PtBr {
            s.replace('.', ",")
        } else {
            s
        }
    };
    match bytes {
        0 => t("metadata.size.empty"),
        b if b < KB => format!("{b} B"),
        b if b < MB => format!("{} KB", (b / KB).max(1)),
        b if b < GB => decimal(b as f64 / MB as f64, "MB"),
        b => decimal(b as f64 / GB as f64, "GB"),
    }
}

/// Aba "Arquivo do PC": as imagens mais recentes das pastas comuns, até um nível de subpasta.
pub mod local_images {
    use super::*;

    pub const EXTENSIONS: [&str; 6] = ["jpg", "jpeg", "png", "webp", "bmp", "gif"];

    pub fn find(folders: &[PathBuf], max: usize) -> Vec<PathBuf> {
        let mut files: Vec<(SystemTime, PathBuf)> = Vec::new();
        let mut seen = Vec::new();
        for folder in folders.iter().filter(|f| f.is_dir()) {
            if seen.contains(folder) {
                continue;
            }
            seen.push(folder.clone());
            images(folder, &mut files);
            // Pasta sem permissão: segue com as outras.
            let Ok(entries) = std::fs::read_dir(folder) else {
                continue;
            };
            let subs = entries
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                .take(40);
            for sub in subs {
                images(&sub.path(), &mut files);
            }
        }
        files.sort_by_key(|f| std::cmp::Reverse(f.0));
        files.dedup_by(|a, b| a.1 == b.1);
        files.into_iter().take(max).map(|(_, p)| p).collect()
    }

    fn images(folder: &Path, out: &mut Vec<(SystemTime, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(folder) else {
            return;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            let ext = p
                .extension()
                .map(|x| x.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_file() && meta.len() > 0 && EXTENSIONS.contains(&ext.as_str()) {
                out.push((meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), p));
            }
        }
    }
}

/// O que o usuário digitou ou colou para fixar o jogo numa fonte ("Fontes e IDs").
#[derive(Debug, Clone, PartialEq)]
pub struct SourceIdParse {
    pub id: Option<String>,
    /// Nome na URL da IGDB (`igdb.com/games/hades`), resolvido por consulta.
    pub slug: Option<String>,
    pub problem: Option<String>,
}

impl SourceIdParse {
    pub fn is_valid(&self) -> bool {
        self.problem.is_none()
    }
}

/// Exemplo mostrado no teclado virtual de cada fonte.
pub fn source_id_example(source: &str) -> String {
    let key = match source {
        sources::STEAM => "steam",
        sources::IGDB => "igdb",
        sources::SCREENSCRAPER => "screenscraper",
        sources::STEAMGRIDDB => "steamgriddb",
        _ => "other",
    };
    t(&format!("metadata.source_id.example.{key}"))
}

pub fn parse_source_id(source: &str, text: &str) -> SourceIdParse {
    static PATTERNS: OnceLock<Option<[Regex; 5]>> = OnceLock::new();
    let value = text.trim();
    let ok = |id: Option<String>, slug: Option<String>| SourceIdParse {
        id,
        slug,
        problem: None,
    };
    let bad = || SourceIdParse {
        id: None,
        slug: None,
        problem: Some(tf(
            "metadata.source_id.unknown",
            &[("example", &source_id_example(source))],
        )),
    };
    if value.is_empty() {
        return SourceIdParse {
            id: None,
            slug: None,
            problem: Some(t("metadata.source_id.empty")),
        };
    }
    let Some([digits, steam, igdb, ss, sgdb]) = PATTERNS
        .get_or_init(|| {
            Some([
                Regex::new(r"^\d{1,12}$").ok()?,
                Regex::new(r"(?i)(?:store\.steampowered\.com|steamcommunity\.com|steamdb\.info|s\.team)/app/(?P<id>\d{1,12})").ok()?,
                Regex::new(r"(?i)igdb\.com/games/(?P<slug>[a-z0-9][a-z0-9\-\.]*)").ok()?,
                Regex::new(r"(?i)screenscraper\.fr/.*[?&](?:gameid|jeuid)=(?P<id>\d{1,12})").ok()?,
                Regex::new(r"(?i)steamgriddb\.com/(?:game|grid|hero|logo|icon)s?/(?P<id>\d{1,12})").ok()?,
            ])
        })
        .as_ref()
    else {
        return bad();
    };
    if digits.is_match(value) {
        let id = value.trim_start_matches('0');
        return if id.is_empty() {
            bad()
        } else {
            ok(Some(id.into()), None)
        };
    }
    let re = match source {
        sources::STEAM => steam,
        sources::IGDB => igdb,
        sources::SCREENSCRAPER => ss,
        sources::STEAMGRIDDB => sgdb,
        _ => return bad(),
    };
    let Some(caps) = re.captures(value) else {
        return bad();
    };
    if let Some(id) = caps.name("id") {
        return ok(Some(id.as_str().into()), None);
    }
    match caps.name("slug") {
        Some(s) => ok(None, Some(s.as_str().to_lowercase())),
        None => bad(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::GameStore;

    #[test]
    fn pasted_ids_and_page_addresses_become_ids() {
        for (source, text, id, slug) in [
            ("Steam", "1145360", Some("1145360"), None),
            (
                "Steam",
                "https://store.steampowered.com/app/1145360/Hades/",
                Some("1145360"),
                None,
            ),
            ("Steam", "steamdb.info/app/570/", Some("570"), None),
            ("IGDB", "113112", Some("113112"), None),
            (
                "IGDB",
                "https://www.igdb.com/games/Hades",
                None,
                Some("hades"),
            ),
            (
                "ScreenScraper",
                "https://www.screenscraper.fr/gameinfos.php?plateforme=4&gameid=3",
                Some("3"),
                None,
            ),
            (
                "SteamGridDB",
                "https://www.steamgriddb.com/game/5249/grids",
                Some("5249"),
                None,
            ),
            ("SteamGridDB", " 005249 ", Some("5249"), None),
        ] {
            let p = parse_source_id(source, text);
            assert!(p.is_valid(), "{text}: {:?}", p.problem);
            assert_eq!(p.id.as_deref(), id);
            assert_eq!(p.slug.as_deref(), slug);
        }
    }

    #[test]
    fn unknown_text_explains_with_an_example() {
        for (source, text) in [
            ("Steam", ""),
            ("Steam", "igdb.com/games/hades"),
            ("SteamGridDB", "store.steampowered.com/app/570"),
            ("IGDB", "0"),
        ] {
            let p = parse_source_id(source, text);
            assert!(!p.is_valid());
            assert!(p.problem.is_some());
        }
    }

    #[test]
    fn pinned_art_is_shown_per_kind_and_survives_metadata_updates() {
        let mut g = Game {
            id: "epic:1".into(),
            store: Some(GameStore::Epic),
            ..Default::default()
        };
        let meta = |c: &str| crate::metadata::GameMetadata {
            source_ids: [("IGDB".to_owned(), "1".to_owned())].into(),
            cover_url: Some(c.into()),
            ..crate::metadata::GameMetadata::new("IGDB")
        };
        meta("https://images.igdb.com/a.jpg").apply_to(&mut g);
        let chosen = "https://cdn2.steamgriddb.com/grid/a.png";
        g.pinned_art = pins::with(&g.pinned_art, ArtKind::Cover, Some(chosen));
        meta("https://images.igdb.com/b.jpg").apply_to(&mut g);
        assert_eq!(g.shown_cover(), Some(chosen));
        assert_eq!(
            pins::automatic(&g, ArtKind::Cover),
            Some("https://images.igdb.com/b.jpg")
        );
        assert_eq!(g.shown_hero(), g.hero_uri.as_deref());
        g.pinned_art = pins::with(&g.pinned_art, ArtKind::Cover, None);
        assert_eq!(
            pins::shown(&g, ArtKind::Cover),
            Some("https://images.igdb.com/b.jpg")
        );
    }

    #[test]
    fn steam_lists_every_kind() {
        let s = steam_candidates(570);
        let kinds: Vec<_> = s.iter().filter_map(|c| c.kind).collect();
        assert_eq!(kinds, ArtKind::ALL);
        assert_eq!(s[2].url, "steam-art://570/logo");
    }

    #[test]
    fn local_images_are_the_newest_files_of_the_common_folders() {
        let root = tempfile::tempdir().unwrap();
        let pictures = root.path().join("Imagens");
        let sub = pictures.join("Capas");
        std::fs::create_dir_all(&sub).unwrap();
        let old = pictures.join("velha.png");
        std::fs::write(&old, [1]).unwrap();
        let f = std::fs::File::options().write(true).open(&old).unwrap();
        f.set_modified(SystemTime::now() - std::time::Duration::from_secs(172_800))
            .unwrap();
        std::fs::write(sub.join("nova.jpg"), [1]).unwrap();
        std::fs::write(pictures.join("notas.txt"), [1]).unwrap();
        let found = local_images::find(&[pictures.clone(), root.path().join("nao-existe")], 60);
        let names: Vec<_> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["nova.jpg", "velha.png"]);
    }

    #[test]
    fn sizes_are_written_for_people() {
        let sep = if i18n::lang() == Lang::PtBr { "," } else { "." };
        assert_eq!(format_bytes(0), t("metadata.size.empty"));
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(12 * 1024), "12 KB");
        assert_eq!(
            format_bytes((12.4 * 1024.0 * 1024.0) as u64),
            format!("12{sep}4 MB")
        );
        assert_eq!(format_bytes(3 * 512 * 1024 * 1024), format!("1{sep}5 GB"));
    }

    #[test]
    fn caption_joins_label_size_and_author() {
        let c = ArtCandidate {
            label: Some("X".into()),
            width: Some(600),
            height: Some(900),
            author: Some("Ana".into()),
            ..ArtCandidate::new("SteamGridDB", ArtKind::Cover, "https://a")
        };
        assert_eq!(
            c.caption(),
            format!(
                "X · 600 × 900 · {}",
                tf("metadata.art.by", &[("author", "Ana")])
            )
        );
    }
}
