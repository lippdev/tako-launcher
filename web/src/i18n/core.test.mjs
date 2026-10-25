import assert from "node:assert/strict";
import { test } from "node:test";
import { createI18n, translate } from "./core.ts";

// Textos no formato do núcleo (`crates/tako-core/locales`): {nome} e chave.one / chave.other.
const pt = {
  "settings.general.title": "Geral",
  "art.summary": "{count} de {total} jogos para conferir",
  "common.games.one": "{n} jogo",
  "common.games.other": "{n} jogos",
  "common.games_in.one": "{n} jogo em {place}",
  "common.games_in.other": "{n} jogos em {place}",
  "dup": "{a}-{a}",
  "literal": "chaves {assim} ficam",
};

test("texto simples", () => {
  assert.equal(translate(pt, "settings.general.title"), "Geral");
});

test("placeholders, com número, repetidos e mais de um", () => {
  assert.equal(translate(pt, "art.summary", { count: 3, total: 10 }), "3 de 10 jogos para conferir");
  assert.equal(translate(pt, "dup", { a: "x" }), "x-x");
});

test("placeholder sem valor fica como está, e valor com $ não é interpretado", () => {
  assert.equal(translate(pt, "art.summary", { count: 3 }), "3 de {total} jogos para conferir");
  assert.equal(translate(pt, "literal"), "chaves {assim} ficam");
  assert.equal(translate(pt, "dup", { a: "$&" }), "$&-$&");
  assert.equal(translate(pt, "literal", { constructor: "x" }), "chaves {assim} ficam");
});

test("plural escolhe one com n == 1 e other no resto, como tn() do núcleo", () => {
  assert.equal(translate(pt, "common.games", { n: 1 }), "1 jogo");
  assert.equal(translate(pt, "common.games", { n: 0 }), "0 jogos");
  assert.equal(translate(pt, "common.games", { n: 3 }), "3 jogos");
  assert.equal(translate(pt, "common.games_in", { n: 1, place: "Steam" }), "1 jogo em Steam");
  assert.equal(translate(pt, "common.games_in", { n: 2, place: "Steam" }), "2 jogos em Steam");
});

test("plural só vale com n numérico; a chave exata tem prioridade", () => {
  const missing = [];
  assert.equal(translate(pt, "common.games", { n: "1" }, (k) => missing.push(k)), "common.games");
  assert.deepEqual(missing, ["common.games"]);
  assert.equal(translate({ ...pt, "common.games": "jogos: {n}" }, "common.games", { n: 1 }), "jogos: 1");
});

test("chave ausente vira a própria chave, avisa e nunca lança", () => {
  const missing = [];
  assert.equal(translate(pt, "nao.existe", undefined, (k) => missing.push(k)), "nao.existe");
  assert.equal(translate(pt, "toString", undefined, (k) => missing.push(k)), "toString");
  assert.equal(translate({}, "a.b", { n: 2 }), "a.b");
  assert.deepEqual(missing, ["nao.existe", "toString"]);
});

function fakeHost(catalogs) {
  const calls = [];
  const pendings = [];
  return {
    calls,
    pendings,
    fetchCatalog() {
      calls.push(1);
      return new Promise((resolve, reject) => pendings.push({ resolve, reject, data: catalogs.shift() }));
    },
  };
}

test("o carregador começa sem idioma, devolve a chave sem avisar e avisa os ouvintes quando o catálogo chega", async () => {
  const warns = [];
  const host = fakeHost([]);
  const i18n = createI18n({ fetchCatalog: host.fetchCatalog, warn: (m) => warns.push(m) });
  assert.equal(i18n.getSnapshot().lang, null);
  assert.equal(i18n.getSnapshot().t("settings.general.title"), "settings.general.title");
  assert.deepEqual(warns, []);

  let notified = 0;
  const off = i18n.subscribe(() => notified++);
  const loading = i18n.reload();
  host.pendings[0].resolve({ lang: "pt-BR", messages: pt });
  await loading;
  assert.equal(notified, 1);
  assert.equal(i18n.getSnapshot().lang, "pt-BR");
  assert.equal(i18n.getSnapshot().t("settings.general.title"), "Geral");
  off();
  const again = i18n.reload();
  host.pendings[1].resolve({ lang: "en-US", messages: {} });
  await again;
  assert.equal(notified, 1);
});

test("a troca de idioma troca o texto e o snapshot (para o React redesenhar)", async () => {
  const host = fakeHost([]);
  const i18n = createI18n({ fetchCatalog: host.fetchCatalog });
  const first = i18n.reload();
  host.pendings[0].resolve({ lang: "pt-BR", messages: pt });
  await first;
  const before = i18n.getSnapshot();
  const second = i18n.reload();
  host.pendings[1].resolve({ lang: "en-US", messages: { "settings.general.title": "General" } });
  await second;
  const after = i18n.getSnapshot();
  assert.notEqual(before, after);
  assert.equal(before.t("settings.general.title"), "Geral");
  assert.equal(after.t("settings.general.title"), "General");
});

test("resposta velha que chega depois da nova é ignorada", async () => {
  const host = fakeHost([]);
  const i18n = createI18n({ fetchCatalog: host.fetchCatalog });
  const old = i18n.reload();
  const recent = i18n.reload();
  host.pendings[1].resolve({ lang: "en-US", messages: { a: "new" } });
  await recent;
  host.pendings[0].resolve({ lang: "pt-BR", messages: { a: "old" } });
  await old;
  assert.equal(i18n.getSnapshot().lang, "en-US");
  assert.equal(i18n.getSnapshot().t("a"), "new");
});

test("chave sem texto avisa uma vez por idioma; falha ao ler o catálogo avisa e mantém o anterior", async () => {
  const warns = [];
  const host = fakeHost([]);
  const i18n = createI18n({ fetchCatalog: host.fetchCatalog, warn: (m) => warns.push(m) });
  const first = i18n.reload();
  host.pendings[0].resolve({ lang: "pt-BR", messages: pt });
  await first;
  const { t } = i18n.getSnapshot();
  assert.equal(t("falta"), "falta");
  assert.equal(t("falta"), "falta");
  assert.equal(warns.length, 1);
  assert.match(warns[0], /"falta".*pt-BR/);

  const failing = i18n.reload();
  host.pendings[1].reject(new Error("boom"));
  await failing;
  assert.equal(warns.length, 2);
  assert.equal(i18n.getSnapshot().t("settings.general.title"), "Geral");
});
