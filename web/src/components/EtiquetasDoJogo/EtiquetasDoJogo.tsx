import { cx } from "../../lib/cx";
import styles from "./EtiquetasDoJogo.module.css";

export interface EtiquetasDoJogoProps {
  /** Lista de etiquetas/tags do jogo atribuídas pela pessoa. */
  tags: readonly string[];
  /**
   * Limite máximo de etiquetas exibidas antes de resumir as excedentes em "+N".
   * Se omitido, exibe todas as etiquetas. Em Detalhes o README especifica até 3 chips + "+2".
   */
  limit?: number;
  /** Formatação do rótulo "+N" para internacionalização (padrão: `(n) => `+${n}``). */
  formatMore?: (remaining: number) => string;
  /** Rótulo da lista de etiquetas para acessibilidade (padrão: "Etiquetas do jogo"). */
  "aria-label"?: string;
  /** Callback opcional de clique em uma etiqueta (ex.: para filtrar na biblioteca). */
  onTagClick?: (tag: string) => void;
  /** Tamanho dos chips (`md` = 32px, padrão; `sm` = 26px, para a seção Detalhes). */
  size?: "md" | "sm";
  className?: string;
}

/**
 * Exibição das etiquetas do jogo atribuídas pelo usuário (LIB-002, README do EtiquetasDoJogo).
 * Componente somente de exibição. Suporta truncamento com chip "+N" quando há limite de espaço.
 */
export function EtiquetasDoJogo({
  tags,
  limit,
  formatMore = (n) => `+${n}`,
  "aria-label": ariaLabel = "Etiquetas do jogo",
  onTagClick,
  size = "md",
  className,
}: EtiquetasDoJogoProps) {
  const temLimite = typeof limit === "number" && limit >= 0;
  const visiveis = temLimite ? tags.slice(0, limit) : tags;
  const restantes = temLimite ? Math.max(0, tags.length - limit) : 0;
  const etiquetasRestantes = restantes > 0 ? tags.slice(limit) : [];

  return (
    <div
      role="list"
      aria-label={ariaLabel}
      className={cx(styles["container"], className)}
    >
      {visiveis.map((etiqueta) => {
        const classeChip = cx(
          styles["chip"],
          size === "sm" && styles["sm"],
        );

        if (onTagClick) {
          return (
            <button
              key={etiqueta}
              type="button"
              role="listitem"
              className={classeChip}
              onClick={() => onTagClick(etiqueta)}
            >
              <span>{etiqueta}</span>
            </button>
          );
        }

        return (
          <span key={etiqueta} role="listitem" className={classeChip}>
            <span>{etiqueta}</span>
          </span>
        );
      })}

      {restantes > 0 && (
        <span
          role="listitem"
          className={cx(
            styles["chip"],
            size === "sm" && styles["sm"],
            styles["mais"],
          )}
          title={etiquetasRestantes.join(", ")}
          aria-label={`${formatMore(restantes)}: ${etiquetasRestantes.join(", ")}`}
        >
          {formatMore(restantes)}
        </span>
      )}
    </div>
  );
}
