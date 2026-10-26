//! Leitura mínima do `%ProgramData%\Battle.net\Agent\product.db` (protobuf, port de `ProductDb.cs`):
//! `product_installs` (1) → { `uid` (1), `product_code` (2), `settings` (3) → `install_path` (1),
//! `cached_product_state` (4) → `base_product_state` (1) → `installed` (1) }. Campos desconhecidos
//! são pulados; arquivo truncado ou inválido é `Err`.

/// Um produto instalado segundo o `product.db`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleNetInstall {
    pub uid: String,
    pub product_code: Option<String>,
    pub install_path: String,
    pub installed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("product.db: {0}")]
pub struct ProductDbError(&'static str);

const VARINT: u64 = 0;
const FIXED64: u64 = 1;
const LEN: u64 = 2;
const FIXED32: u64 = 5;

pub fn parse(data: &[u8]) -> Result<Vec<BattleNetInstall>, ProductDbError> {
    let mut installs = Vec::new();
    let mut r = Reader { data, pos: 0 };
    while let Some((field, wire)) = r.next()? {
        if field == 1 && wire == LEN {
            if let Some(i) = parse_install(r.bytes()?)? {
                installs.push(i);
            }
        } else {
            r.skip(wire)?;
        }
    }
    Ok(installs)
}

fn parse_install(data: &[u8]) -> Result<Option<BattleNetInstall>, ProductDbError> {
    let (mut uid, mut code, mut path, mut installed) = (None, None, None, None);
    let mut r = Reader { data, pos: 0 };
    while let Some((field, wire)) = r.next()? {
        match (field, wire) {
            (1, LEN) => uid = Some(r.string()?),
            (2, LEN) => code = Some(r.string()?),
            (3, LEN) => path = first_string(r.bytes()?, 1)?,
            (4, LEN) => installed = installed_state(r.bytes()?)?,
            _ => r.skip(wire)?,
        }
    }
    Ok(match (uid, path) {
        (Some(uid), Some(path)) if !uid.trim().is_empty() && !path.trim().is_empty() => {
            Some(BattleNetInstall {
                uid,
                product_code: code,
                install_path: path,
                installed,
            })
        }
        _ => None,
    })
}

/// `cached_product_state.base_product_state.installed`; `None` se o arquivo não informa.
fn installed_state(cached: &[u8]) -> Result<Option<bool>, ProductDbError> {
    let mut r = Reader {
        data: cached,
        pos: 0,
    };
    while let Some((field, wire)) = r.next()? {
        if field == 1 && wire == LEN {
            let mut state = Reader {
                data: r.bytes()?,
                pos: 0,
            };
            while let Some((f, w)) = state.next()? {
                if f == 1 && w == VARINT {
                    return Ok(Some(state.varint()? != 0));
                }
                state.skip(w)?;
            }
            return Ok(None);
        }
        r.skip(wire)?;
    }
    Ok(None)
}

fn first_string(message: &[u8], wanted: u64) -> Result<Option<String>, ProductDbError> {
    let mut r = Reader {
        data: message,
        pos: 0,
    };
    while let Some((field, wire)) = r.next()? {
        if field == wanted && wire == LEN {
            return Ok(Some(r.string()?));
        }
        r.skip(wire)?;
    }
    Ok(None)
}

/// Formato de fio do protobuf (varint, 64 bits, tamanho+bytes, 32 bits).
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn next(&mut self) -> Result<Option<(u64, u64)>, ProductDbError> {
        if self.pos >= self.data.len() {
            return Ok(None);
        }
        let tag = self.varint()?;
        let field = tag >> 3;
        if field == 0 {
            return Err(ProductDbError("campo inválido"));
        }
        Ok(Some((field, tag & 7)))
    }

    fn varint(&mut self) -> Result<u64, ProductDbError> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let b = *self
                .data
                .get(self.pos)
                .ok_or(ProductDbError("arquivo truncado"))?;
            self.pos += 1;
            value |= u64::from(b & 0x7F) << shift;
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ProductDbError("número inválido"))
    }

    fn bytes(&mut self) -> Result<&'a [u8], ProductDbError> {
        let len =
            usize::try_from(self.varint()?).map_err(|_| ProductDbError("arquivo truncado"))?;
        self.advance(len)
    }

    fn string(&mut self) -> Result<String, ProductDbError> {
        Ok(String::from_utf8_lossy(self.bytes()?).into_owned())
    }

    fn skip(&mut self, wire: u64) -> Result<(), ProductDbError> {
        match wire {
            VARINT => self.varint().map(drop),
            FIXED64 => self.advance(8).map(drop),
            LEN => self.bytes().map(drop),
            FIXED32 => self.advance(4).map(drop),
            _ => Err(ProductDbError("tipo de campo não suportado")),
        }
    }

    fn advance(&mut self, n: usize) -> Result<&'a [u8], ProductDbError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&e| e <= self.data.len())
            .ok_or(ProductDbError("arquivo truncado"))?;
        let slice = &self.data[self.pos..end];
        self.pos = end;
        Ok(slice)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn varint(out: &mut Vec<u8>, mut v: u64) {
        while v >= 0x80 {
            out.push((v as u8) | 0x80);
            v >>= 7;
        }
        out.push(v as u8);
    }

    /// Campo protobuf de tamanho+bytes (tipo 2).
    pub fn field(number: u64, value: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        varint(&mut out, (number << 3) | 2);
        varint(&mut out, value.len() as u64);
        out.extend_from_slice(value);
        out
    }

    pub fn install(uid: &str, code: &str, path: &str, installed: Option<bool>) -> Vec<u8> {
        let mut m = field(1, uid.as_bytes());
        m.extend(field(2, code.as_bytes()));
        let mut settings = field(1, path.as_bytes());
        settings.extend(field(2, b"us"));
        m.extend(field(3, &settings));
        if let Some(on) = installed {
            let state = [0x08, u8::from(on), 0x10, 0x01]; // installed (1) e playable (2)
            m.extend(field(4, &field(1, &state)));
        }
        m
    }

    pub fn db(installs: &[Vec<u8>]) -> Vec<u8> {
        installs.iter().flat_map(|i| field(1, i)).collect()
    }

    #[test]
    fn reads_uid_code_path_and_state() {
        let data = db(&[
            install(
                "prometheus",
                "pro",
                r"C:\Program Files (x86)\Overwatch",
                Some(true),
            ),
            install(
                "battle.net",
                "bna",
                r"C:\Program Files (x86)\Battle.net",
                Some(true),
            ),
            install("fenris", "fen", r"D:\Diablo IV", Some(false)),
        ]);
        let installs = parse(&data).unwrap();
        assert_eq!(installs.len(), 3);
        assert_eq!(
            installs[0],
            BattleNetInstall {
                uid: "prometheus".into(),
                product_code: Some("pro".into()),
                install_path: r"C:\Program Files (x86)\Overwatch".into(),
                installed: Some(true),
            }
        );
        assert_eq!(installs[2].installed, Some(false));
    }

    #[test]
    fn skips_unknown_fields_and_rejects_truncated_files() {
        let mut data = field(9, b"ignorado");
        data.extend([0x18, 0x96, 0x01]); // varint no campo 3
        data.extend(field(1, &install("s2", "s2", r"C:\StarCraft II", None)));
        let installs = parse(&data).unwrap();
        assert_eq!(installs.len(), 1);
        assert_eq!(installs[0].uid, "s2");
        assert_eq!(installs[0].installed, None);

        let full = db(&[install("wow", "wow", r"C:\WoW", Some(true))]);
        assert!(parse(&full[..full.len() - 3]).is_err());
        assert!(parse(&[0x0A, 0xFF]).is_err());
        assert!(
            parse(&[
                0x0A, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01
            ])
            .is_err()
        );
    }
}
