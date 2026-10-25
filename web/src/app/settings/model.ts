// Formato das configurações que a tela mexe: as chaves do núcleo (`crates/tako-app/src/settings.rs`), os valores
// guardados e a leitura com o padrão de cada uma. Só dados e funções puras (sem importações em execução), para o
// teste rodar direto no Node.

import type { Appearance, Theme, UiScale } from "../../appearance/appearance";
import type { Store } from "../../bridge/generated/Store";

/** Os valores devolvidos por `settings.get`: chave → texto. Chave ausente vale o padrão. */
export type SettingsMap = Readonly<Record<string, string>>;

/** As chaves públicas que a tela usa. Qualquer outra dá `bad_request` na ponte. */
export const KEYS = {
  language: "Language",
  openIn: "OpenIn",
  askBigPicture: "AskBigPictureWithController",
  restoreEnvironment: "RestoreEnvironmentAfterGame",
  theme: "DesktopTheme",
  artColors: "DesktopArtColors",
  accent: "DesktopAccent",
  glassAmbient: "DesktopGlassAmbient",
  glassOpacity: "DesktopGlassOpacity",
  glassPill: "DesktopGlassPill",
  reducedMotion: "ReducedMotion",
  reducedTransparency: "ReducedTransparency",
  textSize: "TextSizePercent",
  disabledStores: "DisabledStores",
  navigationVolume: "NavigationSoundVolume",
  themeMusic: "ThemeMusicEnabled",
  themeMusicVolume: "ThemeMusicVolume",
  screenEnabled: "GameScreenEnabled",
  screenHideOthers: "GameScreenHideOthers",
  screenId: "GameScreenId",
  screenName: "GameScreenName",
  screenMode: "GameScreenMode",
  screenHdr: "GameScreenHdr",
  screenVrr: "GameScreenVrr",
  audioId: "GameAudioDeviceId",
  audioName: "GameAudioDeviceName",
  hideCursor: "HideCursorDuringGame",
  controllerMouse: "ControllerMouseDuringGame",
  discord: "DiscordRichPresence",
  sunshineSync: "SunshineSyncOnRescan",
  developer: "DeveloperOptions",
  downloadsEnabled: "DownloadsEnabled",
  downloadsAccepted: "DownloadsLegalAccepted",
  downloadsUpload: "DownloadsUploadLimitKiB",
} as const;

/** As chaves que a aparência salva depende (lidas ao abrir o app). */
export const APPEARANCE_KEYS: readonly string[] = [
  KEYS.theme,
  KEYS.accent,
  KEYS.glassAmbient,
  KEYS.glassOpacity,
  KEYS.glassPill,
  KEYS.reducedMotion,
  KEYS.reducedTransparency,
  KEYS.textSize,
];

/** Padrões de quando a chave ainda não foi gravada (iguais aos do núcleo antigo). */
export const DEFAULTS = {
  askBigPicture: true,
  restoreEnvironment: true,
  artColors: true,
  glassAmbient: true,
  glassPill: true,
  /** Igual a `GLASS_OPACITY_DEFAULT` de `desktopLook.ts` (este arquivo não importa nada em execução). */
  glassOpacity: 28,
  reducedMotion: false,
  reducedTransparency: false,
  themeMusic: true,
  screenEnabled: false,
  screenHdr: false,
  screenVrr: false,
  hideCursor: false,
  controllerMouse: false,
  discord: false,
  sunshineSync: false,
  developer: false,
  /** Os downloads vêm desligados até o usuário aceitar o aviso legal. */
  downloadsEnabled: false,
  downloadsAccepted: false,
  /** Os sons de navegação vêm desligados; a música do tema, a 70%. */
  navigationVolume: 0,
  themeMusicVolume: 70,
} as const;

export function readBool(values: SettingsMap, key: string, fallback: boolean): boolean {
  const value = values[key];
  return value === "true" ? true : value === "false" ? false : fallback;
}

export function boolText(value: boolean): string {
  return value ? "true" : "false";
}

/** Um número de 0 a 100 guardado como texto; ausente ou fora da faixa vale o padrão. */
export function readPercent(values: SettingsMap, key: string, fallback: number): number {
  const text = values[key];
  if (text === undefined || !/^\d{1,3}$/.test(text)) return fallback;
  const value = Number(text);
  return value <= 100 ? value : fallback;
}

/** `#RRGGBB` (o Slint só aceita assim) em maiúsculas, que é como ele grava; qualquer outra coisa dá `null`. */
export function parseAccent(text: string | undefined): string | null {
  const clean = (text ?? "").trim();
  return /^#[0-9a-fA-F]{6}$/.test(clean) ? clean.toUpperCase() : null;
}

// ---- Idioma e "Abrir em" ----

export const LANGUAGES = ["pt-BR", "en-US"] as const;
export type Language = (typeof LANGUAGES)[number];

/** O núcleo aceita qualquer código que comece por `pt` ou `en`; a tela mostra os dois idiomas do Tako. */
export function readLanguage(values: SettingsMap): Language {
  const code = (values[KEYS.language] ?? "").trim().toLowerCase().replace("_", "-");
  return code.startsWith("en") ? "en-US" : "pt-BR";
}

export const OPEN_IN = ["desktop", "big-picture"] as const;
export type OpenIn = (typeof OPEN_IN)[number];

export function readOpenIn(values: SettingsMap): OpenIn {
  return values[KEYS.openIn] === "big-picture" ? "big-picture" : "desktop";
}

// ---- Aparência ----

/** O tema como o núcleo guarda (`Theme`) e como a interface chama. A ordem é a dos cartões do design system. */
export const THEME_CHOICES: readonly { id: Theme; stored: string }[] = [
  { id: "tako", stored: "tako" },
  { id: "escuro", stored: "dark" },
  { id: "meia-noite", stored: "midnight" },
  { id: "oled", stored: "oled" },
  { id: "vidro", stored: "glass" },
];

export function themeFromStored(stored: string | undefined): Theme {
  return THEME_CHOICES.find((choice) => choice.stored === stored)?.id ?? "tako";
}

export function themeToStored(theme: Theme): string {
  return THEME_CHOICES.find((choice) => choice.id === theme)?.stored ?? "tako";
}

/**
 * Tamanhos que o núcleo aceita em `TextSizePercent` (100, 110 e 125). O 90% do design system não passa pela
 * validação do núcleo (`settings.rs`), então a tela não o oferece.
 */
export const UI_SCALE_CHOICES: readonly UiScale[] = [100, 110, 125];

export function readUiScale(values: SettingsMap): UiScale {
  const value = Number(values[KEYS.textSize]);
  return UI_SCALE_CHOICES.find((scale) => scale === value) ?? 100;
}

/** A aparência que as configurações guardadas descrevem. "Efeitos de vidro" é o `ReducedTransparency` invertido. */
export function appearanceFromSettings(values: SettingsMap): Appearance {
  return {
    theme: themeFromStored(values[KEYS.theme]),
    reduceMotion: readBool(values, KEYS.reducedMotion, DEFAULTS.reducedMotion),
    glassEffects: !readBool(values, KEYS.reducedTransparency, DEFAULTS.reducedTransparency),
    uiScale: readUiScale(values),
    // Cor ilegível vale a do tema; as opções do vidro seguem os padrões do Slint.
    accent: parseAccent(values[KEYS.accent]),
    glassOpacity: readPercent(values, KEYS.glassOpacity, DEFAULTS.glassOpacity),
    glassPill: readBool(values, KEYS.glassPill, DEFAULTS.glassPill),
    glassAmbient: readBool(values, KEYS.glassAmbient, DEFAULTS.glassAmbient),
  };
}

// ---- Lojas ----

/** O nome da loja em `DisabledStores` (o `GameStore.ToString()` das alphas). A comparação ignora maiúsculas. */
const STORE_KEYS: Readonly<Record<Store, string>> = {
  steam: "Steam",
  epic: "Epic",
  gog: "Gog",
  xbox: "Xbox",
  ubisoft: "Ubisoft",
  ea: "Ea",
  battle_net: "BattleNet",
  amazon: "Amazon",
  heroic: "Heroic",
  lutris: "Lutris",
  bottles: "Bottles",
  itch: "Itch",
  emulators: "Emulators",
  manual: "Manual",
};

function disabledList(text: string | undefined): string[] {
  return (text ?? "")
    .split(",")
    .map((name) => name.trim())
    .filter((name) => name.length > 0);
}

export function isStoreDisabled(disabled: string | undefined, store: Store): boolean {
  const key = STORE_KEYS[store].toLowerCase();
  return disabledList(disabled).some((name) => name.toLowerCase() === key);
}

/** O novo `DisabledStores` com a loja ligada ou desligada; as outras lojas (e nomes que não conhecemos) ficam. */
export function withStoreEnabled(disabled: string | undefined, store: Store, enabled: boolean): string {
  const key = STORE_KEYS[store];
  const rest = disabledList(disabled).filter((name) => name.toLowerCase() !== key.toLowerCase());
  return (enabled ? rest : [...rest, key]).join(",");
}

// ---- Tela e áudio do jogo ----

/**
 * "Desligar as outras telas durante o jogo". O núcleo e o app Slint guardam `Disconnect` / `None` (padrão: ligado);
 * só `None` vale como desligado.
 */
export function readHideOthers(values: SettingsMap): boolean {
  const value = values[KEYS.screenHideOthers];
  return value !== "None";
}

/** O valor que o núcleo aceita para a chave acima. */
export function hideOthersText(on: boolean): "Disconnect" | "None" {
  return on ? "Disconnect" : "None";
}

/** "1920 × 1080 · 144 Hz" para a chave `1920x1080@144` do núcleo; chave que não é desse formato aparece como veio. */
export function modeLabel(key: string): string {
  const match = /^(\d+)x(\d+)@(\d+)$/.exec(key);
  return match ? `${match[1] ?? ""} × ${match[2] ?? ""} · ${match[3] ?? ""} Hz` : key;
}
