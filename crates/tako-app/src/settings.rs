//! Preferências públicas da ponte. A lista positiva exclui inclusive segredos legados no SQLite.

use crate::{Core, protocol::*};
use tako_core::metadata::igdb::IgdbSettings;

// A chave do Slint vive no cofre: a ponte só expõe a escolha, nunca o JSON com as credenciais.
const IGDB_KEY: &str = "metadata.igdb";

const BOOL_KEYS: &[&str] = &[
    "DynamicColor",
    "DesktopArtColors",
    "ReducedMotion",
    "ReducedTransparency",
    "RestoreEnvironmentAfterGame",
    "DeveloperOptions",
    "GameScreenEnabled",
    "GameScreenHdr",
    "GameScreenVrr",
    "HideCursorDuringGame",
    "ControllerMouseDuringGame",
    "SafeMode",
    "DiscordRichPresence",
    "SunshineSyncOnRescan",
    "FirstRunCompleted",
    "AskBigPictureWithController",
    "ThemeMusicEnabled",
    "HideControllerFromGame",
    "DownloadsEnabled",
    "DownloadsLegalAccepted",
    // Opções do Liquid Glass (as mesmas chaves do app Slint, `tako-ui/src/desk_theme.rs`).
    "DesktopGlassAmbient",
    "DesktopGlassPill",
    "CloseToTray",
];
const TEXT_KEYS: &[&str] = &[
    "GameScreenId",
    "GameScreenName",
    "GameScreenMode",
    "GameAudioDeviceId",
    "GameAudioDeviceName",
    "DisabledStores",
    "ControlCenterKey",
    "Theme",
    "GameScreenPresets",
    "GameScreenConfirmedSetup",
];

fn known(key: &str) -> bool {
    BOOL_KEYS.contains(&key)
        || TEXT_KEYS.contains(&key)
        || matches!(
            key,
            "Language"
                | "DesktopTheme"
                | "DesktopAccent"
                | "DesktopGlassOpacity"
                | "GameScreenHideOthers"
                | "DownloadsUploadLimitKiB"
                | "DesktopTitleLogo"
                | "ControlCenterChord"
                | "OpenIn"
                | "TextSizePercent"
                | "NavigationSoundVolume"
                | "ThemeMusicVolume"
                | "desktop.view"
                | "desktop.group_by"
                | IGDB_KEY
        )
}

pub(crate) fn get(core: &Core, request: SettingsGetRequest) -> Result<SettingsValues, BridgeError> {
    if let Some(keys) = &request.keys
        && (keys.len() > 64 || keys.iter().any(|key| !known(key)))
    {
        return Err(crate::organize::bad("chaves de configuração inválidas"));
    }
    let mut values = core
        .settings
        .iter()
        .filter(|(key, _)| {
            key.as_str() != IGDB_KEY
                && known(key)
                && request.keys.as_ref().is_none_or(|keys| keys.contains(key))
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    if request
        .keys
        .as_ref()
        .is_some_and(|keys| keys.iter().any(|key| key == IGDB_KEY))
    {
        let enabled = match &core.secrets {
            Some(secrets) => IgdbSettings::load(&Vault(secrets.as_ref())).enabled,
            None => core
                .settings
                .get(IGDB_KEY)
                .is_some_and(|value| value == "true"),
        };
        values.insert(IGDB_KEY.into(), enabled.to_string());
    }
    Ok(SettingsValues { values })
}

pub(crate) fn set(core: &mut Core, setting: Setting) -> Result<(), BridgeError> {
    let Setting { key, value } = setting;
    let valid = known(&key)
        && value.len() <= 4096
        && !value.contains('\0')
        && (!BOOL_KEYS.contains(&key.as_str()) || matches!(value.as_str(), "true" | "false"))
        && (key != IGDB_KEY || matches!(value.as_str(), "true" | "false"))
        && (key != "Language" || tako_core::i18n::Lang::from_code(&value).is_some())
        && (key != "DesktopTitleLogo" || matches!(value.as_str(), "" | "logo" | "text"))
        && (key != "OpenIn" || matches!(value.as_str(), "desktop" | "big-picture"))
        // Tema do modo desktop (o mesmo `DesktopTheme` que o app Slint grava); `Theme` é outra coisa.
        // O app Slint guarda `Disconnect` (desliga as outras telas, o padrão) ou `None`; não é chave de liga/desliga.
        && (key != "GameScreenHideOthers" || matches!(value.as_str(), "Disconnect" | "None"))
        // Limite de envio dos downloads: as escolhas do app atual (KiB/s), 0 = sem limite.
        && (key != "DownloadsUploadLimitKiB" || matches!(value.as_str(), "0" | "64" | "256" | "1024"))
        && (key != "DesktopTheme"
            || matches!(value.as_str(), "tako" | "dark" | "midnight" | "oled" | "glass"))
        // Modo de exibição da lista e agrupamento das Categorias: as chaves e valores que o app Slint grava.
        && (key != "desktop.view" || matches!(value.as_str(), "list" | "grid" | "categories"))
        && (key != "desktop.group_by"
            || matches!(
                value.as_str(),
                "status" | "collection" | "tag" | "genre" | "store" | "install"
            ))
        // Cor de destaque do desktop: `#RRGGBB`, como o Slint grava (`Rgb::to_string`, maiúsculas).
        && (key != "DesktopAccent" || tako_core::presentation::Rgb::parse(&value).is_some())
        // Opacidade das superfícies do vidro: posição do deslizante, 0 a 100.
        && (key != "DesktopGlassOpacity" || value.parse::<u32>().is_ok_and(|n| n <= 100))
        && (key != "ControlCenterChord" || value.parse::<u32>().is_ok())
        && (!matches!(key.as_str(), "NavigationSoundVolume" | "ThemeMusicVolume")
            || value.parse::<u32>().is_ok_and(|n| n <= 100))
        && (key != "TextSizePercent"
            || value
                .parse::<u32>()
                .is_ok_and(|n| [100, 110, 125].contains(&n)));
    if !valid {
        return Err(crate::organize::bad("configuração inválida"));
    }
    if key == IGDB_KEY {
        if let Some(secrets) = &core.secrets {
            let vault = Vault(secrets.as_ref());
            let mut settings = IgdbSettings::load(&vault);
            settings.enabled = value == "true";
            settings.save(&vault).map_err(|_| {
                BridgeError::new(
                    ErrorCode::Internal,
                    "não foi possível gravar a escolha da IGDB",
                )
            })?;
        } else {
            // Demonstração: só memória, sem SQLite nem cofre.
            core.settings.insert(key, value);
        }
        return Ok(());
    }
    save(core, &key, &value)?;
    if key == "Language"
        && let Some(lang) = tako_core::i18n::Lang::from_code(&value)
    {
        core.lang = lang;
    }
    Ok(())
}

struct Vault<'a>(&'a dyn tako_platform::secrets::Secrets);

impl tako_core::metadata::SecretStore for Vault<'_> {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key)
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.0.set(key, value)
    }
}

pub(crate) fn save(core: &mut Core, key: &str, value: &str) -> Result<(), BridgeError> {
    if let Some(store) = &core.settings_store {
        store.set(key, value).map_err(crate::organize::internal)?;
    }
    core.settings.insert(key.into(), value.into());
    Ok(())
}
