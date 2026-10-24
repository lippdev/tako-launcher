import type { Theme } from "../../appearance/appearance";
import { Icon } from "../../components/Icon/Icon";
import { cx } from "../../lib/cx";
import styles from "./CartaoDeTema.module.css";

interface CartaoDeTemaProps {
  theme: Theme;
  name: string;
  description: string;
  current: boolean;
  onSelect: () => void;
}

/**
 * Cartão de um tema (README da TelaTema): a prévia de 118 de altura com lista, destaque e botões, o nome e uma
 * frase. A prévia carrega o `data-theme` do próprio cartão, então se pinta com as cores reais do tema.
 */
export function CartaoDeTema({ theme, name, description, current, onSelect }: CartaoDeTemaProps) {
  return (
    <button type="button" className={cx(styles["card"], current && styles["on"])} aria-pressed={current} onClick={onSelect}>
      <span className={cx(styles["preview"], theme === "vidro" && styles["glass"])} data-theme={theme} aria-hidden="true">
        {theme === "vidro" && <span className={styles["aurora"]} />}
        <i className={styles["side"]} />
        <i className={styles["hero"]} />
        <span className={styles["row"]}>
          <i />
          <i />
          <b />
        </span>
      </span>
      <strong className={styles["name"]}>
        {name}
        {current && <Icon name="check" size="sm" />}
      </strong>
      <small className={styles["description"]}>{description}</small>
    </button>
  );
}
