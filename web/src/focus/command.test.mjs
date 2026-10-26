import assert from "node:assert/strict";
import { test } from "node:test";
import { actionForInput } from "./keymap.ts";

const COMMAND_EVENT = "tako:command";

test("mapeia X para favorite e Menu para menu", () => {
  assert.deepEqual(actionForInput("favorite"), { kind: "command", command: "favorite" });
  assert.deepEqual(actionForInput("menu"), { kind: "command", command: "menu" });
});

test("evento tako:command dispara no elemento focado e sobe na árvore", () => {
  const parent = new EventTarget();
  let received = null;
  parent.addEventListener(COMMAND_EVENT, (e) => {
    received = e.detail?.command;
    e.preventDefault();
  });

  const event = new CustomEvent(COMMAND_EVENT, { detail: { command: "favorite" }, bubbles: true, cancelable: true });
  parent.dispatchEvent(event);

  assert.equal(received, "favorite");
  assert.equal(event.defaultPrevented, true);
});

