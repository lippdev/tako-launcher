import { CampoDeSelecao, type OpcaoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { Chave } from "../../components/Chave/Chave";
import { LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import type { SaveSetting } from "./useSettings";
import type { SettingsMap } from "./model";

/** O que cada categoria recebe da tela: os valores lidos e como gravar. */
export interface CategoryProps {
  values: SettingsMap;
  save: SaveSetting;
  /** Os valores mais novos, para quem precisa deles depois de um `await`. */
  read: () => SettingsMap;
}

interface LinhaChaveProps {
  title: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

/** Linha com chave liga/desliga, nomeada pelo título e pela descrição. */
export function LinhaChave({ title, description, checked, onChange, disabled }: LinhaChaveProps) {
  return (
    <LinhaDeConfiguracao
      title={title}
      {...(description ? { description } : {})}
      control={({ labelId, descriptionId }) => (
        <Chave
          checked={checked}
          onChange={onChange}
          {...(disabled ? { disabled } : {})}
          aria-labelledby={labelId}
          {...(description ? { "aria-describedby": descriptionId } : {})}
        />
      )}
    />
  );
}

interface LinhaSelecaoProps<T extends string> {
  title: string;
  description?: string;
  options: readonly OpcaoDeSelecao<T>[];
  value: T;
  onChange: (value: T) => void;
}

/** Linha com seletor: uma opção entre várias. */
export function LinhaSelecao<T extends string>({ title, description, options, value, onChange }: LinhaSelecaoProps<T>) {
  return (
    <LinhaDeConfiguracao
      title={title}
      {...(description ? { description } : {})}
      control={<CampoDeSelecao label={title} options={options} value={value} onChange={onChange} />}
    />
  );
}
