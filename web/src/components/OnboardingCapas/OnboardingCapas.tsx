import { cx } from "../../lib/cx";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { CapaDoJogo } from "../CapaDoJogo/CapaDoJogo";
import {
  EscolhaDoOnboarding,
  type EscolhaDoOnboardingOption,
} from "../EscolhaDoOnboarding/EscolhaDoOnboarding";
import { Icon } from "../Icon/Icon";
import {
  PassosDoOnboarding,
  PassosDoOnboardingMovimento,
  PassosDoOnboardingStaggerItem,
} from "../PassosDoOnboarding/PassosDoOnboarding";
import styles from "./OnboardingCapas.module.css";

export type OnboardingCapasOption<T extends string = string> = EscolhaDoOnboardingOption<T>;

export interface OnboardingCapasProps<T extends string = string> {
  currentStep?: number;
  totalSteps?: number;
  stepLabel: string;
  stepAriaLabel?: string;
  title: string;
  description: string;
  gameTitle: string;
  gameCoverUrl?: string;
  gameHighlightColor?: string;
  comparisonNote?: string;
  arrowAriaLabel?: string;
  optionsAriaLabel: string;
  options: OnboardingCapasOption<T>[];
  value: T;
  onChange: (value: T) => void;
  continueLabel: string;
  onContinue: () => void;
  backLabel?: string;
  onBack?: () => void;
  footerNote?: string;
  className?: string;
}

export function OnboardingCapas<T extends string = string>({
  currentStep = 3,
  totalSteps = 5,
  stepLabel,
  stepAriaLabel,
  title,
  description,
  gameTitle,
  gameCoverUrl,
  gameHighlightColor,
  comparisonNote,
  arrowAriaLabel,
  optionsAriaLabel,
  options,
  value,
  onChange,
  continueLabel,
  onContinue,
  backLabel,
  onBack,
  footerNote,
  className,
}: OnboardingCapasProps<T>) {
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
        <div className={styles["comparisonSection"]}>
          <div className={styles["coverBefore"]}>
            <CapaDoJogo
              title={gameTitle}
              width={120}
              height={160}
              {...(gameHighlightColor !== undefined ? { accentColor: gameHighlightColor } : {})}
            />
          </div>
          <span
            className={styles["comparisonArrow"]}
            {...(arrowAriaLabel !== undefined ? { "aria-label": arrowAriaLabel } : { "aria-hidden": true })}
          >
            <Icon name="chevron-right" />
          </span>
          <div className={styles["coverAfter"]}>
            <CapaDoJogo
              title={gameTitle}
              width={120}
              height={160}
              {...(gameCoverUrl !== undefined ? { src: gameCoverUrl } : {})}
            />
          </div>
          {comparisonNote && (
            <small className={styles["comparisonNote"]}>{comparisonNote}</small>
          )}
        </div>

        <EscolhaDoOnboarding
          variant="row"
          ariaLabel={optionsAriaLabel}
          options={options}
          value={value}
          onChange={onChange}
        />
      </PassosDoOnboardingStaggerItem>

      <PassosDoOnboardingStaggerItem index={4}>
        <div className={styles["actions"]}>
          <BotaoDesktop variant="primary" icon="chevron-right" onClick={onContinue}>
            {continueLabel}
          </BotaoDesktop>
          {backLabel && (
            <BotaoDesktop variant="outlined" onClick={onBack}>
              {backLabel}
            </BotaoDesktop>
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
