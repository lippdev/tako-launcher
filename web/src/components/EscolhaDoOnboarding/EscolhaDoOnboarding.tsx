import {
  type KeyboardEvent,
  type ReactNode,
  useRef,
} from "react";
import { cx } from "../../lib/cx";
import { Icon } from "../Icon/Icon";
import styles from "./EscolhaDoOnboarding.module.css";

export interface EscolhaDoOnboardingOption<T extends string = string> {
  value: T;
  title: string;
  description?: string;
  hint?: string;
  thumbnail?: ReactNode;
  disabled?: boolean;
}

export interface EscolhaDoOnboardingProps<T extends string = string> {
  variant?: "card" | "row";
  value: T;
  onChange: (value: T) => void;
  options: EscolhaDoOnboardingOption<T>[];
  ariaLabel: string;
  className?: string;
}

export function EscolhaDoOnboarding<T extends string = string>({
  variant = "card",
  value,
  onChange,
  options,
  ariaLabel,
  className,
}: EscolhaDoOnboardingProps<T>) {
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const handleKeyDown = (e: KeyboardEvent<HTMLButtonElement>, currentIndex: number) => {
    let targetIndex = -1;

    if (e.key === "ArrowDown" || e.key === "ArrowRight") {
      e.preventDefault();
      targetIndex = (currentIndex + 1) % options.length;
    } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
      e.preventDefault();
      targetIndex = (currentIndex - 1 + options.length) % options.length;
    } else if (e.key === " " || e.key === "Enter") {
      e.preventDefault();
      const opt = options[currentIndex];
      if (opt && !opt.disabled) {
        onChange(opt.value);
      }
      return;
    }

    if (targetIndex >= 0) {
      const targetOption = options[targetIndex];
      if (targetOption && !targetOption.disabled) {
        onChange(targetOption.value);
        itemRefs.current[targetIndex]?.focus();
      }
    }
  };

  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className={cx(
        styles["group"],
        variant === "card" ? styles["variantCard"] : styles["variantRow"],
        className,
      )}
    >
      {options.map((option, index) => {
        const isSelected = option.value === value;

        if (variant === "card") {
          return (
            <button
              key={option.value}
              ref={(el) => {
                itemRefs.current[index] = el;
              }}
              type="button"
              role="radio"
              aria-checked={isSelected}
              tabIndex={isSelected ? 0 : -1}
              disabled={option.disabled}
              onClick={() => {
                if (!option.disabled) {
                  onChange(option.value);
                }
              }}
              onKeyDown={(e) => handleKeyDown(e, index)}
              className={cx(
                styles["cardItem"],
                isSelected && styles["cardItemSelected"],
                option.disabled && styles["disabled"],
              )}
            >
              {option.thumbnail && (
                <div className={styles["thumbnailContainer"]} aria-hidden="true">
                  {option.thumbnail}
                </div>
              )}
              <div className={styles["cardHeader"]}>
                <span className={styles["cardTitle"]}>{option.title}</span>
                {isSelected && (
                  <span className={styles["cardCheck"]} aria-hidden="true">
                    <Icon name="check" size="sm" />
                  </span>
                )}
              </div>
              {option.description && (
                <span className={styles["cardDescription"]}>
                  {option.description}
                </span>
              )}
              {option.hint && (
                <span className={styles["cardHint"]}>{option.hint}</span>
              )}
            </button>
          );
        }

        return (
          <button
            key={option.value}
            ref={(el) => {
              itemRefs.current[index] = el;
            }}
            type="button"
            role="radio"
            aria-checked={isSelected}
            tabIndex={isSelected ? 0 : -1}
            disabled={option.disabled}
            onClick={() => {
              if (!option.disabled) {
                onChange(option.value);
              }
            }}
            onKeyDown={(e) => handleKeyDown(e, index)}
            className={cx(
              styles["rowItem"],
              isSelected && styles["rowItemSelected"],
              option.disabled && styles["disabled"],
            )}
          >
            <span
              className={cx(
                styles["radioIndicator"],
                isSelected && styles["radioIndicatorSelected"],
              )}
              aria-hidden="true"
            />
            <span className={styles["rowText"]}>
              <strong className={styles["rowTitle"]}>{option.title}</strong>
              {option.description && (
                <span className={styles["rowDescription"]}>
                  {option.description}
                </span>
              )}
            </span>
          </button>
        );
      })}
    </div>
  );
}
