import type { ReactNode } from "react";
import { cx } from "../../lib/cx";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { LojaEncontrada, LojaEncontradaGroup } from "../LojaEncontrada/LojaEncontrada";
import {
  PassosDoOnboarding,
  PassosDoOnboardingMovimento,
  PassosDoOnboardingStaggerItem,
} from "../PassosDoOnboarding/PassosDoOnboarding";
import styles from "./OnboardingPronto.module.css";

export interface OnboardingSummaryItem {
  id: string;
  name: string;
  categoryOrType?: string;
  icon?: ReactNode;
  countLabel?: string;
}

export interface OnboardingSelectedMode {
  title: string;
  description?: string;
  changeLabel: string;
  onChangeMode: () => void;
}

export interface OnboardingProntoProps {
  currentStep?: number;
  totalSteps?: number;
  stepLabel: string;
  stepAriaLabel?: string;
  title: string;
  description: string;
  summaryItems: OnboardingSummaryItem[];
  selectedMode?: OnboardingSelectedMode;
  finishLabel: string;
  onFinish: () => void;
  reviewCoversLabel?: string;
  onReviewCovers?: () => void;
  footerNote?: string;
  className?: string;
}

export function OnboardingPronto({
  currentStep = 5,
  totalSteps = 5,
  stepLabel,
  stepAriaLabel,
  title,
  description,
  summaryItems,
  selectedMode,
  finishLabel,
  onFinish,
  reviewCoversLabel,
  onReviewCovers,
  footerNote,
  className,
}: OnboardingProntoProps) {
  return (
    <PassosDoOnboardingMovimento
      stepKey={currentStep}
      className={cx(styles["root"], className)}
    >
      <PassosDoOnboardingStaggerItem index={0}>
        <PassosDoOnboarding
          currentStep={currentStep}
          totalSteps={totalSteps}
          label={stepLabel}
          {...(stepAriaLabel !== undefined ? { ariaLabel: stepAriaLabel } : {})}
        />
      </PassosDoOnboardingStaggerItem>

      <PassosDoOnboardingStaggerItem index={1}>
        <h1 className={styles["title"]}>{title}</h1>
      </PassosDoOnboardingStaggerItem>

      <PassosDoOnboardingStaggerItem index={2}>
        <p className={styles["description"]}>{description}</p>
      </PassosDoOnboardingStaggerItem>

      <PassosDoOnboardingStaggerItem index={3}>
        <LojaEncontradaGroup>
          {summaryItems.map((item) => (
            <LojaEncontrada
              key={item.id}
              name={item.name}
              status="ready"
              {...(item.categoryOrType !== undefined ? { pathOrDetails: item.categoryOrType } : {})}
              {...(item.icon !== undefined ? { icon: item.icon } : {})}
              {...(item.countLabel !== undefined ? { statusLabel: item.countLabel } : {})}
            />
          ))}
          {selectedMode && (
            <LojaEncontrada
              name={selectedMode.title}
              status="action"
              actionLabel={selectedMode.changeLabel}
              onAction={selectedMode.onChangeMode}
              {...(selectedMode.description !== undefined ? { pathOrDetails: selectedMode.description } : {})}
            />
          )}
        </LojaEncontradaGroup>
      </PassosDoOnboardingStaggerItem>

      <PassosDoOnboardingStaggerItem index={4}>
        <div className={styles["actions"]}>
          <BotaoDesktop variant="primary" icon="chevron-right" onClick={onFinish}>
            {finishLabel}
          </BotaoDesktop>
          {reviewCoversLabel && (
            <button
              type="button"
              className={styles["reviewButton"]}
              onClick={onReviewCovers}
            >
              {reviewCoversLabel}
            </button>
          )}
        </div>
      </PassosDoOnboardingStaggerItem>

      {footerNote && (
        <PassosDoOnboardingStaggerItem index={5}>
          <p className={styles["footerNote"]}>{footerNote}</p>
        </PassosDoOnboardingStaggerItem>
      )}
    </PassosDoOnboardingMovimento>
  );
}
