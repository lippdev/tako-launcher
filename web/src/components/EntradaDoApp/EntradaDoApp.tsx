import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import kana from "../../assets/brand/tako-kana.png";
import mark from "../../assets/brand/tako-mark.png";
import wordmark from "../../assets/brand/tako-wordmark.png";
import { cx } from "../../lib/cx";
import { useLatest } from "../../lib/useLatest";
import styles from "./EntradaDoApp.module.css";

export interface EntradaDoAppProps {
  /** Chamado exatamente uma vez ao finalizar a animação de entrada ou ao ser pulada. */
  onDone?: () => void;
  /** Conteúdo opcional montado por baixo da entrada (a primeira tela do app). */
  children?: ReactNode;
  /** Texto opcional exibido durante a entrada (por props, sem texto fixo no componente). */
  texto?: ReactNode;
  /** Rótulo de acessibilidade opcional para o overlay da entrada. */
  ariaLabel?: string;
  /** Se true, usa posição fixa na tela inteira (`position: fixed`). Padrão: `false` (usa `position: absolute`). */
  fixed?: boolean;
  /** Classe CSS opcional para o contêiner raiz. */
  className?: string;
}

/** Verifica se a opção 'Reduzir animações' está ativa no sistema ou no documento. */
function checkReducedMotion(): boolean {
  if (typeof document === "undefined") return false;
  const attr = document.documentElement.getAttribute("data-reduce-motion");
  if (attr === "true") return true;
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  }
  return false;
}

/**
 * Entrada do app (BRAND-002): animação curta de abertura com o polvo e a marca.
 * Curta (~1 s), pulável por clique ou qualquer tecla (Esc, Enter etc.) e desligada
 * quando 'Reduzir animações' estiver ativo. Não bloqueia a primeira tela por baixo.
 */
export function EntradaDoApp({
  onDone,
  children,
  texto,
  ariaLabel,
  fixed = false,
  className,
}: EntradaDoAppProps) {
  const latestOnDone = useLatest(onDone);
  const doneCalledRef = useRef(false);

  // Inicializa o estado: se 'Reduzir animações' estiver ligado, já entra concluído.
  const [phase, setPhase] = useState<"entrando" | "saindo" | "concluido">(() => {
    return checkReducedMotion() ? "concluido" : "entrando";
  });

  const finish = useCallback(() => {
    if (doneCalledRef.current) return;
    doneCalledRef.current = true;
    setPhase("concluido");
    latestOnDone.current?.();
  }, [latestOnDone]);

  // Se iniciou já concluído (Reduzir animações ativo), chama `onDone` imediatamente.
  useEffect(() => {
    if (phase === "concluido" && !doneCalledRef.current) {
      doneCalledRef.current = true;
      latestOnDone.current?.();
    }
  }, [phase, latestOnDone]);

  // Temporizadores da animação (~800 ms de permanência, 200 ms de saída, totalizando ~1 s).
  useEffect(() => {
    if (phase === "entrando") {
      const holdTimer = setTimeout(() => {
        setPhase("saindo");
      }, 800);
      return () => clearTimeout(holdTimer);
    }

    if (phase === "saindo") {
      const exitTimer = setTimeout(() => {
        finish();
      }, 200);
      return () => clearTimeout(exitTimer);
    }
  }, [phase, finish]);

  // Atalho de teclado para pular a qualquer instante (Esc, Enter ou qualquer tecla).
  useEffect(() => {
    if (phase === "concluido") return;

    const handleKeyDown = (event: KeyboardEvent) => {
      event.stopPropagation();
      finish();
    };

    window.addEventListener("keydown", handleKeyDown, true);

    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
    };
  }, [phase, finish]);

  const overlayElement = phase !== "concluido" && (
    <div
      role="presentation"
      aria-label={ariaLabel}
      className={cx(
        styles["overlay"],
        fixed && styles["overlayFixed"],
        phase === "saindo" && styles["overlaySaindo"],
      )}
      onPointerDown={(e) => {
        e.stopPropagation();
        finish();
      }}
    >
      <div
        className={cx(
          styles["brandGroup"],
          phase === "entrando" && styles["brandGroupEntrando"],
        )}
      >
        <img className={styles["mark"]} src={mark} alt="" />
        <div className={styles["brandName"]}>
          <img className={styles["word"]} src={wordmark} alt="" />
          <img className={styles["kana"]} src={kana} alt="" />
        </div>
        {texto && <div className={styles["texto"]}>{texto}</div>}
      </div>
    </div>
  );

  if (!children) {
    return overlayElement;
  }

  return (
    <div className={cx(styles["container"], className)}>
      {children}
      {overlayElement}
    </div>
  );
}
