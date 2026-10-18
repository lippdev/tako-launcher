//! Jogos da conta Amazon Games que não estão instalados (SRC-005), lidos offline do banco do app no Windows
//! (`%LOCALAPPDATA%\Amazon Games\Data\Games\Sql\Entitlements.sqlite`, SQLite), aberto só para leitura. Mesma
//! leitura do plugin Amazon do GOG Galaxy (`galaxy-integration-amazon`, `plugin.py`, `_get_owned_games`): cada
//! linha de `game_entitlements.value` é um JSON cifrado pelo DPAPI **do próprio usuário** (sem entropia), com
//! `ProductIdStr` (o mesmo `Id` do `GameInstallInfo.sqlite`) e `ProductTitle`. Decifrar é o `CryptUnprotectData`
//! do Windows, que só funciona para o usuário dono dos dados: nada de chave embutida nem DRM. O decifrador vem de
//! fora (o `unsafe` fica no `tako-platform`), para o parser rodar nos testes em qualquer SO.

use super::ACCENT;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use tako_core::library::{Game, GameStore};

/// Decifrador do DPAPI do usuário atual (`tako_platform::windows::dpapi::unprotect`); `None` = não decifrou.
pub type Unprotect = fn(&[u8]) -> Option<Vec<u8>>;

/// O banco tem uma linha por jogo da conta; acima disso não é o banco da Amazon.
const MAX_DB: u64 = 256 << 20;
/// Cada linha é um JSON pequeno; uma maior está fora do formato.
const MAX_ROW: usize = 1 << 20;

/// Jogo da conta, como o app grava.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AmazonOwned {
    pub product_id: String,
    pub title: String,
}

/// Jogos da conta. Linha que não decifra, não é JSON ou não tem id e título é pulada; banco inválido ou sem a
/// tabela é `Err`.
pub fn read(database: &Path, unprotect: Unprotect) -> Result<Vec<AmazonOwned>, rusqlite::Error> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(database, flags)?;
    let mut stmt = conn.prepare("SELECT value FROM game_entitlements")?;
    let mut rows = stmt.query([])?;
    let mut out: Vec<AmazonOwned> = Vec::new();
    while let Some(row) = rows.next()? {
        // O plugin lê como bytes; aceita texto também, caso o app grave assim.
        let cipher = match row.get_ref(0)? {
            ValueRef::Blob(b) | ValueRef::Text(b) => b,
            _ => continue,
        };
        if cipher.len() > MAX_ROW {
            continue;
        }
        let Some(plain) = unprotect(cipher) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&plain) else {
            continue;
        };
        let text = |k: &str| {
            value[k]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        if let (Some(product_id), Some(title)) = (text("ProductIdStr"), text("ProductTitle"))
            && !out.iter().any(|o| o.product_id == product_id)
        {
            out.push(AmazonOwned { product_id, title });
        }
    }
    out.sort_by(|a, b| a.product_id.cmp(&b.product_id));
    Ok(out)
}

/// O banco no disco; ausente, grande demais ou ilegível = nenhum.
pub fn load(database: &Path, unprotect: Unprotect) -> Vec<AmazonOwned> {
    match std::fs::metadata(database) {
        Ok(m) if m.len() <= MAX_DB => {}
        Ok(_) => {
            tracing::warn!(
                "banco de jogos da Amazon grande demais: {}",
                database.display()
            );
            return Vec::new();
        }
        Err(_) => return Vec::new(),
    }
    read(database, unprotect).unwrap_or_else(|e| {
        tracing::warn!(
            "banco de jogos da Amazon ilegível ({}): {e}",
            database.display()
        );
        Vec::new()
    })
}

/// Pede a instalação ao app (o mesmo endereço do plugin Amazon do Playnite, `AmazonGameController.cs`).
pub fn install_uri(product_id: &str) -> String {
    format!("amazon-games://install/{product_id}")
}

/// Jogo não instalado com o id do instalado (`amazon:<id>`), para virar instalado no lugar.
pub fn to_game(o: &AmazonOwned) -> Game {
    Game {
        id: super::id_for(&o.product_id),
        title: o.title.clone(),
        store: Some(GameStore::Amazon),
        accent: ACCENT.to_owned(),
        is_installed: false,
        install_uri: Some(install_uri(&o.product_id)),
        store_product_id: Some(o.product_id.clone()),
        ..Default::default()
    }
}

/// Os da conta que não estão em `installed`.
pub fn not_installed(owned: &[AmazonOwned], installed: &[Game]) -> Vec<Game> {
    owned
        .iter()
        .map(to_game)
        .filter(|g| !installed.iter().any(|i| i.id.eq_ignore_ascii_case(&g.id)))
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// "Cifra" dos testes: um prefixo fixo. O decifrador falso só aceita o que tem o prefixo, como o DPAPI só
    /// aceita o que o próprio usuário cifrou.
    const SEAL: &[u8] = b"DPAPI:";

    pub(crate) fn fake_unprotect(data: &[u8]) -> Option<Vec<u8>> {
        data.strip_prefix(SEAL).map(<[u8]>::to_vec)
    }

    fn seal(json: &str) -> Vec<u8> {
        [SEAL, json.as_bytes()].concat()
    }

    /// Um `Entitlements.sqlite` com a tabela que a leitura usa.
    pub(crate) fn entitlements_db(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch("CREATE TABLE game_entitlements (key TEXT PRIMARY KEY, value BLOB);")
            .unwrap();
        let rows: Vec<Vec<u8>> = vec![
            seal(r#"{"ProductIdStr":"amzn1.adg.product.ilha","ProductTitle":"Ilha Perdida"}"#),
            seal(r#"{"ProductIdStr":"amzn1.adg.product.farol","ProductTitle":" Farol do Norte "}"#),
            // repetido, sem título, sem id, JSON quebrado, de outro usuário (não decifra) e gigante
            seal(r#"{"ProductIdStr":"amzn1.adg.product.farol","ProductTitle":"Farol (cópia)"}"#),
            seal(r#"{"ProductIdStr":"amzn1.adg.product.semnome","ProductTitle":null}"#),
            seal(r#"{"ProductTitle":"Sem Id"}"#),
            seal("{quebrado"),
            b"cifrado por outro usuario".to_vec(),
            [SEAL, &vec![b' '; MAX_ROW]].concat(),
        ];
        for (i, v) in rows.iter().enumerate() {
            conn.execute(
                "INSERT INTO game_entitlements VALUES (?1, ?2)",
                rusqlite::params![i.to_string(), v],
            )
            .unwrap();
        }
        // Texto em vez de blob também serve; número não.
        conn.execute_batch(
            r#"INSERT INTO game_entitlements VALUES ('t', 'DPAPI:{"ProductIdStr":"amzn1.adg.product.texto","ProductTitle":"Texto"}');
               INSERT INTO game_entitlements VALUES ('n', 42);"#,
        )
        .unwrap();
    }

    #[test]
    fn reads_the_account_games_and_skips_what_does_not_decrypt_or_parse() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("Entitlements.sqlite");
        entitlements_db(&db);
        let got = read(&db, fake_unprotect).unwrap();
        let pairs: Vec<_> = got
            .iter()
            .map(|o| (o.product_id.as_str(), o.title.as_str()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("amzn1.adg.product.farol", "Farol do Norte"),
                ("amzn1.adg.product.ilha", "Ilha Perdida"),
                ("amzn1.adg.product.texto", "Texto"),
            ]
        );
    }

    #[test]
    fn broken_database_is_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("Entitlements.sqlite");
        std::fs::write(&bad, "isto não é um banco SQLite").unwrap();
        assert!(read(&bad, fake_unprotect).is_err());
        assert!(load(&bad, fake_unprotect).is_empty());
        let empty = dir.path().join("vazio.sqlite");
        Connection::open(&empty).unwrap();
        assert!(load(&empty, fake_unprotect).is_empty());
        assert!(load(&dir.path().join("nao-existe.sqlite"), fake_unprotect).is_empty());
        // Sem DPAPI (outro usuário ou outro PC): nada decifra, nada entra.
        let db = dir.path().join("e.sqlite");
        entitlements_db(&db);
        assert!(load(&db, |_| None).is_empty());
    }

    #[test]
    fn install_asks_the_app_with_the_same_id_as_the_installed_game() {
        let g = to_game(&AmazonOwned {
            product_id: "amzn1.adg.product.ilha".into(),
            title: "Ilha".into(),
        });
        assert_eq!(g.id, "amazon:amzn1.adg.product.ilha");
        assert_eq!(
            g.install_uri.as_deref(),
            Some("amazon-games://install/amzn1.adg.product.ilha")
        );
        assert!(!g.can_play());
    }
}
