import type { RefObject } from "react";
import { useEscapeLayer } from "./escapeLayers";
import { useFocusTrap } from "./focusTrap";

/**
 * Comportamento de foco de uma camada (painel, diálogo): foco preso dentro enquanto aberta, devolvido a quem o
 * tinha ao fechar, e Esc chama `onEscape` só se esta for a camada de cima. Junta as duas peças comuns para que
 * menus, diálogos e painéis dividam a mesma pilha do Esc.
 */
export function useLayerFocus(containerRef: RefObject<HTMLElement | null>, active: boolean, onEscape: () => void): void {
  useFocusTrap(active, containerRef);
  useEscapeLayer(active, onEscape);
}
