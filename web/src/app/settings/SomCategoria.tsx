import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { settingsStrings } from "../strings";
import { LinhaChave, LinhaDeslizante, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, boolText, readBool, readPercent } from "./model";

const s = settingsStrings.sound;

/** "40%"; zero é "Mudo" (como no app atual). */
function volumeText(value: number): string {
  return value === 0 ? s.muted : `${String(value)}%`;
}

/** Configurações › Som: a música do tema e o volume dos sons de navegação. */
export function SomCategoria({ values, save }: CategoryProps) {
  const musicOn = readBool(values, KEYS.themeMusic, DEFAULTS.themeMusic);
  return (
    <>
      <GrupoDeConfiguracao title={s.music}>
        <LinhaChave
          title={s.musicEnabled.title}
          description={s.musicEnabled.description}
          checked={musicOn}
          onChange={(on) => void save(KEYS.themeMusic, boolText(on))}
        />
        <LinhaDeslizante
          title={s.musicVolume.title}
          description={s.musicVolume.description}
          value={readPercent(values, KEYS.themeMusicVolume, DEFAULTS.themeMusicVolume)}
          format={volumeText}
          disabled={!musicOn}
          onCommit={(value) => void save(KEYS.themeMusicVolume, String(value))}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={s.navigation}>
        <LinhaDeslizante
          title={s.navigationVolume.title}
          description={s.navigationVolume.description}
          value={readPercent(values, KEYS.navigationVolume, DEFAULTS.navigationVolume)}
          format={volumeText}
          onCommit={(value) => void save(KEYS.navigationVolume, String(value))}
        />
      </GrupoDeConfiguracao>
    </>
  );
}
