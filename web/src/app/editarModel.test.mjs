import assert from "node:assert/strict";
import { test } from "node:test";
import {
  addOutcome,
  addProblem,
  describeSource,
  draftOf,
  editProblem,
  isDirty,
  matchDetail,
  matchMarker,
  toSet,
  TRACKING_ORDER,
} from "./editar/editarModel.ts";

const edit = {
  gameId: "manual:1",
  title: "Hades",
  target: "C:\\Jogos\\Hades.exe",
  arguments: "-windowed",
  workingDirectory: "",
  defaultFolder: "C:\\Jogos",
  tracking: "auto",
};

test("o rascunho parte do que o jogo tem e só liga o Salvar quando algo muda", () => {
  const draft = draftOf(edit);
  assert.deepEqual(draft, { title: "Hades", arguments: "-windowed", workingDirectory: "", tracking: "auto" });
  assert.equal(isDirty(draft, edit), false);
  // Espaços nas pontas não contam como mudança.
  assert.equal(isDirty({ ...draft, title: "  Hades " }, edit), false);
  assert.equal(isDirty({ ...draft, title: "Hades II" }, edit), true);
  assert.equal(isDirty({ ...draft, tracking: "process" }, edit), true);
  assert.equal(isDirty({ ...draft, workingDirectory: "C:\\Outra" }, edit), true);
});

test("a gravação leva os textos sem espaços nas pontas", () => {
  const draft = { title: "  Meu Jogo ", arguments: " -x ", workingDirectory: " C:\\Pasta ", tracking: "none" };
  assert.deepEqual(toSet("manual:1", draft), {
    gameId: "manual:1",
    title: "Meu Jogo",
    arguments: "-x",
    workingDirectory: "C:\\Pasta",
    tracking: "none",
  });
  assert.deepEqual(TRACKING_ORDER, ["auto", "process", "install_folder", "none"]);
});

test("a falha da gravação aponta o campo", () => {
  const draft = draftOf(edit);
  assert.equal(editProblem({ ...draft, title: " " }, "bad_request"), "name");
  assert.equal(editProblem(draft, "not_found"), "folder");
  assert.equal(editProblem(draft, "bad_request"), "invalid");
  assert.equal(editProblem(draft, "conflict"), "manual");
  assert.equal(editProblem(draft, "internal"), "save");
});

const result = (patch) => ({ status: "added", gameId: null, title: null, fileName: null, systems: [], others: [], ...patch });

test("o resultado de adicionar vira o próximo passo do diálogo", () => {
  assert.deepEqual(addOutcome(result({ gameId: "manual:2", title: "Hades" })), { kind: "added", title: "Hades" });
  assert.deepEqual(addOutcome(result({ status: "already", title: "Hades" })), { kind: "already", title: "Hades" });
  assert.deepEqual(addOutcome(result({ status: "unreadable" })), { kind: "unreadable" });
  const systems = [{ key: "snes", name: "SNES" }];
  const others = [{ key: "nes", name: "NES" }];
  assert.deepEqual(addOutcome(result({ status: "needs_system", fileName: "Mario.sfc", systems, others })), {
    kind: "ask-system",
    fileName: "Mario.sfc",
    systems,
    others,
  });
});

test("a falha de adicionar escolhe o aviso", () => {
  assert.equal(addProblem("  ", "bad_request"), "empty");
  assert.equal(addProblem("x", "bad_request"), "invalid");
  assert.equal(addProblem("x", "not_found"), "not-found");
  assert.equal(addProblem("x", "internal"), "error");
});

const labels = { noneChosen: "Nenhum · escolhido por você", noMatch: "Sem correspondência", chosen: "escolhido por você", automatic: "automático" };
const state = (patch) => ({ source: "IGDB", id: null, title: null, year: null, pinned: false, none: false, example: "", ...patch });

test("cada fonte diz o que casou, com nome, ano e quem escolheu", () => {
  assert.equal(describeSource(state({}), labels), "Sem correspondência");
  assert.equal(describeSource(state({ none: true, pinned: true }), labels), "Nenhum · escolhido por você");
  assert.equal(describeSource(state({ id: "113112" }), labels), "ID 113112 · automático");
  assert.equal(
    describeSource(state({ id: "113112", title: "Hades", year: 2018, pinned: true }), labels),
    "ID 113112 · Hades (2018) · escolhido por você",
  );
});

test("o candidato casado leva ✓ e o mais provável leva ★", () => {
  const match = (id) => ({ id, title: "Hades", year: null, platforms: null });
  assert.equal(matchMarker(match("1"), state({ id: "1" }), "1"), "current");
  assert.equal(matchMarker(match("2"), state({ id: "1" }), "2"), "best");
  assert.equal(matchMarker(match("3"), state({ id: "1" }), "2"), null);
  assert.equal(matchMarker(match("3"), state({}), null), null);
  assert.equal(matchDetail({ id: "7", title: "x", year: 2018, platforms: "PC" }), "2018 · PC · ID 7");
  assert.equal(matchDetail({ id: "7", title: "x", year: null, platforms: "" }), "ID 7");
});
