// Teste da leitura do `ui.state` (a regra: valor estranho cai no padrão). Rodar em `web/`:
//   node --test src/app/uiState.test.mjs
// É `.mjs` porque o projeto não tem `@types/node` (o `tsc` não aceitaria `node:test`) e o Node 24 lê o `.ts` direto.
import assert from "node:assert/strict";
import { test } from "node:test";
import { defaultScreenState, parseScreenState, serializeScreenState } from "./uiState.ts";

test("sem valor, ou texto que não é JSON, vem o padrão", () => {
  assert.deepEqual(parseScreenState(null), defaultScreenState);
  assert.deepEqual(parseScreenState(undefined), defaultScreenState);
  assert.deepEqual(parseScreenState(""), defaultScreenState);
  assert.deepEqual(parseScreenState("{quebrado"), defaultScreenState);
});

test("JSON que não é um objeto, ou de outra versão, vem o padrão", () => {
  for (const json of ["42", "null", '"configuracoes"', "[1,2]", '{"view":"perfil"}', '{"v":2,"view":"perfil"}', '{"v":"1"}']) {
    assert.deepEqual(parseScreenState(json), defaultScreenState, json);
  }
});

test("campo desconhecido ou do tipo errado cai no padrão sem derrubar os outros", () => {
  assert.deepEqual(parseScreenState('{"v":1,"view":"loja","listCollapsed":true}'), { view: "biblioteca", listCollapsed: true });
  assert.deepEqual(parseScreenState('{"v":1,"view":"perfil","listCollapsed":"sim"}'), { view: "perfil", listCollapsed: false });
  assert.deepEqual(parseScreenState('{"v":1,"view":7}'), defaultScreenState);
  // `toString` e afins não são vista (nada de achar chave herdada do objeto).
  assert.deepEqual(parseScreenState('{"v":1,"view":"toString"}'), defaultScreenState);
});

test("campo a mais é ignorado", () => {
  assert.deepEqual(parseScreenState('{"v":1,"view":"downloads","listCollapsed":true,"futuro":{"a":1}}'), {
    view: "downloads",
    listCollapsed: true,
  });
});

test("o que se guarda volta igual", () => {
  for (const view of ["biblioteca", "configuracoes", "perfil", "downloads"]) {
    for (const listCollapsed of [false, true]) {
      const state = { view, listCollapsed };
      assert.deepEqual(parseScreenState(serializeScreenState(state)), state);
    }
  }
});
