import assert from "node:assert/strict";
import { test } from "node:test";
import { createSessionStore } from "./sessao/sessionStore.ts";

const T0 = 1_000_000;
const conflict = () => ({ error: { code: "conflict", message: "x" } });
const running = (patch = {}) => ({
  gameId: "steam:1",
  title: "Elden Ring",
  startedAt: new Date(T0).toISOString(),
  state: "playing",
  ...patch,
});
const ended = (patch = {}) => ({
  gameId: "steam:1",
  title: "Elden Ring",
  playedSeconds: 600,
  end: "normal",
  exitCode: null,
  exitCodeText: null,
  message: null,
  ...patch,
});

/** Uma ponte de mentira: registra as chamadas e deixa o teste mandar os eventos. */
function fakeBridge(replies = {}) {
  const log = [];
  const handlers = { started: new Set(), ended: new Set() };
  const bridge = {
    running: async () => {
      log.push("running");
      return replies.running ? await replies.running() : null;
    },
    play: async (id) => {
      log.push(`play:${id}`);
      await replies.play?.();
    },
    resume: async () => {
      log.push("resume");
      await replies.resume?.();
    },
    end: async (force) => {
      log.push(`end:${force}`);
      await replies.end?.();
    },
    onStarted: (h) => {
      log.push("onStarted");
      handlers.started.add(h);
      return () => handlers.started.delete(h);
    },
    onEnded: (h) => {
      log.push("onEnded");
      handlers.ended.add(h);
      return () => handlers.ended.delete(h);
    },
  };
  return {
    bridge,
    log,
    emitEnded: (e) => handlers.ended.forEach((h) => h(e)),
    emitStarted: (e) => handlers.started.forEach((h) => h(e)),
  };
}

const tick = () => new Promise((resolve) => setImmediate(resolve));
const make = (replies) => {
  const host = fakeBridge(replies);
  const store = createSessionStore({ bridge: host.bridge, now: () => T0, pollMs: 0 });
  return { ...host, store };
};

test("ao abrir, escuta os eventos antes de ler game.running", async () => {
  const { store, log } = make();
  store.start();
  store.start();
  await tick();
  assert.deepEqual(log, ["onStarted", "onEnded", "running"]);
  store.stop();
});

test("o fim não visto que vem logo depois da leitura vira o aviso de bem-vindo de volta", async () => {
  let deliver = () => {};
  const { store, emitEnded } = make({
    running: async () => {
      // O núcleo manda o evento na mesma entrega, logo depois da resposta.
      deliver = () => emitEnded(ended());
      queueMicrotask(() => deliver());
      return null;
    },
  });
  store.start();
  await tick();
  assert.equal(store.getState().running, null);
  assert.equal(store.getState().notice?.kind, "welcome");
  assert.equal(store.getState().notice?.playedSeconds, 600);
  store.dismissNotice();
  assert.equal(store.getState().notice, null);
  store.stop();
});

test("interface reaberta com jogo aberto: a leitura dá a fase", async () => {
  const { store } = make({ running: async () => running({ state: "closing" }) });
  store.start();
  await tick();
  assert.equal(store.getState().running?.phase, "closing");
  store.stop();
});

test("Jogar abre já no clique; conflict desfaz e conta quem pediu", async () => {
  const { store } = make({
    play: async () => {
      throw conflict();
    },
  });
  store.start();
  await tick();
  const states = [];
  store.subscribe(() => states.push(store.getState().running?.phase ?? null));
  assert.equal(await store.play("steam:1", "Elden Ring"), "conflict");
  assert.deepEqual(states.slice(0, 2), ["launching", null]);
  store.stop();
});

test("com outro jogo aberto, Jogar não apaga o que se mostra", async () => {
  const { store } = make({
    running: async () => running({ gameId: "steam:9", title: "Outro" }),
    play: async () => {
      throw conflict();
    },
  });
  store.start();
  await tick();
  assert.equal(await store.play("steam:1", "Elden Ring"), "conflict");
  assert.equal(store.getState().running?.gameId, "steam:9");
  store.stop();
});

test("Jogar que dá certo fica abrindo até o núcleo dizer o resto", async () => {
  const { store, emitStarted, emitEnded } = make();
  store.start();
  await tick();
  assert.equal(await store.play("steam:1", "Elden Ring"), "ok");
  assert.equal(store.getState().running?.phase, "launching");
  emitStarted({ gameId: "steam:1", title: "Elden Ring", startedAt: new Date(T0).toISOString(), tracked: true, notices: [] });
  assert.equal(store.getState().running?.pendingSince, null);
  emitEnded(ended({ end: "launch_failed", playedSeconds: 0, message: "não abriu" }));
  assert.equal(store.getState().running, null);
  assert.equal(store.getState().notice?.kind, "error");
  store.stop();
});

test("Encerrar mostra fechando; conflict do núcleo oferece forçar; forçar volta a esperar", async () => {
  let refuse = true;
  const { store, log } = make({
    running: async () => running({ state: "closing" }),
    end: async () => {
      if (refuse) throw conflict();
    },
  });
  store.start();
  await tick();
  assert.equal(await store.end(), "conflict");
  assert.equal(store.getState().running?.phase, "closing");
  assert.equal(store.getState().forceOffered, true);
  refuse = false;
  assert.equal(await store.end(true), "ok");
  assert.equal(store.getState().forceOffered, false);
  assert.ok(log.includes("end:false") && log.includes("end:true"));
  store.stop();
});

test("falha que não é conflict ao encerrar volta como erro", async () => {
  const { store } = make({
    end: async () => {
      throw new Error("ponte caiu");
    },
  });
  store.start();
  await tick();
  const original = console.error;
  console.error = () => {};
  try {
    assert.equal(await store.end(), "error");
  } finally {
    console.error = original;
  }
  store.stop();
});

test("uma leitura que partiu antes de um evento não ressuscita o jogo que acabou de terminar", async () => {
  let release = () => {};
  let first = true;
  const { store, emitEnded } = make({
    running: () => {
      if (!first) return Promise.resolve(null);
      first = false;
      return new Promise((resolve) => {
        release = () => resolve(running());
      });
    },
  });
  store.start();
  await tick();
  // O fim chega enquanto a leitura ainda não voltou; ela volta depois dizendo que ainda jogava.
  emitEnded(ended());
  release();
  await tick();
  assert.equal(store.getState().running, null);
  assert.equal(store.getState().notice?.kind, "welcome");
  store.stop();
});

test("Voltar ao jogo chama game.resume; sem jogo aberto (conflict) relê em vez de falhar", async () => {
  const { store, log } = make({
    resume: async () => {
      throw conflict();
    },
  });
  store.start();
  await tick();
  await store.resume();
  await tick();
  assert.deepEqual(log.filter((x) => x === "resume" || x === "running"), ["running", "resume", "running"]);
  store.stop();
});
