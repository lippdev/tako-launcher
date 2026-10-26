import assert from "node:assert/strict";
import { test } from "node:test";
import {
  cleanName,
  hasSavableCriteria,
  MAX_NAME_LENGTH,
  nameProblem,
  ofKind,
  smartRuleFromCriteria,
} from "./colecoes/colecoesModel.ts";

const blank = {
  filter: "all",
  search: "",
  sort: "recently_played",
  reversed: false,
  collection: null,
  stores: [],
  statuses: [],
  noStatus: false,
  tags: [],
  installed: null,
};
const manual = (id, name) => ({ id, name, gameIds: [], criteria: null });

test("o nome perde espaços das pontas e repetidos", () => {
  assert.equal(cleanName("  RPGs   jogando "), "RPGs jogando");
  assert.equal(cleanName("   "), "");
});

test("nome vazio, comprido ou repetido dá o erro do núcleo", () => {
  const existing = [manual("a", "Favoritos do sofá")];
  assert.equal(nameProblem("   ", existing), "empty");
  assert.equal(nameProblem("x".repeat(MAX_NAME_LENGTH), existing), null);
  assert.equal(nameProblem("x".repeat(MAX_NAME_LENGTH + 1), existing), "too_long");
  // As letras contam como no núcleo (pontos de código), não unidades UTF-16.
  assert.equal(nameProblem("🎮".repeat(MAX_NAME_LENGTH), existing), null);
  assert.equal(nameProblem("favoritos  DO sofa", existing), "duplicate");
  assert.equal(nameProblem("Favoritos do sofá", existing, "a"), null, "renomear mantém o próprio nome");
  assert.equal(nameProblem("Favoritos do sofá", existing, "b"), "duplicate");
});

test("manuais e automáticas têm listas de nomes separadas", () => {
  const smart = { id: "s", name: "Auto", gameIds: [], criteria: {} };
  const all = [manual("m", "Manual"), smart];
  assert.deepEqual(
    ofKind(all, true).map((c) => c.id),
    ["s"],
  );
  assert.deepEqual(
    ofKind(all, false).map((c) => c.id),
    ["m"],
  );
});

test("só há o que salvar quando algo foge do padrão", () => {
  assert.equal(hasSavableCriteria(blank), false);
  assert.equal(hasSavableCriteria({ ...blank, search: "  " }), false);
  for (const patch of [
    { search: "hades" },
    { filter: "favorites" },
    { collection: "c" },
    { stores: ["steam"] },
    { statuses: ["playing"] },
    { tags: ["RPG"] },
    { noStatus: true },
    { installed: false },
    { sort: "name" },
    { reversed: true },
  ]) {
    assert.equal(hasSavableCriteria({ ...blank, ...patch }), true, JSON.stringify(patch));
  }
});

test("a regra sai dos filtros da tela", () => {
  const rule = smartRuleFromCriteria({
    ...blank,
    filter: "installed",
    search: "  hades ",
    sort: "name",
    reversed: true,
    stores: ["steam"],
    statuses: ["playing"],
    tags: ["RPG"],
    installed: true,
    collection: "manual-1",
  });
  assert.deepEqual(rule, {
    library: {
      filter: "installed",
      search: "hades",
      sort: "name",
      reversed: true,
      collection: "manual-1",
      tags: ["RPG"],
      statuses: ["playing"],
      noStatus: false,
      stores: ["steam"],
      installed: true,
    },
    genres: [],
    playTime: "any",
    playedOnly: false,
    brokenOnly: false,
    favoritesOnly: false,
    collections: ["manual-1"],
    filterStore: null,
  });
  // O núcleo recusa `offset` diferente de zero e `deny_unknown_fields` barra o resto: nada de paginação na regra.
  assert.equal("offset" in rule.library, false);
  assert.equal("limit" in rule.library, false);
});

test("sem filtros a regra é a lista inteira, sem coleção", () => {
  const rule = smartRuleFromCriteria(blank);
  assert.equal(rule.library.filter, "all");
  assert.equal(rule.library.collection, null);
  assert.deepEqual(rule.collections, []);
  assert.deepEqual(rule.library.stores, []);
});

test("com uma coleção automática aberta, os filtros da tela valem por cima da regra dela", () => {
  const open = {
    id: "auto",
    name: "Steam jogando",
    gameIds: [],
    criteria: {
      library: {
        filter: "favorites",
        search: "",
        sort: "play_time",
        reversed: false,
        collection: "manual-1",
        tags: [],
        statuses: ["playing"],
        noStatus: false,
        stores: ["steam"],
        installed: null,
      },
      genres: ["RPG"],
      playTime: "over_ten_hours",
      playedOnly: true,
      brokenOnly: false,
      favoritesOnly: true,
      collections: ["manual-1"],
      filterStore: null,
    },
  };
  // A tela só abriu a coleção (collection = id dela): a regra continua igual, sem apontar para si mesma.
  const same = smartRuleFromCriteria({ ...blank, collection: "auto" }, open);
  assert.deepEqual(same, open.criteria);

  // Filtros novos trocam o campo correspondente; o resto fica como estava.
  const changed = smartRuleFromCriteria(
    { ...blank, collection: "auto", search: "elden", installed: true, sort: "name", stores: ["gog"] },
    open,
  );
  assert.equal(changed.library.search, "elden");
  assert.equal(changed.library.installed, true);
  assert.equal(changed.library.sort, "name");
  assert.deepEqual(changed.library.stores, ["gog"]);
  assert.deepEqual(changed.library.statuses, ["playing"]);
  assert.equal(changed.library.filter, "favorites");
  assert.equal(changed.library.collection, "manual-1");
  assert.deepEqual(changed.genres, ["RPG"]);
  assert.equal(changed.playTime, "over_ten_hours");
  assert.equal(changed.playedOnly, true);
});

test("filtro de loja só vale na aba Todos", () => {
  const open = {
    id: "auto",
    name: "GOG",
    gameIds: [],
    criteria: { ...smartRuleFromCriteria(blank), filterStore: "gog" },
  };
  assert.equal(smartRuleFromCriteria({ ...blank, collection: "auto" }, open).filterStore, "gog");
  assert.equal(smartRuleFromCriteria({ ...blank, collection: "auto", filter: "favorites" }, open).filterStore, null);
});
