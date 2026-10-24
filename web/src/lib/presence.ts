import { useEffect, useState, type RefObject } from "react";

/** closed → mounted (já no DOM, com o estilo de fechado) → entering → open → leaving → closed. */
type Phase = "closed" | "mounted" | "entering" | "open" | "leaving";

interface Presence {
  /** A camada está no DOM (inclui a saída animada). */
  mounted: boolean;
  /** Estilo de aberto: as transições de entrada rodam quando vira `true` e as de saída quando volta a `false`. */
  visible: boolean;
  /** Há transição em andamento: é a hora de `will-change: transform` (e só então). */
  animating: boolean;
}

/** Espera as transições CSS em andamento dentro de `el` (nenhuma com "Reduzir animações"). */
export function settled(el: HTMLElement | null): Promise<unknown> {
  if (!el) return Promise.resolve();
  return Promise.allSettled(el.getAnimations({ subtree: true }).map((a) => a.finished));
}

/**
 * Mantém a camada no DOM durante a saída animada, sem medir tempo na mão: espera as transições reais do
 * elemento (e dos filhos) acabarem. `ref` aponta para o elemento raiz da camada.
 */
export function usePresence(open: boolean, ref: RefObject<HTMLElement | null>): Presence {
  const [phase, setPhase] = useState<Phase>(open ? "open" : "closed");

  // Ajuste durante a render (padrão do React): reage ao `open` sem efeito síncrono.
  if (open && (phase === "closed" || phase === "leaving")) setPhase("mounted");
  if (!open && (phase === "mounted" || phase === "entering" || phase === "open")) setPhase("leaving");

  useEffect(() => {
    let cancelled = false;
    const finish = (next: Phase) => {
      void settled(ref.current).then(() => {
        if (!cancelled) setPhase(next);
      });
    };
    let frame = 0;
    if (phase === "mounted") {
      // Dois quadros: o primeiro pinta o estado de fechado, o segundo dispara a transição.
      frame = requestAnimationFrame(() => {
        frame = requestAnimationFrame(() => {
          if (!cancelled) setPhase("entering");
        });
      });
    } else if (phase === "entering") {
      finish("open");
    } else if (phase === "leaving") {
      finish("closed");
    }
    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
    };
  }, [phase, ref]);

  return {
    mounted: phase !== "closed",
    visible: phase === "entering" || phase === "open",
    animating: phase !== "open" && phase !== "closed",
  };
}
