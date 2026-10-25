import type { ReactNode } from "react";
import { cx } from "../../lib/cx";
import { EtiquetasDoJogo } from "../EtiquetasDoJogo/EtiquetasDoJogo";
import styles from "./CartaoDetalhes.module.css";

export interface DetalheDoJogo {
  id: string;
  label: string;
  valor: string;
  /** Ponto verde à frente do valor (instalado). */
  ligado?: boolean;
}

export interface CartaoDetalhesProps {
  /** As linhas que o núcleo respondeu; as sem dado nem entram. Sem nenhuma, o cartão não existe. */
  linhas: readonly DetalheDoJogo[];
  /** Etiquetas da pessoa para o jogo: até 3 chips e "+N". */
  etiquetas?: readonly string[];
  rotuloEtiquetas: string;
  /**
   * Quem edita as etiquetas (EditorDeEtiquetas) no lugar dos chips de leitura. Com ele a linha existe mesmo sem
   * etiquetas: é por ela que a primeira entra.
   */
  editorDeEtiquetas?: ReactNode;
  /** Nome do cartão para leitores de tela. */
  "aria-label": string;
  className?: string | undefined;
}

/**
 * Detalhes em chave/valor (README do CartaoDetalhes): linhas de 39, alternadas no `tile`, numa caixa de raio 16 com
 * contorno. O radar de gênero do protótipo fica de fora: o núcleo não tem os eixos, e métrica não se inventa.
 */
export function CartaoDetalhes({
  linhas,
  etiquetas = [],
  rotuloEtiquetas,
  editorDeEtiquetas,
  "aria-label": label,
  className,
}: CartaoDetalhesProps) {
  const temEtiquetas = etiquetas.length > 0 || editorDeEtiquetas !== undefined;
  if (linhas.length === 0 && !temEtiquetas) return null;
  return (
    <section className={cx(styles["caixa"], className)} aria-label={label}>
      {linhas.map((linha) => (
        <div key={linha.id} className={styles["linha"]}>
          <span className={styles["rotulo"]}>{linha.label}</span>
          <strong className={styles["valor"]} title={linha.valor}>
            {linha.ligado && <i className={styles["ponto"]} aria-hidden="true" />}
            <span className={styles["texto"]}>{linha.valor}</span>
          </strong>
        </div>
      ))}
      {temEtiquetas && (
        <div className={styles["linha"]}>
          <span className={styles["rotulo"]}>{rotuloEtiquetas}</span>
          {editorDeEtiquetas ?? <EtiquetasDoJogo tags={etiquetas} limit={3} size="sm" aria-label={rotuloEtiquetas} />}
        </div>
      )}
    </section>
  );
}
