//! Regras puras da fonte Xbox (port de `XboxGameRules`): o que é jogo, o que é app do sistema, cor e
//! textos para o usuário.

use super::manifests::AppxManifest;
use super::package::{family_from_full_name, name_of};
use std::collections::BTreeSet;
use tako_core::i18n::{t, tf, tn};

/// O que o Windows tem para os jogos do app Xbox funcionarem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XboxAvailability {
    /// App Xbox e Serviços de Jogos presentes.
    Ready,
    /// Sem o app Xbox: o PC Game Pass não instala nem atualiza jogos.
    NoXboxApp,
    /// Sem os Serviços de Jogos: jogos do GDK não abrem.
    NoGamingServices,
    /// Não foi possível ler os pacotes do usuário.
    Unknown,
}

/// Cor da marca Xbox (token `store-xbox`).
pub const BRAND_COLOR: &str = "#107C10";
pub const XBOX_APP_FAMILY: &str = "Microsoft.GamingApp_8wekyb3d8bbwe";
/// Configuração do Xbox Live em jogos UWP antigos (antes do GDK).
pub const XBOX_SERVICES_CONFIG: &str = "xboxservices.config";

// Pacotes do próprio ecossistema Xbox/Store que não são jogos (alguns têm configuração do Xbox Live).
const SYSTEM_PACKAGES: [&str; 12] = [
    "Microsoft.GamingApp",
    "Microsoft.XboxApp",
    "Microsoft.GamingServices",
    "Microsoft.XboxGamingOverlay",
    "Microsoft.XboxGameOverlay",
    "Microsoft.XboxIdentityProvider",
    "Microsoft.Xbox.TCUI",
    "Microsoft.XboxSpeechToTextOverlay",
    "Microsoft.WindowsStore",
    "Microsoft.StorePurchaseApp",
    "Microsoft.XboxGameCallableUI",
    "Microsoft.XboxDevices",
];

pub fn is_system_package(family_or_name: &str) -> bool {
    let name = name_of(family_or_name);
    SYSTEM_PACKAGES.iter().any(|p| p.eq_ignore_ascii_case(name))
}

/// Jogo = tem `MicrosoftGame.config`, `xboxservices.config` ou os Serviços de Jogos o registram;
/// nunca framework, recursos ou apps do próprio Xbox.
pub fn is_game(
    family: &str,
    has_game_config: bool,
    has_xbox_services_config: bool,
    known_to_gaming_services: bool,
    manifest: Option<&AppxManifest>,
) -> bool {
    if is_system_package(family)
        || manifest.is_some_and(|m| {
            m.is_framework_or_resource || (m.main_application().is_none() && !has_game_config)
        })
    {
        return false;
    }
    has_game_config || has_xbox_services_config || known_to_gaming_services
}

/// Família de cada entrada da lista dos Serviços de Jogos (nomes completos ou famílias), em minúsculas.
pub fn families<'a>(full_or_family_names: impl IntoIterator<Item = &'a str>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in full_or_family_names {
        if let Some(f) = family_from_full_name(entry) {
            out.insert(f.to_lowercase());
        } else if let [name, id] = entry.split('_').collect::<Vec<_>>()[..]
            && !name.is_empty()
            && id.len() == 13
        {
            out.insert(entry.to_lowercase());
        }
    }
    out
}

/// Cor de fundo do manifesto (#RRGGBB) como destaque; "transparent" ou inválida → cor do Xbox.
pub fn accent(colors: &[Option<&str>]) -> String {
    colors
        .iter()
        .flatten()
        .find(|c| {
            c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit())
        })
        .map_or_else(|| BRAND_COLOR.to_owned(), |c| c.to_ascii_uppercase())
}

/// Nome legível quando o manifesto não dá um: pasta do jogo ou o fim do nome do pacote.
pub fn fallback_title(family: &str, game_folder_name: Option<&str>) -> String {
    if let Some(f) = game_folder_name
        .map(str::trim)
        .filter(|f| !f.is_empty() && !f.eq_ignore_ascii_case("Content"))
    {
        return f.to_owned();
    }
    let name = name_of(family);
    match name.rfind('.') {
        Some(i) if i < name.len() - 1 => name[i + 1..].to_owned(),
        _ => name.to_owned(),
    }
}

pub fn availability<'a>(
    user_families: Option<impl IntoIterator<Item = &'a str>>,
    gaming_services_installed: bool,
) -> XboxAvailability {
    let Some(mut families) = user_families.map(IntoIterator::into_iter) else {
        return XboxAvailability::Unknown;
    };
    if !families.any(|f| f.eq_ignore_ascii_case(XBOX_APP_FAMILY)) {
        XboxAvailability::NoXboxApp
    } else if gaming_services_installed {
        XboxAvailability::Ready
    } else {
        XboxAvailability::NoGamingServices
    }
}

/// Aviso quando falta algo para os jogos do Xbox funcionarem.
pub fn notice(a: XboxAvailability) -> Option<String> {
    match a {
        XboxAvailability::NoXboxApp => Some(t("sources.xbox.no_app")),
        XboxAvailability::NoGamingServices => Some(t("sources.xbox.no_gaming_services")),
        XboxAvailability::Unknown => Some(t("sources.xbox.unknown")),
        XboxAvailability::Ready => None,
    }
}

/// Linha de ajuda nas Configurações.
pub fn describe(enabled: bool, a: XboxAvailability, games: usize) -> String {
    if !enabled {
        return t("sources.xbox.disabled");
    }
    let found = match games {
        0 => t("sources.xbox.found_none"),
        n => tn("sources.xbox.found", n as i64, &[]),
    };
    match notice(a) {
        Some(n) => format!("{n} ({found})"),
        None => tf("sources.xbox.ready", &[("found", &found)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xbox::manifests::tests::fixture;

    #[test]
    fn games_are_told_apart_from_apps() {
        let game = AppxManifest::parse(&fixture("AppxManifest.xml")).unwrap();
        let app = AppxManifest::parse(&fixture("AppxManifest-App.xml")).unwrap();
        assert!(is_game(
            "Contoso.ContosoRacing_x",
            true,
            false,
            false,
            Some(&game)
        ));
        assert!(is_game("Contoso.ContosoRacing_x", true, false, false, None));
        assert!(is_game(
            "Fabrikam.PuzzleQuest_x",
            false,
            true,
            false,
            Some(&app)
        ));
        assert!(is_game("Fabrikam.Other_x", false, false, true, Some(&app)));
        assert!(!is_game(
            "Fabrikam.NotesApp_x",
            false,
            false,
            false,
            Some(&app)
        ));
        // Apps do próprio Xbox têm configuração do Xbox Live, mas não são jogos.
        assert!(!is_game(
            "Microsoft.GamingApp_8wekyb3d8bbwe",
            false,
            true,
            true,
            Some(&app)
        ));
        assert!(!is_game(
            "Microsoft.XboxGamingOverlay_8wekyb3d8bbwe",
            true,
            true,
            true,
            Some(&game)
        ));
        // Sem app visível e sem MicrosoftGame.config: nada para abrir.
        let empty = AppxManifest {
            applications: vec![],
            ..game.clone()
        };
        assert!(!is_game(
            "Fabrikam.Service_x",
            false,
            true,
            true,
            Some(&empty)
        ));
    }

    #[test]
    fn rules_give_color_title_and_availability() {
        assert_eq!(accent(&[Some("#1e3a5f")]), "#1E3A5F");
        assert_eq!(
            accent(&[Some("transparent"), None, Some("#1E3A5F")]),
            "#1E3A5F"
        );
        assert_eq!(
            accent(&[
                Some("transparent"),
                Some("#12"),
                Some("#GGGGGG"),
                Some("#ççç")
            ]),
            BRAND_COLOR
        );

        assert_eq!(
            fallback_title("Contoso.ContosoRacing_x", Some("Contoso Racing")),
            "Contoso Racing"
        );
        assert_eq!(
            fallback_title("Contoso.ContosoRacing_x", Some("Content")),
            "ContosoRacing"
        );
        assert_eq!(fallback_title("Solo_x", None), "Solo");

        let f = families([
            "Contoso.Racing_1.0.0.0_x64__abcdefghjkmnp",
            "Fabrikam.Quest_abcdefghjkmnp",
            "lixo",
            "a_b",
        ]);
        assert_eq!(
            f.into_iter().collect::<Vec<_>>(),
            [
                "contoso.racing_abcdefghjkmnp",
                "fabrikam.quest_abcdefghjkmnp"
            ]
        );

        let ready = Some([XBOX_APP_FAMILY]);
        assert_eq!(availability(ready, true), XboxAvailability::Ready);
        assert_eq!(
            availability(ready, false),
            XboxAvailability::NoGamingServices
        );
        assert_eq!(
            availability(Some([""; 0]), true),
            XboxAvailability::NoXboxApp
        );
        assert_eq!(
            availability(None::<[&str; 0]>, true),
            XboxAvailability::Unknown
        );

        assert_eq!(
            describe(true, XboxAvailability::Ready, 2),
            "PC Game Pass e Microsoft Store · 2 jogos encontrados"
        );
        assert!(
            describe(true, XboxAvailability::NoXboxApp, 0).starts_with("App Xbox não encontrado")
        );
        assert!(
            describe(true, XboxAvailability::NoGamingServices, 1)
                .starts_with("Serviços de Jogos não encontrados")
        );
        assert!(describe(false, XboxAvailability::NoXboxApp, 0).starts_with("Desligado"));
    }
}
