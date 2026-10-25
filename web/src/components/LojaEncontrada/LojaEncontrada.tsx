import { type ReactNode } from "react";
import { cx } from "../../lib/cx";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Icon } from "../Icon/Icon";
import styles from "./LojaEncontrada.module.css";

export type LojaEncontradaStatus = "ready" | "missing" | "not-installed" | "action";

export interface LojaEncontradaProps {
  name: string;
  pathOrDetails?: string;
  icon?: ReactNode;
  status: LojaEncontradaStatus;
  statusLabel?: string;
  actionLabel?: string;
  onAction?: () => void;
  /** Controle opcional da fonte (onboarding): conserva a linha e seus separadores. */
  control?: ReactNode;
  className?: string;
}

export function LojaEncontrada({
  name,
  pathOrDetails,
  icon,
  status,
  statusLabel,
  actionLabel,
  onAction,
  control,
  className,
}: LojaEncontradaProps) {
  const isNotInstalled = status === "not-installed";

  return (
    <div
      className={cx(
        styles["row"],
        isNotInstalled && styles["notInstalled"],
        className,
      )}
    >
      {icon && <div className={styles["iconWrapper"]}>{icon}</div>}

      <div className={styles["content"]}>
        <strong className={styles["name"]}>{name}</strong>
        {pathOrDetails && (
          <small className={styles["details"]}>{pathOrDetails}</small>
        )}
      </div>

      <div className={styles["control"]}>
        {control}
        {status === "ready" && statusLabel && (
          <span className={styles["readyChip"]}>
            <i className={styles["statusDot"]} aria-hidden="true" />
            <span>{statusLabel}</span>
          </span>
        )}

        {status === "missing" && (
          <div className={styles["missingContainer"]}>
            {statusLabel && (
              <span className={styles["warningAlert"]}>
                <Icon name="badge-alert" size="sm" />
                <span>{statusLabel}</span>
              </span>
            )}
            {actionLabel && (
              <BotaoDesktop variant="outlined" size="sm" onClick={onAction}>
                {actionLabel}
              </BotaoDesktop>
            )}
          </div>
        )}

        {status === "not-installed" && statusLabel && (
          <span className={styles["dimText"]}>{statusLabel}</span>
        )}

        {status === "action" && actionLabel && (
          <BotaoDesktop variant="outlined" size="sm" onClick={onAction}>
            {actionLabel}
          </BotaoDesktop>
        )}
      </div>
    </div>
  );
}

export interface LojaEncontradaGroupProps {
  children: ReactNode;
  className?: string;
}

export function LojaEncontradaGroup({
  children,
  className,
}: LojaEncontradaGroupProps) {
  return <div className={cx(styles["group"], className)}>{children}</div>;
}
