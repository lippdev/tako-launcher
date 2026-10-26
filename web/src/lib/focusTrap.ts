import { useEffect, useRef, type RefObject } from "react";

export const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** Armadilhas ativas, da mais antiga para a mais nova: com diálogos empilhados, só a de cima prende o Tab. */
const traps: HTMLElement[] = [];

function focusables(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter((el) => el.getClientRects().length > 0);
}

/**
 * Prende o foco dentro de `ref` enquanto `active`: leva o foco para o primeiro controle ao abrir, faz o Tab dar
 * a volta e devolve o foco a quem o tinha ao fechar.
 */
export function useFocusTrap(active: boolean, ref: RefObject<HTMLElement | null>): void {
  const openerRef = useRef<HTMLElement | null>(null);
  useEffect(() => {
    const root = ref.current;
    if (!active || !root) return;
    // Quem tinha o foco antes de abrir. Se o foco já está dentro (efeito refeito pelo StrictMode), mantém o anterior.
    const focused = document.activeElement;
    if (focused instanceof HTMLElement && !root.contains(focused)) openerRef.current = focused;
    const opener = openerRef.current;

    // O foco inicial vai para o primeiro controle que não seja o ✕ de fechar (marcado com `data-autofocus-last`).
    const preferred = focusables(root).find((el) => !el.hasAttribute("data-autofocus-last"));
    (preferred ?? root).focus({ preventScroll: true });

    traps.push(root);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Tab" || traps[traps.length - 1] !== root) return;
      const items = focusables(root);
      const first = items[0];
      const last = items[items.length - 1];
      if (!first || !last) {
        event.preventDefault();
        root.focus();
        return;
      }
      const current = document.activeElement;
      if (event.shiftKey && (current === first || current === root || !root.contains(current))) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && (current === last || !root.contains(current))) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      traps.splice(traps.indexOf(root), 1);
      if (opener?.isConnected) opener.focus({ preventScroll: true });
    };
  }, [active, ref]);
}
