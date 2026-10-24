import { type CSSProperties, type ReactNode } from "react";
import { cx } from "../../lib/cx";
import styles from "./PassosDoOnboarding.module.css";

export interface PassosDoOnboardingProps {
  currentStep: number;
  totalSteps?: number;
  label: string;
  ariaLabel?: string;
  className?: string;
}

export function PassosDoOnboarding({
  currentStep,
  totalSteps = 5,
  label,
  ariaLabel,
  className,
}: PassosDoOnboardingProps) {
  const steps = Array.from({ length: totalSteps }, (_, i) => i + 1);

  return (
    <div
      className={cx(styles["root"], className)}
      role="status"
      aria-label={ariaLabel}
    >
      <span className={styles["counter"]}>{label}</span>
      <span className={styles["bars"]} aria-hidden="true">
        {steps.map((step) => {
          const isActive = step <= currentStep;
          return (
            <i
              key={step}
              className={cx(
                styles["bar"],
                isActive ? styles["barActive"] : styles["barInactive"],
              )}
            />
          );
        })}
      </span>
    </div>
  );
}

export interface PassosDoOnboardingMovimentoProps {
  stepKey: string | number;
  children: ReactNode;
  className?: string;
}

export function PassosDoOnboardingMovimento({
  stepKey,
  children,
  className,
}: PassosDoOnboardingMovimentoProps) {
  return (
    <div key={stepKey} className={cx(styles["staggerContainer"], className)}>
      {children}
    </div>
  );
}

export interface PassosDoOnboardingStaggerItemProps {
  index: number;
  children: ReactNode;
  className?: string;
}

export function PassosDoOnboardingStaggerItem({
  index,
  children,
  className,
}: PassosDoOnboardingStaggerItemProps) {
  const style = {
    "--stagger-index": index,
  } as CSSProperties;

  return (
    <div className={cx(styles["staggerItem"], className)} style={style}>
      {children}
    </div>
  );
}
