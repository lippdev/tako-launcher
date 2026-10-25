import { useEffect, useRef, useState } from "react";
import { cx } from "../../lib/cx";
import { useLatest } from "../../lib/useLatest";
import { CapaDoJogo } from "../CapaDoJogo/CapaDoJogo";
import { Icon } from "../Icon/Icon";
import styles from "./FileiraDeCapas.module.css";

export interface CapaDaFileira {
  id: string;
  title: string;
  /** Rótulo pequeno acima do nome ("7 h jogadas"). Sem dado, não aparece. */
  label?: string | null;
  accentColor?: string | null;
  /** Tem capa (`hasCover`). Sem ela, vale o EstadoSemArte da CapaDoJogo. */
  hasArt: boolean;
  /** URL da capa quando não vem do núcleo (galeria e hospedeiro falso). */
  src?: string | undefined;
  /** O jogo aberto na página dele (anel). */
  active?: boolean;
  /** Marcado na seleção múltipla (anel de destaque). */
  selected?: boolean;
}

export interface FileiraDeCapasProps {
  title: string;
  /** Contagem ao lado do título ("9 jogos"). Só quando o número é o da fileira inteira. */
  count?: string | undefined;
  covers: readonly CapaDaFileira[];
  previousLabel: string;
  nextLabel: string;
  /** Abrir o jogo (clique, Enter ou Espaço na capa). */
  onOpen: (id: string) => void;
  /** O mouse passou por uma capa ou o foco chegou nela. */
  onHighlight?: ((id: string) => void) | undefined;
  /** A rolagem chegou perto do fim: hora de pedir a página seguinte. */
  onNearEnd?: (() => void) | undefined;
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
  title,
  count,
  covers,
  previousLabel,
  nextLabel,
  onOpen,
  onHighlight,
  onNearEnd,
  className,
}: FileiraDeCapasProps) {
  const trilho = useRef<HTMLUListElement>(null);
  const [pode, setPode] = useState({ anterior: false, proximo: false });
  const pertoDoFim = useLatest(onNearEnd);

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
  }, [covers.length, pertoDoFim]);

  // A rolagem suave é do CSS (`scroll-behavior`), que "Reduzir animações" zera.
  const rolar = (sentido: -1 | 1) => {
    const el = trilho.current;
    if (el) el.scrollBy({ left: sentido * el.clientWidth });
  };

  return (
    <section className={cx(styles["fileira"], className)} aria-label={title}>
      <div className={styles["cabeca"]}>
        <h2 className={styles["titulo"]}>
          {title}
          {count && <small className={styles["contagem"]}>{count}</small>}
        </h2>
        {/* As setas só rolam; pelo teclado e pelo controle a fileira rola sozinha com o foco (`data-focus-skip`). */}
        <div className={styles["setas"]} data-focus-skip="">
          <button
            type="button"
            className={styles["seta"]}
            aria-label={previousLabel}
            title={previousLabel}
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
            aria-label={nextLabel}
            title={nextLabel}
            disabled={!pode.proximo}
            onClick={() => {
              rolar(1);
            }}
          >
            <Icon name="chevron-right" size="sm" />
          </button>
        </div>
      </div>
      <ul ref={trilho} className={styles["trilho"]} data-focus-group="">
        {covers.map((capa) => (
          <li
            key={capa.id}
            className={styles["cartao"]}
            data-id={capa.id}
            data-ativa={capa.active ? "" : undefined}
            data-selecionada={capa.selected ? "" : undefined}
            onPointerEnter={() => onHighlight?.(capa.id)}
            onFocus={() => onHighlight?.(capa.id)}
          >
            <CapaDoJogo
              fit
              {...(capa.hasArt ? { id: capa.id } : {})}
              title={capa.title}
              {...(capa.hasArt && capa.src ? { src: capa.src } : {})}
              width={LARGURA}
              height={ALTURA}
              {...(capa.accentColor ? { accentColor: capa.accentColor } : {})}
              onClick={() => {
                onOpen(capa.id);
              }}
            />
            <div className={styles["meta"]}>
              {capa.label && <small className={styles["rotulo"]}>{capa.label}</small>}
              <strong className={styles["nome"]}>{capa.title}</strong>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
