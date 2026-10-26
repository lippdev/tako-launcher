import { useState, type ComponentPropsWithRef } from "react";
import { cx } from "../../lib/cx";
import { Icon } from "../Icon/Icon";
import { MenuDeContexto } from "../MenuDeContexto/MenuDeContexto";
import styles from "./CampoDeSelecao.module.css";

export interface OpcaoDeSelecao<T extends string> {
  value: T;
  label: string;
}

interface CampoDeSelecaoProps<T extends string>
  extends Omit<ComponentPropsWithRef<"button">, "value" | "onChange" | "children" | "type" | "aria-label"> {
  options: readonly OpcaoDeSelecao<T>[];
  value: T;
  onChange: (value: T) => void;
  /** Nome da linha (ex.: "Idioma"): nome do campo para leitores de tela e título do menu. */
  label: string;
}

/**
 * Seletor (README do CampoDeSelecao): uma opção entre várias. O botão mostra a atual e abre um MenuDeContexto
 * com o título da linha e ✓ na atual. Enter e Espaço abrem; ↑ ↓ ficam para a navegação entre as linhas.
 */
export function CampoDeSelecao<T extends string>({
  options,
  value,
  onChange,
  label,
  className,
  disabled,
  ...button
}: CampoDeSelecaoProps<T>) {
  const [open, setOpen] = useState(false);
  const [anchor, setAnchor] = useState<HTMLButtonElement | null>(null);
  const current = options.find((option) => option.value === value);

  return (
    <>
      <button
        {...button}
        ref={(node) => {
          setAnchor(node);
        }}
        type="button"
        disabled={disabled ?? false}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={`${label}: ${current?.label ?? ""}`}
        className={cx(styles["sel"], open && styles["open"], className)}
        onClick={() => {
          setOpen((v) => !v);
        }}
      >
        <span className={styles["value"]}>{current?.label}</span>
        <Icon name="updown" size="sm" />
      </button>
      <MenuDeContexto
        open={open}
        onClose={() => {
          setOpen(false);
        }}
        anchor={anchor}
        title={label}
        entries={options.map((option) => ({
          id: option.value,
          label: option.label,
          kind: "radio" as const,
          checked: option.value === value,
          onSelect: () => {
            onChange(option.value);
          },
        }))}
      />
    </>
  );
}
