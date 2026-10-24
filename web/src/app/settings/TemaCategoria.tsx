import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { settingsStrings } from "../strings";
import { CartaoDeTema } from "./CartaoDeTema";
import { changeAppearance } from "./changeAppearance";
import { LinhaChave, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, THEME_CHOICES, appearanceFromSettings, boolText, readBool, themeToStored } from "./model";
import styles from "./TemaCategoria.module.css";

const s = settingsStrings.theme;
const names = settingsStrings.appearance.themes;
const artColors = settingsStrings.appearance.artColors;

/**
 * Configurações › Tema: os cinco temas em cartões e, no TAKO, "Cores da arte do jogo". É o mesmo `DesktopTheme` do
 * seletor de Aparência, então os dois mostram sempre o mesmo valor. A cor de destaque e as opções do Liquid Glass
 * do design não têm chave no núcleo e ficam de fora.
 */
export function TemaCategoria({ values, save, read }: CategoryProps) {
  const { theme } = appearanceFromSettings(values);
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
    </>
  );
}
