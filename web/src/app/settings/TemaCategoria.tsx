import { glassAlpha } from "../../appearance/desktopLook";
import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { settingsStrings } from "../strings";
import { CartaoDeTema } from "./CartaoDeTema";
import { changeAppearance } from "./changeAppearance";
import { DestaqueDoDesktop } from "./DestaqueDoDesktop";
import { LinhaChave, LinhaDeslizante, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, THEME_CHOICES, appearanceFromSettings, boolText, readBool, themeToStored } from "./model";
import styles from "./TemaCategoria.module.css";

const s = settingsStrings.theme;
const names = settingsStrings.appearance.themes;
const artColors = settingsStrings.appearance.artColors;
const glass = s.glass;

/**
 * Configurações › Tema (`settings_desk_theme` do Slint): os cinco temas em cartões e, conforme o escolhido, a cor de
 * destaque (fora do TAKO, onde o destaque vem da arte do jogo), "Cores da arte do jogo" (TAKO) ou as opções do Liquid
 * Glass. É o mesmo `DesktopTheme` do seletor de Aparência, então os dois mostram sempre o mesmo valor. A cor e o
 * vidro valem na hora (variáveis do tema) e são gravados com as mesmas chaves do Slint.
 */
export function TemaCategoria({ values, save, read }: CategoryProps) {
  const appearance = appearanceFromSettings(values);
  const { theme } = appearance;
  return (
    <>
      <GrupoDeConfiguracao title={s.section}>
        <div className={styles["grid"]} role="group" aria-label={s.cards}>
          {THEME_CHOICES.map(({ id }) => (
            <CartaoDeTema
              key={id}
              theme={id}
              name={names[id]}
              description={s.descriptions[id]}
              current={id === theme}
              onSelect={() => {
                if (id !== theme) changeAppearance({ save, read }, { theme: id }, KEYS.theme, themeToStored(id));
              }}
            />
          ))}
        </div>
      </GrupoDeConfiguracao>
      {theme !== "tako" && (
        <GrupoDeConfiguracao title={s.accent.section}>
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
        <GrupoDeConfiguracao title={s.takoOptions}>
          <LinhaChave
            title={artColors.title}
            description={artColors.description}
            checked={readBool(values, KEYS.artColors, DEFAULTS.artColors)}
            onChange={(on) => void save(KEYS.artColors, boolText(on))}
          />
        </GrupoDeConfiguracao>
      )}
      {theme === "vidro" && (
        <GrupoDeConfiguracao title={glass.section}>
          <LinhaChave
            title={glass.ambient.title}
            description={glass.ambient.description}
            checked={appearance.glassAmbient}
            onChange={(on) => {
              changeAppearance({ save, read }, { glassAmbient: on }, KEYS.glassAmbient, boolText(on));
            }}
          />
          <LinhaDeslizante
            title={glass.opacity.title}
            description={glass.opacity.description}
            value={appearance.glassOpacity}
            format={(position) => `${String(glassAlpha(position))}%`}
            onCommit={(position) => {
              changeAppearance({ save, read }, { glassOpacity: position }, KEYS.glassOpacity, String(position));
            }}
          />
          <LinhaChave
            title={glass.pill.title}
            description={glass.pill.description}
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
