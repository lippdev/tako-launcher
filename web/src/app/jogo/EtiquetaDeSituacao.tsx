import { cx } from "../../lib/cx";
import styles from "./EtiquetaDeSituacao.module.css";

interface EtiquetaDeSituacaoProps {
  texto: string;
  /** Ponto verde (instalado, jogando); sem ele, cinza. */
  ligado?: boolean;
}

/** A etiqueta acima do título do jogo: INSTALADO, NÃO INSTALADO, EM BREVE, JOGANDO (README do TituloDoJogo). */
export function EtiquetaDeSituacao({ texto, ligado = false }: EtiquetaDeSituacaoProps) {
  return (
    <span className={styles["etiqueta"]}>
      <i className={cx(styles["ponto"], ligado && styles["ligado"])} aria-hidden="true" />
      {texto}
    </span>
  );
}
