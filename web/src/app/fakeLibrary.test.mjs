import assert from "node:assert/strict";
import { test } from "node:test";
const storage = new Map();
globalThis.sessionStorage = {
  getItem: (k) => storage.get(k) ?? null,
  setItem: (k, v) => storage.set(k, v),
};
const pending = new Map();
const events = [];
globalThis.window = {
  __takoDeliver(text) {
    const wire = JSON.parse(text);
    if (wire.event) events.push(wire.event);
    else {
      const done = pending.get(wire.id);
      pending.delete(wire.id);
      done(wire);
    }
  },
};
const { installFakeHost } = await import("../bridge/fake.ts");
installFakeHost({}, 0);
let next = 0;
function request(name, payload) {
  const id = ++next;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    window.takoHost.post(JSON.stringify({ v: 1, id, name, payload }));
  });
}
async function call(name, payload) {
  const r = await request(name, payload);
  assert.equal(r.ok, true);
  return r.reply.data;
}

test("hospedeiro pagina, busca, filtra e ordena antes de cortar a página", async () => {
  const first = await call("library.list", {
    offset: 0,
    limit: 5,
    sort: "name",
  });
  const next = await call("library.list", {
    offset: 5,
    limit: 5,
    sort: "name",
  });
  assert.equal(first.total, 13);
  assert.equal(first.rows.length, 5);
  assert.equal(new Set([...first.rows, ...next.rows].map((g) => g.id)).size, 10);
  const search = await call("library.list", { search: "hAdEs" });
  assert.equal(search.total, 1);
  assert.equal(search.rows[0].title, "Hades II");
  const favorites = await call("library.list", { filter: "favorites" });
  assert.equal(favorites.total, 2);
  assert.ok(favorites.rows.every((g) => g.favorite));
  const installed = await call("library.list", { filter: "installed" });
  assert.equal(installed.total, 9);
  assert.ok(installed.rows.every((g) => g.installed));
  const stores = await call("library.list", {
    stores: ["gog"],
    installed: true,
  });
  assert.equal(stores.total, 3);
  const reversed = await call("library.list", {
    sort: "name",
    reversed: true,
  });
  assert.equal(reversed.rows.at(-1).id, first.rows[0].id);
});
test("ações alteram o domínio e emitem library.changed", async () => {
  const gameIds = ["steam:1000", "steam:1001"];
  await call("game.favorite.set", { gameIds, favorite: false });
  const favorites = await call("library.list", { filter: "favorites" });
  assert.equal(favorites.total, 1);
  await call("game.status.set", { gameIds, status: "paused" });
  assert.equal((await call("library.list", { statuses: ["paused"] })).total, 2);
  await call("game.tags.set", { gameIds, tags: ["Teste"], on: true });
  assert.equal((await call("library.list", { tags: ["Teste"] })).total, 2);
  assert.ok((await call("tags.list")).some((t) => t.name === "Teste" && t.count === 2));
  await call("game.tags.set", { gameIds, tags: ["Teste"], on: false });
  assert.equal((await call("library.list", { tags: ["Teste"] })).total, 0);
  const collections = await call("collections.list");
  const collectionId = collections[1].id;
  assert.equal((await call("library.list", { collection: collectionId })).total, 0);
  await call("collections.add_games", { gameIds, collectionId });
  assert.equal((await call("library.list", { collection: collectionId })).total, 2);
  await call("collections.remove_games", { gameIds, collectionId });
  assert.equal((await call("library.list", { collection: collectionId })).total, 0);
  assert.equal(events.filter((e) => e.name === "library.changed").length, 6);
  assert.equal(
    (
      await request("collections.add_games", {
        gameIds,
        collectionId: "missing",
      })
    ).ok,
    false,
  );
});
test("ui.state.set grava o estado no armazenamento da sessão de desenvolvimento", async () => {
  await call("ui.state.set", { json: '{"v":1}' });
  assert.equal(storage.get("tako.dev.ui.state"), '{"v":1}');
});
test("coleções automáticas: salvar, abrir, trocar os filtros, renomear e apagar", async () => {
  const rule = (library) => ({ library, genres: [], playTime: "any", collections: [] });
  const ok = (name, data) => request(name, data).then((r) => r.ok);
  const before = (await call("collections.list")).length;
  const saved = await call("collections.smart.set", {
    collectionId: null,
    name: "  Instalados   da Steam ",
    criteria: rule({ filter: "installed", stores: ["steam"], sort: "name" }),
  });
  assert.equal(saved.name, "Instalados da Steam");
  assert.ok(saved.criteria);
  assert.equal((await call("collections.list")).length, before + 1);

  // A coleção mostra o que a regra acha na hora e a ordenação salva vale com a ordenação padrão.
  const open = await call("library.list", { collection: saved.id, limit: 50 });
  const direct = await call("library.list", { filter: "installed", stores: ["steam"], sort: "name", limit: 50 });
  assert.deepEqual(
    open.rows.map((g) => g.id),
    direct.rows.map((g) => g.id),
  );
  assert.ok(open.total > 0);
  // Filtros do pedido valem por cima da regra.
  const narrowed = await call("library.list", { collection: saved.id, search: "zzzz" });
  assert.equal(narrowed.total, 0);

  // Nome vazio ou repetido (sem caixa nem acento) é recusado; renomear mantém o próprio nome.
  for (const name of ["   ", "instalados da STEAM", "x".repeat(41)]) {
    assert.equal(await ok("collections.smart.set", { collectionId: null, name, criteria: rule({}) }), false, name);
  }
  assert.equal(await ok("collections.rename", { collectionId: saved.id, name: "INSTALADOS da steam" }), true);

  // Trocar os filtros: a mesma coleção passa a mostrar outros jogos.
  const updated = await call("collections.smart.set", {
    collectionId: saved.id,
    name: "Instalados da Steam",
    criteria: rule({ filter: "favorites" }),
  });
  assert.equal(updated.id, saved.id);
  const favorites = await call("library.list", { collection: saved.id, limit: 50 });
  assert.ok(favorites.rows.every((g) => g.favorite));

  assert.equal(await ok("collections.smart.set", { collectionId: "missing", name: "Outra", criteria: rule({}) }), false);
  await call("collections.delete", { collectionId: saved.id });
  assert.equal((await call("collections.list")).length, before);
  assert.equal(await ok("collections.delete", { collectionId: saved.id }), false);

  // Manuais: criar, nome repetido, renomear e apagar.
  const manual = await call("collections.create", { name: "Para o fim de semana" });
  assert.equal(manual.criteria, null);
  assert.equal(await ok("collections.create", { name: "para o fim de semana" }), false);
  await call("collections.rename", { collectionId: manual.id, name: "Fim de semana" });
  assert.ok((await call("collections.list")).some((c) => c.name === "Fim de semana"));
  await call("collections.delete", { collectionId: manual.id });
  assert.equal((await call("collections.list")).length, before);
});
