import { useArtRevision } from "../../lib/useImageVersion";
import { imageVersion } from "../../lib/imageVersions";
import { imageUrl } from "../../lib/imageVersions";
import {
  useState,
  useRef,
  useEffect,
  useId,
  type KeyboardEvent,
  type MouseEvent,
} from "react";
import mark from "../../assets/brand/tako-mark.png";
import { Icon } from "../Icon/Icon";
import { useVirtualRange } from "../../lib/list-virtual";
import { cx } from "../../lib/cx";
import type { GameRow, GameStatus } from "../ListaDeJogos/ListaDeJogos";
import styles from "./BibliotecaGrade.module.css";

export type { GameRow, GameStatus };

export interface BibliotecaGradeLabels {
  running: string;
  updateAvailable: string;
  notInstalled: string;
  favorite: string;
  downloading: (percent: number) => string;
  status: Record<Exclude<GameStatus, "none">, string>;
  loading: string;
  empty: string;
  play?: string;
}

export interface BibliotecaGradeProps {
  rows: readonly GameRow[];
  labels: BibliotecaGradeLabels;
  "aria-label": string;
  loading?: boolean;
  activeId: string | null;
  selection: ReadonlySet<string>;
  /** Clique, Enter ou duplo clique: abre a página do jogo. */
  onActivate: (id: string) => void;
  onSelectionChange: (ids: Set<string>) => void;
  /** O botão ▶ do cartão. */
  onPlay?: (id: string) => void;
  resolveCover?: (id: string, width: number, height: number) => string | null;
  resolveIcon?: (id: string, px: number) => string | null;
  columns?: number;
  className?: string;
  onNearEnd?: () => void;
}

function defaultResolveCover(id: string, w: number, h: number): string {
  return imageUrl("cover", id, w, h);
}

/**
 * Componente BibliotecaGrade: exibe a lista da biblioteca em grade virtualizada de capas.
 * Proporção 3:4 das capas, 3 por linha no painel lateral padrão (292px).
 * Suporta navegação por setas na grade, seleção múltipla (Ctrl e Shift + clique),
 * atalhos de teclado (Enter, Espaço, Ctrl+A, Esc) e 5.000+ jogos com alto desempenho.
 */
export function BibliotecaGrade({
  rows,
  labels,
  "aria-label": ariaLabel,
  loading = false,
  activeId,
  selection,
  onActivate,
  onSelectionChange,
  onPlay,
  resolveCover = defaultResolveCover,
  resolveIcon,
  columns: columnsProp,
  className,
  onNearEnd,
}: BibliotecaGradeProps) {
  useArtRevision();
  const viewportRef = useRef<HTMLDivElement>(null);
  const gridId = useId();
  const anchorRef = useRef<string | null>(null);
  const [focusIndex, setFocusIndex] = useState<number | null>(null);
  const [containerWidth, setContainerWidth] = useState(268);
  const [failedImages, setFailedImages] = useState<ReadonlySet<string>>(() => new Set());

  // Observa a largura do contêiner para calcular colunas dinâmicas se columnsProp não for definido
  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    const update = () => {
      const w = el.clientWidth;
      if (w > 0) setContainerWidth(w);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(el);
    return () => {
      observer.disconnect();
    };
  }, []);

  // Na largura padrão de ~292px da sidebar (ou <= 360px), são 3 colunas
  const cols = columnsProp ?? (containerWidth > 360 ? Math.max(3, Math.floor((containerWidth + 8) / (84 + 8))) : 3);
  const gap = 8;
  const cardWidth = Math.max(40, (containerWidth - (cols - 1) * gap) / cols);
  const cardHeight = Math.round((cardWidth * 4) / 3);
  const rowPitch = cardHeight + gap;

  const rowCount = Math.ceil(rows.length / cols);
  const { start, end, totalSize, scrollToIndex } = useVirtualRange({
    count: rowCount,
    itemSize: rowPitch,
    overscan: 4,
    viewportRef,
  });

  useEffect(() => {
    if (!loading && rows.length > 0 && end >= rowCount - 2) {
      onNearEnd?.();
    }
  }, [end, rowCount, rows.length, loading, onNearEnd]);

  const selecting = selection.size > 0;

  function indexOf(id: string | null): number {
    return id === null ? -1 : rows.findIndex((row) => row.id === id);
  }

  function rangeSet(from: number, to: number): Set<string> {
    const [low, high] = from <= to ? [from, to] : [to, from];
    return new Set(rows.slice(low, high + 1).map((row) => row.id));
  }

  /** Ctrl + clique marca ou desmarca; o jogo ativo entra na seleção quando ela começa. */
  function toggle(id: string) {
    const next = new Set(selection);
    if (!selecting && activeId !== null) next.add(activeId);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    anchorRef.current = id;
    onSelectionChange(next);
  }

  /** Shift + clique marca da âncora até o item clicado. */
  function extendTo(index: number, additive: boolean) {
    const anchor = indexOf(anchorRef.current ?? activeId);
    const range = rangeSet(anchor < 0 ? index : anchor, index);
    onSelectionChange(additive ? new Set([...selection, ...range]) : range);
  }

  function onCardClick(event: MouseEvent, index: number, row: GameRow) {
    setFocusIndex(index);
    if (event.shiftKey) {
      extendTo(index, event.ctrlKey || event.metaKey);
    } else if (event.ctrlKey || event.metaKey) {
      toggle(row.id);
    } else {
      if (selecting) onSelectionChange(new Set());
      anchorRef.current = row.id;
      onActivate(row.id);
    }
  }

  function moveFocus(index: number, event: KeyboardEvent) {
    const clamped = Math.min(rows.length - 1, Math.max(0, index));
    setFocusIndex(clamped);
    const targetRow = Math.floor(clamped / cols);
    scrollToIndex(targetRow);
    if (event.shiftKey) extendTo(clamped, false);
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (rows.length === 0) return;
    const current = focusIndex ?? Math.max(0, indexOf(activeId));
    const pageRows = Math.max(1, Math.floor((viewportRef.current?.clientHeight ?? 0) / rowPitch) - 1);
    const key = event.key;

    // Na primeira ou última linha, seta para cima/baixo sem Shift pode fluir para fora
    if (!event.shiftKey && !event.repeat) {
      if (key === "ArrowUp" && current < cols) return;
      if (key === "ArrowDown" && current + cols >= rows.length && Math.floor(current / cols) === rowCount - 1) return;
    }

    if (key === "ArrowDown") moveFocus(current + cols, event);
    else if (key === "ArrowUp") moveFocus(current - cols, event);
    else if (key === "ArrowRight") moveFocus(current + 1, event);
    else if (key === "ArrowLeft") moveFocus(current - 1, event);
    else if (key === "PageDown") moveFocus(current + pageRows * cols, event);
    else if (key === "PageUp") moveFocus(current - pageRows * cols, event);
    else if (key === "Home") moveFocus(0, event);
    else if (key === "End") moveFocus(rows.length - 1, event);
    else if (key === "Enter") {
      const row = rows[current];
      if (row) onActivate(row.id);
    } else if (key === " ") {
      const row = rows[current];
      if (row) toggle(row.id);
    } else if ((key === "a" || key === "A") && (event.ctrlKey || event.metaKey)) {
      onSelectionChange(new Set(rows.map((row) => row.id)));
    } else if (key === "Escape" && selecting) {
      onSelectionChange(new Set());
    } else {
      return;
    }

    event.preventDefault();
    if (key === "Escape") event.stopPropagation();
  }

  const optionId = (index: number) => `${gridId}-${String(index)}`;
  const focusedRow = focusIndex !== null ? Math.floor(focusIndex / cols) : null;
  const focusedInDom = focusedRow !== null && focusedRow >= start && focusedRow < end;

  // Renderiza linhas virtuais
  const renderedRows = [];
  for (let r = start; r < end; r++) {
    const rowStartIndex = r * cols;
    const rowGames = rows.slice(rowStartIndex, rowStartIndex + cols);
    renderedRows.push(
      <div
        key={r}
        className={styles["row"]}
        style={{
          transform: `translateY(${String(r * rowPitch)}px)`,
          height: `${String(cardHeight)}px`,
          gridTemplateColumns: `repeat(${String(cols)}, 1fr)`,
        }}
      >
        {rowGames.map((row, colIdx) => {
          const index = rowStartIndex + colIdx;
          const isActive = row.id === activeId;
          const isSelected = selection.has(row.id);
          const isFocused = focusIndex === index;
          const imageKey = `${row.id}:${String(imageVersion(row.id))}`;
          const hasImageError = failedImages.has(imageKey);
          const coverUrl = !hasImageError
            ? resolveCover(row.id, Math.round(cardWidth * 2), Math.round(cardHeight * 2)) ??
              (resolveIcon ? resolveIcon(row.id, 64) : null)
            : null;

          return (
            <div
              key={row.id}
              id={optionId(index)}
              role="option"
              aria-selected={isSelected || isActive}
              className={cx(
                styles["card"],
                isActive && styles["active"],
                isSelected && styles["selected"],
                isFocused && styles["focused"],
                !row.installed && styles["notInstalled"],
                row.running && styles["running"],
              )}
              data-id={row.id}
              data-open={row.id}
              title={`${row.title}${row.hint ? ` · ${row.hint}` : ""}`}
              onClick={(e) => onCardClick(e, index, row)}
              onDoubleClick={() => {
                onActivate(row.id);
              }}
            >
              {/* Capa do jogo ou fallback com letra inicial */}
              <div className={styles["coverContainer"]}>
                {coverUrl ? (
                  <img
                    src={coverUrl}
                    alt=""
                    loading="lazy"
                    className={styles["coverImg"]}
                    onError={() => {
                      setFailedImages((prev) => new Set([...prev, imageKey]));
                    }}
                  />
                ) : (
                  <span className={styles["coverFallback"]}>
                    {Array.from(row.title)[0]?.toUpperCase() ?? "?"}
                  </span>
                )}
              </div>

              {/* Indicador de seleção múltipla */}
              {selecting && (
                <span
                  className={cx(styles["checkBadge"], isSelected && styles["checked"])}
                  aria-hidden="true"
                >
                  {isSelected && <Icon name="check" size="sm" />}
                </span>
              )}

              {/* Canto superior: indicador de status e botão play */}
              <div className={styles["corner"]}>
                {row.running ? (
                  <i className={cx(styles["dot"], styles["running"])} aria-label={labels.running} />
                ) : row.updateAvailable ? (
                  <i className={cx(styles["dot"], styles["update"])} aria-label={labels.updateAvailable} />
                ) : null}

                {row.installed && (
                  <button
                    type="button"
                    className={cx(styles["playBtn"], row.updateAvailable && styles["playBtnUpdate"])}
                    title={labels.play ?? "Jogar"}
                    aria-label={`${labels.play ?? "Jogar"} ${row.title}`}
                    tabIndex={-1}
                    onClick={(e) => {
                      e.stopPropagation();
                      onPlay?.(row.id);
                    }}
                  >
                    <Icon name="play" filled size="sm" />
                  </button>
                )}
              </div>

              {/* Título sobre degradê escuro revelado no hover */}
              <div className={styles["titleScrim"]}>
                <span className={styles["gameTitle"]}>{row.title}</span>
                {row.downloadPercent !== null && (
                  <span className={styles["downloadPct"]}>
                    {labels.downloading(row.downloadPercent)}
                  </span>
                )}
              </div>

              {/* Barra de progresso para jogos em download */}
              {row.downloadPercent !== null && (
                <div className={styles["progressTrack"]}>
                  <div
                    className={styles["progressBar"]}
                    style={{
                      transform: `scaleX(${String(Math.max(0, Math.min(100, row.downloadPercent)) / 100)})`,
                    }}
                  />
                </div>
              )}
            </div>
          );
        })}
      </div>,
    );
  }

  return (
    <div className={cx(styles["gradeRoot"], className)}>
      <div
        ref={viewportRef}
        className={styles["viewport"]}
        role="listbox"
        aria-label={ariaLabel}
        aria-multiselectable="true"
        aria-busy={loading || undefined}
        aria-activedescendant={focusedInDom && focusIndex !== null ? optionId(focusIndex) : undefined}
        tabIndex={loading || rows.length === 0 ? -1 : 0}
        onKeyDown={onKeyDown}
        onFocus={() => {
          if (focusIndex === null && rows.length > 0) {
            setFocusIndex(Math.max(0, indexOf(activeId)));
          }
        }}
      >
        {loading ? (
          <div className={styles["row"]} style={{ position: "relative", height: "auto" }}>
            {Array.from({ length: cols * 3 }).map((_, i) => (
              <div key={i} className={styles["skeletonCard"]} />
            ))}
          </div>
        ) : rows.length === 0 ? (
          <div className={styles["emptyState"]}>
            <img src={mark} alt="" className={styles["emptyIcon"]} />
            <p>{labels.empty}</p>
          </div>
        ) : (
          <div className={styles["spacer"]} style={{ height: `${String(totalSize)}px` }}>
            {renderedRows}
          </div>
        )}
      </div>
    </div>
  );
}
