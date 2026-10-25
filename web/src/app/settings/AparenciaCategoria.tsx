import { useMemo } from "react";
import type { Appearance } from "../../appearance/appearance";
import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { changeAppearance } from "./changeAppearance";
import { LinhaChave, LinhaSelecao, type CategoryProps } from "./Linhas";
import {
  DEFAULTS,
  KEYS,
  THEME_CHOICES,
  UI_SCALE_CHOICES,
  appearanceFromSettings,
  boolText,
  readBool,
  themeToStored,
} from "./model";

/**
 * Configurações › Aparência. Cada mudança vale na hora (`setAppearance`) e é gravada pela ponte; se a gravação
 * falhar, a página volta à aparência que ficou nas configurações.
 */
export function AparenciaCategoria({ values, save, read }: CategoryProps) {
  const t = useT();
  const appearance = appearanceFromSettings(values);
  const artColors = readBool(values, KEYS.artColors, DEFAULTS.artColors);

  function change(patch: Partial<Appearance>, key: string, value: string) {
    changeAppearance({ save, read }, patch, key, value);
  }

  const themeOptions = useMemo(
    () =>
      THEME_CHOICES.map(({ id }) => {
        let label: string = id;
        switch (id) {
          case "tako":
            label = t("settings.appearance.desk_theme_tako");
            break;
          case "escuro":
            label = t("settings.appearance.desk_theme_dark");
            break;
          case "meia-noite":
            label = t("settings.appearance.desk_theme_midnight");
            break;
          case "oled":
            label = t("settings.appearance.desk_theme_oled");
            break;
          case "vidro":
            label = t("settings.appearance.desk_theme_glass");
            break;
        }
        return { value: id, label };
      }),
    [t],
  );
  const scaleOptions = useMemo(
    () => UI_SCALE_CHOICES.map((scale) => ({ value: String(scale), label: `${String(scale)}%` })),
    [],
  );

  return (
    <>
      <GrupoDeConfiguracao title={t("settings.appearance_web.theme_section")}>
        <LinhaSelecao
          title={t("settings.appearance_web.theme_row_title")}
          description={t("settings.appearance_web.theme_row_desc")}
          options={themeOptions}
          value={appearance.theme}
          onChange={(theme) => {
            change({ theme }, KEYS.theme, themeToStored(theme));
          }}
        />
        <LinhaChave
          title={t("settings.appearance.art_colors")}
          description={
            appearance.theme === "tako"
              ? t("settings.appearance.art_colors_desc")
              : `${t("settings.appearance.art_colors_desc")}. ${t("settings.appearance_web.art_colors_only_tako")}`
          }
          checked={artColors}
          disabled={appearance.theme !== "tako"}
          onChange={(on) => void save(KEYS.artColors, boolText(on))}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={t("settings.appearance_web.layout_section")}>
        <LinhaSelecao
          title={t("settings.appearance_web.scale_title")}
          description={t("settings.appearance_web.scale_desc")}
          options={scaleOptions}
          value={String(appearance.uiScale)}
          onChange={(value) => {
            const uiScale = UI_SCALE_CHOICES.find((scale) => String(scale) === value);
            if (uiScale) change({ uiScale }, KEYS.textSize, value);
          }}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={t("settings.appearance_web.effects_section")}>
        <LinhaChave
          title={t("settings.appearance.motion")}
          description={t("settings.appearance.motion_desc")}
          checked={appearance.reduceMotion}
          onChange={(on) => {
            change({ reduceMotion: on }, KEYS.reducedMotion, boolText(on));
          }}
        />
        <LinhaChave
          title={t("settings.appearance_web.glass_title")}
          description={t("settings.appearance_web.glass_desc")}
          checked={appearance.glassEffects}
          onChange={(on) => {
            // "Efeitos de vidro" ligado = `ReducedTransparency` desligado.
            change({ glassEffects: on }, KEYS.reducedTransparency, boolText(!on));
          }}
        />
      </GrupoDeConfiguracao>
    </>
  );
}
