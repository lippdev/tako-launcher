//! Battle.net (port de `Library/BattleNet`): o `product.db` do agente e, se faltar ou não abrir, as
//! chaves de desinstalação (`--uid=`). Abre sempre pelo cliente: `Battle.net.exe --exec="launch <código>"`;
//! jogo sem código conhecido abre o Battle.net para o usuário escolher.

pub mod product_db;

use crate::fs::StoreFs;
use crate::registry::RegistryReader;
use crate::uninstall::UninstallEntry;
use crate::windows_path;
use crate::{LibraryScan, LibrarySource};
use chrono::{DateTime, Utc};
use product_db::BattleNetInstall;
use std::sync::{Arc, OnceLock};
use tako_core::i18n::t;
use tako_core::library::{Game, GameStore};

const ACCENT: &str = "#0B74DE";

/// Um jogo conhecido: uid no disco, código de abrir e nome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BattleNetProduct {
    pub uid: &'static str,
    pub launch_code: &'static str,
    pub name: &'static str,
}

const fn p(uid: &'static str, launch_code: &'static str, name: &'static str) -> BattleNetProduct {
    BattleNetProduct {
        uid,
        launch_code,
        name,
    }
}

// Adaptado de JosefNemec/PlayniteExtensions: BattleNetLibrary/Models/BattleNetGames.cs (MIT).
pub const PRODUCTS: [BattleNetProduct; 31] = [
    p("wow", "WoW", "World of Warcraft"),
    p("diablo3", "D3", "Diablo III"),
    p("s2", "S2", "StarCraft II"),
    p("s1", "S1", "StarCraft"),
    p("hs_beta", "WTCG", "Hearthstone"),
    p("heroes", "Hero", "Heroes of the Storm"),
    p("prometheus", "Pro", "Overwatch 2"),
    p("viper", "VIPR", "Call of Duty: Black Ops 4"),
    p("odin", "ODIN", "Call of Duty: Modern Warfare"),
    p("w3", "W3", "Warcraft III: Reforged"),
    p(
        "lazarus",
        "LAZR",
        "Call of Duty: Modern Warfare 2 Campaign Remastered",
    ),
    p("zeus", "ZEUS", "Call of Duty: Black Ops Cold War"),
    p("wlby", "WLBY", "Crash Bandicoot 4: It's About Time"),
    p("osi", "OSI", "Diablo II: Resurrected"),
    p("rtro", "RTRO", "Blizzard Arcade Collection"),
    p("fore", "FORE", "Call of Duty: Vanguard"),
    p("anbs", "ANBS", "Diablo Immortal"),
    p("auks", "AUKS", "Call of Duty"),
    p("fenris", "Fen", "Diablo IV"),
    p("d1", "D1", "Diablo"),
    p("w1r", "W1R", "Warcraft: Remastered"),
    p("w2r", "W2R", "Warcraft II: Remastered"),
    p("w1", "W1", "Warcraft: Orcs & Humans"),
    p("w2", "W2", "Warcraft II: Battle.net Edition"),
    p("gryphon", "GRY", "Warcraft Rumble"),
    p("aris", "ARIS", "DOOM: The Dark Ages"),
    p("scorpio", "SCOR", "Sea of Thieves"),
    p("arkansas", "ARK", "The Outer Worlds 2"),
    p("libra", "LBRA", "Tony Hawk's Pro Skater 3 + 4"),
    p("pinta", "PNTA", "Call of Duty: Modern Warfare III"),
    p("aqua", "AQUA", "Avowed"),
];

/// Programas do Battle.net que não são jogos (o próprio cliente e o agente).
const NOT_GAMES: [&str; 4] = ["battle.net", "agent", "bna", "bts"];
/// Idiomas que aparecem no fim do uid ("diablo3_enus").
const LOCALES: [&str; 16] = [
    "enus", "engb", "dede", "eses", "esmx", "frfr", "itit", "kokr", "plpl", "ptbr", "ptpt", "ruru",
    "zhcn", "zhtw", "jajp", "thth",
];

pub fn is_client(uid: &str) -> bool {
    NOT_GAMES.iter().any(|n| n.eq_ignore_ascii_case(uid))
}

/// O jogo do uid: igual ao da tabela ou seguido de idioma ("diablo3_enus"). Outras variações
/// ("wow_classic") não herdam o código, para não abrir o jogo errado.
pub fn find(uid: &str) -> Option<&'static BattleNetProduct> {
    PRODUCTS.iter().find(|p| {
        uid.eq_ignore_ascii_case(p.uid)
            || (uid.len() == p.uid.len() + 5
                && uid
                    .get(..p.uid.len() + 1)
                    .is_some_and(|h| h.eq_ignore_ascii_case(&format!("{}_", p.uid)))
                && uid
                    .get(uid.len() - 4..)
                    .is_some_and(|l| LOCALES.iter().any(|x| x.eq_ignore_ascii_case(l))))
    })
}

/// O uid no comando de desinstalação: `"…\Blizzard Uninstaller.exe" --lang=enUS --uid=prometheus …`.
pub fn uid_of(uninstall_string: Option<&str>) -> Option<String> {
    static RE: OnceLock<Option<regex::Regex>> = OnceLock::new();
    let re = RE
        .get_or_init(|| regex::Regex::new(r#"(?i)--uid="?([^"\s]+)"#).ok())
        .as_ref()?;
    Some(re.captures(uninstall_string?)?.get(1)?.as_str().to_owned())
}

pub fn id_for(uid: &str) -> String {
    format!("battlenet:{}", uid.to_lowercase())
}

/// Pelo cliente com `--exec="launch Pro"`; sem código conhecido, só abre o Battle.net. Sem o
/// cliente, o protocolo `battlenet://` (falha ao abrir e mostra o aviso de cliente ausente).
pub fn to_game(
    uid: &str,
    product: Option<&BattleNetProduct>,
    title: &str,
    folder: &str,
    client: Option<&str>,
    added_at: Option<DateTime<Utc>>,
) -> Game {
    Game {
        id: id_for(uid),
        title: title.trim().to_owned(),
        store: Some(GameStore::BattleNet),
        accent: ACCENT.to_owned(),
        is_installed: true,
        added_at,
        launch_target: Some(match client {
            Some(c) => c.to_owned(),
            None => format!("battlenet://{}", product.map_or("", |p| p.launch_code)),
        }),
        launch_arguments: match (client, product) {
            (Some(_), Some(p)) => Some(format!("--exec=\"launch {}\"", p.launch_code)),
            _ => None,
        },
        install_path: Some(folder.to_owned()),
        store_product_id: product.map(|p| p.launch_code.to_owned()),
        ..Default::default()
    }
}

pub struct BattleNetLibrarySource {
    registry: Arc<dyn RegistryReader>,
    program_data: String,
    program_files_x86: Option<String>,
    pub fs: StoreFs,
}

impl BattleNetLibrarySource {
    pub fn new(
        registry: Arc<dyn RegistryReader>,
        program_data: &str,
        program_files_x86: Option<String>,
    ) -> Self {
        BattleNetLibrarySource {
            registry,
            program_data: program_data.to_owned(),
            program_files_x86,
            fs: StoreFs::real(),
        }
    }

    pub fn product_db_path(&self) -> String {
        windows_path::combine(&self.program_data, &["Battle.net", "Agent", "product.db"])
    }

    /// `Battle.net.exe`: pela chave de desinstalação do cliente (`--uid=battle.net`) ou na pasta padrão.
    pub fn client_executable(&self, entries: &[UninstallEntry]) -> Option<String> {
        let mut candidates: Vec<String> = entries
            .iter()
            .filter(|e| {
                uid_of(e.uninstall_string.as_deref())
                    .is_some_and(|u| u.eq_ignore_ascii_case("battle.net"))
            })
            .filter_map(|e| e.install_location.as_deref())
            .map(|l| windows_path::combine(l, &["Battle.net.exe"]))
            .collect();
        if let Some(pf) = self
            .program_files_x86
            .as_deref()
            .filter(|p| !p.trim().is_empty())
        {
            candidates.push(windows_path::combine(pf, &["Battle.net", "Battle.net.exe"]));
        }
        candidates.into_iter().find(|c| (self.fs.exists)(c))
    }

    fn read_product_db(&self) -> Option<Vec<BattleNetInstall>> {
        // Formato novo ou arquivo sendo gravado: o registro dá conta.
        product_db::parse(&(self.fs.read)(&self.product_db_path())?).ok()
    }
}

/// Plano B: os jogos que o Battle.net registra como programas instalados (sem testes e betas).
fn from_uninstall(entries: &[UninstallEntry]) -> Vec<BattleNetInstall> {
    entries
        .iter()
        .filter(|e| {
            let name = e.display_name.as_deref().unwrap_or("").to_lowercase();
            e.install_location.is_some()
                && e.uninstall_string
                    .as_deref()
                    .is_some_and(|u| u.to_lowercase().contains("battle.net"))
                && !name.ends_with("test")
                && !name.ends_with("beta")
        })
        .filter_map(|e| {
            Some(BattleNetInstall {
                uid: uid_of(e.uninstall_string.as_deref())?,
                product_code: None,
                install_path: e.install_location.clone()?,
                installed: Some(true),
            })
        })
        .collect()
}

fn display_name_of(entries: &[UninstallEntry], uid: &str) -> Option<String> {
    entries
        .iter()
        .find(|e| {
            uid_of(e.uninstall_string.as_deref()).is_some_and(|u| u.eq_ignore_ascii_case(uid))
        })
        .and_then(|e| e.display_name.clone())
}

impl LibrarySource for BattleNetLibrarySource {
    fn store(&self) -> GameStore {
        GameStore::BattleNet
    }

    fn scan(&self) -> LibraryScan {
        let entries = crate::uninstall::read(self.registry.as_ref());
        let client = self.client_executable(&entries);
        let installs = self
            .read_product_db()
            .unwrap_or_else(|| from_uninstall(&entries));
        let mut games: Vec<Game> = Vec::new();
        for install in installs {
            let path = windows_path::normalize(&install.install_path);
            if is_client(&install.uid)
                || install.installed == Some(false)
                || !(self.fs.exists)(&path)
                || games.iter().any(|g| g.id == id_for(&install.uid))
            {
                continue;
            }
            let product = find(&install.uid);
            let title = product
                .map(|p| p.name.to_owned())
                .or_else(|| display_name_of(&entries, &install.uid))
                .unwrap_or_else(|| windows_path::last_name(&path));
            games.push(to_game(
                &install.uid,
                product,
                &title,
                &path,
                client.as_deref(),
                crate::install_dates::of(&path),
            ));
        }
        if games.is_empty() && client.is_none() {
            return LibraryScan::missing(t("sources.battlenet.missing"));
        }
        crate::text::sort_by_title(&mut games);
        LibraryScan {
            games,
            client_installed: client.is_some(),
            notice: client.is_none().then(|| t("sources.battlenet.no_client")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::product_db::tests::{db, install};
    use super::*;
    use crate::fs::fake;
    use crate::registry::MemoryRegistry;
    use std::collections::HashMap;

    const UNINSTALL: &str =
        r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

    #[test]
    fn products_map_uid_to_launch_code() {
        for (uid, code) in [
            ("prometheus", "Pro"),
            ("hs_beta", "WTCG"),
            ("diablo3_enus", "D3"),
            ("fenris", "Fen"),
        ] {
            assert_eq!(find(uid).unwrap().launch_code, code);
        }
        for uid in ["wow_classic", "wow_beta", "desconhecido", "wow_çççç"] {
            assert_eq!(find(uid), None);
        }
    }

    #[test]
    fn source_launches_through_client_and_opens_client_for_unknown_games() {
        let client = r"C:\Program Files (x86)\Battle.net";
        let reg = MemoryRegistry::default()
            .with(&format!(r"{UNINSTALL}\Battle.net"), "InstallLocation", client)
            .with(
                &format!(r"{UNINSTALL}\Battle.net"),
                "UninstallString",
                r#""C:\ProgramData\Battle.net\Agent\Blizzard Uninstaller.exe" --lang=enUS --uid=battle.net"#,
            )
            .with(&format!(r"{UNINSTALL}\Classic"), "DisplayName", "World of Warcraft Classic")
            .with(
                &format!(r"{UNINSTALL}\Classic"),
                "UninstallString",
                r#""C:\ProgramData\Battle.net\Agent\Blizzard Uninstaller.exe" --lang=enUS --uid=wow_classic --displayname="World of Warcraft Classic""#,
            );
        let data = db(&[
            install("prometheus", "pro", r"C:\Jogos\Overwatch", Some(true)),
            install(
                "wow_classic",
                "wow_classic",
                r"C:\Jogos\WoW Classic",
                Some(true),
            ),
            install("battle.net", "bna", client, Some(true)),
            install("fenris", "fen", r"D:\Diablo IV", Some(false)),
        ]);
        let exe = format!(r"{client}\Battle.net.exe");
        let mut source = BattleNetLibrarySource::new(Arc::new(reg), r"C:\ProgramData", None);
        source.fs = fake::with(
            &[
                r"C:\Jogos\Overwatch",
                r"C:\Jogos\WoW Classic",
                client,
                r"D:\Diablo IV",
                &exe,
            ],
            HashMap::from([(
                r"C:\ProgramData\Battle.net\Agent\product.db".to_owned(),
                data,
            )]),
        );

        let scan = source.scan();

        assert!(scan.client_installed);
        let titles: Vec<_> = scan.games.iter().map(|g| g.title.as_str()).collect();
        assert_eq!(titles, ["Overwatch 2", "World of Warcraft Classic"]);
        let ow = &scan.games[0];
        assert_eq!(ow.id, "battlenet:prometheus");
        assert_eq!(ow.launch_target.as_deref(), Some(exe.as_str()));
        assert_eq!(
            ow.launch_arguments.as_deref(),
            Some("--exec=\"launch Pro\"")
        );
        assert_eq!(ow.install_path.as_deref(), Some(r"C:\Jogos\Overwatch"));
        let classic = &scan.games[1];
        assert_eq!(classic.launch_target.as_deref(), Some(exe.as_str()));
        assert_eq!(classic.launch_arguments, None);
    }

    #[test]
    fn falls_back_to_uninstall_keys_without_product_db() {
        let reg = MemoryRegistry::default()
            .with(&format!(r"{UNINSTALL}\Hearthstone"), "DisplayName", "Hearthstone")
            .with(&format!(r"{UNINSTALL}\Hearthstone"), "InstallLocation", r"E:\Hearthstone")
            .with(
                &format!(r"{UNINSTALL}\Hearthstone"),
                "UninstallString",
                r#""C:\ProgramData\Battle.net\Agent\Blizzard Uninstaller.exe" --lang=ptBR --uid=hs_beta --displayname="Hearthstone""#,
            )
            .with(&format!(r"{UNINSTALL}\PTR"), "DisplayName", "Overwatch Test")
            .with(&format!(r"{UNINSTALL}\PTR"), "InstallLocation", r"E:\PTR")
            .with(&format!(r"{UNINSTALL}\PTR"), "UninstallString", "Battle.net Uninstaller.exe --uid=prometheus_test");
        let mut source = BattleNetLibrarySource::new(Arc::new(reg), r"C:\ProgramData", None);
        source.fs = fake::folders(&[r"E:\Hearthstone", r"E:\PTR"]);

        let scan = source.scan();

        assert_eq!(scan.games.len(), 1);
        assert_eq!(scan.games[0].title, "Hearthstone");
        assert!(!scan.client_installed);
        assert_eq!(
            scan.notice.as_deref(),
            Some("Battle.net não encontrado: instale-o para abrir estes jogos")
        );
        assert_eq!(
            scan.games[0].launch_target.as_deref(),
            Some("battlenet://WTCG")
        );
    }

    #[test]
    fn unreadable_product_db_uses_registry() {
        let mut source = BattleNetLibrarySource::new(
            Arc::new(MemoryRegistry::default()),
            r"C:\ProgramData",
            None,
        );
        source.fs = StoreFs {
            exists: Arc::new(|_| false),
            read: Arc::new(|_| Some(vec![0x0A, 0xFF])),
        };
        let scan = source.scan();
        assert!(scan.games.is_empty());
        assert_eq!(
            scan.notice.as_deref(),
            Some("Battle.net não encontrado neste PC")
        );
    }
}
