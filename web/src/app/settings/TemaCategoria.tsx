import { glassAlpha } from "../../appearance/desktopLook";
import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { CartaoDeTema } from "./CartaoDeTema";
import { changeAppearance } from "./changeAppearance";
import { DestaqueDoDesktop } from "./DestaqueDoDesktop";
import { LinhaChave, LinhaDeslizante, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, THEME_CHOICES, appearanceFromSettings, boolText, readBool, themeToStored } from "./model";
import styles from "./TemaCategoria.module.css";

/**
 * Configurações › Tema (`settings_desk_theme` do Slint): os cinco temas em cartões e, conforme o escolhido, a cor de
 * destaque (fora do TAKO, onde o destaque vem da arte do jogo), "Cores da arte do jogo" (TAKO) ou as opções do Liquid
 * Glass. É o mesmo `DesktopTheme` do seletor de Aparência, então os dois mostram sempre o mesmo valor. A cor e o
 * vidro valem na hora (variáveis do tema) e são gravados com as mesmas chaves do Slint.
 */
export function TemaCategoria({ values, save, read }: CategoryProps) {
  const t = useT();
  const appearance = appearanceFromSettings(values);
  const { theme } = appearance;

  const themeName = (id: string) => {
    switch (id) {
      case "tako":
        return t("settings.appearance.desk_theme_tako");
      case "escuro":
        return t("settings.appearance.desk_theme_dark");
      case "meia-noite":
        return t("settings.appearance.desk_theme_midnight");
      case "oled":
        return t("settings.appearance.desk_theme_oled");
      case "vidro":
        return t("settings.appearance.desk_theme_glass");
      default:
        return id;
    }
  };

  const themeDesc = (id: string) => {
    switch (id) {
      case "tako":
        return t("settings.theme.desc_tako");
      case "escuro":
        return t("settings.theme.desc_dark");
      case "meia-noite":
        return t("settings.theme.desc_midnight");
      case "oled":
        return t("settings.theme.desc_oled");
      case "vidro":
        return t("settings.theme.desc_glass");
      default:
        return "";
    }
  };

  return (
    <>
      <GrupoDeConfiguracao title={t("settings.theme.title")}>
        <div className={styles["grid"]} role="group" aria-label={t("settings.theme_web.cards")}>
          {THEME_CHOICES.map(({ id }) => (
            <CartaoDeTema
              key={id}
              theme={id}
              name={themeName(id)}
              description={themeDesc(id)}
              current={id === theme}
              onSelect={() => {
                if (id !== theme) changeAppearance({ save, read }, { theme: id }, KEYS.theme, themeToStored(id));
              }}
            />
          ))}
        </div>
      </GrupoDeConfiguracao>
      {theme !== "tako" && (
        <GrupoDeConfiguracao title={t("settings.theme_web.accent_section")}>
          <DestaqueDoDesktop
            value={appearance.accent}
            theme={theme}
            onChange={(hex) => {
              changeAppearance({ save, read }, { accent: hex }, KEYS.accent, hex);
            }}
          />
        </GrupoDeConfiguracao>
      )}
      {theme === "tako" && (
        <GrupoDeConfiguracao title={t("settings.theme_web.tako_options")}>
          <LinhaChave
            title={t("settings.appearance.art_colors")}
            description={t("settings.appearance.art_colors_desc")}
            checked={readBool(values, KEYS.artColors, DEFAULTS.artColors)}
            onChange={(on) => void save(KEYS.artColors, boolText(on))}
          />
        </GrupoDeConfiguracao>
      )}
      {theme === "vidro" && (
        <GrupoDeConfiguracao title={t("settings.theme_web.glass_section")}>
          <LinhaChave
            title={t("settings.appearance.glass_ambient")}
            description={t("settings.appearance.glass_ambient_desc")}
            checked={appearance.glassAmbient}
            onChange={(on) => {
              changeAppearance({ save, read }, { glassAmbient: on }, KEYS.glassAmbient, boolText(on));
            }}
          />
          <LinhaDeslizante
            title={t("settings.appearance.glass_opacity")}
            description={t("settings.appearance.glass_opacity_desc")}
            value={appearance.glassOpacity}
            format={(position) => `${String(glassAlpha(position))}%`}
            onCommit={(position) => {
              changeAppearance({ save, read }, { glassOpacity: position }, KEYS.glassOpacity, String(position));
            }}
          />
          <LinhaChave
            title={t("settings.appearance.glass_pill")}
            description={t("settings.appearance.glass_pill_desc")}
            checked={appearance.glassPill}
            onChange={(on) => {
              changeAppearance({ save, read }, { glassPill: on }, KEYS.glassPill, boolText(on));
            }}
          />
        </GrupoDeConfiguracao>
      )}
    </>
  );
}
