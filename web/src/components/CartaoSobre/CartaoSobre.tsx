import { cx } from "../../lib/cx";
import styles from "./CartaoSobre.module.css";

export interface CartaoSobreProps {
  /** O título da seção ("Sobre o jogo"); vai em caixa alta. */
  title: string;
  /** Os parágrafos do texto, já separados. Sem nenhum, o cartão não existe. */
  paragrafos: readonly string[];
  className?: string | undefined;
}

/**
 * O texto da página do jogo (README do CartaoSobre): título 14,5/800 em caixa alta e parágrafos de 13 com
 * entrelinha 1,65. Imagens, DLC, recursos e requisitos são de quando o núcleo tiver esses dados.
 */
export function CartaoSobre({ title, paragrafos, className }: CartaoSobreProps) {
  if (paragrafos.length === 0) return null;
  return (
    <article className={cx(styles["sobre"], className)}>
      <h2 className={styles["titulo"]}>{title}</h2>
      {paragrafos.map((paragrafo, i) => (
        <p key={i} className={styles["texto"]}>
          {paragrafo}
        </p>
      ))}
    </article>
  );
}
