import { useMemo } from "react";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { PerfisNasConfiguracoes } from "../perfil/PerfisNasConfiguracoes";
import { windowAction } from "../windowActions";
import { LinhaChave, LinhaSelecao, type CategoryProps } from "./Linhas";
import {
  DEFAULTS,
  KEYS,
  boolText,
  readBool,
  readLanguage,
  readOpenIn,
  type Language,
  type OpenIn,
} from "./model";

/** Configurações › Geral: modo de abertura, idioma e o que acontece ao fim do jogo. */
export function GeralCategoria({ values, save }: CategoryProps) {
  const t = useT();

  const openInOptions = useMemo(
    () =>
      [
        { value: "desktop", label: t("settings.general_web.open_in_window") },
        { value: "big-picture", label: t("settings.general_web.open_in_big_picture") },
      ] as const satisfies readonly { value: OpenIn; label: string }[],
    [t],
  );

  const languageOptions = useMemo(
    () =>
      [
        { value: "pt-BR", label: t("settings.general_web.language_pt") },
        { value: "en-US", label: t("settings.general_web.language_en") },
      ] as const satisfies readonly { value: Language; label: string }[],
    [t],
  );

  return (
    <>
      <GrupoDeConfiguracao title={t("settings.general_web.mode")}>
        <LinhaDeConfiguracao
          title={t("settings.general_web.big_picture_title")}
          description={t("settings.general_web.big_picture_desc")}
          control={
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                windowAction("big_picture");
              }}
            >
              {t("settings.general_web.big_picture_action")}
            </BotaoDesktop>
          }
        />
        <LinhaChave
          title={t("settings.general_web.ask_title")}
          description={t("settings.general_web.ask_desc")}
          checked={readBool(values, KEYS.askBigPicture, DEFAULTS.askBigPicture)}
          onChange={(on) => void save(KEYS.askBigPicture, boolText(on))}
        />
        <LinhaSelecao
          title={t("settings.general_web.open_in")}
          options={openInOptions}
          value={readOpenIn(values)}
          onChange={(value) => void save(KEYS.openIn, value)}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={t("settings.general_web.language_section")}>
        <LinhaSelecao
          title={t("settings.general.language")}
          description={t("settings.general.language_desc")}
          options={languageOptions}
          value={readLanguage(values)}
          onChange={(value) => void save(KEYS.language, value)}
        />
      </GrupoDeConfiguracao>
      <PerfisNasConfiguracoes />
      <GrupoDeConfiguracao title={t("settings.general_web.games")}>
        <LinhaChave
          title={t("settings.general_web.restore_title")}
          description={t("settings.general_web.restore_desc")}
          checked={readBool(values, KEYS.restoreEnvironment, DEFAULTS.restoreEnvironment)}
          onChange={(on) => void save(KEYS.restoreEnvironment, boolText(on))}
        />
      </GrupoDeConfiguracao>
    </>
  );
}
