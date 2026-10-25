import { useCallback } from "react";
import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { LinhaChave, LinhaDeslizante, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, boolText, readBool, readPercent } from "./model";

/** Configurações › Som: a música do tema e o volume dos sons de navegação. */
export function SomCategoria({ values, save }: CategoryProps) {
  const t = useT();
  const musicOn = readBool(values, KEYS.themeMusic, DEFAULTS.themeMusic);

  const volumeText = useCallback(
    (value: number): string => (value === 0 ? t("settings.sound.music_muted") : `${String(value)}%`),
    [t],
  );

  return (
    <>
      <GrupoDeConfiguracao title={t("settings.sound_web.music")}>
        <LinhaChave
          title={t("settings.sound.music_enabled")}
          description={t("settings.sound.music_enabled_desc")}
          checked={musicOn}
          onChange={(on) => void save(KEYS.themeMusic, boolText(on))}
        />
        <LinhaDeslizante
          title={t("settings.sound.music_volume")}
          description={t("settings.sound.music_volume_desc")}
          value={readPercent(values, KEYS.themeMusicVolume, DEFAULTS.themeMusicVolume)}
          format={volumeText}
          disabled={!musicOn}
          onCommit={(value) => void save(KEYS.themeMusicVolume, String(value))}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={t("settings.sound_web.navigation")}>
        <LinhaDeslizante
          title={t("settings.sound.navigation_volume")}
          description={t("settings.sound.navigation_volume_desc")}
          value={readPercent(values, KEYS.navigationVolume, DEFAULTS.navigationVolume)}
          format={volumeText}
          onCommit={(value) => void save(KEYS.navigationVolume, String(value))}
        />
      </GrupoDeConfiguracao>
    </>
  );
}
