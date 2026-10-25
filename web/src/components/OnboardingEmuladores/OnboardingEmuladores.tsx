import type { ReactNode } from "react";
import { cx } from "../../lib/cx";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { LojaEncontrada, LojaEncontradaGroup } from "../LojaEncontrada/LojaEncontrada";
import {
  PassosDoOnboarding,
  PassosDoOnboardingMovimento,
  PassosDoOnboardingStaggerItem,
} from "../PassosDoOnboarding/PassosDoOnboarding";
import styles from "./OnboardingEmuladores.module.css";

export type OnboardingEmulatorStatus = "ready" | "missing" | "not-installed";

export interface OnboardingEmulatorItem {
  id: string;
  name: string;
  details?: string;
  icon?: ReactNode;
  status: OnboardingEmulatorStatus;
  statusLabel?: string;
  actionLabel?: string;
  onAction?: () => void;
}

export interface OnboardingRomsFolder {
  name: string;
  details?: string;
  actionLabel: string;
  onAction: () => void;
}

export interface OnboardingEmuladoresProps {
  currentStep?: number;
  totalSteps?: number;
  stepLabel: string;
  stepAriaLabel?: string;
  title: string;
  description: string;
  emulators: OnboardingEmulatorItem[];
  romsFolder?: OnboardingRomsFolder;
  continueLabel: string;
  onContinue: () => void;
  addEmulatorLabel?: string;
  onAddEmulator?: () => void;
  skipEmulatorsLabel?: string;
  onSkipEmulators?: () => void;
  footerNote?: string;
  className?: string;
}

export function OnboardingEmuladores({
  currentStep = 2,
  totalSteps = 5,
  stepLabel,
  stepAriaLabel,
  title,
  description,
  emulators,
  romsFolder,
  continueLabel,
  onContinue,
  addEmulatorLabel,
  onAddEmulator,
  skipEmulatorsLabel,
  onSkipEmulators,
  footerNote,
  className,
}: OnboardingEmuladoresProps) {
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
          {emulators.map((emulator) => (
            <LojaEncontrada
              key={emulator.id}
              name={emulator.name}
              status={emulator.status}
              {...(emulator.details !== undefined ? { pathOrDetails: emulator.details } : {})}
              {...(emulator.icon !== undefined ? { icon: emulator.icon } : {})}
              {...(emulator.statusLabel !== undefined ? { statusLabel: emulator.statusLabel } : {})}
              {...(emulator.actionLabel !== undefined ? { actionLabel: emulator.actionLabel } : {})}
              {...(emulator.onAction !== undefined ? { onAction: emulator.onAction } : {})}
            />
          ))}
          {romsFolder && (
            <LojaEncontrada
              name={romsFolder.name}
              status="action"
              actionLabel={romsFolder.actionLabel}
              onAction={romsFolder.onAction}
              {...(romsFolder.details !== undefined ? { pathOrDetails: romsFolder.details } : {})}
            />
          )}
        </LojaEncontradaGroup>
      </PassosDoOnboardingStaggerItem>

      <PassosDoOnboardingStaggerItem index={4}>
        <div className={styles["actions"]}>
          <BotaoDesktop variant="primary" icon="chevron-right" onClick={onContinue}>
            {continueLabel}
          </BotaoDesktop>
          {addEmulatorLabel && (
            <BotaoDesktop variant="outlined" icon="plus" onClick={onAddEmulator}>
              {addEmulatorLabel}
            </BotaoDesktop>
          )}
          {skipEmulatorsLabel && (
            <button
              type="button"
              className={styles["skipButton"]}
              onClick={onSkipEmulators}
            >
              {skipEmulatorsLabel}
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
