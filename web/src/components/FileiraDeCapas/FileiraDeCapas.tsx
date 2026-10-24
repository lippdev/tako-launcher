import { useEffect, useRef, useState } from "react";
import { cx } from "../../lib/cx";
import { useLatest } from "../../lib/useLatest";
import { CapaDoJogo } from "../CapaDoJogo/CapaDoJogo";
import { Icon } from "../Icon/Icon";
import styles from "./FileiraDeCapas.module.css";

export interface CapaDaFileira {
  id: string;
  titulo: string;
  /** Rótulo pequeno acima do nome ("7 h jogadas"). Sem dado, não aparece. */
  rotulo?: string | null;
  corDestaque?: string | null;
  /** Tem capa (`hasCover`). Sem ela, vale o EstadoSemArte da CapaDoJogo. */
  temArte: boolean;
  /** URL da capa quando não vem do núcleo (galeria e hospedeiro falso). */
  src?: string | undefined;
}

export interface FileiraDeCapasProps {
  titulo: string;
  /** Contagem ao lado do título ("9 jogos"). Só quando o número é o da fileira inteira. */
  contagem?: string | undefined;
  capas: readonly CapaDaFileira[];
  rotuloAnterior: string;
  rotuloProximo: string;
  /** Abrir o jogo (clique, Enter ou Espaço na capa). */
  onAbrir: (id: string) => void;
  /** O mouse passou por uma capa ou o foco chegou nela. */
  onDestacar?: ((id: string) => void) | undefined;
  /** A rolagem chegou perto do fim: hora de pedir a página seguinte. */
  onPertoDoFim?: (() => void) | undefined;
  className?: string | undefined;
}

/** Largura da capa nas medidas da prévia (1600 px de janela, cinco por fileira) e a altura de 3:4, para pedir a arte. */
const LARGURA = 233;
const ALTURA = 311;

/**
 * Uma fileira de capas com título e as setas ‹ › (README da TelaBiblioteca e da BibliotecaCategorias): cinco capas
 * por largura, rolando na horizontal e parando em cada capa. As setas só rolam a fileira, nunca trocam um valor.
 * Quem usa decide o que passar o mouse ou abrir faz; a fileira não conhece jogo nem ponte.
 */
export function FileiraDeCapas({
  titulo,
  contagem,
  capas,
  rotuloAnterior,
  rotuloProximo,
  onAbrir,
  onDestacar,
  onPertoDoFim,
  className,
}: FileiraDeCapasProps) {
  const trilho = useRef<HTMLUListElement>(null);
  const [pode, setPode] = useState({ anterior: false, proximo: false });
  const pertoDoFim = useLatest(onPertoDoFim);

  // Mede a rolagem quando a fileira muda de tamanho, ganha capas ou rola.
  useEffect(() => {
    const el = trilho.current;
    if (!el) return;
    const medir = () => {
      const anterior = el.scrollLeft > 1;
      const proximo = el.scrollLeft + el.clientWidth < el.scrollWidth - 1;
      setPode((antes) => (antes.anterior === anterior && antes.proximo === proximo ? antes : { anterior, proximo }));
      if (el.scrollWidth - el.scrollLeft - el.clientWidth < el.clientWidth) pertoDoFim.current?.();
    };
    const observer = new ResizeObserver(medir);
    observer.observe(el);
    el.addEventListener("scroll", medir, { passive: true });
    return () => {
      observer.disconnect();
      el.removeEventListener("scroll", medir);
    };
  }, [capas.length, pertoDoFim]);

  // A rolagem suave é do CSS (`scroll-behavior`), que "Reduzir animações" zera.
  const rolar = (sentido: -1 | 1) => {
    const el = trilho.current;
    if (el) el.scrollBy({ left: sentido * el.clientWidth });
  };

  return (
    <section className={cx(styles["fileira"], className)} aria-label={titulo}>
      <div className={styles["cabeca"]}>
        <h2 className={styles["titulo"]}>
          {titulo}
          {contagem && <small className={styles["contagem"]}>{contagem}</small>}
        </h2>
        <div className={styles["setas"]}>
          <button
            type="button"
            className={styles["seta"]}
            aria-label={rotuloAnterior}
            title={rotuloAnterior}
            disabled={!pode.anterior}
            onClick={() => {
              rolar(-1);
            }}
          >
            <Icon name="chevron-left" size="sm" />
          </button>
          <button
            type="button"
            className={styles["seta"]}
            aria-label={rotuloProximo}
            title={rotuloProximo}
            disabled={!pode.proximo}
            onClick={() => {
              rolar(1);
            }}
          >
            <Icon name="chevron-right" size="sm" />
          </button>
        </div>
      </div>
      <ul ref={trilho} className={styles["trilho"]}>
        {capas.map((capa) => (
          <li
            key={capa.id}
            className={styles["cartao"]}
            onPointerEnter={() => onDestacar?.(capa.id)}
            onFocus={() => onDestacar?.(capa.id)}
          >
            <CapaDoJogo
              ajustar
              {...(capa.temArte ? { id: capa.id } : {})}
              titulo={capa.titulo}
              {...(capa.temArte && capa.src ? { src: capa.src } : {})}
              largura={LARGURA}
              altura={ALTURA}
              {...(capa.corDestaque ? { corDestaque: capa.corDestaque } : {})}
              onClick={() => {
                onAbrir(capa.id);
              }}
            />
            <div className={styles["meta"]}>
              {capa.rotulo && <small className={styles["rotulo"]}>{capa.rotulo}</small>}
              <strong className={styles["nome"]}>{capa.titulo}</strong>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
