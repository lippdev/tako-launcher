import { GrupoDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { settingsStrings } from "../strings";
import { LinhaChave, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, boolText, readBool } from "./model";

const s = settingsStrings.system;

/**
 * Configurações › Sistema: as opções de desenvolvedor. Relatório, logs, backup e desempenho são ações do hospedeiro
 * (ainda sem chamada na ponte). O modo seguro não é uma configuração: o Tako entra nele pelo argumento de
 * inicialização, e a chave `SafeMode` nenhuma parte do núcleo lê.
 */
export function SistemaCategoria({ values, save }: CategoryProps) {
  return (
    <GrupoDeConfiguracao title={s.advanced}>
      <LinhaChave
        title={s.developer.title}
        description={s.developer.description}
        checked={readBool(values, KEYS.developer, DEFAULTS.developer)}
        onChange={(on) => void save(KEYS.developer, boolText(on))}
      />
    </GrupoDeConfiguracao>
  );
}
