import assert from "node:assert/strict";
import { test } from "node:test";
import { decideFirstRun, nextStep, previousStep, storeChoice, coverChoice, modeChoice, finishChoice, initialMode } from "./primeiraVez/model.ts";

test("instalação nova mostra o assistente; conclusão e volta de jogo não repetem", () => {
  assert.equal(decideFirstRun({}, 0, false), "show");
  assert.equal(decideFirstRun({ FirstRunCompleted: "true" }, 0, false), "skip");
  assert.equal(decideFirstRun({}, 0, true), "skip");
  assert.equal(decideFirstRun({ SafeMode: "true" }, 0, false), "skip");
});

test("atualização de instalação existente marca concluído, como o Slint", () => {
  assert.equal(decideFirstRun({}, 12, false), "complete-existing");
  assert.equal(decideFirstRun({ Language: "pt-BR" }, 0, false), "complete-existing");
  assert.equal(decideFirstRun({ FirstRunCompleted: "false" }, 0, false), "complete-existing");
  assert.equal(decideFirstRun({ "metadata.igdb": "false" }, 0, false), "show");
});

test("cinco passos; sem emulador nem pasta o Slint vai das lojas às capas", () => {
  assert.deepEqual([1, 2, 3, 4].map((step) => nextStep(step, true)), [2, 3, 4, 5]);
  assert.equal(nextStep(1, false), 3);
  assert.deepEqual([5, 4, 3, 2, 1].map((step) => previousStep(step, true)), [4, 3, 2, 1, null]);
  assert.equal(previousStep(3, false), 1);
});

test("lojas gravam DisabledStores com as chaves antigas e preservam as demais", () => {
  assert.deepEqual(storeChoice("Steam,Desconhecida", "battle_net", false), { key: "DisabledStores", value: "Steam,Desconhecida,BattleNet" });
  assert.deepEqual(storeChoice("steam,Epic", "steam", true), { key: "DisabledStores", value: "Epic" });
  assert.equal(storeChoice("", "gog", false).value, "Gog");
});

test("capas só mandam a escolha para metadata.igdb; modo usa OpenIn", () => {
  assert.deepEqual(coverChoice(true), { key: "metadata.igdb", value: "true" });
  assert.deepEqual(coverChoice(false), { key: "metadata.igdb", value: "false" });
  assert.deepEqual(modeChoice("desktop"), { key: "OpenIn", value: "desktop" });
  assert.deepEqual(modeChoice("big-picture"), { key: "OpenIn", value: "big-picture" });
  assert.equal(initialMode({}, false), "desktop");
  assert.equal(initialMode({}, true), "big-picture");
  assert.equal(initialMode({ OpenIn: "desktop" }, true), "desktop");
});

test("terminar, revisar capas ou pular concluem pela mesma chave do Slint", () => {
  const setting = finishChoice();
  assert.deepEqual(setting, { key: "FirstRunCompleted", value: "true" });
  assert.equal(decideFirstRun({ [setting.key]: setting.value }, 0, false), "skip");
});
