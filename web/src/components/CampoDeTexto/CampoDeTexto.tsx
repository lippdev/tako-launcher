import {
  useId,
  type ChangeEvent,
  type ComponentPropsWithRef,
  type ReactNode,
  type Ref,
} from "react";
import { cx } from "../../lib/cx";
import styles from "./CampoDeTexto.module.css";

export interface CampoDeTextoProps
  extends Omit<
    ComponentPropsWithRef<"input">,
    | "value"
    | "onChange"
    | "type"
    | "ref"
    | "className"
    | "children"
    | "disabled"
    | "maxLength"
    | "id"
  > {
  id?: string;
  value: string;
  onChange: (value: string) => void;
  /** Rótulo visível ou acessível do campo de texto. */
  label?: string;
  /** Oculta visualmente o rótulo (mantendo acessível para leitores de tela). */
  hideLabel?: boolean;
  /** Mensagem de instrução ou ajuda exibida abaixo do campo. */
  help?: ReactNode;
  /** Mensagem de erro que invalida o campo e acende o contorno de erro. */
  error?: ReactNode;
  /** Campo desabilitado para interação. */
  disabled?: boolean;
  /** Limite máximo de caracteres. */
  maxLength?: number;
  /** Exibe o contador de caracteres quando `maxLength` estiver presente. */
  showCount?: boolean;
  /** Formatação personalizada para o contador de caracteres (ex.: `(atual, max) => `${atual} de ${max}``). */
  renderCount?: (atual: number, max: number) => ReactNode;
  /** Tipo do campo de entrada (padrão: "text"). */
  type?: "text" | "number" | "password" | "search" | "url" | "email";
  inputRef?: Ref<HTMLInputElement>;
  className?: string;
  /** Força o estado visual para exibição estática na galeria de componentes. */
  "data-force"?: "hover" | "focus";
}

/**
 * Campo de texto de formulário do TAKO (WEB-04g):
 * Altura de 44px, rótulo acessível, ajuda, erro com alerta, desabilitado e contador opcional.
 */
export function CampoDeTexto({
  id,
  value,
  onChange,
  label,
  hideLabel,
  help,
  error,
  disabled,
  maxLength,
  showCount,
  renderCount,
  type,
  placeholder,
  inputRef,
  className,
  "data-force": dataForce,
  ...inputProps
}: CampoDeTextoProps) {
  const generatedId = useId();
  const inputId = id ?? generatedId;
  const labelId = `${inputId}-label`;
  const helpId = `${inputId}-help`;
  const errorId = `${inputId}-error`;
  const countId = `${inputId}-count`;

  const textoRotulo = label;
  const ocultarRotulo = hideLabel ?? false;
  const mensagemAjuda = help;
  const mensagemErro = error;
  const estaDesabilitado = disabled ?? false;
  const exibirContador = (showCount ?? false) && maxLength !== undefined;
  const tipoCampo = type ?? "text";

  const temErro = Boolean(mensagemErro);

  const describedIds = [
    temErro ? errorId : undefined,
    mensagemAjuda && !temErro ? helpId : undefined,
    exibirContador ? countId : undefined,
  ].filter(Boolean);
  const ariaDescribedBy = describedIds.length > 0 ? describedIds.join(" ") : undefined;

  const contagem = value ? value.length : 0;

  return (
    <div
      className={cx(
        styles["container"],
        estaDesabilitado && styles["disabled"],
        temErro && styles["hasError"],
        className,
      )}
    >
      {textoRotulo && (
        <label
          id={labelId}
          htmlFor={inputId}
          className={cx(styles["label"], ocultarRotulo && styles["srOnly"])}
        >
          {textoRotulo}
        </label>
      )}

      <div
        className={cx(
          styles["field"],
          temErro && styles["fieldError"],
          dataForce === "hover" && styles["forceHover"],
          dataForce === "focus" && styles["forceFocus"],
        )}
      >
        <span className={styles["box"]} aria-hidden="true" />
        <input
          {...inputProps}
          ref={inputRef}
          id={inputId}
          type={tipoCampo}
          value={value}
          disabled={estaDesabilitado}
          maxLength={maxLength}
          placeholder={placeholder}
          aria-labelledby={textoRotulo ? labelId : undefined}
          aria-label={!textoRotulo ? placeholder : undefined}
          aria-invalid={temErro ? true : undefined}
          aria-describedby={ariaDescribedBy}
          className={styles["input"]}
          data-force={dataForce}
          onChange={(e: ChangeEvent<HTMLInputElement>) => {
            onChange(e.target.value);
          }}
        />
      </div>

      {(mensagemErro || mensagemAjuda || exibirContador) && (
        <div className={styles["bottomRow"]}>
          <div className={styles["messageWrap"]}>
            {temErro ? (
              <span id={errorId} role="alert" className={styles["errorText"]}>
                {mensagemErro}
              </span>
            ) : mensagemAjuda ? (
              <span id={helpId} className={styles["helpText"]}>
                {mensagemAjuda}
              </span>
            ) : null}
          </div>
          {exibirContador && maxLength !== undefined && (
            <span
              id={countId}
              aria-live="polite"
              className={cx(
                styles["counter"],
                contagem >= maxLength && styles["counterFull"],
              )}
            >
              {renderCount
                ? renderCount(contagem, maxLength)
                : `${contagem} / ${maxLength}`}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
