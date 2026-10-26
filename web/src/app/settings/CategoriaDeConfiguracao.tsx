import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../../components/Icon/Icon";
import styles from "./CategoriaDeConfiguracao.module.css";

interface CategoriaDeConfiguracaoProps {
  icon: IconName;
  title: string;
  /** A categoria aberta: quadrado do ícone branco com o ícone escuro. */
  current: boolean;
  /** Contagem azul à direita quando algo pede atenção; sem ela some. */
  count?: number;
  onSelect: () => void;
  /** Id do painel que a categoria abre. */
  controls?: string;
}

/** Item da lista de categorias (README da CategoriaDeConfiguracao): 44 de altura, ícone de 15 num quadrado de 28. */
export function CategoriaDeConfiguracao({ icon, title, current, count, onSelect, controls }: CategoriaDeConfiguracaoProps) {
  return (
    <button
      type="button"
      className={cx(styles["item"], current && styles["current"])}
      aria-current={current ? "page" : undefined}
      aria-controls={current ? controls : undefined}
      onClick={onSelect}
    >
      <span className={styles["icon"]}>
        <Icon name={icon} size="sm" />
      </span>
      <span className={styles["title"]}>{title}</span>
      {count !== undefined && count > 0 && <span className={styles["count"]}>{count}</span>}
    </button>
  );
}
