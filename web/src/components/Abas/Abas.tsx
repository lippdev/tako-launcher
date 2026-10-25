import { useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import styles from "./Abas.module.css";

export interface Aba<T extends string> {
  id: T;
  label: string;
  /** Contagem em etiqueta (variante `loja`). */
  count?: number | string;
  /** Só para a galeria: mostra o estado parado (hover ou foco) sem precisar do mouse. */
  force?: "hover" | "focus";
}

interface AbasProps<T extends string> {
  /**
   * `topo`: caixa alta com sublinhado em tentáculo (LOJA · BIBLIOTECA). `lista`: Todos · Favoritos · Instalados
   * (TAKO: texto sobre uma linha; outros temas: trilho). `loja`: botões de 36. `segmentado`: trilho com a pílula
   * que desliza.
   */
  variant: "topo" | "lista" | "loja" | "segmentado";
  tabs: readonly Aba<T>[];
  /** A aba atual; fora de `tabs` (ex.: `""`) nenhuma fica marcada (a barra de cima nas Configurações). */
  value: T | "";
  onChange: (id: T) => void;
  /** Nome do grupo de abas para leitores de tela. */
  label: string;
  /**
   * Prefixo dos ids: cada aba ganha `${idPrefix}-tab-${id}` e `aria-controls="${idPrefix}-panel-${id}"`; quem
   * mostra o conteúdo dá ao painel o `id` e o `aria-labelledby` correspondentes.
   */
  idPrefix?: string;
  className?: string;
}

/**
 * Abas (README do Abas), com teclado de `tablist`: ← → entre as abas (ativa ao focar), Home e End. Na primeira e na
 * última aba a seta não é das abas (não dá a volta): segue para o navegador de foco.
 */
export function Abas<T extends string>({ variant, tabs, value, onChange, label, idPrefix, className }: AbasProps<T>) {
  const listRef = useRef<HTMLDivElement>(null);
  const pillRef = useRef<HTMLSpanElement>(null);
  const [ready, setReady] = useState(false);

  // Pílula do segmentado: posição e largura medidas na aba atual, aplicadas com `transform` (sem animar layout).
  useLayoutEffect(() => {
    if (variant !== "segmentado") return;
    const list = listRef.current;
    const pill = pillRef.current;
    if (!list || !pill) return;
    const place = () => {
      const current = list.querySelector<HTMLElement>('[aria-selected="true"]');
      if (!current) return;
      pill.style.transform = `translateX(${current.offsetLeft}px) scaleX(${current.offsetWidth / pill.offsetWidth})`;
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(list);
    return () => {
      observer.disconnect();
    };
  }, [variant, value, tabs]);

  // A primeira posição não anima (a pílula não "chega" de lugar nenhum ao montar).
  useLayoutEffect(() => {
    const frame = requestAnimationFrame(() => {
      setReady(true);
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, []);

  // Nenhuma marcada: a primeira aba segue sendo a entrada do Tab (tabulação por `tablist`: só uma aba tabulável).
  const hasSelection = tabs.some((tab) => tab.id === value);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = tabs.findIndex((tab) => tab.id === value);
    let next = -1;
    if (event.key === "ArrowRight") next = index + 1 < tabs.length ? index + 1 : -1;
    else if (event.key === "ArrowLeft") next = index - 1;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = tabs.length - 1;
    const target = tabs[next];
    if (!target) return;
    event.preventDefault();
    onChange(target.id);
    listRef.current?.querySelector<HTMLElement>(`[data-tab="${target.id}"]`)?.focus();
  };

  return (
    <div
      ref={listRef}
      role="tablist"
      aria-label={label}
      aria-orientation="horizontal"
      className={cx(styles["tabs"], styles[variant], ready && styles["ready"], className)}
      onKeyDown={onKeyDown}
    >
      {variant === "segmentado" && <span ref={pillRef} className={styles["pill"]} aria-hidden="true" />}
      {tabs.map((tab, index) => {
        const selected = tab.id === value;
        return (
          <button
            key={tab.id}
            type="button"
            role="tab"
            data-tab={tab.id}
            aria-selected={selected}
            tabIndex={selected || (!hasSelection && index === 0) ? 0 : -1}
            data-force={tab.force}
            {...(idPrefix ? { id: `${idPrefix}-tab-${tab.id}`, "aria-controls": `${idPrefix}-panel-${tab.id}` } : {})}
            className={cx(styles["tab"], selected && styles["selected"])}
            onClick={() => {
              onChange(tab.id);
            }}
          >
            {tab.label}
            {tab.count !== undefined && tab.count !== "" && <span className={styles["count"]}>{tab.count}</span>}
          </button>
        );
      })}
    </div>
  );
}
