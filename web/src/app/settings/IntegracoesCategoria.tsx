import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { settingsStrings } from "../strings";
import { LinhaChave, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, boolText, readBool } from "./model";

const s = settingsStrings.integrations;

/**
 * Configurações › Integrações: só o que tem chave no núcleo (Sunshine e Discord). Exportar os jogos para o Sunshine,
 * o Moonlight, o Console Mode e a API local ainda não passam pela ponte.
 */
export function IntegracoesCategoria({ values, save }: CategoryProps) {
  return (
    <>
      <GrupoDeConfiguracao title={s.sunshine}>
        <LinhaChave
          title={s.sunshineSync.title}
          description={s.sunshineSync.description}
          checked={readBool(values, KEYS.sunshineSync, DEFAULTS.sunshineSync)}
          onChange={(on) => void save(KEYS.sunshineSync, boolText(on))}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={s.discord}>
        <LinhaChave
          title={s.discordPresence.title}
          description={s.discordPresence.description}
          checked={readBool(values, KEYS.discord, DEFAULTS.discord)}
          onChange={(on) => void save(KEYS.discord, boolText(on))}
        />
      </GrupoDeConfiguracao>
    </>
  );
}
