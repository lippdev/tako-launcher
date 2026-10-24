import { useEffect } from "react";
import { useLatest } from "./useLatest";

/**
 * Pilha de camadas abertas: Esc fecha só a de cima (README do design system, "Entradas"). Quem abre depois
 * fica por cima; ao fechar a de cima, o Esc seguinte vale para a de baixo.
 */
const order: symbol[] = [];
const handlers = new Map<symbol, () => void>();

function onKeyDown(event: KeyboardEvent): void {
  if (event.key !== "Escape" || event.defaultPrevented) return;
  const top = order[order.length - 1];
  if (top === undefined) return;
  event.preventDefault();
  event.stopPropagation();
  handlers.get(top)?.();
}

function register(id: symbol, handler: () => void): () => void {
  if (order.length === 0) document.addEventListener("keydown", onKeyDown, true);
  order.push(id);
  handlers.set(id, handler);
  return () => {
    handlers.delete(id);
    order.splice(order.indexOf(id), 1);
    if (order.length === 0) document.removeEventListener("keydown", onKeyDown, true);
  };
}

/** Enquanto `active`, a camada entra na pilha do Esc e `onEscape` roda quando ela está por cima. */
export function useEscapeLayer(active: boolean, onEscape: () => void): void {
  const latest = useLatest(onEscape);
  useEffect(() => {
    if (!active) return;
    return register(Symbol("layer"), () => {
      latest.current();
    });
  }, [active, latest]);
}
