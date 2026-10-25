import type { CSSProperties, ReactNode } from "react";
import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./GrupoDeBlocos.module.css";

interface GrupoDeBlocosProps {
  children: ReactNode;
  /** Fundo opaco e sem desfoque (o grupo da semana, sobre `surface`). */
  flat?: boolean;
  /** Nome do grupo para leitores de tela (vira `role="group"`). */
  "aria-label"?: string;
  className?: string;
}

/** Superfície principal: grupo com contorno e calha de 6, com blocos dentro (Sua semana, números do jogo, Sobre). */
export function GrupoDeBlocos({ children, flat = false, className, "aria-label": label }: GrupoDeBlocosProps) {
  return (
    <div
      className={cx(styles["group"], flat && styles["flat"], className)}
      {...(label ? { role: "group", "aria-label": label } : {})}
    >
      {children}
    </div>
  );
}

interface BlocoProps {
  /** O valor ("19 h", "27"), ou qualquer conteúdo (um gráfico) no lugar dele. */
  children: ReactNode;
  /** O rótulo da faixa de baixo (vai em caixa alta). */
  label: string;
  /** Ícone de 19 ao lado do valor. */
  icon?: IconName;
  /** Largura fixa de 132 em vez de dividir o espaço do grupo com os outros blocos. */
  fixed?: boolean;
}

/** Bloco com o valor em cima (78) e o rótulo numa faixa embaixo (40). */
export function Bloco({ children, label, icon, fixed = false }: BlocoProps) {
  return (
    <div className={cx(styles["tile"], fixed && styles["fixed"])}>
      <div className={styles["value"]}>
        {children}
        {icon && <Icon name={icon} />}
      </div>
      <div className={styles["label"]}>{label}</div>
    </div>
  );
}

interface BlocoDeJogoProps {
  /** A capa (uma imagem 2:3 ou um marcador). */
  cover: ReactNode;
  title: string;
  text: string;
}

/** Bloco que traz um jogo: capa e texto lado a lado, sem faixa de rótulo. */
export function BlocoDeJogo({ cover, title, text }: BlocoDeJogoProps) {
  return (
    <div className={cx(styles["tile"], styles["game"])}>
      <div className={styles["cover"]}>{cover}</div>
      <div className={styles["gameText"]}>
        <strong className={styles["gameTitle"]}>{title}</strong>
        <span className={styles["gameSub"]}>{text}</span>
      </div>
    </div>
  );
}

interface BarrasDaSemanaProps {
  /** Um item por dia: a letra e a altura de 0 a 1. */
  days: { label: string; value: number }[];
  /** Nome do gráfico para leitores de tela, com os valores ("Esta semana: 80% …"). */
  "aria-label": string;
}

/** Gráfico de barras para dentro de um Bloco. A barra cresce por `transform`, nunca por `height`. */
export function BarrasDaSemana({ days, "aria-label": label }: BarrasDaSemanaProps) {
  return (
    <div className={styles["bars"]} role="img" aria-label={label}>
      {days.map((day, i) => (
        <div key={i} className={styles["bar"]}>
          <i className={styles["barTrack"]}>
            <b className={styles["barFill"]} style={{ "--v": Math.min(1, Math.max(0, day.value)) } as CSSProperties} />
          </i>
          <small className={styles["barLabel"]}>{day.label}</small>
        </div>
      ))}
    </div>
  );
}
