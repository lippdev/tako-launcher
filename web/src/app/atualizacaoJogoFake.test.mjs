import assert from "node:assert/strict";
import { test } from "node:test";

globalThis.location = { search: "?atualizacao=jogo-aberto" };
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
}
const states = () => events.filter((e) => e.name === "updates.changed").map((e) => e.payload);
async function until(state) {
  for (let i = 0; i < 600; i++) {
    const found = states().findLast((s) => s.state === state);
    if (found) return found;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`o estado ${state} não chegou`);
}

test("com um jogo aberto o reinício espera, e a procura ao abrir roda sozinha", { timeout: 40_000 }, async () => {
  // O parâmetro da URL liga a procura automática ao abrir (a do núcleo roda ao abrir o núcleo): ninguém clicou.
  const ready = await until("ready");
  assert.equal(ready.gameRunning, true);
  assert.equal(states()[0].state, "searching");
  await fail("updates.apply", undefined, "conflict");
  assert.equal((await call("updates.status")).state, "ready");
});
