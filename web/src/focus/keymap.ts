// Teclas → ações de navegação. Mesmo mapa do app atual (`crates/tako-ui/src/shell/keys.rs`): setas movem,
// Enter e Espaço ativam (A), Esc e Backspace voltam (B), Q e E trocam de seção (LB e RB). F, / e M ainda não
// têm tela na interface web.

import type { Direction } from "./spatial";

/** O que a interface sabe fazer por quem a controla (teclado hoje, controle pela ponte amanhã). */
export type NavAction =
  | { kind: "navigate"; direction: Direction }
  | { kind: "activate" }
  | { kind: "back" }
  | { kind: "section"; delta: -1 | 1 };

/** O que importa de um `KeyboardEvent` (e o que `typing` diz do foco), para o mapa rodar sem DOM. */
export interface KeyInfo {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  /** O foco está num campo de texto: letras e Backspace são do campo. */
  typing: boolean;
}

const ARROWS: Record<string, Direction> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
};

/** `null`: a tecla não é do navegador (atalho, texto ou tecla sem função aqui). */
export function actionForKey(info: KeyInfo): NavAction | null {
  if (info.ctrlKey || info.altKey || info.metaKey) return null;
  const direction = ARROWS[info.key];
  // Shift + seta seleciona (campo de texto) ou marca um intervalo (lista): não é movimento de foco.
  if (direction) return info.shiftKey ? null : { kind: "navigate", direction };
  if (info.shiftKey) return null;
  switch (info.key) {
    case "Escape":
      return { kind: "back" };
    case "Backspace":
      return info.typing ? null : { kind: "back" };
    case "Enter":
      return { kind: "activate" };
    case " ":
      return info.typing ? null : { kind: "activate" };
    case "q":
    case "Q":
      return info.typing ? null : { kind: "section", delta: -1 };
    case "e":
    case "E":
      return info.typing ? null : { kind: "section", delta: 1 };
    default:
      return null;
  }
}

const ARROW_KEYS: Record<Direction, string> = {
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
};

/**
 * A tecla que o componente focado entenderia no lugar da ação: o controle não gera tecla, então a ação é oferecida
 * ao componente como se fosse esta. `null`: nenhum componente trata a ação (as seções são do navegador).
 */
export function keyForAction(action: NavAction): string | null {
  switch (action.kind) {
    case "navigate":
      return ARROW_KEYS[action.direction];
    case "activate":
      return "Enter";
    case "back":
      return "Escape";
    case "section":
      return null;
  }
}
