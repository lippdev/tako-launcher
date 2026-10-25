import { cx } from "../../lib/cx";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import {
  EscolhaDoOnboarding,
  type EscolhaDoOnboardingOption,
} from "../EscolhaDoOnboarding/EscolhaDoOnboarding";
import {
  PassosDoOnboarding,
  PassosDoOnboardingMovimento,
  PassosDoOnboardingStaggerItem,
} from "../PassosDoOnboarding/PassosDoOnboarding";
import styles from "./OnboardingModo.module.css";

export type OnboardingModoOption<T extends string = string> = EscolhaDoOnboardingOption<T>;

export interface OnboardingModoProps<T extends string = string> {
  currentStep?: number;
  totalSteps?: number;
  stepLabel: string;
  stepAriaLabel?: string;
  title: string;
  description: string;
  optionsAriaLabel: string;
  options: OnboardingModoOption<T>[];
  value: T;
  onChange: (value: T) => void;
  continueLabel: string;
  onContinue: () => void;
  backLabel?: string;
  onBack?: () => void;
  footerNote?: string;
  className?: string;
}

export function OnboardingModo<T extends string = string>({
  currentStep = 4,
  totalSteps = 5,
  stepLabel,
  stepAriaLabel,
  title,
  description,
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
}: OnboardingModoProps<T>) {
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
        <EscolhaDoOnboarding
          variant="card"
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
