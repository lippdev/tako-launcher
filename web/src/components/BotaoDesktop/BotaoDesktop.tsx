import type { ComponentPropsWithRef, ReactNode } from "react";
import { cx } from "../../lib/cx";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./BotaoDesktop.module.css";

interface BaseProps extends Omit<ComponentPropsWithRef<"button">, "children"> {
  /** `sm` = 38 (primário e secundário). */
  size?: "md" | "sm";
  /** Ícone do chip (primário e secundário), do botão quadrado ou à frente do texto (contornado e chip). */
  icon?: IconName;
  /** Ícone preenchido: só ▶, estrela e coração ligados. */
  iconFilled?: boolean;
  /** Ocupa a largura toda. */
  full?: boolean;
}

interface ActionProps extends BaseProps {
  /** Primário: um por região. Secundário: o mesmo formato, fantasma. */
  variant?: "primary" | "secondary";
  children: ReactNode;
  /** Primário de estado (Atualizar): na cor de destaque. */
  tone?: "state";
  /** Baixando: barra de 3 no pé do botão, de 0 a 1. */
  progress?: number;
  /** Ligado (instalado, online): contorno verde. */
  on?: boolean;
}

interface SmallProps extends BaseProps {
  /** Contornado (Trocar, Procurar, Copiar) ou chip (situação, Windows, Compartilhar). */
  variant: "outlined" | "chip";
  children: ReactNode;
  on?: boolean;
  /** Ponto de situação à frente do texto. */
  dot?: "success" | "accent";
  /** Ícone depois do texto (ex.: a seta de um chip que abre menu). */
  trailingIcon?: IconName;
}

interface SquareProps extends BaseProps {
  /** Quadrado só de ícone (★, ♥, ✕): o nome é obrigatório. */
  variant: "square";
  icon: IconName;
  "aria-label": string;
  children?: never;
  /** Ligado (favorito, desejo): vira botão de alternância (`aria-pressed`). */
  on?: boolean;
  /** Cor de quando ligado: estrela em `warning`, coração em `wish`. */
  tone?: "warning" | "wish";
}

export type BotaoDesktopProps = ActionProps | SmallProps | SquareProps;

/** Todas as props possíveis, para o corpo do componente; a união acima é o que quem usa enxerga. */
type AnyProps = BaseProps & {
  variant?: "primary" | "secondary" | "outlined" | "chip" | "square";
  children?: ReactNode;
  on?: boolean;
  tone?: "state" | "warning" | "wish";
  progress?: number;
  dot?: "success" | "accent";
  trailingIcon?: IconName;
};

/** Botão do modo desktop (README do BotaoDesktop): primário, secundário, quadrado, contornado e chip. */
export function BotaoDesktop(props: BotaoDesktopProps) {
  const {
    variant = "primary",
    size = "md",
    icon,
    iconFilled,
    full,
    className,
    type = "button",
    on,
    tone,
    progress,
    dot,
    trailingIcon,
    children,
    ...button
  } = props as AnyProps;

  if (variant === "square") {
    const color = styles[`on-${tone === "wish" ? "wish" : "warning"}`];
    return (
      <button
        {...button}
        type={type}
        className={cx(styles["btn"], styles["square"], on && color, full && styles["full"], className)}
        {...(on === undefined ? {} : { "aria-pressed": on })}
      >
        {icon && <Icon name={icon} filled={iconFilled ?? !!on} />}
      </button>
    );
  }

  if (variant === "outlined" || variant === "chip") {
    return (
      <button
        {...button}
        type={type}
        className={cx(styles["btn"], styles[variant], on && styles["on"], full && styles["full"], className)}
      >
        {dot && <span className={cx(styles["dot"], styles[`dot-${dot}`])} aria-hidden="true" />}
        {icon && <Icon name={icon} size="sm" filled={iconFilled ?? false} />}
        {children}
        {trailingIcon && <Icon name={trailingIcon} size="sm" />}
      </button>
    );
  }

  return (
    <button
      {...button}
      type={type}
      className={cx(
        styles["btn"],
        styles["action"],
        styles[variant],
        size === "sm" && styles["sm"],
        tone === "state" && styles["state"],
        on && styles["on"],
        !icon && styles["noChip"],
        full && styles["full"],
        className,
      )}
    >
      <span className={styles["text"]}>{children}</span>
      {icon && (
        <span className={styles["iconChip"]}>
          <Icon name={icon} filled={iconFilled ?? false} />
        </span>
      )}
      {progress !== undefined && (
        <span
          className={styles["bar"]}
          aria-hidden="true"
          style={{ transform: `scaleX(${Math.min(1, Math.max(0, progress))})` }}
        />
      )}
    </button>
  );
}
