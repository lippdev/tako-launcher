import assert from "node:assert/strict";
import { test } from "node:test";
const storage = new Map();
globalThis.sessionStorage = {
  getItem: (k) => storage.get(k) ?? null,
  setItem: (k, v) => storage.set(k, v),
};
const pending = new Map();
globalThis.window = {
  __takoDeliver(text) {
    const wire = JSON.parse(text);
    if (wire.event) return;
    const done = pending.get(wire.id);
    pending.delete(wire.id);
    done(wire);
  },
};
const fake = await import("../bridge/fake.ts");
const { isDevSnapshot } = await import("../bridge/devSnapshot.ts");

const row = (id, title, extra = {}) => ({
  id,
  title,
  store: "steam",
  installed: true,
  favorite: false,
  status: null,
  lastPlayedAt: null,
  playTimeSeconds: 0,
  accent: null,
  hasCover: true,
  hasIcon: true,
  hasHero: false,
  hasLogo: false,
  ...extra,
});
const sekiro = row("steam:814380", "Sekiro", { lastPlayedAt: "2026-10-24T20:00:00.000Z", playTimeSeconds: 3600 });
const dota = row("steam:570", "Dota 2");
const session = { game: sekiro, startedAt: "2026-10-24T20:00:00.000Z", durationSeconds: 3600, quick: false };
const snapshot = {
  version: 1,
  createdAt: "2026-10-25T12:00:00Z",
  appInfo: { version: "0.2.0", bridgeVersion: 1 },
  games: [sekiro, dota],
  details: {
    "steam:814380": {
      game: sekiro,
      canPlay: true,
      upcoming: false,
      platform: "Windows",
      description: "Shinobi",
      developer: "FromSoftware",
      publisher: null,
      releaseDate: null,
      genres: null,
      metacriticScore: 90,
      criticScore: null,
      recentSessions: null,
      tags: ["Difícil"],
      sizeBytes: null,
      sizeLabel: null,
    },
  },
  collections: [{ id: "c1", name: "Soulslike", gameIds: ["steam:814380"], criteria: null }],
  tags: [{ name: "Difícil", count: 1 }],
  settings: { values: { Language: "pt-BR", DesktopTheme: "midnight" } },
  profileSummary: null,
  profileSessions: {
    week: null,
    month: null,
    all: { today: "2026-10-25", count: 1, totalSeconds: 3600, days: [{ day: "2026-10-24", totalSeconds: 3600, sessions: [session] }] },
  },
  profileCollections: null,
  profiles: null,
  images: { "steam:814380": { cover: "images/g0-cover.jpg", icon: "images/g0-icon.png" } },
};

assert.equal(isDevSnapshot(snapshot), true);
assert.equal(isDevSnapshot({ ...snapshot, version: 2 }), false);
assert.equal(fake.usingDevSnapshot(), false);
fake.applyDevSnapshot(snapshot);
fake.installFakeHost({}, 0);

let next = 0;
function call(name, payload) {
  const id = ++next;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    window.takoHost.post(JSON.stringify({ v: 1, id, name, payload }));
  }).then((r) => {
    assert.equal(r.ok, true, JSON.stringify(r));
    return r.reply.data;
  });
}

test("a biblioteca do snapshot substitui os jogos de exemplo", async () => {
  assert.equal(fake.usingDevSnapshot(), true);
  const page = await call("library.list", { limit: 50 });
  assert.equal(page.total, 2);
  assert.deepEqual(page.rows.map((g) => g.title), ["Sekiro", "Dota 2"]);
  const inCollection = await call("library.list", { collection: "c1", limit: 50 });
  assert.deepEqual(inCollection.rows.map((g) => g.id), ["steam:814380"]);
  assert.deepEqual(await call("tags.list"), [{ name: "Difícil", count: 1 }]);
  const settings = await call("settings.get", {});
  assert.equal(settings.values.DesktopTheme, "midnight");
});

test("detalhes vêm do núcleo com as mudanças da sessão por cima", async () => {
  await call("game.favorite.set", { gameIds: ["steam:814380"], favorite: true });
  const details = await call("game.details", { gameId: "steam:814380" });
  assert.equal(details.developer, "FromSoftware");
  assert.equal(details.game.favorite, true);
  assert.deepEqual(details.tags, ["Difícil"]);
  // Sem detalhe no snapshot, o falso ainda responde com o mínimo.
  const bare = await call("game.details", { gameId: "steam:570" });
  assert.equal(bare.game.title, "Dota 2");
});

test("sessões do snapshot; período sem resposta cai no exemplo", async () => {
  const all = await call("profile.sessions", { period: "all" });
  assert.equal(all.count, 1);
  assert.equal(all.days[0].sessions[0].game.favorite, true);
  const week = await call("profile.sessions", { period: "week" });
  // Só 2 jogos: as sessões de exemplo com posições que não existem saem.
  assert.ok(week.days.every((d) => d.sessions.every((s) => typeof s.game.id === "string")));
});

test("imagens copiadas no lugar da arte de exemplo", () => {
  assert.equal(fake.fakeGameImage("cover", "steam:814380", "Sekiro"), "/dev-data/images/g0-cover.jpg?v=2026-10-25T12%3A00%3A00Z");
  assert.equal(fake.fakeGameIcon("steam:814380", 32), "/dev-data/images/g0-icon.png?v=2026-10-25T12%3A00%3A00Z");
  assert.match(fake.fakeGameImage("hero", "steam:814380", "Sekiro"), /^data:image\/svg\+xml,/);
  assert.match(fake.fakeGameImage("cover", "steam:570", "Dota 2"), /^data:image\/svg\+xml,/);
});
