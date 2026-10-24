import { useState, type CSSProperties, type KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import styles from "./CapaDoJogo.module.css";

export interface CapaDoJogoProps {
  /** ID do jogo (`steam:570`, `heroic:legendary:Fortnite`) para montar a URL `tako-img://cover`. */
  id?: string;
  /** Título do jogo, usado no texto alternativo da imagem e no EstadoSemArte. */
  titulo: string;
  /**
   * Cor de destaque do jogo (hex, CSS). Usada no degradê do `EstadoSemArte` quando não houver imagem
   * ou em caso de erro no carregamento. Padrão: var(--accent).
   */
  corDestaque?: string;
  /**
   * URL direta da imagem da capa. Quando informada (ex.: na demo ou teste), tem precedência sobre o
   * protocolo `tako-img://`.
   */
  src?: string;
  /** Largura da capa em pixels CSS para calcular a resolução física (w e h com devicePixelRatio). */
  largura?: number;
  /** Altura da capa em pixels CSS (se omitida, calculada pela proporção 2:3 da Steam). */
  altura?: number;
  /** Clique na capa do jogo. */
  onClick?: () => void;
  /** Carregamento sob demanda (`lazy`, padrão) ou prioritário (`eager`). */
  carregamento?: "lazy" | "eager";
  className?: string;
  /** Força o estado hover para exibição na galeria de componentes. */
  "data-force"?: "hover";
}

/**
 * Monta a URL de capa no protocolo tako-img em pixels físicos
 * (largura exibida × devicePixelRatio), proporção 2:3 da Steam (docs/web/ponte.md §4).
 */
function montaUrlCapa(id: string, larguraCss: number, alturaCss: number): string {
  const dpr = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
  const w = Math.max(1, Math.round(larguraCss * dpr));
  const h = Math.max(1, Math.round(alturaCss * dpr));
  return `tako-img://cover?id=${encodeURIComponent(id)}&w=${w}&h=${h}`;
}

/**
 * Capa de jogo na proporção 2:3 (600×900 da Steam), raio de 11 e borda interna fina.
 * Suporta o protocolo `tako-img://cover`, hover suave (zoom 1.05 + brilho) e o `EstadoSemArte`
 * com degradê da cor de destaque do jogo quando a imagem não existir ou falhar.
 */
export function CapaDoJogo({
  id,
  titulo,
  corDestaque,
  src,
  largura = 200,
  altura,
  onClick,
  carregamento = "lazy",
  className,
  "data-force": dataForce,
}: CapaDoJogoProps) {
  const [erroImagem, setErroImagem] = useState(false);

  const alturaCalculada = altura ?? Math.round(largura * 1.5);
  const urlFinal = src ?? (id ? montaUrlCapa(id, largura, alturaCalculada) : undefined);
  const semArte = !urlFinal || erroImagem;

  const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (onClick && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      onClick();
    }
  };

  const styleCustom: CSSProperties = {
    ...(corDestaque ? ({ "--capa-cor": corDestaque } as CSSProperties) : {}),
    ...(largura ? { width: `${largura}px` } : {}),
  };

  return (
    <div
      className={cx(styles["container"], onClick && styles["clicavel"], className)}
      style={styleCustom}
      data-force={dataForce}
      {...(onClick
        ? {
            role: "button",
            tabIndex: 0,
            onClick,
            onKeyDown: handleKeyDown,
            "aria-label": titulo,
          }
        : {})}
    >
      {semArte ? (
        <div className={styles["semArte"]} aria-label={titulo}>
          <span className={styles["semArteTitulo"]}>{titulo}</span>
        </div>
      ) : (
        <img
          className={styles["img"]}
          src={urlFinal}
          alt={titulo}
          loading={carregamento}
          decoding="async"
          draggable={false}
          onError={() => {
            setErroImagem(true);
          }}
        />
      )}
    </div>
  );
}
