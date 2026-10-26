import { call } from "../bridge";
import type { WindowAction } from "../bridge/generated/WindowAction";

/** Pede ao hospedeiro uma ação da janela. Sem hospedeiro (ou se ele recusar) só registra: a barra não quebra. */
export function windowAction(action: WindowAction): void {
  call("window.action", action).catch((error: unknown) => {
    console.error(`janela: ${action} falhou`, error);
  });
}
