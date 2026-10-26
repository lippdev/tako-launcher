import { cx } from "../../lib/cx";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../LinhaDeConfiguracao/LinhaDeConfiguracao";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { CampoDeSelecao, type OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { Chave } from "../Chave/Chave";
import { ControleDeslizante } from "../ControleDeslizante/ControleDeslizante";
import { Icon } from "../Icon/Icon";
import styles from "./PainelControles.module.css";

export interface PainelControlesProps {
  /** Título principal do painel ("Controles"). */
  title?: string | undefined;
  /** Descrição do painel ("Botões, vibração e navegação."). */
  description?: string | undefined;
  /** Título da seção de grupo ("Controle"). */
  sectionTitle?: string | undefined;

  /** Linha 1: Testar controle */
  testTitle: string;
  controllerDetails: string;
  testActionLabel: string;
  onTest?: (() => void) | undefined;
  testDisabled?: boolean | undefined;

  /** Linha 2: Estilo dos botões */
  buttonStyleTitle: string;
  buttonStyleDescription?: string | undefined;
  buttonStyle: string;
  buttonStyleOptions: readonly OpcaoDeSelecao<string>[];
  onButtonStyleChange: (value: string) => void;
  buttonStyleAriaLabel?: string | undefined;

  /** Linha 3: Vibração */
  vibrationTitle: string;
  vibrationDescription?: string | undefined;
  vibration: boolean;
  onVibrationChange: (enabled: boolean) => void;
  vibrationAriaLabel?: string | undefined;

  /** Linha 4: Zona morta */
  deadzoneTitle: string;
  deadzoneDescription?: string | undefined;
  deadzone: number;
  onDeadzoneChange: (value: number) => void;
  deadzoneMin?: number | undefined;
  deadzoneMax?: number | undefined;
  deadzoneStep?: number | undefined;
  deadzoneFormat?: ((value: number) => string) | undefined;
  deadzoneAriaLabel?: string | undefined;

  /** Linha 5: Navegar com controle */
  navigationTitle: string;
  navigationDescription?: string | undefined;
  navigation: boolean;
  onNavigationChange: (enabled: boolean) => void;
  navigationAriaLabel?: string | undefined;

  className?: string | undefined;
}

export function PainelControles({
  title,
  description,
  sectionTitle,
  testTitle,
  controllerDetails,
  testActionLabel,
  onTest,
  testDisabled = false,
  buttonStyleTitle,
  buttonStyleDescription,
  buttonStyle,
  buttonStyleOptions,
  onButtonStyleChange,
  buttonStyleAriaLabel,
  vibrationTitle,
  vibrationDescription,
  vibration,
  onVibrationChange,
  vibrationAriaLabel,
  deadzoneTitle,
  deadzoneDescription,
  deadzone,
  onDeadzoneChange,
  deadzoneMin = 0,
  deadzoneMax = 30,
  deadzoneStep = 1,
  deadzoneFormat,
  deadzoneAriaLabel,
  navigationTitle,
  navigationDescription,
  navigation,
  onNavigationChange,
  navigationAriaLabel,
  className,
}: PainelControlesProps) {
  return (
    <div className={cx(styles["root"], className)}>
      {title && (
        <header className={styles["header"]}>
          <span className={styles["iconWrapper"]} aria-hidden="true">
            <Icon name="gamepad" />
          </span>
          <div className={styles["headerText"]}>
            <h1 className={styles["title"]}>{title}</h1>
            {description && <p className={styles["description"]}>{description}</p>}
          </div>
        </header>
      )}

      <GrupoDeConfiguracao {...(sectionTitle !== undefined ? { title: sectionTitle } : {})}>
        <LinhaDeConfiguracao
          title={testTitle}
          description={controllerDetails}
          control={
            <BotaoDesktop
              variant="outlined"
              disabled={testDisabled}
              onClick={onTest}
            >
              {testActionLabel}
            </BotaoDesktop>
          }
        />
        <LinhaDeConfiguracao
          title={buttonStyleTitle}
          {...(buttonStyleDescription !== undefined ? { description: buttonStyleDescription } : {})}
          control={
            <CampoDeSelecao
              label={buttonStyleAriaLabel ?? buttonStyleTitle}
              options={buttonStyleOptions}
              value={buttonStyle}
              onChange={onButtonStyleChange}
            />
          }
        />
        <LinhaDeConfiguracao
          title={vibrationTitle}
          {...(vibrationDescription !== undefined ? { description: vibrationDescription } : {})}
          control={(ids) => (
            <Chave
              checked={vibration}
              onChange={onVibrationChange}
              aria-labelledby={ids.labelId}
              aria-describedby={ids.descriptionId}
              {...(vibrationAriaLabel !== undefined ? { "aria-label": vibrationAriaLabel } : {})}
            />
          )}
        />
        <LinhaDeConfiguracao
          title={deadzoneTitle}
          {...(deadzoneDescription !== undefined ? { description: deadzoneDescription } : {})}
          control={(ids) => (
            <ControleDeslizante
              value={deadzone}
              onChange={onDeadzoneChange}
              min={deadzoneMin}
              max={deadzoneMax}
              step={deadzoneStep}
              aria-labelledby={ids.labelId}
              aria-describedby={ids.descriptionId}
              {...(deadzoneFormat !== undefined ? { format: deadzoneFormat } : {})}
              {...(deadzoneAriaLabel !== undefined ? { "aria-label": deadzoneAriaLabel } : {})}
            />
          )}
        />
        <LinhaDeConfiguracao
          title={navigationTitle}
          {...(navigationDescription !== undefined ? { description: navigationDescription } : {})}
          control={(ids) => (
            <Chave
              checked={navigation}
              onChange={onNavigationChange}
              aria-labelledby={ids.labelId}
              aria-describedby={ids.descriptionId}
              {...(navigationAriaLabel !== undefined ? { "aria-label": navigationAriaLabel } : {})}
            />
          )}
        />
      </GrupoDeConfiguracao>
    </div>
  );
}
