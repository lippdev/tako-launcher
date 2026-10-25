import assert from "node:assert/strict";
import { test } from "node:test";
import {
  SHELF_PAGE,
  appendPage,
  gamesById,
  hasMore,
  shelfSpecs,
  visibleShelves,
} from "./visao/visaoModel.ts";

function game(id, lastPlayedAt = null) {
  return { id, title: id, lastPlayedAt };
}
const played = "2026-10-24T16:17:00Z";

test("as fileiras vêm na ordem: continuar jogando, recentes, coleções", () => {
  const specs = shelfSpecs([
    { id: "a", name: "Noite", gameIds: ["x"], criteria: null },
    { id: "vazia", name: "Nada", gameIds: [], criteria: null },
    { id: "auto", name: "Automática", gameIds: [], criteria: { stores: ["steam"] } },
  ]);
  assert.deepEqual(
    specs.map((s) => s.id),
    ["continue", "recent", "collection:a", "collection:auto"],
  );
  assert.deepEqual(specs[0].request, { installed: true, sort: "recently_played" });
  assert.deepEqual(specs[1].request, { sort: "recently_played" });
  assert.deepEqual(specs[2].request, { collection: "a", sort: "recently_played" });
  assert.equal(specs[2].name, "Noite");
  assert.equal(SHELF_PAGE, 20);
});

test("continuar jogando e recentes só guardam jogo já jogado", () => {
  const [spec] = shelfSpecs([]);
  const state = appendPage(null, spec, { rows: [game("a", played), game("b", played), game("c")], total: 30 }, 0);
  assert.deepEqual(
    state.games.map((g) => g.id),
    ["a", "b"],
  );
  // O primeiro nunca jogado encerra a fileira: não há mais o que pedir.
  assert.equal(hasMore(state), false);
});

test("páginas seguintes se juntam sem repetir jogo", () => {
  const spec = shelfSpecs([{ id: "a", name: "A", gameIds: ["1"], criteria: null }])[2];
  const first = appendPage(null, spec, { rows: [game("1"), game("2")], total: 3 }, 0);
  assert.equal(hasMore(first), true);
  assert.equal(first.fetched, 2);
  const second = appendPage(first, spec, { rows: [game("2"), game("3")], total: 3 }, 2);
  assert.deepEqual(
    second.games.map((g) => g.id),
    ["1", "2", "3"],
  );
  assert.equal(hasMore(second), false);
});

test("fileira sem jogos não aparece", () => {
  const [spec] = shelfSpecs([]);
  const empty = appendPage(null, spec, { rows: [game("a")], total: 1 }, 0);
  const full = appendPage(null, spec, { rows: [game("a", played)], total: 1 }, 0);
  assert.deepEqual(visibleShelves([empty, full]), [full]);
});

test("o destaque conhece os jogos de todas as fileiras e o primeiro da biblioteca", () => {
  const [spec] = shelfSpecs([]);
  const shelf = appendPage(null, spec, { rows: [game("a", played)], total: 1 }, 0);
  const found = gamesById([shelf], game("novo"));
  assert.deepEqual([...found.keys()].sort(), ["a", "novo"]);
});
