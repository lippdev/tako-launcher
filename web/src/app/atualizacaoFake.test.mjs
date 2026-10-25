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
}
const states = () => events.filter((e) => e.name === "updates.changed").map((e) => e.payload);
async function until(state) {
  for (let i = 0; i < 400; i++) {
    const found = states().findLast((s) => s.state === state);
    if (found) return found;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`o estado ${state} não chegou`);
}

test("o ciclo do Slint no hospedeiro falso: procurar, baixar com progresso, pronta, reiniciar", { timeout: 40_000 }, async () => {
  const idle = await call("updates.status");
  assert.equal(idle.state, "idle");
  assert.equal(idle.offer, null);
  // Reiniciar sem nada pronto é recusado.
  await fail("updates.apply", undefined, "conflict");

  const searching = await call("updates.check");
  assert.equal(searching.state, "searching");
  const downloading = await until("downloading");
  assert.equal(downloading.offer.version, "0.3.0");
  assert.equal(downloading.offer.installable, true);
  assert.equal(downloading.offer.notes.length, 3);
  // Durante o download também não reinicia, e procurar de novo não refaz nada.
  await fail("updates.apply", undefined, "conflict");
  assert.equal((await call("updates.check")).state, "downloading");

  const ready = await until("ready");
  assert.equal(ready.progress, 1);
  assert.equal(ready.downloadedBytes, ready.offer.size);
  const steps = states().filter((s) => s.state === "downloading").map((s) => s.progress);
  assert.ok(steps.length > 3 && steps.some((p) => p > 0 && p < 1), String(steps));
  assert.deepEqual(steps, [...steps].sort((a, b) => a - b));
  assert.equal(states()[0].state, "searching");

  // Pronta, reiniciar é aceito e o núcleo avisa que está saindo (o falso volta a pronta depois, para repetir).
  await call("updates.apply");
  assert.equal((await until("installing")).offer.version, "0.3.0");
});

test("as preferências de atualização guardam as três chaves", async () => {
  assert.deepEqual(await call("updates.preferences"), { autoUpdate: true, prereleases: false, nightly: false });
  const wanted = { autoUpdate: false, prereleases: true, nightly: false };
  assert.deepEqual(await call("updates.preferences.set", wanted), wanted);
  assert.deepEqual(await call("updates.preferences"), wanted);
  await fail("updates.preferences.set", { autoUpdate: "sim", prereleases: true, nightly: false }, "bad_request");
});
