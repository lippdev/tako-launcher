import { useImageVersion } from "../../lib/useImageVersion";
import { versionedImageUrl } from "../../lib/imageVersions";
import { imageUrl } from "../../lib/imageVersions";
import { useState, type CSSProperties, type KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import styles from "./CapaDoJogo.module.css";

export interface CapaDoJogoProps {
  /** ID do jogo (`steam:570`, `heroic:legendary:Fortnite`) para montar a URL `tako-img://cover`. */
  id?: string;
  /** Título do jogo, usado no texto alternativo da imagem e no EstadoSemArte. */
  title: string;
  /**
   * Cor de destaque do jogo (hex, CSS). Usada no degradê do `EstadoSemArte` quando não houver imagem
   * ou em caso de erro no carregamento. Padrão: var(--accent).
   */
  accentColor?: string;
  /**
   * URL direta da imagem da capa. Quando informada (ex.: na demo ou teste), tem precedência sobre o
   * protocolo `tako-img://`.
   */
  src?: string;
  /** Largura da capa em pixels CSS para calcular a resolução física (w e h com devicePixelRatio). */
  width?: number;
  /** Altura da capa em pixels CSS (se omitida, calculada pela proporção 2:3 da Steam). */
  height?: number;
  /** A capa ocupa a largura do contêiner (fileiras e grades); `width` só dimensiona o pedido da imagem. */
  fit?: boolean;
  /** Clique na capa do jogo. */
  onClick?: () => void;
  /** Carregamento sob demanda (`lazy`, padrão) ou prioritário (`eager`). */
  loading?: "lazy" | "eager";
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
  return imageUrl("cover", id, w, h);
}

/**
 * Capa de jogo na proporção 2:3 (600×900 da Steam), raio de 11 e borda interna fina.
 * Suporta o protocolo `tako-img://cover`, hover suave (zoom 1.05 + brilho) e o `EstadoSemArte`
 * com degradê da cor de destaque do jogo quando a imagem não existir ou falhar.
 */
export function CapaDoJogo({
  id,
  title,
  accentColor,
  src,
  width = 200,
  height,
  fit = false,
  onClick,
  loading = "lazy",
  className,
  "data-force": dataForce,
}: CapaDoJogoProps) {
  useImageVersion(id);
  const [erroImagem, setErroImagem] = useState<string | undefined>(undefined);

  const alturaCalculada = height ?? Math.round(width * 1.5);
  const urlFinal = (src ? versionedImageUrl(src) : undefined) ?? (id ? montaUrlCapa(id, width, alturaCalculada) : undefined);
  const semArte = !urlFinal || erroImagem === urlFinal;

  const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (onClick && (e.key === "Enter" || e.key === " ")) {
      e.preventDefault();
      onClick();
    }
  };

  const styleCustom: CSSProperties = {
    ...(accentColor ? ({ "--capa-cor": accentColor } as CSSProperties) : {}),
    ...(width && !fit ? { width: `${width}px` } : {}),
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
            "aria-label": title,
          }
        : {})}
    >
      {semArte ? (
        <div className={styles["semArte"]} aria-label={title}>
          <span className={styles["semArteTitulo"]}>{title}</span>
        </div>
      ) : (
        <img
          className={styles["img"]}
          src={urlFinal}
          alt={title}
          loading={loading}
          decoding="async"
          draggable={false}
          onError={() => {
            setErroImagem(urlFinal);
          }}
        />
      )}
    </div>
  );
}
