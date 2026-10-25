import { useImageVersion } from "../../lib/useImageVersion";
import { imageUrl } from "../../lib/imageVersions";
import { useEffect, useId, useRef, useState, type KeyboardEvent, type MouseEvent } from "react";
import mark from "../../assets/brand/tako-mark.png";
import { useAppearance } from "../../appearance/appearance";
import { cx } from "../../lib/cx";
import { useVirtualRange } from "../../lib/list-virtual";
import { Icon } from "../Icon/Icon";
import styles from "./ListaDeJogos.module.css";

/** A situação que a pessoa dá ao jogo (StatusDoJogo). */
export type GameStatus = "none" | "playing" | "completed" | "queued" | "paused" | "dropped";

/** Uma linha da lista. Os tipos da ponte para a biblioteca ainda não existem: o orquestrador mapeia para esta forma. */
export interface GameRow {
  /** ID do jogo (`steam:570`, `heroic:legendary:Fortnite`): vai sem alteração para a imagem e para os eventos. */
  id: string;
  title: string;
  /** Nome da loja ("Steam", "Epic"), para a dica da linha. */
  store: string;
  installed: boolean;
  favorite: boolean;
  status: GameStatus;
  /** O jogo está aberto agora ("Jogando"). */
  running: boolean;
  /** 0 a 100 enquanto baixa ("Baixando · n%"); `null` fora disso. */
  downloadPercent: number | null;
  updateAvailable: boolean;
  /** Dica complementar ("Jogado ontem, 12:38"). */
  hint?: string;
}

/** Os textos da lista; os catálogos de idioma chegam depois (WEB-06). */
export interface ListaDeJogosLabels {
  running: string;
  updateAvailable: string;
  notInstalled: string;
  favorite: string;
  /** "Baixando · 42%". */
  downloading: (percent: number) => string;
  status: Record<Exclude<GameStatus, "none">, string>;
  /** "Lendo a biblioteca…". */
  loading: string;
  /** "Nem com oito braços deu para achar esse." */
  empty: string;
}

interface ListaDeJogosProps {
  rows: readonly GameRow[];
  labels: ListaDeJogosLabels;
  /** Nome da lista para leitores de tela. */
  "aria-label": string;
  /** Lendo a biblioteca: a estrutura inteira com blocos de espera. */
  loading?: boolean;
  /** Lista recolhida (76): só os ícones de 46. */
  collapsed?: boolean;
  /** O jogo aberto na página (o escolhido). */
  activeId: string | null;
  /** Jogos marcados com Ctrl/Shift + clique; vazio fora da seleção múltipla. */
  selection: ReadonlySet<string>;
  /** Clique, ou Enter: abre o jogo. */
  onActivate: (id: string) => void;
  onSelectionChange: (ids: Set<string>) => void;
  /** Duplo clique: joga. */
  onPlay?: (id: string) => void;
  /**
   * URL do ícone para o tamanho pedido em pixels físicos; `null` mostra a inicial. Sem ela, o ícone vem do núcleo
   * (`tako-img://icon`, docs/web/ponte.md §4).
   */
  resolveIcon?: (id: string, px: number) => string | null;
  className?: string;
  /** Avisa quando a janela virtual alcança as últimas linhas da página carregada. */
  onNearEnd?: () => void;
}

/** Ícones que o núcleo respondeu 404: lembrados para a rolagem não pedir de novo a cada linha que reaparece. */
const failedIcons = new Set<string>();

function coreIcon(id: string, px: number): string {
  return imageUrl("icon", id, px, px);
}

/** Tamanho pedido: o ícone de 28 na escala da interface e da tela, em degraus de 8 (para o cache reaproveitar), mínimo 40. */
function iconPixels(uiScale: number): number {
  const cssSize = 28 * (uiScale / 100) * window.devicePixelRatio;
  return Math.max(40, Math.ceil(cssSize / 8) * 8);
}

const FALLBACK_PITCH = 44;

function RowIcon({ row, px, resolveIcon }: { row: GameRow; px: number; resolveIcon: (id: string, px: number) => string | null }) {
  const version = useImageVersion(row.id);
  const key = `${row.id}:${String(version)}`;
  const [failed, setFailed] = useState<string | null>(null);
  const src = failed === key || failedIcons.has(key) ? null : resolveIcon(row.id, px);
  const percent = row.downloadPercent;
  return (
    <span className={styles["icon"]} aria-hidden="true">
      {src ? (
        <img
          className={styles["img"]}
          src={src}
          alt=""
          width={px}
          height={px}
          loading="lazy"
          decoding="async"
          draggable={false}
          onError={() => {
            failedIcons.add(key);
            setFailed(key);
          }}
        />
      ) : (
        <span className={styles["initial"]}>{Array.from(row.title)[0]?.toUpperCase() ?? "?"}</span>
      )}
      {percent !== null && (
        <span className={styles["prog"]}>
          <b style={{ transform: `scaleX(${Math.min(100, Math.max(0, percent)) / 100})` }} />
        </span>
      )}
      {/* Recolhida o fim da linha some: o aviso de atualização e de "jogando" sobe para o canto do ícone. */}
      {(row.running || row.updateAvailable) && (
        <span className={cx(styles["cornerDot"], row.running && styles["running"])} />
      )}
    </span>
  );
}

/** A lista fixa da biblioteca. Linhas de altura fixa, só as visíveis (com folga) no DOM: aguenta milhares de jogos. */
export function ListaDeJogos({
  rows,
  labels,
  loading = false,
  collapsed = false,
  activeId,
  selection,
  onActivate,
  onSelectionChange,
  onPlay,
  resolveIcon = coreIcon,
  className,
  onNearEnd,
  "aria-label": ariaLabel,
}: ListaDeJogosProps) {
  const listId = useId();
  const viewportRef = useRef<HTMLDivElement>(null);
  const anchorRef = useRef<string | null>(null);
  const { uiScale } = useAppearance();
  const [pitch, setPitch] = useState(FALLBACK_PITCH);
  const [focusIndex, setFocusIndex] = useState<number | null>(null);

  // A altura da linha vem do CSS (uma fonte só): lida de novo se a lista recolhe ou a escala da interface muda.
  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    const read = () => {
      const css = getComputedStyle(el);
      const next = parseFloat(css.getPropertyValue("--row-height")) + parseFloat(css.getPropertyValue("--row-gap"));
      if (next > 0) setPitch(next);
    };
    const observer = new ResizeObserver(read);
    observer.observe(el);
    return () => {
      observer.disconnect();
    };
  }, [collapsed, uiScale]);

  const { start, end, totalSize, scrollToIndex } = useVirtualRange({ count: rows.length, itemSize: pitch, viewportRef });
  useEffect(() => {
    if (!loading && rows.length > 0 && end >= rows.length - 4) onNearEnd?.();
  }, [end, rows.length, loading, onNearEnd]);
  const selecting = selection.size > 0;
  const px = iconPixels(uiScale);

  function indexOf(id: string | null): number {
    return id === null ? -1 : rows.findIndex((row) => row.id === id);
  }

  function rangeSet(from: number, to: number): Set<string> {
    const [low, high] = from <= to ? [from, to] : [to, from];
    return new Set(rows.slice(low, high + 1).map((row) => row.id));
  }

  /** Ctrl + clique marca ou desmarca; o jogo aberto entra na seleção quando ela começa (SelecaoMultipla). */
  function toggle(id: string) {
    const next = new Set(selection);
    if (!selecting && activeId !== null) next.add(activeId);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    anchorRef.current = id;
    onSelectionChange(next);
  }

  /** Shift + clique marca do âncora até a linha; com Ctrl junto, soma ao que já estava marcado. */
  function extendTo(index: number, additive: boolean) {
    const anchor = indexOf(anchorRef.current ?? activeId);
    const range = rangeSet(anchor < 0 ? index : anchor, index);
    onSelectionChange(additive ? new Set([...selection, ...range]) : range);
  }

  function onRowClick(event: MouseEvent, index: number, row: GameRow) {
    setFocusIndex(index);
    if (event.shiftKey) extendTo(index, event.ctrlKey || event.metaKey);
    else if (event.ctrlKey || event.metaKey) toggle(row.id);
    else {
      if (selecting) onSelectionChange(new Set());
      anchorRef.current = row.id;
      onActivate(row.id);
    }
  }

  function moveFocus(index: number, event: KeyboardEvent) {
    const clamped = Math.min(rows.length - 1, Math.max(0, index));
    setFocusIndex(clamped);
    scrollToIndex(clamped);
    if (event.shiftKey) extendTo(clamped, false);
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (rows.length === 0) return;
    const current = focusIndex ?? Math.max(0, indexOf(activeId));
    const page = Math.max(1, Math.floor((viewportRef.current?.clientHeight ?? 0) / pitch) - 1);
    const key = event.key;
    // Na primeira e na última linha a seta não é da lista: segue para o navegador de foco (sobe para a busca).
    // Segurar a seta até a ponta não pula para fora: só um aperto novo passa.
    if (!event.shiftKey && !event.repeat && ((key === "ArrowUp" && current <= 0) || (key === "ArrowDown" && current >= rows.length - 1)))
      return;
    if (key === "ArrowDown") moveFocus(current + 1, event);
    else if (key === "ArrowUp") moveFocus(current - 1, event);
    else if (key === "PageDown") moveFocus(current + page, event);
    else if (key === "PageUp") moveFocus(current - page, event);
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
    } else return;
    event.preventDefault();
    if (key === "Escape") event.stopPropagation();
  }

  const optionId = (index: number) => `${listId}-${index}`;
  const focusedInDom = focusIndex !== null && focusIndex >= start && focusIndex < end;

  return (
    <div className={cx(styles["list"], collapsed && styles["collapsed"], className)}>
      <div
        ref={viewportRef}
        className={styles["viewport"]}
        role="listbox"
        aria-label={ariaLabel}
        aria-multiselectable="true"
        aria-busy={loading || undefined}
        aria-activedescendant={focusedInDom ? optionId(focusIndex) : undefined}
        tabIndex={loading || rows.length === 0 ? -1 : 0}
        onKeyDown={onKeyDown}
        onFocus={() => {
          if (focusIndex === null && rows.length > 0) setFocusIndex(Math.max(0, indexOf(activeId)));
        }}
      >
        {loading && <Skeleton label={labels.loading} />}
        {!loading && rows.length === 0 && (
          <p className={styles["empty"]}>
            <img className={styles["emptyMark"]} src={mark} alt="" />
            {labels.empty}
          </p>
        )}
        {!loading && rows.length > 0 && (
          <div className={styles["spacer"]} style={{ height: totalSize }}>
            {rows.slice(start, end).map((row, offset) => {
              const index = start + offset;
              const marked = selecting ? selection.has(row.id) : row.id === activeId;
              const hints = [
                row.hint,
                row.store,
                row.status !== "none" ? labels.status[row.status] : null,
                row.favorite ? labels.favorite : null,
                row.running ? labels.running : null,
                !row.installed && row.downloadPercent === null ? labels.notInstalled : null,
                row.downloadPercent !== null ? labels.downloading(Math.round(row.downloadPercent)) : null,
                row.updateAvailable ? labels.updateAvailable : null,
              ].filter(Boolean);
              return (
                <div
                  key={row.id}
                  id={optionId(index)}
                  role="option"
                  aria-selected={marked}
                  aria-current={row.id === activeId ? "true" : undefined}
                  aria-posinset={index + 1}
                  aria-setsize={rows.length}
                  title={`${row.title} · ${hints.join(" · ")}`}
                  className={cx(
                    styles["row"],
                    (marked || row.id === activeId) && styles["active"],
                    !row.installed && styles["notInstalled"],
                    focusIndex === index && styles["focused"],
                  )}
                  style={{ transform: `translateY(${index * pitch}px)` }}
                  onClick={(event) => {
                    onRowClick(event, index, row);
                  }}
                  onDoubleClick={() => {
                    if (row.installed) onPlay?.(row.id);
                  }}
                >
                  {selecting && (
                    <span className={cx(styles["check"], selection.has(row.id) && styles["checked"])} aria-hidden="true">
                      {selection.has(row.id) && <Icon name="check" size="sm" />}
                    </span>
                  )}
                  <RowIcon row={row} px={px} resolveIcon={resolveIcon} />
                  <span className={styles["name"]}>{row.title}</span>
                  <span className={styles["end"]}>
                    {row.downloadPercent !== null && (
                      <span className={styles["pct"]}>{Math.round(row.downloadPercent)}%</span>
                    )}
                    {row.running && <span className={cx(styles["dot"], styles["running"])} />}
                    {row.updateAvailable && row.downloadPercent === null && <span className={styles["dot"]} />}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

const SKELETON_ROWS = 14;

/** Estrutura inteira com blocos de espera (branco 6%, pulsando em 1,6 s), nos tamanhos do que vai chegar. */
function Skeleton({ label }: { label: string }) {
  return (
    <>
      <span className={styles["sr"]} role="status">
        {label}
      </span>
      <div aria-hidden="true">
        {Array.from({ length: SKELETON_ROWS }, (_, i) => (
          <div key={i} className={styles["skeletonRow"]}>
            <span className={cx(styles["skeleton"], styles["skeletonIcon"])} />
            <span className={cx(styles["skeleton"], styles["skeletonName"])} style={{ width: `${55 + ((i * 17) % 35)}%` }} />
          </div>
        ))}
      </div>
    </>
  );
}
