import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./PilulaDaLoja.module.css";

export interface PilulaDaLojaProps {
  /** Rótulo da pílula (ex.: "Steam", "Epic Games", "GOG", "Windows", "Compartilhar"). */
  label: string;
  /** Ícone genérico do design system (opcional). Sem marcas registradas de terceiros. */
  icon?: IconName;
  /** Callback ao clicar; quando presente, renderiza a pílula como um botão interativo. */
  onClick?: () => void;
  /** Estado ativo ou ligado (ex.: feedback "Link copiado" com destaque de sucesso). */
  active?: boolean;
  /** Desabilita o botão quando interativo. */
  disabled?: boolean;
  /** Rótulo para leitores de tela quando necessário. */
  "aria-label"?: string;
  className?: string;
  /** Força um estado visual para a galeria de componentes. */
  "data-force"?: "hover" | "focus" | "pressed";
}

/**
 * Pílula/chip de metadados no cabeçalho da página do jogo (README da PilulaDaLoja).
 * Mede 32px de altura, raio de 9px e fundo translúcido escuro com desfoque de vidro.
 * Suporta ícones genéricos (sem marcas registradas de terceiros) e modo botão com feedback.
 */
export function PilulaDaLoja({
  label,
  icon,
  onClick,
  active = false,
  disabled = false,
  "aria-label": ariaLabel,
  className,
  "data-force": dataForce,
}: PilulaDaLojaProps) {
  const classeFinal = cx(styles["chip"], active && styles["ativo"], className);

  if (onClick) {
    return (
      <button
        type="button"
        className={classeFinal}
        onClick={onClick}
        disabled={disabled}
        aria-label={ariaLabel}
        data-force={dataForce}
      >
        {icon && <Icon name={icon} size="sm" />}
        <span>{label}</span>
      </button>
    );
  }

  return (
    <span className={classeFinal} aria-label={ariaLabel} data-force={dataForce}>
      {icon && <Icon name={icon} size="sm" />}
      <span>{label}</span>
    </span>
  );
}
