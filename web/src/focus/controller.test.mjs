import assert from "node:assert/strict";
import { test } from "node:test";
import { actionForInput, keyForAction } from "./keymap.ts";

// Comandos do controle (`input.nav` do núcleo, WEB-05b) → ações do navegador de foco. Mesmo mapa do app atual.
test("o controle chega como as mesmas ações do teclado", () => {
  assert.deepEqual(actionForInput("up"), { kind: "navigate", direction: "up" });
  assert.deepEqual(actionForInput("right"), { kind: "navigate", direction: "right" });
  assert.deepEqual(actionForInput("accept"), { kind: "activate" });
  assert.deepEqual(actionForInput("back"), { kind: "back" });
  assert.deepEqual(actionForInput("previous_section"), { kind: "section", delta: -1 });
  assert.deepEqual(actionForInput("next_section"), { kind: "section", delta: 1 });
});

test("Y é a busca geral; X e Menu vão ao componente focado", () => {
  assert.deepEqual(actionForInput("search"), { kind: "search" });
  assert.deepEqual(actionForInput("favorite"), { kind: "command", command: "favorite" });
  assert.deepEqual(actionForInput("menu"), { kind: "command", command: "menu" });
});

test("as ações do controle são oferecidas ao componente como a tecla equivalente", () => {
  assert.equal(keyForAction(actionForInput("down")), "ArrowDown");
  assert.equal(keyForAction(actionForInput("accept")), "Enter");
  assert.equal(keyForAction(actionForInput("back")), "Escape");
  // Seções, busca e comandos não têm tecla para o componente: o navegador trata.
  assert.equal(keyForAction(actionForInput("next_section")), null);
  assert.equal(keyForAction(actionForInput("search")), null);
  assert.equal(keyForAction(actionForInput("menu")), null);
});
