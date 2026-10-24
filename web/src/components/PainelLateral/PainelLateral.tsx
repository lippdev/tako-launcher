import { useEffect, useId, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { useLayerFocus } from "../../lib/layer-focus-trap";
import { cx } from "../../lib/cx";
import { Icon } from "../Icon/Icon";
import styles from "./PainelLateral.module.css";

interface PainelLateralProps {
  open: boolean;
  onClose: () => void;
  title: string;
  /** Contagem ao lado do título ("2 itens"). */
  count?: string;
  /** Nome do botão ✕ para leitores de tela e dica. */
  closeLabel: string;
  /** Largura de 480 (configurações do jogo) em vez de 400. */
  wide?: boolean;
  /** Rodapé fixo (totais e o botão primário na largura toda). */
  footer?: ReactNode;
  children: ReactNode;
}

/** Duração em ms da transição de `transform` do painel, lida do estilo (zero com "Reduzir animações"). */
function exitDuration(panel: HTMLElement | null): number {
  if (!panel) return 0;
  const [first] = getComputedStyle(panel).transitionDuration.split(",");
  const value = parseFloat(first ?? "0");
  return (first ?? "").trim().endsWith("ms") ? value : value * 1000;
}

/**
 * Camada que entra pela direita, num portal no `body`, sobre um véu. Esc e clique no véu fecham; o foco fica preso
 * dentro e volta para quem o tinha ao fechar. O painel fica montado até a saída terminar.
 */
export function PainelLateral({ open, onClose, title, count, closeLabel, wide = false, footer, children }: PainelLateralProps) {
  const titleId = useId();
  const panelRef = useRef<HTMLElement>(null);
  const [mounted, setMounted] = useState(open);
  const [entered, setEntered] = useState(false);

  // Montar na hora em que abre (ajuste de estado durante a renderização, sem quadro vazio).
  if (open && !mounted) setMounted(true);

  // Abrindo: espera o painel existir fora da tela para a transição ter de onde sair.
  useEffect(() => {
    if (!open || !mounted) return;
    const frame = requestAnimationFrame(() => {
      setEntered(true);
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [open, mounted]);

  // Fechando: tira do DOM quando a saída termina.
  useEffect(() => {
    if (open || !mounted) return;
    const timer = setTimeout(() => {
      setMounted(false);
      setEntered(false);
    }, exitDuration(panelRef.current));
    return () => {
      clearTimeout(timer);
    };
  }, [open, mounted]);

  useLayerFocus(panelRef, open && mounted, onClose);

  if (!mounted) return null;
  const shown = open && entered;

  return createPortal(
    <>
      <div className={cx(styles["scrim"], shown && styles["show"])} onClick={onClose} aria-hidden="true" />
      <aside
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        className={cx(styles["panel"], wide && styles["wide"], shown && styles["show"])}
      >
        <header className={styles["head"]}>
          <h2 id={titleId} className={styles["title"]}>
            {title}
          </h2>
          {count && <span className={styles["count"]}>{count}</span>}
          <button type="button" className={styles["close"]} aria-label={closeLabel} title={closeLabel} onClick={onClose}>
            <Icon name="x" />
          </button>
        </header>
        <div className={styles["body"]}>{children}</div>
        {footer && <footer className={styles["foot"]}>{footer}</footer>}
      </aside>
    </>,
    document.body,
  );
}
