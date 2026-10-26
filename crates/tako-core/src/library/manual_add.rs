//! Regras de "Adicionar jogo" (port de `ShellViewModel.Library.AddGame`): que tipo de arquivo é, qual o título e se
//! já está na biblioteca. Quem ler o arquivo (`.desktop`, `.app`, versão do `.exe`) e gravar fica com a interface.

use super::Game;
use crate::metadata::screenscraper_parse::looks_like_rom;
use std::path::Path;

/// O que o arquivo escolhido é, pela extensão.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddKind {
    /// Atalho `.desktop` (Linux): nome e comando saem do próprio arquivo.
    Desktop,
    /// Aplicativo `.app` (macOS).
    MacApp,
    /// ROM ou imagem de disco: pergunta o sistema (ADR-0022).
    Rom,
    /// O resto vira jogo de PC.
    Program,
}

pub fn kind_of(path: &Path) -> AddKind {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "desktop" => AddKind::Desktop,
        "app" => AddKind::MacApp,
        _ if looks_like_rom(&path.to_string_lossy()) => AddKind::Rom,
        _ => AddKind::Program,
    }
}

/// O jogo da biblioteca que já abre este arquivo (sem diferenciar maiúsculas, como o Windows).
pub fn find_by_target<'a>(games: &'a [Game], target: &str) -> Option<&'a Game> {
    games.iter().find(|g| {
        g.launch_target
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case(target))
    })
}

/// Título do jogo de PC. `product_names` = ProductName e FileDescription do `.exe` (só no Windows): o primeiro que
/// servir; senão o nome do arquivo sem extensão nem separadores ("hollow_knight.exe" → "hollow knight").
pub fn guess_title(path: &Path, product_names: [Option<String>; 2]) -> String {
    if let Some(t) = product_title(product_names) {
        return t;
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let t = stem.replace(['_', '-'], " ").trim().to_owned();
    if t.is_empty() { stem } else { t }
}

/// Primeiro de ProductName e FileDescription que sirva de título: com mais de 2 letras, sem "launcher" (o .exe do
/// launcher da loja não é o jogo); tira ™ e ®. ("ELDEN RING™" → "ELDEN RING").
pub fn product_title(candidates: [Option<String>; 2]) -> Option<String> {
    candidates.into_iter().flatten().find_map(|c| {
        let ok =
            !c.trim().is_empty() && !c.to_lowercase().contains("launcher") && c.chars().count() > 2;
        ok.then(|| c.replace(['™', '®'], "").trim().to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_title_skips_launchers_and_short_names() {
        let s = |v: &str| Some(v.to_owned());
        assert_eq!(
            product_title([s("ELDEN RING™"), s("elden ring")]).as_deref(),
            Some("ELDEN RING")
        );
        assert_eq!(
            product_title([s("Rockstar Games Launcher"), s("GTA V®")]).as_deref(),
            Some("GTA V")
        );
        assert_eq!(product_title([None, s("Hades")]).as_deref(), Some("Hades"));
        assert_eq!(product_title([s("  "), s("UE")]), None);
        assert_eq!(product_title([None, None]), None);
    }

    #[test]
    fn titles_come_from_the_product_or_the_file_name() {
        let none = || [None, None];
        assert_eq!(
            guess_title(Path::new("C:/Jogos/hollow_knight.exe"), none()),
            "hollow knight"
        );
        assert_eq!(
            guess_title(
                Path::new("a/Hades-II.exe"),
                [Some("Hades II™".into()), None]
            ),
            "Hades II"
        );
        // Só separadores: fica o nome do arquivo.
        assert_eq!(guess_title(Path::new("a/__.exe"), none()), "__");
    }

    #[test]
    fn kinds_follow_the_extension() {
        assert_eq!(kind_of(Path::new("a/jogo.DESKTOP")), AddKind::Desktop);
        assert_eq!(kind_of(Path::new("a/Jogo.app")), AddKind::MacApp);
        assert_eq!(kind_of(Path::new("a/Mario (USA).sfc")), AddKind::Rom);
        assert_eq!(kind_of(Path::new("a/jogo.exe")), AddKind::Program);
        assert_eq!(kind_of(Path::new("a/jogo")), AddKind::Program);
    }

    #[test]
    fn duplicates_ignore_case() {
        let game = Game {
            launch_target: Some(r"C:\Jogos\A.exe".into()),
            ..Game::default()
        };
        let games = [game];
        assert!(find_by_target(&games, r"c:\jogos\a.EXE").is_some());
        assert!(find_by_target(&games, r"c:\jogos\b.exe").is_none());
    }
}
