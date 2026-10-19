//! `appcache/appinfo.vdf` da Steam (VDF binário, versões 27, 28 e 29): só os hashes dos ícones de cada app,
//! `common/clienticon` (arquivo `.ico` que a Steam guarda em `steam/games`) e `common/icon` (JPG 32x32 da CDN).
//! O arquivo passa de 100 MB e é dado externo: lê em fluxo, pula os apps que não interessam sem carregá-los,
//! corta entradas grandes e profundidade, e qualquer coisa fora do formato é "sem ícone" (nunca pânico).

use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read, Seek};
use std::path::Path;

const V27: u32 = 0x0756_4427;
const V28: u32 = 0x0756_4428;
const V29: u32 = 0x0756_4429;
/// Maior entrada lida. Os apps têm dezenas de KB; acima disso não é um appinfo de verdade.
const MAX_ENTRY: u32 = 4 << 20;
/// A tabela de textos da v29 fica no fim do arquivo; este é o teto do que se lê dela.
const MAX_STRING_TABLE: u64 = 64 << 20;
const MAX_DEPTH: usize = 16;

/// Hashes de ícone de um app (só letras e números, como a CDN os nomeia).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AppIcons {
    pub client_icon: Option<String>,
    pub icon: Option<String>,
}

/// Hashes dos apps de `wanted` que o arquivo traz. Arquivo ausente ou quebrado = nenhum.
pub fn read(path: &Path, wanted: &HashSet<u32>) -> HashMap<u32, AppIcons> {
    let Ok(file) = std::fs::File::open(path) else {
        return HashMap::new();
    };
    parse(file, wanted).unwrap_or_else(|| {
        tracing::debug!("appinfo.vdf da Steam fora do formato: {}", path.display());
        HashMap::new()
    })
}

/// Como [`read`], sobre qualquer fonte. `None` quando o cabeçalho não é de um appinfo conhecido.
pub fn parse<R: Read + Seek>(source: R, wanted: &HashSet<u32>) -> Option<HashMap<u32, AppIcons>> {
    let mut r = BufReader::with_capacity(64 << 10, source);
    let magic = u32_le(&mut r)?;
    if !matches!(magic, V27 | V28 | V29) {
        return None;
    }
    u32_le(&mut r)?; // universo
    let strings = if magic == V29 {
        let table = i64::try_from(u64_le(&mut r)?).ok()?;
        let here = r.stream_position().ok()?;
        r.seek(std::io::SeekFrom::Start(u64::try_from(table).ok()?))
            .ok()?;
        let strings = string_table(&mut r)?;
        r.seek(std::io::SeekFrom::Start(here)).ok()?;
        Some(strings)
    } else {
        None
    };
    // Antes dos dados: estado, data, token, 2 hashes SHA-1 (um só na v27) e número da mudança.
    let header: u32 = if magic == V27 { 40 } else { 60 };
    let mut found = HashMap::new();
    while found.len() < wanted.len() {
        let (Some(app), Some(size)) = (u32_le(&mut r), u32_le(&mut r)) else {
            break; // fim do arquivo (ou cortado)
        };
        if app == 0 {
            break;
        }
        let skip = |r: &mut BufReader<R>, n: u32| r.seek_relative(i64::from(n)).is_ok();
        if !wanted.contains(&app) || size < header || size > MAX_ENTRY {
            if !skip(&mut r, size) {
                break;
            }
            continue;
        }
        let mut data = Vec::new();
        if !skip(&mut r, header)
            || r.by_ref()
                .take(u64::from(size - header))
                .read_to_end(&mut data)
                .is_err()
        {
            break;
        }
        let mut walker = Walker {
            bytes: &data,
            pos: 0,
            strings: strings.as_deref(),
        };
        let mut icons = AppIcons::default();
        // Entrada quebrada perde só o ícone dela.
        if walker.object(0, false, &mut icons).is_some() {
            found.insert(app, icons);
        }
    }
    Some(found)
}

fn u32_le<R: Read>(r: &mut R) -> Option<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b).ok()?;
    Some(u32::from_le_bytes(b))
}

fn u64_le<R: Read>(r: &mut R) -> Option<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b).ok()?;
    Some(u64::from_le_bytes(b))
}

/// Tabela de textos da v29: quantidade e depois os textos terminados em NUL.
fn string_table<R: Read>(r: &mut R) -> Option<Vec<String>> {
    let count = usize::try_from(u32_le(r)?).ok()?;
    let mut raw = Vec::new();
    r.take(MAX_STRING_TABLE).read_to_end(&mut raw).ok()?;
    let strings: Vec<String> = raw
        .split(|&b| b == 0)
        .take(count)
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    (strings.len() == count).then_some(strings)
}

/// Percorre o KeyValues binário de uma entrada: 0 = bloco, 1 = texto, 2/3/4/6 = 4 bytes, 5 = texto UTF-16,
/// 7/10 = 8 bytes, 8 = fim do bloco.
struct Walker<'a> {
    bytes: &'a [u8],
    pos: usize,
    strings: Option<&'a [String]>,
}

impl Walker<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let end = self.pos.checked_add(n)?;
        let out = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(out)
    }

    fn text(&mut self) -> Option<String> {
        let rest = self.bytes.get(self.pos..)?;
        let len = rest.iter().position(|&b| b == 0)?;
        self.pos += len + 1;
        Some(String::from_utf8_lossy(&rest[..len]).into_owned())
    }

    /// Chave: texto no v27/v28, posição na tabela de textos no v29.
    fn key(&mut self) -> Option<String> {
        match self.strings {
            None => self.text(),
            Some(table) => {
                let i = u32::from_le_bytes(self.take(4)?.try_into().ok()?);
                table.get(usize::try_from(i).ok()?).cloned()
            }
        }
    }

    /// `in_common`: este bloco é o `common` do app. A raiz (`depth` 0) guarda `appinfo`, que guarda `common`.
    fn object(&mut self, depth: usize, in_common: bool, out: &mut AppIcons) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        loop {
            let Some(&kind) = self.bytes.get(self.pos) else {
                return (depth == 0).then_some(());
            };
            self.pos += 1;
            if kind == 8 {
                return Some(());
            }
            let key = self.key()?;
            match kind {
                0 => {
                    let common = depth == 1 && key.eq_ignore_ascii_case("common");
                    self.object(depth + 1, common, out)?;
                }
                1 => {
                    let value = self.text()?;
                    if in_common && valid_hash(&value) {
                        match key.to_ascii_lowercase().as_str() {
                            "clienticon" => out.client_icon = Some(value),
                            "icon" => out.icon = Some(value),
                            _ => {}
                        }
                    }
                }
                2..=4 | 6 => {
                    self.take(4)?;
                }
                7 | 10 => {
                    self.take(8)?;
                }
                5 => {
                    let rest = self.bytes.get(self.pos..)?;
                    let units = rest.chunks_exact(2).position(|c| c == [0, 0])?;
                    self.pos += (units + 1) * 2;
                }
                _ => return None,
            }
        }
    }
}

/// Hash de ícone: curto e só letras e números (vai para nome de arquivo e endereço).
pub fn valid_hash(s: &str) -> bool {
    (1..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Cursor;

    /// Montador do KeyValues binário (texto no v27/v28; chaves por posição na tabela no v29).
    struct Kv {
        out: Vec<u8>,
        table: Option<Vec<String>>,
    }

    impl Kv {
        fn new(v29: bool) -> Kv {
            Kv {
                out: Vec::new(),
                table: v29.then(Vec::new),
            }
        }
        fn key(&mut self, key: &str) {
            match &mut self.table {
                None => {
                    self.out.extend(key.as_bytes());
                    self.out.push(0);
                }
                Some(table) => {
                    let i = table.iter().position(|k| k == key).unwrap_or_else(|| {
                        table.push(key.into());
                        table.len() - 1
                    });
                    self.out.extend((i as u32).to_le_bytes());
                }
            }
        }
        fn open(&mut self, key: &str) -> &mut Kv {
            self.out.push(0);
            self.key(key);
            self
        }
        fn text(&mut self, key: &str, value: &str) -> &mut Kv {
            self.out.push(1);
            self.key(key);
            self.out.extend(value.as_bytes());
            self.out.push(0);
            self
        }
        fn int(&mut self, key: &str, value: u32) -> &mut Kv {
            self.out.push(2);
            self.key(key);
            self.out.extend(value.to_le_bytes());
            self
        }
        fn close(&mut self) -> &mut Kv {
            self.out.push(8);
            self
        }
    }

    /// Dados de um app: `appinfo { appid, common { name, icon, clienticon }, config { installdir } }`.
    fn fill(kv: &mut Kv, app: u32, icon: &str, client: &str) {
        kv.out.clear();
        kv.open("appinfo").int("appid", app).open("common");
        kv.text("name", "Jogo").text("icon", icon);
        kv.int("metacritic_score", 80).text("clienticon", client);
        kv.close().open("config").text("installdir", "x").close();
        kv.close().close();
    }

    /// Arquivo com as entradas `(id, dados)` e, na v29, a tabela de textos no fim.
    fn build(magic: u32, entries: &[(u32, Vec<u8>)], table: &[String]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(magic.to_le_bytes());
        out.extend(1u32.to_le_bytes());
        if magic == V29 {
            out.extend([0u8; 8]); // posição da tabela, preenchida no fim
        }
        let header = if magic == V27 { 40 } else { 60 };
        for (app, data) in entries {
            out.extend(app.to_le_bytes());
            out.extend(((header + data.len()) as u32).to_le_bytes());
            out.extend(vec![0u8; header]);
            out.extend(data);
        }
        out.extend(0u32.to_le_bytes());
        if magic == V29 {
            let at = out.len() as u64;
            out[8..16].copy_from_slice(&at.to_le_bytes());
            out.extend((table.len() as u32).to_le_bytes());
            for s in table {
                out.extend(s.as_bytes());
                out.push(0);
            }
        }
        out
    }

    /// Arquivo de apps `(id, icon, clienticon)`.
    pub(crate) fn apps_file(magic: u32, apps: &[(u32, &str, &str)]) -> Vec<u8> {
        let mut kv = Kv::new(magic == V29);
        let entries: Vec<_> = apps
            .iter()
            .map(|&(app, icon, client)| {
                fill(&mut kv, app, icon, client);
                (app, kv.out.clone())
            })
            .collect();
        build(magic, &entries, kv.table.as_deref().unwrap_or(&[]))
    }

    fn wanted(ids: &[u32]) -> HashSet<u32> {
        ids.iter().copied().collect()
    }

    fn icons(client: &str, icon: &str) -> AppIcons {
        AppIcons {
            client_icon: Some(client.into()),
            icon: Some(icon.into()),
        }
    }

    #[test]
    fn reads_the_icon_hashes_of_wanted_apps_in_every_version() {
        for magic in [V27, V28, V29] {
            let bytes = apps_file(
                magic,
                &[
                    (10, "aaa", "bbb"),
                    (570, "c0ffee", "d00d"),
                    (620, "e1", "e2"),
                ],
            );
            let got = parse(Cursor::new(&bytes), &wanted(&[570, 620, 999])).unwrap();
            assert_eq!(got.len(), 2, "versão {magic:x}");
            assert_eq!(got[&570], icons("d00d", "c0ffee"), "versão {magic:x}");
            assert_eq!(got[&620], icons("e2", "e1"), "versão {magic:x}");
        }
    }

    #[test]
    fn hash_that_is_not_a_file_name_is_ignored() {
        let bytes = apps_file(V28, &[(7, "../../x", "ok1")]);
        let got = parse(Cursor::new(bytes), &wanted(&[7])).unwrap();
        assert_eq!(
            got[&7],
            AppIcons {
                client_icon: Some("ok1".into()),
                icon: None
            }
        );
    }

    #[test]
    fn broken_input_is_nothing_and_never_panics() {
        let good = apps_file(V29, &[(1, "aa", "bb")]);
        // Cortado em qualquer ponto.
        for n in 0..good.len() {
            let _ = parse(Cursor::new(&good[..n]), &wanted(&[1]));
        }
        // Um byte trocado em qualquer ponto.
        for i in 0..good.len() {
            let mut bad = good.clone();
            bad[i] = bad[i].wrapping_add(0x5b);
            let _ = parse(Cursor::new(bad), &wanted(&[1]));
        }
        assert!(parse(Cursor::new(b"nada a ver".to_vec()), &wanted(&[1])).is_none());
        assert!(parse(Cursor::new(Vec::new()), &wanted(&[1])).is_none());
        // Tamanho absurdo na entrada: pulado sem alocar.
        let mut huge = apps_file(V28, &[(1, "aa", "bb")]);
        huge[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(Cursor::new(huge), &wanted(&[1])).unwrap().is_empty());
        // Blocos dentro de blocos sem fim: corta na profundidade.
        let mut deep = Kv::new(false);
        for _ in 0..10_000 {
            deep.open("a");
        }
        let bytes = build(V28, &[(1, deep.out)], &[]);
        assert!(parse(Cursor::new(bytes), &wanted(&[1])).unwrap().is_empty());
    }

    #[test]
    fn missing_file_is_nothing() {
        assert!(read(Path::new("/nao/existe/appinfo.vdf"), &wanted(&[1])).is_empty());
    }
}
