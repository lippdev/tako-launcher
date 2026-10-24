import { useId, useRef, type ReactNode } from "react";
import { cx } from "../../lib/cx";
import { useEscapeLayer } from "../../lib/escapeLayers";
import { useFocusTrap } from "../../lib/focusTrap";
import { Portal } from "../../lib/Portal";
import { usePresence } from "../../lib/presence";
import { Icon } from "../Icon/Icon";
import styles from "./Dialogo.module.css";

interface DialogoProps {
  open: boolean;
  onClose: () => void;
  /** Título que diz o que aconteceu (ou a pergunta, quando pede decisão). */
  title: string;
  /** Nome do ✕ para leitores de tela (ex.: "Fechar"). */
  closeLabel: string;
  children: ReactNode;
  /** Rodapé de ações: texto à esquerda, secundária contornada e um primário por último (PadraoPopup). */
  actions?: ReactNode;
  /** Letra miúda centralizada embaixo. */
  fine?: ReactNode;
  /** 480 (padrão do Dialogo); `s` 440, `m` 540 e `l` 680 são os tamanhos do PadraoPopup. */
  size?: "padrao" | "s" | "m" | "l";
  /** `id` do elemento do conteúdo que descreve o diálogo (`aria-describedby`). */
  describedBy?: string;
  /** Clicar no véu fecha. Padrão `false`: o véu é de uma decisão que bloqueia. */
  dismissOnVeil?: boolean;
  className?: string;
}

/**
 * Diálogo (README do Dialogo e PadraoPopup): portal no `body`, véu escuro com desfoque, foco preso e devolvido a
 * quem abriu, Esc fecha. Entra subindo 12 com escala 0,98 em 350 ms; o véu escurece em 250.
 */
export function Dialogo({
  open,
  onClose,
  title,
  closeLabel,
  children,
  actions,
  fine,
  size = "padrao",
  describedBy,
  dismissOnVeil = false,
  className,
}: DialogoProps) {
  const veil = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const { mounted, visible, animating } = usePresence(open, veil);

  useEscapeLayer(open, onClose);
  useFocusTrap(open, box);

  if (!mounted) return null;

  return (
    <Portal>
      <div
        ref={veil}
        data-visible={visible}
        data-animating={animating}
        className={styles["veil"]}
        onMouseDown={(event) => {
          if (dismissOnVeil && event.target === event.currentTarget) onClose();
        }}
      >
        <div
          ref={box}
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          {...(describedBy ? { "aria-describedby": describedBy } : {})}
          tabIndex={-1}
          className={cx(styles["box"], styles[size], className)}
        >
          <header className={styles["head"]}>
            <h2 id={titleId} className={styles["title"]}>
              {title}
            </h2>
            <button type="button" className={styles["close"]} aria-label={closeLabel} data-autofocus-last="" onClick={onClose}>
              <Icon name="x" />
            </button>
          </header>
          <div className={styles["content"]}>{children}</div>
          {actions && <footer className={styles["actions"]}>{actions}</footer>}
          {fine && <p className={styles["fine"]}>{fine}</p>}
        </div>
      </div>
    </Portal>
  );
}
