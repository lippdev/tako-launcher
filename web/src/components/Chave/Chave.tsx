import { cx } from "../../lib/cx";
import styles from "./Chave.module.css";

interface ChaveProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  /** Nome da chave para leitores de tela, quando não há um texto visível para apontar com `aria-labelledby`. */
  "aria-label"?: string;
  "aria-labelledby"?: string;
  "aria-describedby"?: string;
  className?: string;
}

/** Liga e desliga (46×28). Só liga/desliga: escolha entre opções é select e valor contínuo é deslizante. */
export function Chave({ checked, onChange, disabled = false, className, ...aria }: ChaveProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      className={cx(styles["switch"], checked && styles["on"], className)}
      onClick={() => {
        onChange(!checked);
      }}
      {...aria}
    >
      <span className={styles["thumb"]} aria-hidden="true" />
    </button>
  );
}
