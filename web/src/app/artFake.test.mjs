import assert from "node:assert/strict";
import { test } from "node:test";

const storage = new Map();
globalThis.sessionStorage = {
  getItem: (k) => storage.get(k) ?? null,
  setItem: (k, v) => storage.set(k, v),
  removeItem: (k) => storage.delete(k),
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

function waitForEvent(name, timeoutMs = 1500) {
  const start = Date.now();
  return new Promise((resolve, reject) => {
    const check = () => {
      const ev = events.find((e) => e.name === name);
      if (ev) return resolve(ev);
      if (Date.now() - start > timeoutMs) return reject(new Error(`timeout esperando por ${name}`));
      setTimeout(check, 10);
    };
    check();
  });
}

test("art.candidates responde loading: true e emite art.candidates.ready com data URIs", async () => {
  events.length = 0;
  const reply = await call("art.candidates", { gameId: "steam:1000", kind: "cover" });
  assert.equal(reply.loading, true);
  const ev = await waitForEvent("art.candidates.ready");
  assert.equal(ev.payload.gameId, "steam:1000");
  assert.equal(ev.payload.kind, "cover");
  const cands = ev.payload.candidates;
  assert.ok(Array.isArray(cands));
  assert.ok(cands.length > 0);
  assert.equal(cands[0].id, "cand-0");
  assert.ok(cands[0].url.startsWith("data:image/svg+xml"));
  assert.equal(cands[0].current, true);
});

test("art.choose troca candidato, atualiza atual e emite art.changed e library.changed", async () => {
  events.length = 0;
  await call("art.choose", { gameId: "steam:1000", kind: "cover", candidateId: "cand-1" });

  await call("art.candidates", { gameId: "steam:1000", kind: "cover" });
  const ev = await waitForEvent("art.candidates.ready");
  const cands = ev.payload.candidates;
  const cand1 = cands.find((c) => c.id === "cand-1");
  assert.ok(cand1);
  assert.equal(cand1.current, true);

  const cand0 = cands.find((c) => c.id === "cand-0");
  assert.ok(cand0);
  assert.equal(cand0.current, false);

  assert.ok(events.some((e) => e.name === "art.changed" && e.payload?.gameId === "steam:1000"));
  assert.ok(events.some((e) => e.name === "library.changed"));
});

test("art.reset volta ao automatico e emite eventos", async () => {
  events.length = 0;
  await call("art.reset", { gameId: "steam:1000", kind: "cover" });

  await call("art.candidates", { gameId: "steam:1000", kind: "cover" });
  const ev = await waitForEvent("art.candidates.ready");
  const cands = ev.payload.candidates;
  const cand0 = cands.find((c) => c.id === "cand-0");
  assert.ok(cand0);
  assert.equal(cand0.current, true);

  assert.ok(events.some((e) => e.name === "art.changed" && e.payload?.gameId === "steam:1000"));
  assert.ok(events.some((e) => e.name === "library.changed"));
});

test("art.review lista jogos para conferir e art.review.accept avanca removendo o jogo", async () => {
  events.length = 0;
  const review = await call("art.review", undefined);
  assert.ok(review.total > 0);
  assert.ok(Array.isArray(review.items));
  assert.ok(review.items.length >= 2);

  const initialCount = review.items.length;
  const first = review.items[0];
  assert.ok(first.gameId);
  assert.ok(first.title);
  assert.ok(first.reason);

  await call("art.review.accept", { gameId: first.gameId });
  assert.ok(events.some((e) => e.name === "library.changed"));

  const after = await call("art.review", undefined);
  assert.equal(after.items.length, initialCount - 1);
  assert.ok(!after.items.some((i) => i.gameId === first.gameId));
});
