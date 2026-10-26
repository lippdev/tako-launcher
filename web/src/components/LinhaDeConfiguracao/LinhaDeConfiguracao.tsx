import { useId, type ReactNode } from "react";
import { cx } from "../../lib/cx";
import { Icon } from "../Icon/Icon";
import styles from "./LinhaDeConfiguracao.module.css";

interface GrupoDeConfiguracaoProps {
  /** Título da seção, acima do grupo, em caixa alta. */
  title?: string;
  children: ReactNode;
}

/** Caixa das linhas de configuração (raio 16, `surface`, contorno `line-strong`), com um título opcional em cima. */
export function GrupoDeConfiguracao({ title, children }: GrupoDeConfiguracaoProps) {
  const headingId = useId();
  return (
    <section className={styles["section"]} aria-labelledby={title ? headingId : undefined}>
      {title && (
        <h3 id={headingId} className={cx("overline", styles["heading"])}>
          {title}
        </h3>
      )}
      <div className={styles["group"]}>{children}</div>
    </section>
  );
}

/** Ids para ligar o controle ao título e à descrição da linha (`aria-labelledby` / `aria-describedby`). */
export interface LinhaIds {
  labelId: string;
  descriptionId: string;
}

interface LinhaDeConfiguracaoProps {
  title: string;
  description?: string;
  /** Etiqueta ao lado do título, na cor de destaque ("Novo", "Reinicia"). */
  tag?: string;
  /** Linha de perigo: título vermelho. */
  danger?: boolean;
  /** O controle à direita: chave, seletor, deslizante, botão. Recebe os ids para nomear o controle. */
  control?: ReactNode | ((ids: LinhaIds) => ReactNode);
  /** Linha clicável (abre outra tela): mostra `value` e a seta ›, e o controle é ignorado. */
  onClick?: () => void;
  /** Valor atual de uma linha clicável ("Principal"). */
  value?: string;
}

/** Linha de configuração: título e descrição à esquerda, o controle à direita. Mínimo de 64 de altura. */
export function LinhaDeConfiguracao({ title, description, tag, danger = false, control, onClick, value }: LinhaDeConfiguracaoProps) {
  const labelId = useId();
  const descriptionId = useId();

  const text = (
    <span className={styles["text"]}>
      <strong id={labelId} className={styles["title"]}>
        {title}
        {tag && <em className={styles["tag"]}>{tag}</em>}
      </strong>
      {description && (
        <small id={descriptionId} className={styles["description"]}>
          {description}
        </small>
      )}
    </span>
  );

  if (onClick) {
    return (
      <button
        type="button"
        className={cx(styles["row"], styles["link"], danger && styles["danger"])}
        aria-labelledby={labelId}
        aria-describedby={description ? descriptionId : undefined}
        onClick={onClick}
      >
        {text}
        <span className={styles["control"]}>
          {value}
          <Icon name="chevron-right" size="sm" />
        </span>
      </button>
    );
  }

  const ids = { labelId, descriptionId };
  return (
    <div className={cx(styles["row"], danger && styles["danger"])}>
      {text}
      {control !== undefined && (
        <span className={styles["control"]}>{typeof control === "function" ? control(ids) : control}</span>
      )}
    </div>
  );
}
