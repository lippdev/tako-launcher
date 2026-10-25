import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { LinhaChave, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, boolText, readBool } from "./model";

/**
 * Configurações › Integrações: só o que tem chave no núcleo (Sunshine e Discord). Exportar os jogos para o Sunshine,
 * o Moonlight, o Console Mode e a API local ainda não passam pela ponte.
 */
export function IntegracoesCategoria({ values, save }: CategoryProps) {
  const t = useT();
  return (
    <>
      <GrupoDeConfiguracao title={t("settings.system.sunshine")}>
        <LinhaChave
          title={t("settings.system.sunshine_sync")}
          description={t("settings.system.sunshine_sync_desc")}
          checked={readBool(values, KEYS.sunshineSync, DEFAULTS.sunshineSync)}
          onChange={(on) => void save(KEYS.sunshineSync, boolText(on))}
        />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={t("settings.integrations_web.discord")}>
        <LinhaChave
          title={t("settings.system.discord")}
          description={t("settings.system.discord_desc")}
          checked={readBool(values, KEYS.discord, DEFAULTS.discord)}
          onChange={(on) => void save(KEYS.discord, boolText(on))}
        />
      </GrupoDeConfiguracao>
    </>
  );
}
