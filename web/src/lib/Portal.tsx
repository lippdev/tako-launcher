import type { ReactNode } from "react";
import { createPortal } from "react-dom";

/** Renderiza as camadas (menus, diálogos, avisos) direto no `body`, fora de qualquer `overflow` ou `transform`. */
export function Portal({ children }: { children: ReactNode }) {
  return createPortal(children, document.body);
}
