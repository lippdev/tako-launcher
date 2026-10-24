import { useState } from "react";
import { cx } from "../../lib/cx";
import styles from "./TituloDoJogo.module.css";

export interface TituloDoJogoProps {
  /** ID do jogo, para montar `tako-img://logo`. */
  id: string;
  /** Nome do jogo: o texto alternativo do logo e, sem logo, o próprio título. */
  titulo: string;
  /** Tem logo (`hasLogo`). Sem ele, o nome em texto. */
  temLogo: boolean;
  /** URL do logo quando não vem do núcleo (galeria e testes). */
  src?: string | undefined;
  /** `pagina`: até 440×140; `visao`: até 320×100 (README do TituloDoJogo). */
  tamanho?: "pagina" | "visao";
  className?: string | undefined;
}

const MEDIDAS = { pagina: { w: 440, h: 140 }, visao: { w: 320, h: 100 } } as const;

function urlDoLogo(id: string, largura: number, altura: number): string {
  const dpr = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  const w = Math.round(largura * dpr);
  const h = Math.round(altura * dpr);
  return `tako-img://logo?id=${encodeURIComponent(id)}&w=${String(w)}&h=${String(h)}`;
}

/**
 * O nome do jogo é o logo dele (README do TituloDoJogo), alinhado embaixo à direita, com sombra. Sem logo, ou se a
 * imagem falhar, o nome em texto (Poppins 54/800).
 */
export function TituloDoJogo({ id, titulo, temLogo, src, tamanho = "pagina", className }: TituloDoJogoProps) {
  // Guarda de qual jogo foi a falha: ao trocar de jogo a nova tentativa vale.
  const [falhou, setFalhou] = useState<string | null>(null);
  const medida = MEDIDAS[tamanho];
  const logo = temLogo && falhou !== id ? (src ?? urlDoLogo(id, medida.w, medida.h)) : null;

  return (
    <h1 className={cx(styles["titulo"], styles[tamanho], className)}>
      {logo ? (
        <img
          className={styles["logo"]}
          src={logo}
          alt={titulo}
          decoding="async"
          draggable={false}
          onError={() => {
            setFalhou(id);
          }}
        />
      ) : (
        <span className={styles["texto"]}>{titulo}</span>
      )}
    </h1>
  );
}
