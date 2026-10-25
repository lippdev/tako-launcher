import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { LinhaChave, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, boolText, readBool } from "./model";

/**
 * Configurações › Sistema: as opções de desenvolvedor. Relatório, logs, backup e desempenho são ações do hospedeiro
 * (ainda sem chamada na ponte). O modo seguro não é uma configuração: o Tako entra nele pelo argumento de
 * inicialização, e a chave `SafeMode` nenhuma parte do núcleo lê.
 */
export function SistemaCategoria({ values, save }: CategoryProps) {
  const t = useT();
  return (
    <GrupoDeConfiguracao title={t("settings.system.advanced_header")}>
      <LinhaChave
        title={t("settings.system.developer")}
        description={t("settings.system.developer_desc")}
        checked={readBool(values, KEYS.developer, DEFAULTS.developer)}
        onChange={(on) => void save(KEYS.developer, boolText(on))}
      />
    </GrupoDeConfiguracao>
  );
}
