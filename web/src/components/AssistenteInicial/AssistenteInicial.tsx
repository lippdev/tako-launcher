import { Fragment, type ReactNode } from "react";
import { cx } from "../../lib/cx";
import { Dialogo } from "../Dialogo/Dialogo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./AssistenteInicial.module.css";

export interface AssistenteInicialStep {
  id: string;
  label: string;
}

export interface AssistenteInicialProps {
  /** Se o diálogo modal está aberto. */
  open?: boolean | undefined;
  /** Se deve renderizar inline (sem Portal nem véu), útil para visualização na galeria. */
  inline?: boolean | undefined;
  /** Passos exibidos na barra superior. */
  steps: readonly AssistenteInicialStep[];
  /** Passo atual (1-based, ex.: 1, 2, 3). */
  currentStep: number;
  /** Rótulo acessível da lista de passos (`aria-label`). */
  stepsAriaLabel?: string | undefined;
  /** Texto superior em caixa alta ("PRIMEIROS PASSOS · n DE 3"). */
  kicker: string;
  /** Título principal do passo. */
  title: string;
  /** Descrição explicativa do passo. */
  description?: string | undefined;
  /** Conteúdo customizado do passo (ex.: lista de origens em LinhaDeConfiguracao). */
  children?: ReactNode;
  /** Rótulo da ação de pular ("Pular por agora"). */
  skipLabel?: string | undefined;
  /** Callback ao acionar pular. */
  onSkip?: (() => void) | undefined;
  /** Rótulo do botão voltar ("Voltar"). */
  backLabel?: string | undefined;
  /** Callback ao clicar em voltar. */
  onBack?: (() => void) | undefined;
  /** Rótulo do botão de avançar ("Continuar"). */
  nextLabel?: string | undefined;
  /** Ícone do botão de avançar. */
  nextIcon?: IconName | undefined;
  /** Se o botão de avançar está desabilitado. */
  nextDisabled?: boolean | undefined;
  /** Callback ao avançar. */
  onNext?: (() => void) | undefined;
  /** Callback de fechamento do diálogo. */
  onClose?: (() => void) | undefined;
  /** Rótulo de fechamento para acessibilidade. */
  closeLabel?: string | undefined;
  /** Classe CSS adicional. */
  className?: string | undefined;
}

export function AssistenteInicial({
  open = true,
  inline = false,
  steps,
  currentStep,
  stepsAriaLabel,
  kicker,
  title,
  description,
  children,
  skipLabel,
  onSkip,
  backLabel,
  onBack,
  nextLabel,
  nextIcon = "chevron-right",
  nextDisabled = false,
  onNext,
  onClose,
  closeLabel,
  className,
}: AssistenteInicialProps) {
  const content = (
    <div className={styles["inner"]}>
      <ol className={styles["steps"]} aria-label={stepsAriaLabel}>
        {steps.map((step, index) => {
          const stepNumber = index + 1;
          const isCompleted = stepNumber < currentStep;
          const isCurrent = stepNumber === currentStep;

          return (
            <Fragment key={step.id}>
              {index > 0 && <li className={styles["stepDivider"]} aria-hidden="true" />}
              <li
                className={cx(
                  styles["stepItem"],
                  isCompleted && styles["stepCompleted"],
                  isCurrent && styles["stepCurrent"],
                )}
                aria-current={isCurrent ? "step" : undefined}
              >
                {isCompleted && (
                  <span className={styles["checkIcon"]} aria-hidden="true">
                    <Icon name="check" size="sm" />
                  </span>
                )}
                <span className={styles["stepText"]}>{step.label}</span>
              </li>
            </Fragment>
          );
        })}
      </ol>

      <span className={styles["kicker"]}>{kicker}</span>
      <h2 className={styles["title"]}>{title}</h2>
      {description && <p className={styles["description"]}>{description}</p>}

      {children && <div className={styles["body"]}>{children}</div>}

      <footer className={styles["actions"]}>
        {skipLabel && (
          <button
            type="button"
            className={styles["skipButton"]}
            onClick={onSkip}
          >
            {skipLabel}
          </button>
        )}
        {backLabel && (
          <BotaoDesktop variant="outlined" onClick={onBack}>
            {backLabel}
          </BotaoDesktop>
        )}
        {nextLabel && (
          <BotaoDesktop
            variant="primary"
            size="sm"
            icon={nextIcon}
            disabled={nextDisabled}
            onClick={onNext}
          >
            {nextLabel}
          </BotaoDesktop>
        )}
      </footer>
    </div>
  );

  if (inline) {
    return (
      <div className={cx(styles["root"], styles["box"], className)} role="region" aria-label={title}>
        {content}
      </div>
    );
  }

  return (
    <Dialogo
      open={open}
      onClose={onClose ?? onSkip ?? (() => undefined)}
      title={title}
      closeLabel={closeLabel ?? ""}
      className={cx(styles["root"], styles["dialogo"], className)}
    >
      {content}
    </Dialogo>
  );
}
