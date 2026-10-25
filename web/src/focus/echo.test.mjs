import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import { isEcho, resetEchoFilter, sameAction, shouldDropEcho } from "./echo.ts";

beforeEach(() => {
  resetEchoFilter();
});

test("sameAction compara corretamente ações de navegação", () => {
  assert.equal(sameAction({ kind: "navigate", direction: "up" }, { kind: "navigate", direction: "up" }), true);
  assert.equal(sameAction({ kind: "navigate", direction: "up" }, { kind: "navigate", direction: "down" }), false);
  assert.equal(sameAction({ kind: "back" }, { kind: "back" }), true);
  assert.equal(sameAction({ kind: "activate" }, { kind: "activate" }), true);
  assert.equal(sameAction({ kind: "section", delta: -1 }, { kind: "section", delta: -1 }), true);
  assert.equal(sameAction({ kind: "section", delta: -1 }, { kind: "section", delta: 1 }), false);
  assert.equal(sameAction({ kind: "command", command: "favorite" }, { kind: "command", command: "favorite" }), true);
  assert.equal(sameAction({ kind: "command", command: "favorite" }, { kind: "command", command: "menu" }), false);
  assert.equal(sameAction({ kind: "back" }, { kind: "activate" }), false);
});

test("isEcho descarta eco de teclado após controle dentro da janela de 80 ms", () => {
  const last = { action: { kind: "navigate", direction: "down" }, source: "controller", time: 1000 };
  const echo = { action: { kind: "navigate", direction: "down" }, source: "keyboard", time: 1040 };
  assert.equal(isEcho(echo, last, 80), true);
});

test("isEcho descarta eco de controle após teclado dentro da janela de 80 ms", () => {
  const last = { action: { kind: "back" }, source: "keyboard", time: 1000 };
  const echo = { action: { kind: "back" }, source: "controller", time: 1050 };
  assert.equal(isEcho(echo, last, 80), true);
});

test("isEcho não descarta se a janela de 80 ms expirou", () => {
  const last = { action: { kind: "navigate", direction: "left" }, source: "controller", time: 1000 };
  const late = { action: { kind: "navigate", direction: "left" }, source: "keyboard", time: 1080 };
  assert.equal(isEcho(late, last, 80), false);

  const muchLater = { action: { kind: "navigate", direction: "left" }, source: "keyboard", time: 1200 };
  assert.equal(isEcho(muchLater, last, 80), false);
});

test("isEcho não descarta ações diferentes mesmo dentro da janela", () => {
  const last = { action: { kind: "navigate", direction: "up" }, source: "controller", time: 1000 };
  const diff = { action: { kind: "navigate", direction: "down" }, source: "keyboard", time: 1020 };
  assert.equal(isEcho(diff, last, 80), false);
});

test("isEcho não descarta ações repetidas da mesma origem", () => {
  const last = { action: { kind: "navigate", direction: "right" }, source: "controller", time: 1000 };
  const repeat = { action: { kind: "navigate", direction: "right" }, source: "controller", time: 1030 };
  assert.equal(isEcho(repeat, last, 80), false);
});

test("shouldDropEcho permite primeira ação na hora e descarta o eco da outra origem", () => {
  // 1. Controle envia a ação no tempo 1000 -> passa imediatamente
  assert.equal(shouldDropEcho({ kind: "back" }, "controller", 1000), false);

  // 2. Eco do teclado chega 30 ms depois -> descartado
  assert.equal(shouldDropEcho({ kind: "back" }, "keyboard", 1030), true);

  // 3. Novo comando do teclado 100 ms depois -> passa imediatamente
  assert.equal(shouldDropEcho({ kind: "back" }, "keyboard", 1130), false);

  // 4. Eco do controle chega 25 ms depois -> descartado
  assert.equal(shouldDropEcho({ kind: "back" }, "controller", 1155), true);
});
