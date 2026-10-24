import { useRef, type CSSProperties, type KeyboardEvent, type PointerEvent } from "react";
import { cx } from "../../lib/cx";
import styles from "./ControleDeslizante.module.css";

interface ControleDeslizanteProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  /** Texto do valor ("60%", "28px"). Sem ele, mostra o número. */
  format?: (value: number) => string;
  /** Esconde o valor à direita (o `aria-valuetext` continua). */
  hideValue?: boolean;
  disabled?: boolean;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  "aria-describedby"?: string;
  className?: string;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

/**
 * Valor contínuo (trilho 180×4, bolinha de 18, valor à direita). Teclado: ← → (e Page Up/Down, Home e End) mexem no
 * valor; ↑ ↓ ficam para a navegação entre as linhas. No mínimo e no máximo a seta não é do deslizante: um aperto novo
 * segue para o navegador de foco (← vai às categorias); segurar a tecla até a ponta não pula para fora.
 */
export function ControleDeslizante({
  value,
  onChange,
  min = 0,
  max = 100,
  step = 1,
  format,
  hideValue = false,
  disabled = false,
  className,
  ...aria
}: ControleDeslizanteProps) {
  const railRef = useRef<HTMLDivElement>(null);
  const thumbRef = useRef<HTMLSpanElement>(null);
  const dragging = useRef(false);

  const text = format ? format(value) : String(value);
  const ratio = max > min ? (clamp(value, min, max) - min) / (max - min) : 0;

  /** Valor sob o ponteiro: o centro da bolinha percorre o trilho menos a largura dela. */
  function valueAt(clientX: number): number {
    const rail = railRef.current?.getBoundingClientRect();
    const thumb = thumbRef.current?.getBoundingClientRect();
    if (!rail || !thumb || rail.width <= thumb.width) return value;
    const travel = rail.width - thumb.width;
    const at = clamp((clientX - rail.left - thumb.width / 2) / travel, 0, 1);
    const stepped = Math.round((at * (max - min)) / step) * step + min;
    return clamp(Number(stepped.toFixed(10)), min, max);
  }

  function commit(next: number) {
    if (next !== value) onChange(next);
  }

  function onPointerDown(event: PointerEvent<HTMLDivElement>) {
    if (disabled || event.button !== 0) return;
    dragging.current = true;
    event.currentTarget.setPointerCapture(event.pointerId);
    event.currentTarget.focus();
    commit(valueAt(event.clientX));
  }

  function onPointerMove(event: PointerEvent<HTMLDivElement>) {
    if (dragging.current) commit(valueAt(event.clientX));
  }

  function onPointerUp(event: PointerEvent<HTMLDivElement>) {
    dragging.current = false;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  }

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (disabled) return;
    const page = step * 10;
    if (!event.repeat && ((event.key === "ArrowLeft" && value <= min) || (event.key === "ArrowRight" && value >= max))) return;
    const targets: Record<string, number> = {
      ArrowRight: value + step,
      ArrowLeft: value - step,
      PageUp: value + page,
      PageDown: value - page,
      Home: min,
      End: max,
    };
    const target = targets[event.key];
    if (target === undefined) return;
    event.preventDefault();
    commit(clamp(Number(target.toFixed(10)), min, max));
  }

  return (
    <div className={cx(styles["range"], className)} style={{ "--p": ratio } as CSSProperties}>
      <div
        ref={railRef}
        role="slider"
        tabIndex={disabled ? -1 : 0}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={value}
        aria-valuetext={text}
        aria-orientation="horizontal"
        aria-disabled={disabled || undefined}
        className={styles["hit"]}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onKeyDown={onKeyDown}
        {...aria}
      >
        <span className={styles["track"]} aria-hidden="true">
          <span className={styles["fill"]} />
        </span>
        <span ref={thumbRef} className={styles["thumb"]} aria-hidden="true" />
      </div>
      {!hideValue && (
        <output className={styles["value"]} aria-hidden="true">
          {text}
        </output>
      )}
    </div>
  );
}
