import type { ChangeEvent, ComponentPropsWithRef, Ref } from "react";
import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./CampoDeBusca.module.css";

interface AcaoDaBusca {
  /** Padrão: `sliders` (Ordenar, ver e filtrar). */
  icon?: IconName;
  /** Nome do botão (só de ícone, então obrigatório). */
  label: string;
  onClick: () => void;
  /** Algum filtro está ativo: acende o ponto de destaque. */
  active?: boolean;
  /** O menu que o botão abre está aberto. */
  expanded?: boolean;
  count?: number;
  popup?: "menu" | "dialog";
}

interface CampoDeBuscaProps
  extends Omit<ComponentPropsWithRef<"input">, "value" | "onChange" | "type" | "ref" | "className" | "children"> {
  /**
   * `lista`: 38, ao lado do botão de recolher, com o botão de opções dentro. `loja`: 300 × 40, pílula, que cresce
   * para 360 no foco. `geral`: a linha de 58 do topo do painel da busca geral (Ctrl K).
   */
  variant: "lista" | "loja" | "geral";
  value: string;
  /** Filtra na hora: chamado a cada tecla. */
  onChange: (value: string) => void;
  placeholder: string;
  /** Nome do campo para leitores de tela. */
  label: string;
  /** Atalho mostrado à direita (ex.: "Ctrl K"). */
  hint?: string;
  /** Valor de `aria-keyshortcuts` do campo (ex.: "Control+K"). */
  keyShortcuts?: string;
  /** Botão de opções dentro do campo (variante `lista`). */
  action?: AcaoDaBusca;
  inputRef?: Ref<HTMLInputElement>;
  className?: string;
}

/** Campo de busca (README do CampoDeBusca): a mesma peça nas três buscas. */
export function CampoDeBusca({
  variant,
  value,
  onChange,
  placeholder,
  label,
  hint,
  keyShortcuts,
  action,
  inputRef,
  className,
  ...input
}: CampoDeBuscaProps) {
  return (
    <label className={cx(styles["field"], styles[variant], className)}>
      <span className={styles["box"]} aria-hidden="true" />
      <Icon name="search" size={variant === "geral" ? "md" : "sm"} />
      <input
        {...input}
        ref={inputRef}
        type="search"
        className={styles["input"]}
        value={value}
        placeholder={placeholder}
        aria-label={label}
        {...(keyShortcuts ? { "aria-keyshortcuts": keyShortcuts } : {})}
        autoComplete="off"
        spellCheck={false}
        onChange={(event: ChangeEvent<HTMLInputElement>) => {
          onChange(event.target.value);
        }}
      />
      {hint && (
        <kbd className={styles["hint"]} aria-hidden="true">
          {hint}
        </kbd>
      )}
      {action && (
        <button
          type="button"
          className={cx(styles["action"], action.active && styles["active"])}
          aria-label={action.label}
          aria-haspopup={action.popup ?? "menu"}
          {...(action.expanded === undefined ? {} : { "aria-expanded": action.expanded })}
          onClick={action.onClick}
        >
          <Icon name={action.icon ?? "sliders"} size="sm" />
          {action.count ? <span className={styles["count"]}>{action.count}</span> : <span className={styles["indicator"]} aria-hidden="true" />}
        </button>
      )}
    </label>
  );
}
