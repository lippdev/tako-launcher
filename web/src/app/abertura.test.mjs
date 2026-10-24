import assert from "node:assert/strict";
import { test } from "node:test";
import { decideAbertura, remainingHold, scanView, HOLD_AFTER_SCAN_MS, SCAN_LIMIT_MS } from "./abertura/model.ts";

test("biblioteca vazia com leitura em andamento mostra a primeira leitura", () => {
  assert.equal(decideAbertura({ hasSavedState: false, libraryCount: 0, scanning: true }), "primeira-leitura");
});
test("biblioteca com jogos, ou vazia sem leitura, mostra só a entrada", () => {
  assert.equal(decideAbertura({ hasSavedState: false, libraryCount: 12, scanning: true }), "entrada");
  assert.equal(decideAbertura({ hasSavedState: false, libraryCount: 0, scanning: false }), "entrada");
});
test("reabrir com ui.state gravado não repete a abertura", () => {
  assert.equal(decideAbertura({ hasSavedState: true, libraryCount: 0, scanning: true }), "nenhuma");
  assert.equal(decideAbertura({ hasSavedState: true, libraryCount: 9, scanning: false }), "nenhuma");
});
test("a barra: sem loja lida corre sem número, depois mostra o progresso, no fim 'Iniciando'", () => {
  assert.deepEqual(scanView({ scanning: true, completed: 0, total: 7 }), {
    textKey: "first_run.splash.scanning",
    progress: null,
    percent: "",
  });
  const mid = scanView({ scanning: true, completed: 3, total: 7 });
  assert.equal(mid.percent, "42%");
  assert.ok(Math.abs((mid.progress ?? 0) - 0.428) < 0.001);
  assert.deepEqual(scanView({ scanning: false, completed: 3, total: 7 }), {
    textKey: "first_run.splash.starting",
    progress: 1,
    percent: "",
  });
  assert.equal(scanView({ scanning: true, completed: 9, total: 7 }).percent, "100%");
  assert.equal(scanView({ scanning: true, completed: 1, total: 0 }).progress, null);
});
test("a tela espera a leitura, segura 1 s depois dela e desiste aos 15 s", () => {
  const base = { reduceMotion: false };
  assert.equal(remainingHold({ ...base, scanning: true, elapsedMs: 3000, doneForMs: null }), null);
  assert.equal(remainingHold({ ...base, scanning: false, elapsedMs: 3000, doneForMs: 0 }), HOLD_AFTER_SCAN_MS);
  assert.equal(remainingHold({ ...base, scanning: false, elapsedMs: 4000, doneForMs: 400 }), HOLD_AFTER_SCAN_MS - 400);
  assert.equal(remainingHold({ ...base, scanning: false, elapsedMs: 5000, doneForMs: 2000 }), 0);
  // leitura travada: passou do limite, vale como pronta
  assert.equal(remainingHold({ ...base, scanning: true, elapsedMs: SCAN_LIMIT_MS, doneForMs: 0 }), HOLD_AFTER_SCAN_MS);
  assert.equal(remainingHold({ ...base, scanning: true, elapsedMs: SCAN_LIMIT_MS - 1, doneForMs: 0 }), null);
});
test("com 'Reduzir animações' não há espera depois da leitura", () => {
  assert.equal(remainingHold({ reduceMotion: true, scanning: false, elapsedMs: 100, doneForMs: 0 }), 0);
});
