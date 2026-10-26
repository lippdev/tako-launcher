//! Armazenamento (LIB-005): quanto cada jogo ocupa e o total por disco. A Steam e a Epic informam o tamanho; dos
//! outros jogos instalados o Tako mede a pasta de instalação, em segundo plano.

use super::Game;
use std::path::Path;

/// Arquivos lidos no máximo por pasta: uma pasta gigante (ou um link para a raiz do disco) não trava a medição.
pub const MAX_ENTRIES: usize = 500_000;

/// Soma o tamanho dos arquivos da pasta, sem seguir links. `None` = a pasta não existe, não abriu ou passou de
/// `max_entries` (um número errado seria pior que nenhum).
pub fn folder_size(path: &Path, max_entries: usize) -> Option<u64> {
    if !path.is_dir() {
        return None;
    }
    let mut total = 0u64;
    let mut seen = 0usize;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > max_entries {
                return None;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    Some(total)
}

/// Disco de um caminho: a unidade no Windows (`C:`), o compartilhamento de rede (`\\servidor\pasta`) ou `/`.
pub fn disk_of(path: &str) -> String {
    let p = path.trim();
    let b = p.as_bytes();
    if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        return format!("{}:", (b[0] as char).to_ascii_uppercase());
    }
    if let Some(rest) = p.strip_prefix(r"\\").or_else(|| p.strip_prefix("//")) {
        let mut parts = rest.split(['\\', '/']).filter(|s| !s.is_empty());
        if let (Some(server), Some(share)) = (parts.next(), parts.next()) {
            return format!(r"\\{server}\{share}");
        }
    }
    "/".to_owned()
}

/// Espaço usado pelos jogos num disco.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskUsage {
    pub disk: String,
    pub bytes: u64,
    pub games: usize,
}

/// Total por disco dos jogos instalados com tamanho conhecido, do disco mais cheio para o mais vazio.
pub fn totals<'a>(games: impl IntoIterator<Item = &'a Game>) -> Vec<DiskUsage> {
    let mut out: Vec<DiskUsage> = Vec::new();
    for g in games.into_iter().filter(|g| g.is_installed) {
        let (Some(bytes), Some(path)) = (g.size_bytes, g.install_path.as_deref()) else {
            continue;
        };
        let disk = disk_of(path);
        match out.iter_mut().find(|d| d.disk == disk) {
            Some(d) => {
                d.bytes += bytes;
                d.games += 1;
            }
            None => out.push(DiskUsage {
                disk,
                bytes,
                games: 1,
            }),
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.disk.cmp(&b.disk)));
    out
}

/// Jogos instalados com pasta e sem tamanho: os que a medição precisa ler.
pub fn needs_measure(game: &Game) -> bool {
    game.is_installed && game.size_bytes.is_none() && game.install_path.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_a_folder_and_gives_up_on_huge_or_missing_ones() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.bin"), vec![0u8; 1000]).unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub").join("b.bin"), vec![0u8; 24]).unwrap();
        assert_eq!(folder_size(dir.path(), MAX_ENTRIES), Some(1024));
        assert_eq!(folder_size(dir.path(), 2), None);
        assert_eq!(
            folder_size(&dir.path().join("nao-existe"), MAX_ENTRIES),
            None
        );
        assert_eq!(folder_size(&dir.path().join("a.bin"), MAX_ENTRIES), None);
    }

    #[test]
    fn groups_installed_games_by_disk() {
        assert_eq!(disk_of(r"d:\Games\Hades"), "D:");
        assert_eq!(disk_of(r"\\nas\jogos\Hades"), r"\\nas\jogos");
        assert_eq!(disk_of("/home/eu/Games"), "/");
        let game = |path: &str, size: Option<u64>, installed: bool| Game {
            install_path: Some(path.into()),
            size_bytes: size,
            is_installed: installed,
            ..Default::default()
        };
        let games = [
            game(r"C:\A", Some(10), true),
            game(r"D:\B", Some(50), true),
            game(r"c:\C", Some(5), true),
            game(r"E:\D", None, true),
            game(r"F:\E", Some(99), false),
        ];
        assert_eq!(
            totals(&games),
            [
                DiskUsage {
                    disk: "D:".into(),
                    bytes: 50,
                    games: 1
                },
                DiskUsage {
                    disk: "C:".into(),
                    bytes: 15,
                    games: 2
                },
            ]
        );
        assert!(needs_measure(&games[3]));
        assert!(!needs_measure(&games[0]));
        assert!(!needs_measure(&games[4]));
    }
}
