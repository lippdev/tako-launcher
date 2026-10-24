import { useEffect, useRef, type RefObject } from "react";

/** Camadas abertas, da mais antiga para a mais nova: só a de cima reage ao Esc (README: "Esc fecha a camada de cima"). */
const openLayers: symbol[] = [];

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function focusableIn(container: HTMLElement): HTMLElement[] {
  // `offsetParent` é nulo em elemento `fixed`; `getClientRects` serve para os dois casos.
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => el.getClientRects().length > 0);
}

/**
 * Comportamento de foco de uma camada (painel, diálogo): ao abrir, o foco vai para o primeiro controle; Tab e
 * Shift + Tab ficam presos dentro; ao fechar, o foco volta para quem o tinha antes; Esc chama `onEscape` só se esta
 * for a camada de cima.
 */
export function useLayerFocus(containerRef: RefObject<HTMLElement | null>, active: boolean, onEscape: () => void): void {
  const onEscapeRef = useRef(onEscape);
  useEffect(() => {
    onEscapeRef.current = onEscape;
  });

  useEffect(() => {
    const container = containerRef.current;
    if (!active || !container) return;

    const id = Symbol("layer");
    openLayers.push(id);
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    (focusableIn(container)[0] ?? container).focus({ preventScroll: true });

    const onKeyDown = (event: KeyboardEvent) => {
      if (openLayers[openLayers.length - 1] !== id) return;
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onEscapeRef.current();
        return;
      }
      if (event.key !== "Tab") return;
      const items = focusableIn(container);
      const first = items[0];
      const last = items[items.length - 1];
      if (!first || !last) {
        event.preventDefault();
        container.focus({ preventScroll: true });
        return;
      }
      const current = document.activeElement;
      if (!(current instanceof Node) || !container.contains(current)) {
        event.preventDefault();
        first.focus({ preventScroll: true });
      } else if (event.shiftKey && (current === first || current === container)) {
        event.preventDefault();
        last.focus({ preventScroll: true });
      } else if (!event.shiftKey && current === last) {
        event.preventDefault();
        first.focus({ preventScroll: true });
      }
    };
    // Se o foco escapar (clique no fundo, leitor de tela), volta para dentro.
    const onFocusIn = (event: FocusEvent) => {
      if (openLayers[openLayers.length - 1] !== id) return;
      if (event.target instanceof Node && !container.contains(event.target)) {
        (focusableIn(container)[0] ?? container).focus({ preventScroll: true });
      }
    };
    document.addEventListener("keydown", onKeyDown, true);
    document.addEventListener("focusin", onFocusIn);

    return () => {
      document.removeEventListener("keydown", onKeyDown, true);
      document.removeEventListener("focusin", onFocusIn);
      const at = openLayers.indexOf(id);
      if (at >= 0) openLayers.splice(at, 1);
      if (previous?.isConnected) previous.focus({ preventScroll: true });
    };
  }, [active, containerRef]);
}
