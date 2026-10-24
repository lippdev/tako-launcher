import { useEffect, useRef, useState, type ReactNode } from "react";
import mark from "../../assets/brand/tako-mark.png";
import { cx } from "../../lib/cx";
import { Portal } from "../../lib/Portal";
import { usePresence } from "../../lib/presence";
import { useLatest } from "../../lib/useLatest";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./Aviso.module.css";

export type AvisoTone = "success" | "error" | "info" | "tako";

const DEFAULT_ICON: Record<Exclude<AvisoTone, "tako">, IconName> = {
  success: "check",
  error: "badge-alert",
  info: "info",
};

interface AvisoCartaoProps {
  children: ReactNode;
  /** `tako` é notícia do próprio TAKO (jogo pronto, abrindo): o ícone é o polvo. */
  tone?: AvisoTone;
  /** Ícone no lugar do padrão do tom. */
  icon?: IconName;
  /** Ação curta (ex.: Desfazer). */
  action?: { label: string; onAction: () => void };
  className?: string;
}

/** O cartão do aviso: fundo claro, texto escuro 13/500 e ícone. `Aviso` o põe embaixo, no centro. */
export function AvisoCartao({ children, tone = "success", icon, action, className }: AvisoCartaoProps) {
  return (
    <div className={cx(styles["toast"], className)}>
      {tone === "tako" ? (
        <img className={styles["mark"]} src={mark} alt="" />
      ) : (
        <Icon name={icon ?? DEFAULT_ICON[tone]} />
      )}
      <span className={styles["message"]}>{children}</span>
      {action && (
        <button type="button" className={styles["action"]} onClick={action.onAction}>
          {action.label}
        </button>
      )}
    </div>
  );
}

interface AvisoProps extends Omit<AvisoCartaoProps, "className"> {
  open: boolean;
  /** Chamado quando o tempo acaba (a menos que o ponteiro ou o foco estejam no aviso). */
  onClose: () => void;
  /** Quanto o aviso fica na tela, em ms. Padrão 2300 (README do Aviso); `null` não some sozinho. */
  duration?: number | null;
  /** Troque a cada aviso novo com o anterior ainda aberto: reinicia a contagem. */
  noticeKey?: string | number;
}

/**
 * Confirmação curta embaixo no centro, 26 acima da borda (README do Aviso): sobe 20 em 350 ms e some em 2,3 s.
 * Fica montado o tempo todo com as regiões ao vivo (`status` e `alert`) já no DOM, que é o que faz o leitor de
 * tela anunciar o texto; erro vai para o `alert`.
 */
export function Aviso({ open, onClose, duration = 2300, noticeKey, tone = "success", ...card }: AvisoProps) {
  const slotRef = useRef<HTMLDivElement>(null);
  const { mounted, visible, animating } = usePresence(open, slotRef);
  const [paused, setPaused] = useState(false);
  const latestOnClose = useLatest(onClose);

  // Fechou com o ponteiro em cima: o `mouseleave` não vem, então a pausa acaba aqui.
  if (!open && paused) setPaused(false);

  useEffect(() => {
    if (!open || duration === null || paused) return;
    const timer = setTimeout(() => {
      latestOnClose.current();
    }, duration);
    return () => {
      clearTimeout(timer);
    };
  }, [open, duration, paused, noticeKey, latestOnClose]);

  const pause = {
    onMouseEnter: () => {
      setPaused(true);
    },
    onMouseLeave: () => {
      setPaused(false);
    },
    onFocus: () => {
      setPaused(true);
    },
    onBlur: () => {
      setPaused(false);
    },
  };

  const toast = mounted && (
    <div ref={slotRef} data-visible={visible} data-animating={animating} className={styles["slot"]} {...pause}>
      <AvisoCartao tone={tone} {...card} />
    </div>
  );

  return (
    <Portal>
      <div className={styles["wrap"]}>
        <div role="status" className={styles["region"]}>
          {tone !== "error" && toast}
        </div>
        <div role="alert" className={styles["region"]}>
          {tone === "error" && toast}
        </div>
      </div>
    </Portal>
  );
}
