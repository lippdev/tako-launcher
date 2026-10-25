//! Preferências públicas da ponte. A lista positiva exclui inclusive segredos legados no SQLite.

use crate::{Core, protocol::*};

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
        )
}

pub(crate) fn get(core: &Core, request: SettingsGetRequest) -> Result<SettingsValues, BridgeError> {
    if let Some(keys) = &request.keys
        && (keys.len() > 64 || keys.iter().any(|key| !known(key)))
    {
        return Err(crate::organize::bad("chaves de configuração inválidas"));
    }
    Ok(SettingsValues {
        values: core
            .settings
            .iter()
            .filter(|(key, _)| {
                known(key) && request.keys.as_ref().is_none_or(|keys| keys.contains(key))
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    })
}

pub(crate) fn set(core: &mut Core, setting: Setting) -> Result<(), BridgeError> {
    let Setting { key, value } = setting;
    let valid = known(&key)
        && value.len() <= 4096
        && !value.contains('\0')
        && (!BOOL_KEYS.contains(&key.as_str()) || matches!(value.as_str(), "true" | "false"))
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
    save(core, &key, &value)?;
    if key == "Language"
        && let Some(lang) = tako_core::i18n::Lang::from_code(&value)
    {
        core.lang = lang;
    }
    Ok(())
}

pub(crate) fn save(core: &mut Core, key: &str, value: &str) -> Result<(), BridgeError> {
    if let Some(store) = &core.settings_store {
        store.set(key, value).map_err(crate::organize::internal)?;
    }
    core.settings.insert(key.into(), value.into());
    Ok(())
}
