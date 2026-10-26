// Filtro de eco (WEB-05b): com o Steam Input convertendo comandos do controle em teclas de sistema (setas, Esc),
// o mesmo aperto chega duplicado (uma vez pelo controle e outra pelo teclado). Descarta a repetição da mesma
// ação vinda da outra origem numa janela de 80 ms, sem atrasar a primeira ocorrência.

import type { NavAction } from "./keymap";

export type EchoSource = "controller" | "keyboard";

export interface RecordedAction {
  action: NavAction;
  source: EchoSource;
  time: number;
}

export function sameAction(a: NavAction, b: NavAction): boolean {
  if (a.kind !== b.kind) return false;
  switch (a.kind) {
    case "navigate":
      return a.direction === (b as typeof a).direction;
    case "section":
      return a.delta === (b as typeof a).delta;
    case "command":
      return a.command === (b as typeof a).command;
    case "activate":
    case "back":
    case "search":
      return true;
  }
}

/**
 * Função pura que determina se `incoming` é um eco de `last`:
 * - Mesma ação de navegação;
 * - Origem oposta (controle ↔ teclado);
 * - Dentro da janela de tempo (< `windowMs`, padrão 80 ms).
 */
export function isEcho(
  incoming: RecordedAction,
  last: RecordedAction | null,
  windowMs = 80,
): boolean {
  if (!last) return false;
  if (incoming.source === last.source) return false;
  const delta = incoming.time - last.time;
  if (delta < 0 || delta >= windowMs) return false;
  return sameAction(incoming.action, last.action);
}

let lastRecorded: RecordedAction | null = null;

/**
 * Descarta a ocorrência se for eco da outra origem na janela de 80 ms.
 * A primeira ocorrência passa na hora (retorna `false`) e fica registrada.
 */
export function shouldDropEcho(
  action: NavAction,
  source: EchoSource,
  now: number = typeof performance !== "undefined" ? performance.now() : Date.now(),
  windowMs = 80,
): boolean {
  const incoming: RecordedAction = { action, source, time: now };
  if (isEcho(incoming, lastRecorded, windowMs)) {
    return true;
  }
  lastRecorded = incoming;
  return false;
}

export function resetEchoFilter(): void {
  lastRecorded = null;
}
