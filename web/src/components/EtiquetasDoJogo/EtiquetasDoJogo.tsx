import { cx } from "../../lib/cx";
import styles from "./EtiquetasDoJogo.module.css";

export interface EtiquetasDoJogoProps {
  /** Lista de etiquetas/tags do jogo atribuídas pela pessoa. */
  etiquetas: readonly string[];
  /**
   * Limite máximo de etiquetas exibidas antes de resumir as excedentes em "+N".
   * Se omitido, exibe todas as etiquetas. Em Detalhes o README especifica até 3 chips + "+2".
   */
  limite?: number;
  /** Formatação do rótulo "+N" para internacionalização (padrão: `(n) => `+${n}``). */
  formatarMais?: (restantes: number) => string;
  /** Rótulo da lista de etiquetas para acessibilidade (padrão: "Etiquetas do jogo"). */
  "aria-label"?: string;
  /** Callback opcional de clique em uma etiqueta (ex.: para filtrar na biblioteca). */
  onClickEtiqueta?: (etiqueta: string) => void;
  /** Tamanho dos chips (`md` = 32px, padrão; `sm` = 26px, para a seção Detalhes). */
  tamanho?: "md" | "sm";
  className?: string;
}

/**
 * Exibição das etiquetas do jogo atribuídas pelo usuário (LIB-002, README do EtiquetasDoJogo).
 * Componente somente de exibição. Suporta truncamento com chip "+N" quando há limite de espaço.
 */
export function EtiquetasDoJogo({
  etiquetas,
  limite,
  formatarMais = (n) => `+${n}`,
  "aria-label": ariaLabel = "Etiquetas do jogo",
  onClickEtiqueta,
  tamanho = "md",
  className,
}: EtiquetasDoJogoProps) {
  const temLimite = typeof limite === "number" && limite >= 0;
  const visiveis = temLimite ? etiquetas.slice(0, limite) : etiquetas;
  const restantes = temLimite ? Math.max(0, etiquetas.length - limite) : 0;
  const etiquetasRestantes = restantes > 0 ? etiquetas.slice(limite) : [];

  return (
    <div
      role="list"
      aria-label={ariaLabel}
      className={cx(styles["container"], className)}
    >
      {visiveis.map((etiqueta) => {
        const classeChip = cx(
          styles["chip"],
          tamanho === "sm" && styles["sm"],
        );

        if (onClickEtiqueta) {
          return (
            <button
              key={etiqueta}
              type="button"
              role="listitem"
              className={classeChip}
              onClick={() => onClickEtiqueta(etiqueta)}
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
            tamanho === "sm" && styles["sm"],
            styles["mais"],
          )}
          title={etiquetasRestantes.join(", ")}
          aria-label={`${formatarMais(restantes)}: ${etiquetasRestantes.join(", ")}`}
        >
          {formatarMais(restantes)}
        </span>
      )}
    </div>
  );
}
