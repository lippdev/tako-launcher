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
  assert.equal(r.ok, true, JSON.stringify(r));
  return r.reply.data;
}
async function fail(name, payload, code) {
  const r = await request(name, payload);
  assert.equal(r.ok, false);
  assert.equal(r.error.code, code, r.error.message);
  return r.error.message;
}
const eventOf = async (name) => {
  for (let i = 0; i < 100; i++) {
    const found = events.find((e) => e.name === name);
    if (found) return found.payload;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error(`evento ${name} não chegou`);
};

test("adicionar, editar e remover um jogo no hospedeiro falso", async () => {
  const before = (await call("library.list", {})).total;
  const added = await call("library.add_game", { path: "C:\\Jogos\\hollow_knight.exe" });
  assert.equal(added.status, "added");
  assert.equal(added.title, "hollow knight");
  assert.equal((await call("library.list", {})).total, before + 1);
  assert.ok(events.some((e) => e.name === "library.changed"));

  const again = await call("library.add_game", { path: "c:\\jogos\\HOLLOW_KNIGHT.exe" });
  assert.deepEqual([again.status, again.title], ["already", "hollow knight"]);
  await fail("library.add_game", { path: "C:\\Jogos\\inexistente.exe" }, "not_found");
  await fail("library.add_game", { path: "  " }, "bad_request");
  await fail("library.add_game", { path: "C:\\Jogos\\a.exe", system: "snes" }, "bad_request");

  const id = added.gameId;
  const view = await call("game.edit", { gameId: id });
  assert.equal(view.target, "C:\\Jogos\\hollow_knight.exe");
  assert.equal(view.defaultFolder, "C:\\Jogos");
  const body = { gameId: id, title: " Hollow Knight ", arguments: "-x", workingDirectory: "", tracking: "process" };
  const saved = await call("game.edit.set", body);
  assert.equal(saved.title, "Hollow Knight");
  assert.equal(saved.tracking, "process");
  assert.equal((await call("game.details", { gameId: id })).game.title, "Hollow Knight");
  await fail("game.edit.set", { ...body, title: "  " }, "bad_request");
  await fail("game.edit.set", { ...body, workingDirectory: "C:\\inexistente" }, "not_found");
  await fail("game.edit.set", { ...body, tracking: "outro" }, "bad_request");
  // Jogo de loja não se edita nem se remove.
  await fail("game.edit", { gameId: "steam:1000" }, "conflict");
  await fail("game.remove", { gameId: "steam:1000" }, "conflict");
  await fail("game.edit", { gameId: "manual:nao" }, "not_found");

  await call("game.remove", { gameId: id });
  assert.equal((await call("library.list", {})).total, before);
  await fail("game.details", { gameId: id }, "not_found");
});

test("ROM pergunta o sistema e entra com o título limpo", async () => {
  const ask = await call("library.add_game", { path: "C:\\Roms\\Super_Mario_World (USA).sfc" });
  assert.equal(ask.status, "needs_system");
  assert.equal(ask.fileName, "Super_Mario_World (USA).sfc");
  assert.equal(ask.systems[0].key, "snes");
  assert.ok(ask.others.length > 0 && !ask.others.some((s) => s.key === "snes"));
  await fail("library.add_game", { path: "C:\\Roms\\Super_Mario_World (USA).sfc", system: "naoexiste" }, "bad_request");
  const done = await call("library.add_game", { path: "C:\\Roms\\Super_Mario_World (USA).sfc", system: "snes" });
  assert.equal(done.title, "Super Mario World");
});

test("Fontes e IDs: fixar, nenhum, automático, buscar e conferir", async () => {
  const view = await call("game.source_ids", { gameId: "steam:1000" });
  assert.deepEqual(
    view.sources.map((s) => s.source),
    ["Steam", "IGDB", "SteamGridDB"],
  );
  assert.equal(view.sources[0].id, "1000");
  const pin = (choice, source = "IGDB") => call("game.source_ids.set", { gameId: "steam:1000", source, choice });
  const pinned = (await pin({ kind: "id", value: " 77 " })).sources[1];
  assert.deepEqual([pinned.id, pinned.pinned, pinned.none], ["77", true, false]);
  const none = (await pin({ kind: "none" })).sources[1];
  assert.deepEqual([none.id, none.pinned, none.none], [null, true, true]);
  const auto = (await pin({ kind: "auto" })).sources[1];
  assert.deepEqual([auto.id, auto.pinned, auto.none], ["113112", false, false]);
  await fail("game.source_ids.set", { gameId: "steam:1000", source: "Outra", choice: { kind: "none" } }, "bad_request");
  await fail("game.source_ids.set", { gameId: "steam:1000", source: "IGDB", choice: { kind: "id", value: " " } }, "bad_request");

  const job = await call("game.source_ids.search", { gameId: "steam:1000", term: "Hades" });
  const found = await eventOf("game.source_ids.found");
  assert.equal(found.requestId, job.requestId);
  assert.equal(found.results.length, 3);
  assert.equal(found.results[0].matches[0].title, "Hades");

  const message = await fail("game.source_ids.check", { gameId: "steam:1000", source: "IGDB", text: "isso não é um id" }, "bad_request");
  assert.ok(message.startsWith("Não reconheci"), message);
  const check = await call("game.source_ids.check", { gameId: "steam:1000", source: "IGDB", text: "igdb.com/games/113112" });
  const checked = await eventOf("game.source_ids.checked");
  assert.equal(checked.requestId, check.requestId);
  assert.equal(checked.found.name, "Hades (2018)");
});
