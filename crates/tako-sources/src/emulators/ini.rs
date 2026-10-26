//! INI dos emuladores (Dolphin, PCSX2, DuckStation, PPSSPP) e o `retroarch.cfg` (sem seções): `chave = valor`,
//! seções `[Nome]`, chaves repetidas (o PCSX2 e o DuckStation repetem `RecursivePaths`). Comentários com `#` e `;`.

#[derive(Debug, Default)]
pub struct Ini {
    /// `(seção, chave, valor)` na ordem do arquivo; seção vazia antes do primeiro `[...]`.
    entries: Vec<(String, String, String)>,
}

impl Ini {
    pub fn parse(text: &str) -> Ini {
        let mut section = String::new();
        let mut entries = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.trim().to_owned();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            // `retroarch.cfg` põe os valores entre aspas.
            let v = v
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(v);
            entries.push((section.clone(), k.trim().to_owned(), v.to_owned()));
        }
        Ini { entries }
    }

    /// Todos os valores da chave na seção (sem diferenciar maiúsculas).
    pub fn all<'a>(&'a self, section: &'a str, key: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.entries
            .iter()
            .filter(move |(s, k, _)| s.eq_ignore_ascii_case(section) && k.eq_ignore_ascii_case(key))
            .map(|(_, _, v)| v.as_str())
    }

    pub fn get<'a>(&'a self, section: &str, key: &str) -> Option<&'a str> {
        self.entries
            .iter()
            .rev()
            .find(|(s, k, _)| s.eq_ignore_ascii_case(section) && k.eq_ignore_ascii_case(key))
            .map(|(_, _, v)| v.as_str())
            .filter(|v| !v.is_empty())
    }

    /// Valores das chaves `<prefixo>0`, `<prefixo>1`… (Dolphin: `ISOPath0`; PPSSPP: `FileName0`).
    pub fn numbered(&self, section: &str, prefix: &str) -> Vec<String> {
        let mut v: Vec<(u32, String)> = self
            .entries
            .iter()
            .filter(|(s, _, _)| s.eq_ignore_ascii_case(section))
            .filter_map(|(_, k, val)| {
                let n = k
                    .get(..prefix.len())
                    .filter(|p| p.eq_ignore_ascii_case(prefix))
                    .and_then(|_| k[prefix.len()..].parse::<u32>().ok())?;
                (!val.is_empty()).then(|| (n, val.clone()))
            })
            .collect();
        v.sort_by_key(|(n, _)| *n);
        v.into_iter().map(|(_, s)| s).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_repeated_keys_and_quotes() {
        let ini = Ini::parse(
            "playlist_directory = \"~/pl\"\n# comentário\n[GameList]\nRecursivePaths = D:\\PS2\nRecursivePaths = E:\\Mais\nPaths =\n[General]\nISOPath1 = /b\nISOPath0 = /a\nISOPaths = 2\nlixo\n",
        );
        assert_eq!(ini.get("", "playlist_directory"), Some("~/pl"));
        assert_eq!(
            ini.all("gamelist", "recursivepaths").collect::<Vec<_>>(),
            ["D:\\PS2", "E:\\Mais"]
        );
        assert_eq!(ini.get("GameList", "Paths"), None);
        assert_eq!(ini.numbered("General", "ISOPath"), ["/a", "/b"]);
    }
}
