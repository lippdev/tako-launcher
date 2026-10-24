import assert from "node:assert/strict";
import { test } from "node:test";
import { defaultScreenState, defaultLibraryCriteria, parseScreenState, serializeScreenState } from "./uiState.ts";

test("sem valor, ou texto que não é JSON, vem o padrão", () => {
  for (const json of [null, undefined, "", "{quebrado"]) assert.deepEqual(parseScreenState(json), defaultScreenState);
});
test("JSON que não é objeto, ou de outra versão, vem o padrão", () => {
  for (const json of [
    "42",
    "null",
    '"configuracoes"',
    "[1,2]",
    '{"view":"perfil"}',
    '{"v":2,"view":"perfil"}',
    '{"v":"1"}',
  ])
    assert.deepEqual(parseScreenState(json), defaultScreenState);
});
test("campo desconhecido ou do tipo errado cai no padrão sem derrubar os outros", () => {
  assert.deepEqual(parseScreenState('{"v":1,"view":"loja","listCollapsed":true}'), {
    ...defaultScreenState,
    listCollapsed: true,
  });
  assert.deepEqual(parseScreenState('{"v":1,"view":"perfil","listCollapsed":"sim"}'), {
    ...defaultScreenState,
    view: "perfil",
  });
  assert.deepEqual(parseScreenState('{"v":1,"view":7}'), defaultScreenState);
  assert.deepEqual(parseScreenState('{"v":1,"view":"toString"}'), defaultScreenState);
});
test("campo a mais é ignorado", () => {
  assert.deepEqual(parseScreenState('{"v":1,"view":"downloads","listCollapsed":true,"futuro":{"a":1}}'), {
    ...defaultScreenState,
    view: "downloads",
    listCollapsed: true,
  });
});
test("o estado antigo volta igual e recebe os critérios padrão", () => {
  for (const view of ["biblioteca", "configuracoes", "perfil", "downloads"]) {
    for (const listCollapsed of [false, true]) {
      const state = { ...defaultScreenState, view, listCollapsed };
      assert.deepEqual(parseScreenState(serializeScreenState(state)), state);
    }
  }
});
test("critérios desconhecidos caem no padrão campo a campo", () => {
  assert.deepEqual(
    parseScreenState(
      '{"v":1,"library":{"sort":"future","filter":"bad","stores":["steam","alien"],"tags":[1,"RPG"],"statuses":["beaten","bad"],"installed":"sim"}}',
    ),
    {
      ...defaultScreenState,
      library: {
        ...defaultLibraryCriteria,
        stores: ["steam"],
        tags: ["RPG"],
        statuses: ["beaten"],
      },
    },
  );
  for (const library of [null, [], 7, "bad"])
    assert.deepEqual(parseScreenState(JSON.stringify({ v: 1, library })), defaultScreenState);
});
test("todos os critérios de tela voltam iguais", () => {
  const library = {
    filter: "favorites",
    search: "Hades",
    sort: "name",
    reversed: true,
    collection: "c1",
    stores: ["epic"],
    statuses: ["paused"],
    noStatus: true,
    tags: ["RPG"],
    installed: false,
  };
  const state = {
    view: "biblioteca",
    listCollapsed: true,
    library,
    activeGame: "heroic:legendary:Fortnite",
    settingsCategory: "library",
  };
  assert.deepEqual(parseScreenState(serializeScreenState(state)), state);
});
test("a categoria aberta em Configurações volta igual; valor desconhecido cai em Geral", () => {
  for (const settingsCategory of ["general", "appearance", "library", "about"]) {
    const state = { ...defaultScreenState, view: "configuracoes", settingsCategory };
    assert.deepEqual(parseScreenState(serializeScreenState(state)), state);
  }
  for (const settingsCategory of ["loja", 3, null, "toString", ["library"]])
    assert.deepEqual(parseScreenState(JSON.stringify({ v: 1, settingsCategory })), defaultScreenState);
  assert.equal(defaultScreenState.settingsCategory, "general");
  // Estado guardado antes da categoria existir continua valendo, com a categoria padrão.
  assert.deepEqual(parseScreenState('{"v":1,"view":"configuracoes"}'), {
    ...defaultScreenState,
    view: "configuracoes",
  });
});
test("o jogo aberto volta igual; valor ruim ou ausente é a Visão geral", () => {
  assert.equal(defaultScreenState.activeGame, null);
  for (const activeGame of ["steam:1000", "heroic:legendary:Fortnite", "gog:1207658924"]) {
    const state = { ...defaultScreenState, activeGame };
    assert.deepEqual(parseScreenState(serializeScreenState(state)), state);
  }
  for (const activeGame of ["", 7, null, true, ["steam:1"], {}, "x".repeat(513)])
    assert.deepEqual(parseScreenState(JSON.stringify({ v: 1, activeGame })), defaultScreenState);
  // Estado guardado antes do jogo aberto existir continua valendo: abre na Visão geral.
  assert.deepEqual(parseScreenState('{"v":1,"view":"biblioteca","listCollapsed":true}'), {
    ...defaultScreenState,
    listCollapsed: true,
  });
});
