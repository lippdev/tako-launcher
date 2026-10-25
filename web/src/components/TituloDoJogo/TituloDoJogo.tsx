import { useImageVersion } from "../../lib/useImageVersion";
import { versionedImageUrl } from "../../lib/imageVersions";
import { imageUrl } from "../../lib/imageVersions";
import { useState } from "react";
import { cx } from "../../lib/cx";
import styles from "./TituloDoJogo.module.css";

export interface TituloDoJogoProps {
  /** ID do jogo, para montar `tako-img://logo`. */
  id: string;
  /** Nome do jogo: o texto alternativo do logo e, sem logo, o próprio título. */
  title: string;
  /** Tem logo (`hasLogo`). Sem ele, o nome em texto. */
  hasLogo: boolean;
  /** URL do logo quando não vem do núcleo (galeria e testes). */
  src?: string | undefined;
  /** `pagina`: até 440×140; `visao`: até 320×100 (README do TituloDoJogo). */
  size?: "pagina" | "visao";
  className?: string | undefined;
}

const MEDIDAS = { pagina: { w: 440, h: 140 }, visao: { w: 320, h: 100 } } as const;

function urlDoLogo(id: string, largura: number, altura: number): string {
  const dpr = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  const w = Math.round(largura * dpr);
  const h = Math.round(altura * dpr);
  return imageUrl("logo", id, w, h);
}

/**
 * O nome do jogo é o logo dele (README do TituloDoJogo), alinhado embaixo à direita, com sombra. Sem logo, ou se a
 * imagem falhar, o nome em texto (Poppins 54/800).
 */
export function TituloDoJogo({ id, title, hasLogo, src, size = "pagina", className }: TituloDoJogoProps) {
  useImageVersion(id);
  // Guarda de qual jogo foi a falha: ao trocar de jogo a nova tentativa vale.
  const [falhou, setFalhou] = useState<string | null>(null);
  const medida = MEDIDAS[size];
  const url = src ? versionedImageUrl(src) : urlDoLogo(id, medida.w, medida.h);
  const logo = hasLogo && falhou !== url ? url : null;

  return (
    <h1 className={cx(styles["titulo"], styles[size], className)}>
      {logo ? (
        <img
          className={styles["logo"]}
          src={logo}
          alt={title}
          decoding="async"
          draggable={false}
          onError={() => {
            setFalhou(url);
          }}
        />
      ) : (
        <span className={styles["texto"]}>{title}</span>
      )}
    </h1>
  );
}
