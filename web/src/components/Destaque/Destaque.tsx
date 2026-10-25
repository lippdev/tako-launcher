import type { CSSProperties, ReactNode } from "react";
import { cx } from "../../lib/cx";
import styles from "./Destaque.module.css";

export interface DestaqueProps {
  /** A arte de fundo (`FundoDoJogo`), de borda a borda. */
  fundo: ReactNode;
  /** Embaixo à esquerda: a barra de dados (`CartaoSessoes`) ou um grupo de blocos. */
  dados?: ReactNode;
  /** Canto superior direito: chips (situação, plataforma). */
  chips?: ReactNode;
  /** Embaixo à direita, de cima para baixo: situação do jogo, título (logo), descrição e ações. */
  situacao?: ReactNode;
  title?: ReactNode;
  descricao?: ReactNode;
  acoes?: ReactNode;
  /** `pagina`: de borda a borda (560–700). `visao`: 400 de altura e raio de 18 (Visão geral). */
  variante?: "pagina" | "visao";
  /** Trocando de jogo: as partes saem (opacidade e 14 para baixo) e voltam em escada (TrocaDeJogo). */
  trocando?: boolean;
  /** Nome para leitores de tela. */
  "aria-label"?: string;
  className?: string | undefined;
}

/** Uma parte que sai e volta na troca de jogo; `d` (0 a 5) é o degrau da escada de 55 ms. */
function Parte({ d, className, children }: { d: number; className?: string | undefined; children: ReactNode }) {
  return (
    <div className={cx(styles["parte"], className)} style={{ "--d": d } as CSSProperties} data-parte="">
      {children}
    </div>
  );
}

/**
 * O bloco grande com a arte do jogo (README do Destaque), igual na página do jogo e na Visão geral: dados embaixo
 * à esquerda, identidade e ações embaixo à direita. Só organiza; a arte, o título e as ações vêm de fora.
 */
export function Destaque({
  fundo,
  dados,
  chips,
  situacao,
  title,
  descricao,
  acoes,
  variante = "pagina",
  trocando = false,
  "aria-label": label,
  className,
}: DestaqueProps) {
  return (
    <section
      className={cx(styles["destaque"], styles[variante], className)}
      data-trocando={trocando}
      {...(label ? { "aria-label": label } : {})}
    >
      {fundo}
      <div className={styles["degrade"]} aria-hidden="true" />
      {chips && (
        <Parte d={0} className={styles["chips"]}>
          {chips}
        </Parte>
      )}
      <div className={styles["interno"]}>
        <div className={styles["esquerda"]}>{dados && <Parte d={4}>{dados}</Parte>}</div>
        <div className={styles["direita"]}>
          {situacao && <Parte d={0}>{situacao}</Parte>}
          {title && (
            <Parte d={1} className={styles["titulo"]}>
              {title}
            </Parte>
          )}
          {descricao && (
            <Parte d={2} className={styles["descricao"]}>
              {descricao}
            </Parte>
          )}
          {acoes && <Parte d={3}>{acoes}</Parte>}
        </div>
      </div>
    </section>
  );
}
