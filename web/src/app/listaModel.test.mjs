import assert from "node:assert/strict";
import { test } from "node:test";
import { parseGroupBy, parseListView, planGroups, ROW_PAGE } from "./lista/listaModel.ts";

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
const sources = {
  collections: [
    { id: "a", name: "Noite", gameIds: ["x"], criteria: null },
    { id: "b", name: "Vazia", gameIds: [], criteria: null },
  ],
  tags: [
    { name: "RPG", count: 2 },
    { name: "Velha", count: 0 },
  ],
  stores: ["steam", "epic", "gog"],
};
const ids = (specs) => specs.map((g) => g.id);

test("o modo guardado vale, o resto é a Lista", () => {
  assert.equal(parseListView("grid"), "grid");
  assert.equal(parseListView("categories"), "categories");
  assert.equal(parseListView(undefined), "list");
  assert.equal(parseListView("cards"), "list");
});

test("o agrupamento guardado reconhece gênero e usa situação para valor desconhecido", () => {
  assert.equal(parseGroupBy("store"), "store");
  assert.equal(parseGroupBy("install"), "install");
  assert.equal(parseGroupBy("genre"), "genre");
  assert.equal(parseGroupBy(undefined), "status");
});

test("situação: a ordem do Slint, cada fileira pede só a sua situação", () => {
  const groups = planGroups("status", blank, sources);
  assert.deepEqual(ids(groups), [
    "status:playing",
    "status:wishlist",
    "status:paused",
    "status:beaten",
    "status:abandoned",
  ]);
  assert.deepEqual(groups[1].request.statuses, ["wishlist"]);
  assert.equal(groups[1].request.noStatus, false);
});

test("situação respeita o filtro da tela", () => {
  const some = planGroups("status", { ...blank, statuses: ["beaten", "playing"] }, sources);
  assert.deepEqual(ids(some), ["status:playing", "status:beaten"]);
  assert.deepEqual(planGroups("status", { ...blank, noStatus: true }, sources), []);
});

test("coleção: todas, na ordem do núcleo; com uma aberta, só ela", () => {
  assert.deepEqual(ids(planGroups("collection", blank, sources)), ["collection:a", "collection:b"]);
  const open = planGroups("collection", { ...blank, collection: "b" }, sources);
  assert.deepEqual(ids(open), ["collection:b"]);
  assert.equal(open[0].title, "Vazia");
  assert.equal(open[0].request.collection, "b");
});

test("etiqueta: a que não tem jogo não vira fileira e as etiquetas do filtro continuam no pedido", () => {
  const groups = planGroups("tag", { ...blank, tags: ["Co-op"] }, sources);
  assert.deepEqual(ids(groups), ["tag:RPG"]);
  assert.deepEqual(groups[0].request.tags, ["Co-op", "RPG"]);
  assert.deepEqual(planGroups("tag", { ...blank, tags: ["RPG"] }, sources)[0].request.tags, ["RPG"]);
});

test("loja: a ordem recebida; com lojas no filtro, só elas", () => {
  assert.deepEqual(ids(planGroups("store", blank, sources)), ["store:steam", "store:epic", "store:gog"]);
  const some = planGroups("store", { ...blank, stores: ["gog"] }, sources);
  assert.deepEqual(ids(some), ["store:gog"]);
  assert.deepEqual(some[0].request.stores, ["gog"]);
});

test("instalação: instalados primeiro; o filtro escolhe um lado", () => {
  const both = planGroups("install", blank, sources);
  assert.deepEqual(ids(both), ["install:installed", "install:not_installed"]);
  assert.deepEqual([both[0].request.installed, both[1].request.installed], [true, false]);
  assert.deepEqual(ids(planGroups("install", { ...blank, installed: false }, sources)), ["install:not_installed"]);
});

test("o pedido da fileira leva busca e abas à parte e não mexe nos critérios", () => {
  const criteria = { ...blank, search: "elden", stores: ["steam"], installed: true };
  const [group] = planGroups("status", criteria, sources);
  assert.deepEqual(group.request.stores, ["steam"]);
  assert.equal(group.request.installed, true);
  assert.equal("search" in group.request, false);
  assert.deepEqual(criteria.statuses, []);
  assert.equal(ROW_PAGE, 24);
});
