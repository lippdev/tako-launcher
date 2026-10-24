import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { PerfisNasConfiguracoes } from "../perfil/PerfisNasConfiguracoes";
import { settingsStrings } from "../strings";
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

const s = settingsStrings.general;

const OPEN_IN_OPTIONS = [
  { value: "desktop", label: s.openIn.window },
  { value: "big-picture", label: s.openIn.bigPicture },
] as const satisfies readonly { value: OpenIn; label: string }[];

const LANGUAGE_OPTIONS = [
  { value: "pt-BR", label: s.language.pt },
  { value: "en-US", label: s.language.en },
] as const satisfies readonly { value: Language; label: string }[];

/** Configurações › Geral: modo de abertura, idioma e o que acontece ao fim do jogo. */
export function GeralCategoria({ values, save }: CategoryProps) {
  return (
    <>
      <GrupoDeConfiguracao title={s.mode}>
        <LinhaDeConfiguracao
          title={s.bigPicture.title}
          description={s.bigPicture.description}
          control={
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                windowAction("big_picture");
              }}
            >
              {s.bigPicture.action}
            </BotaoDesktop>
          }
        />
        <LinhaChave
          title={s.ask.title}
          description={s.ask.description}
          checked={readBool(values, KEYS.askBigPicture, DEFAULTS.askBigPicture)}
          onChange={(on) => void save(KEYS.askBigPicture, boolText(on))}
        />
        <LinhaSelecao
          title={s.openIn.title}
          options={OPEN_IN_OPTIONS}
          value={readOpenIn(values)}
          onChange={(value) => void save(KEYS.openIn, value)}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={s.language.section}>
        <LinhaSelecao
          title={s.language.title}
          description={s.language.description}
          options={LANGUAGE_OPTIONS}
          value={readLanguage(values)}
          onChange={(value) => void save(KEYS.language, value)}
        />
      </GrupoDeConfiguracao>
      <PerfisNasConfiguracoes />
      <GrupoDeConfiguracao title={s.games}>
        <LinhaChave
          title={s.restore.title}
          description={s.restore.description}
          checked={readBool(values, KEYS.restoreEnvironment, DEFAULTS.restoreEnvironment)}
          onChange={(on) => void save(KEYS.restoreEnvironment, boolText(on))}
        />
      </GrupoDeConfiguracao>
    </>
  );
}
