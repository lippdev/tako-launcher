import { useImageVersion } from "../../lib/useImageVersion";
import { versionedImageUrl } from "../../lib/imageVersions";
import { imageUrl } from "../../lib/imageVersions";
import { useEffect, useState, type CSSProperties } from "react";
import { settled } from "../../lib/presence";
import { cx } from "../../lib/cx";
import styles from "./FundoDoJogo.module.css";

export interface FundoDoJogoProps {
  /** ID do jogo; trocar de ID troca a camada (a nova entra por cima da anterior). */
  id: string;
  /** Tem a arte larga (`hasHero`). Sem ela, vale o EstadoSemArte: um degradê da cor do jogo. */
  temArte: boolean;
  /** Cor de destaque do jogo (hex). Sem ela, a cor de destaque do tema. */
  corDestaque?: string | null;
  /**
   * URL da arte, quando não vem do núcleo (galeria e testes). Sem ela a URL é `tako-img://hero`, no tamanho
   * pedido em pixels físicos (docs/web/ponte.md §4).
   */
  src?: string | undefined;
  /** Largura e altura do destaque em pixels CSS, para pedir a arte no tamanho certo. */
  largura?: number;
  altura?: number;
  className?: string | undefined;
}

interface Camada {
  key: string;
  src: string | null;
  cor: string | null;
  /** Já com o estilo de entrada (opacidade 1, escala 1). */
  on: boolean;
}

/** Arte larga do jogo: 3840×1240 na origem (`library_hero`). Pede no tamanho da janela, nunca a original. */
function urlDaArte(id: string, largura: number, altura: number): string {
  const dpr = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  const w = Math.min(3840, Math.max(1, Math.round(largura * dpr)));
  const h = Math.min(1240, Math.max(1, Math.round(altura * dpr)));
  return imageUrl("hero", id, w, h);
}

/**
 * A arte de fundo do destaque (README do FundoDoJogo). Duas camadas trocam de lugar na troca de jogo: a nova entra
 * em 800 ms (opacidade) e assenta de 1,05 para 1 em 1,8 s (escala); a antiga fica por baixo até acabar. Só
 * `transform` e `opacity` animam. Sem arte, ou se a imagem falhar, a camada é o degradê da cor do jogo.
 */
export function FundoDoJogo({ id, temArte, corDestaque, src, largura = 1308, altura = 700, className }: FundoDoJogoProps) {
  useImageVersion(id);
  const url = temArte ? (src ? versionedImageUrl(src) : urlDaArte(id, largura, altura)) : null;
  const cor = corDestaque ?? null;
  const [camadas, setCamadas] = useState<Camada[]>([{ key: id, src: url, cor, on: false }]);
  const [falhou, setFalhou] = useState<ReadonlySet<string>>(new Set());

  const atual = camadas[camadas.length - 1];
  // Jogo novo: a camada nova entra por cima, com o estilo de fechado (ajuste durante a renderização).
  if (atual && atual.key !== id) {
    setCamadas([atual, { key: id, src: url, cor, on: false }]);
  } else if (atual && (atual.src !== url || atual.cor !== cor)) {
    // Mesmo jogo, dado novo (a arte acabou de chegar, ou a cor mudou): atualiza no lugar.
    setCamadas([...camadas.slice(0, -1), { ...atual, src: url, cor }]);
  }

  const ultima = camadas[camadas.length - 1];
  const precisaEntrar = ultima !== undefined && !ultima.on;
  const sobrando = camadas.length > 1;

  useEffect(() => {
    if (!precisaEntrar) return;
    let frame = requestAnimationFrame(() => {
      frame = requestAnimationFrame(() => {
        setCamadas((todas) => todas.map((camada, i) => (i === todas.length - 1 ? { ...camada, on: true } : camada)));
      });
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [precisaEntrar]);

  // Só quando a nova já entrou: a antiga sai do DOM depois de as transições acabarem.
  const [raiz, setRaiz] = useState<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!sobrando || precisaEntrar) return;
    let cancelado = false;
    void settled(raiz).then(() => {
      if (!cancelado) setCamadas((todas) => todas.slice(-1));
    });
    return () => {
      cancelado = true;
    };
  }, [sobrando, precisaEntrar, raiz]);

  return (
    <div ref={setRaiz} className={cx(styles["fundo"], className)} aria-hidden="true">
      {camadas.map((camada, i) => {
        const semArte = camada.src === null || falhou.has(camada.src ?? "");
        const topo = i === camadas.length - 1;
        return (
          <div
            key={camada.key}
            className={cx(styles["camada"], camada.on && styles["on"], !topo && styles["antiga"])}
            style={camada.cor ? ({ "--cor-do-jogo": camada.cor } as CSSProperties) : undefined}
          >
            {semArte ? (
              <div className={styles["semArte"]} data-estado="sem-arte" />
            ) : (
              <img
                className={styles["arte"]}
                src={camada.src ?? undefined}
                alt=""
                decoding="async"
                draggable={false}
                onError={() => {
                  setFalhou((antes) => new Set(antes).add(camada.src ?? ""));
                }}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}
