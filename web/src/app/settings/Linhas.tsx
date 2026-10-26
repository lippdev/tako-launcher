import { useCallback, useEffect, useRef, useState } from "react";
import { CampoDeSelecao, type OpcaoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { Chave } from "../../components/Chave/Chave";
import { ControleDeslizante } from "../../components/ControleDeslizante/ControleDeslizante";
import { LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useLatest } from "../../lib/useLatest";
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
  disabled?: boolean;
}

/** Linha com seletor: uma opção entre várias. */
export function LinhaSelecao<T extends string>({ title, description, options, value, onChange, disabled }: LinhaSelecaoProps<T>) {
  return (
    <LinhaDeConfiguracao
      title={title}
      {...(description ? { description } : {})}
      control={
        <CampoDeSelecao label={title} options={options} value={value} onChange={onChange} {...(disabled ? { disabled } : {})} />
      }
    />
  );
}

/** Quanto o deslizante espera parar antes de gravar: arrastar não manda uma gravação por passo. */
const SLIDER_SETTLE_MS = 180;

interface LinhaDeslizanteProps {
  title: string;
  description?: string;
  /** O valor guardado. */
  value: number;
  /** Grava o valor em que o deslizante parou. */
  onCommit: (value: number) => void;
  format?: (value: number) => string;
  disabled?: boolean;
}

/**
 * Linha com deslizante (0 a 100). O valor acompanha o dedo na hora e a gravação vai quando ele para (ou quando a
 * categoria fecha). Se a gravação falhar, `value` volta ao de antes e o deslizante com ele.
 */
export function LinhaDeslizante({ title, description, value, onCommit, format, disabled }: LinhaDeslizanteProps) {
  const [draft, setDraft] = useState<number | null>(null);
  const pending = useRef<number | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const commit = useLatest(onCommit);

  const flush = useCallback(() => {
    clearTimeout(timer.current);
    timer.current = undefined;
    const next = pending.current;
    pending.current = null;
    if (next !== null) commit.current(next);
    setDraft(null);
  }, [commit]);

  // Fechar a categoria no meio do arrasto não perde o valor.
  useEffect(() => flush, [flush]);

  function onChange(next: number) {
    setDraft(next);
    pending.current = next;
    clearTimeout(timer.current);
    timer.current = setTimeout(flush, SLIDER_SETTLE_MS);
  }

  return (
    <LinhaDeConfiguracao
      title={title}
      {...(description ? { description } : {})}
      control={({ labelId, descriptionId }) => (
        <ControleDeslizante
          value={draft ?? value}
          onChange={onChange}
          {...(format ? { format } : {})}
          {...(disabled ? { disabled } : {})}
          aria-labelledby={labelId}
          {...(description ? { "aria-describedby": descriptionId } : {})}
        />
      )}
    />
  );
}
