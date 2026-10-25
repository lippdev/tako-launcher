import { useState, type CSSProperties } from "react";
import { cx } from "../../lib/cx";
import { Icon } from "../Icon/Icon";
import { MenuDeContexto, type MenuEntry } from "../MenuDeContexto/MenuDeContexto";
import styles from "./StatusDoJogo.module.css";

/**
 * Os 6 estados possíveis para um jogo na biblioteca.
 * Baseado no modelo Takō e no README do componente.
 */
export type GameStatus =
  | "none"
  | "playing"
  | "completed"
  | "queued"
  | "paused"
  | "dropped";

const DEFAULT_LABELS: Record<GameStatus, string> = {
  none: "Sem situação",
  playing: "Jogando",
  completed: "Zerado",
  queued: "Na fila",
  paused: "Pausado",
  dropped: "Abandonado",
};

/**
 * Cores semânticas para as bolinhas de status.
 * Seguem o padrão do sistema (--success, --accent, --warning, --error).
 */
const STATUS_COLORS: Record<GameStatus, string> = {
  none: "transparent",
  playing: "var(--accent)",
  completed: "var(--success)",
  queued: "var(--color-primary-light, #90caf9)",
  paused: "var(--warning)",
  dropped: "var(--error)",
};

const ALL_STATUSES: readonly GameStatus[] = [
  "none",
  "playing",
  "completed",
  "queued",
  "paused",
  "dropped",
];

export interface StatusDoJogoProps {
  /** Situação atual do jogo. */
  status: GameStatus;
  /** Callback ao escolher uma nova situação; se omitido, o chip é estático (somente leitura). */
  onStatusChange?: (newStatus: GameStatus) => void;
  /** Dicionário de rótulos para cada situação (internacionalização). */
  labels?: Partial<Record<GameStatus, string>>;
  /** Rótulo exibido no botão quando status for "none" (padrão: "Definir situação"). */
  defineLabel?: string;
  /** Título do menu de seleção que se abre ao clicar (padrão: "Situação do jogo"). */
  menuTitle?: string;
  /** Rótulo para leitores de tela. */
  "aria-label"?: string;
  className?: string;
  /** Força o estado visual na galeria. */
  "data-force"?: "hover" | "focus" | "pressed";
}

/**
 * Chip de situação do jogo no canto superior da página (README do StatusDoJogo).
 * Exibe o ponto colorido e o nome da situação (a cor nunca vem sozinha).
 * Quando interativo, clicar abre o menu para escolher a nova situação, com indicação na atual.
 */
export function StatusDoJogo({
  status,
  onStatusChange,
  labels,
  defineLabel = "Definir situação",
  menuTitle = "Situação do jogo",
  "aria-label": ariaLabel,
  className,
  "data-force": dataForce,
}: StatusDoJogoProps) {
  const [menuAberto, setMenuAberto] = useState(false);
  const [anchor, setAnchor] = useState<HTMLButtonElement | null>(null);

  const rotulosCompletos: Record<GameStatus, string> = {
    ...DEFAULT_LABELS,
    ...labels,
  };

  const rotuloAtual =
    status === "none" && onStatusChange ? defineLabel : rotulosCompletos[status];

  const dotStyle: CSSProperties | undefined =
    status !== "none"
      ? ({ "--sc": STATUS_COLORS[status] } as CSSProperties)
      : undefined;

  const entries: readonly MenuEntry[] = ALL_STATUSES.map((st) => ({
    type: "item",
    id: st,
    label: rotulosCompletos[st],
    color: STATUS_COLORS[st],
    kind: "radio",
    checked: st === status,
    onSelect: () => {
      setMenuAberto(false);
      onStatusChange?.(st);
    },
  }));

  if (onStatusChange) {
    return (
      <>
        <button
          ref={(node) => {
            setAnchor(node);
          }}
          type="button"
          className={cx(styles["chip"], className)}
          onClick={() => setMenuAberto((v) => !v)}
          aria-haspopup="menu"
          aria-expanded={menuAberto}
          aria-label={ariaLabel ?? `Situação do jogo: ${rotuloAtual}`}
          data-force={dataForce}
        >
          <span
            className={cx(styles["sdot"], status !== "none" && styles["on"])}
            style={dotStyle}
            aria-hidden="true"
          />
          <span>{rotuloAtual}</span>
          <Icon name="chevron-down" size="sm" />
        </button>

        <MenuDeContexto
          open={menuAberto}
          onClose={() => setMenuAberto(false)}
          anchor={anchor}
          entries={entries}
          title={menuTitle}
        />
      </>
    );
  }

  return (
    <span
      className={cx(styles["chip"], styles["estatico"], className)}
      aria-label={ariaLabel ?? `Situação do jogo: ${rotuloAtual}`}
      data-force={dataForce}
    >
      <span
        className={cx(styles["sdot"], status !== "none" && styles["on"])}
        style={dotStyle}
        aria-hidden="true"
      />
      <span>{rotuloAtual}</span>
    </span>
  );
}
