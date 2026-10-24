import type { Appearance } from "../../appearance/appearance";
import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { settingsStrings } from "../strings";
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

const s = settingsStrings.appearance;

const THEME_OPTIONS = THEME_CHOICES.map(({ id }) => ({ value: id, label: s.themes[id] }));
const SCALE_OPTIONS = UI_SCALE_CHOICES.map((scale) => ({ value: String(scale), label: `${String(scale)}%` }));

/**
 * Configurações › Aparência. Cada mudança vale na hora (`setAppearance`) e é gravada pela ponte; se a gravação
 * falhar, a página volta à aparência que ficou nas configurações.
 */
export function AparenciaCategoria({ values, save, read }: CategoryProps) {
  const appearance = appearanceFromSettings(values);
  const artColors = readBool(values, KEYS.artColors, DEFAULTS.artColors);

  function change(patch: Partial<Appearance>, key: string, value: string) {
    changeAppearance({ save, read }, patch, key, value);
  }

  return (
    <>
      <GrupoDeConfiguracao title={s.theme}>
        <LinhaSelecao
          title={s.themeRow.title}
          description={s.themeRow.description}
          options={THEME_OPTIONS}
          value={appearance.theme}
          onChange={(theme) => {
            change({ theme }, KEYS.theme, themeToStored(theme));
          }}
        />
        <LinhaChave
          title={s.artColors.title}
          description={appearance.theme === "tako" ? s.artColors.description : `${s.artColors.description}. ${s.artColors.onlyTako}`}
          checked={artColors}
          disabled={appearance.theme !== "tako"}
          onChange={(on) => void save(KEYS.artColors, boolText(on))}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={s.layout}>
        <LinhaSelecao
          title={s.scale.title}
          description={s.scale.description}
          options={SCALE_OPTIONS}
          value={String(appearance.uiScale)}
          onChange={(value) => {
            const uiScale = UI_SCALE_CHOICES.find((scale) => String(scale) === value);
            if (uiScale) change({ uiScale }, KEYS.textSize, value);
          }}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={s.effects}>
        <LinhaChave
          title={s.motion.title}
          description={s.motion.description}
          checked={appearance.reduceMotion}
          onChange={(on) => {
            change({ reduceMotion: on }, KEYS.reducedMotion, boolText(on));
          }}
        />
        <LinhaChave
          title={s.glass.title}
          description={s.glass.description}
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
