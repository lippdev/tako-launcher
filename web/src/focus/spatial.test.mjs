import assert from "node:assert/strict";
import { test } from "node:test";
import { actionForKey, keyForAction } from "./keymap.ts";
import { choose, entry, findNext } from "./spatial.ts";

const rect = (left, top, width, height) => ({ left, top, width, height });
const cand = (item, r, region = "", extra = {}) => ({ item, rect: r, region, ...extra });

// Como a Início: botão do destaque e duas fileiras de capas (as mesmas medidas do teste do núcleo em Rust).
const hero = [
  cand("play", rect(184, 380, 210, 76), "content"),
  cand("r1c1", rect(184, 560, 204, 306), "content"),
  cand("r1c2", rect(414, 560, 204, 306), "content"),
  cand("r1c3", rect(644, 560, 204, 306), "content"),
  cand("r2c1", rect(184, 940, 204, 306), "content"),
  cand("r2c2", rect(414, 940, 204, 306), "content"),
];
const byId = (list, id) => list.find((c) => c.item === id);
const from = (list, id, direction, options) => {
  const c = byId(list, id);
  return choose(c.rect, c.region, c.group, list, direction, { current: id, ...options });
};
const next = (list, id, direction, options) => from(list, id, direction, options)?.candidate.item ?? null;

test("o vizinho mais próximo na direção", () => {
  assert.equal(next(hero, "r1c1", "right"), "r1c2");
  assert.equal(next(hero, "r1c2", "left"), "r1c1");
  assert.equal(next(hero, "r1c2", "down"), "r2c2");
  assert.equal(next(hero, "r1c3", "up"), "play");
  assert.equal(next(hero, "play", "down"), "r1c1");
});

test("na borda, sem vizinho: nada", () => {
  assert.equal(next(hero, "r1c3", "right"), null);
  assert.equal(next(hero, "play", "up"), null);
  assert.equal(next(hero, "r1c1", "left"), null);
});

test("o desvio lateral pesa mais que a distância: prefere o alinhado ao torto mais perto", () => {
  const list = [
    cand("a", rect(100, 100, 100, 40)),
    cand("alinhado", rect(100, 400, 100, 40)),
    cand("torto", rect(300, 150, 100, 40)),
  ];
  // Para baixo de "a": o torto está mais perto (10 px) mas 200 px ao lado.
  assert.equal(next(list, "a", "down"), "alinhado");
});

test("não escolhe a si mesmo nem quem está do outro lado", () => {
  const list = [cand("a", rect(0, 0, 50, 50)), cand("b", rect(100, 0, 50, 50))];
  assert.equal(next(list, "a", "left"), null);
  assert.equal(next(list, "b", "right"), null);
  assert.equal(findNext(list[0].rect, [list[0]], "right"), null);
});

test("um vizinho que se sobrepõe um pouco mas está além do centro ainda conta", () => {
  const list = [cand("a", rect(0, 0, 100, 40)), cand("b", rect(90, 0, 100, 40))];
  assert.equal(next(list, "a", "right"), "b");
});

// Barra de cima, lista à esquerda e conteúdo.
const app = [
  cand("brand", rect(10, 0, 60, 50), "top"),
  cand("biblioteca", rect(150, 0, 100, 50), "top"),
  cand("busca", rect(1300, 0, 40, 50), "top"),
  cand("tab-todos", rect(10, 70, 100, 30), "list"),
  cand("tab-fav", rect(110, 70, 100, 30), "list"),
  cand("campo", rect(10, 110, 280, 38), "list"),
  cand("lista", rect(10, 200, 330, 680), "list"),
  cand("jogar", rect(380, 380, 210, 76), "content"),
  cand("capa1", rect(380, 560, 200, 300), "content"),
  cand("capa2", rect(600, 560, 200, 300), "content"),
];

test("dentro da região, sem atravessar enquanto houver vizinho", () => {
  const choice = from(app, "tab-todos", "down");
  assert.equal(choice.candidate.item, "campo");
  assert.equal(choice.crossed, false);
  assert.equal(next(app, "campo", "down"), "lista");
});

test("atravessa para a região vizinha quando a região acaba", () => {
  const down = from(app, "biblioteca", "down");
  assert.equal(down.crossed, true);
  assert.equal(down.candidate.region, "list");
  const right = from(app, "lista", "right");
  assert.equal(right.crossed, true);
  assert.equal(right.candidate.region, "content");
  assert.equal(next(app, "capa1", "up"), "jogar");
  const up = from(app, "jogar", "up");
  assert.equal(up.candidate.region, "top");
});

test("só atravessa para região alinhada: sem pular na diagonal", () => {
  // A busca (canto direito da barra) não tem lista embaixo: desce para o conteúdo, cuja caixa vai até a borda
  // mesmo que os focáveis dele estejam todos à esquerda.
  const regions = new Map([["content", rect(344, 50, 1256, 850)]]);
  assert.equal(from(app, "busca", "down", { regions }).candidate.region, "content");
  assert.equal(from(app, "busca", "down"), null, "sem as caixas das regiões, vale a que envolve os candidatos");
  // O marcador da lista, no alto, não alcança o conteúdo à direita pela diagonal da barra.
  const lonely = [cand("a", rect(0, 0, 100, 40), "x"), cand("b", rect(500, 400, 100, 40), "y")];
  assert.equal(from(lonely, "a", "down"), null);
  assert.equal(from(lonely, "a", "right"), null);
});

test("ao voltar para uma região, devolve o último foco dela", () => {
  const memory = new Map([["list", "lista"]]);
  // Sem memória, a seta para a esquerda cai no mais próximo (o campo, na altura do botão Jogar? não: a lista inteira).
  const sem = from(app, "capa1", "left");
  assert.equal(sem.candidate.region, "list");
  const com = from(app, "capa1", "left", { memory });
  assert.equal(com.candidate.item, "lista");
  const outra = from(app, "capa1", "left", { memory: new Map([["list", "tab-fav"]]) });
  assert.equal(outra.candidate.item, "tab-fav", "a memória vale mesmo que o item não seja o geometricamente mais perto");
});

test("memória de elemento que sumiu é ignorada", () => {
  const choice = from(app, "capa1", "left", { memory: new Map([["list", "jogo-que-saiu"]]) });
  assert.equal(choice.candidate.region, "list");
  assert.notEqual(choice.candidate.item, "jogo-que-saiu");
});

test("a memória só vale ao atravessar, não dentro da região", () => {
  const memory = new Map([["list", "lista"]]);
  assert.equal(next(app, "tab-todos", "down", { memory }), "campo");
});

test("fileira rolável: dentro dela as capas escondidas contam; de fora, não", () => {
  const row1 = { id: "fileira-1" };
  const row2 = { id: "fileira-2" };
  const rows = [
    cand("a1", rect(0, 100, 200, 300), "content", { group: row1, clipped: false }),
    cand("a2", rect(220, 100, 200, 300), "content", { group: row1, clipped: false }),
    cand("a9", rect(2000, 100, 200, 300), "content", { group: row1, clipped: true }),
    cand("b1", rect(0, 500, 200, 300), "content", { group: row2, clipped: false }),
    cand("b2", rect(220, 500, 200, 300), "content", { group: row2, clipped: false }),
    // Escondida na fileira de baixo, mas num ponto que o ← → de dentro dela alcançaria.
    cand("b9", rect(220, 500, 200, 300), "content", { group: row2, clipped: true }),
  ];
  const go = (id, direction) => {
    const c = byId(rows, id);
    return choose(c.rect, c.region, c.group, rows, direction, { current: id })?.candidate.item ?? null;
  };
  // Dentro da fileira, a escondida é o vizinho da direita da última visível.
  assert.equal(go("a2", "right"), "a9");
  // Descendo, nunca para uma escondida: b2 vence mesmo com b9 no mesmo lugar.
  assert.equal(go("a2", "down"), "b2");
  assert.equal(go("a1", "down"), "b1");
  assert.equal(go("b1", "up"), "a1");
});

test("entrada: o lembrado da região, senão o primeiro de cima para baixo e da esquerda para a direita", () => {
  assert.equal(entry(app, "list").item, "tab-todos");
  assert.equal(entry(app, "content").item, "jogar");
  assert.equal(entry(app, "list", new Map([["list", "lista"]])).item, "lista");
  assert.equal(entry(app, "list", new Map([["list", "sumiu"]])).item, "tab-todos");
  assert.equal(entry(app, "nenhuma"), null);
  // Escondidas pela rolagem não servem de entrada.
  const rows = [cand("x", rect(0, 0, 10, 10), "r", { group: 1, clipped: true }), cand("y", rect(50, 0, 10, 10), "r")];
  assert.equal(entry(rows, "r").item, "y");
});

const key = (key, extra = {}) => ({
  key,
  ctrlKey: false,
  altKey: false,
  metaKey: false,
  shiftKey: false,
  typing: false,
  ...extra,
});

test("teclas: o mesmo mapa do app atual", () => {
  assert.deepEqual(actionForKey(key("ArrowUp")), { kind: "navigate", direction: "up" });
  assert.deepEqual(actionForKey(key("ArrowRight")), { kind: "navigate", direction: "right" });
  assert.deepEqual(actionForKey(key("Enter")), { kind: "activate" });
  assert.deepEqual(actionForKey(key(" ")), { kind: "activate" });
  assert.deepEqual(actionForKey(key("Escape")), { kind: "back" });
  assert.deepEqual(actionForKey(key("Backspace")), { kind: "back" });
  assert.deepEqual(actionForKey(key("q")), { kind: "section", delta: -1 });
  assert.deepEqual(actionForKey(key("E")), { kind: "section", delta: 1 });
  assert.equal(actionForKey(key("x")), null);
});

test("teclas: atalhos e seleção não são do navegador; em campo de texto, as letras são do campo", () => {
  assert.equal(actionForKey(key("ArrowDown", { shiftKey: true })), null);
  assert.equal(actionForKey(key("ArrowDown", { ctrlKey: true })), null);
  assert.equal(actionForKey(key("e", { ctrlKey: true })), null);
  assert.equal(actionForKey(key("q", { altKey: true })), null);
  assert.equal(actionForKey(key("q", { typing: true })), null);
  assert.equal(actionForKey(key(" ", { typing: true })), null);
  assert.equal(actionForKey(key("Backspace", { typing: true })), null);
  // Setas e Esc continuam valendo no campo (a ponta do texto e o conteúdo decidem depois).
  assert.deepEqual(actionForKey(key("ArrowDown", { typing: true })), { kind: "navigate", direction: "down" });
  assert.deepEqual(actionForKey(key("Escape", { typing: true })), { kind: "back" });
});

test("o controle é oferecido ao componente como a tecla equivalente", () => {
  assert.equal(keyForAction({ kind: "navigate", direction: "left" }), "ArrowLeft");
  assert.equal(keyForAction({ kind: "activate" }), "Enter");
  assert.equal(keyForAction({ kind: "back" }), "Escape");
  assert.equal(keyForAction({ kind: "section", delta: 1 }), null);
});

test("alinhado vence o torto: ← do botão não cai numa capa duas fileiras abaixo, vai para a lista", () => {
  // O botão Jogar tem capas só abaixo e à esquerda de seu centro; a região vizinha da esquerda é a lista.
  const list = [
    cand("jogar", rect(380, 380, 210, 76), "content"),
    cand("capa-esq", rect(100, 560, 200, 300), "content"),
    cand("lista", rect(10, 200, 330, 680), "list"),
  ];
  const regions = new Map([
    ["list", rect(0, 50, 344, 850)],
    ["content", rect(344, 50, 1256, 850)],
  ]);
  const choice = from(list, "jogar", "left", { regions });
  assert.equal(choice.candidate.item, "lista");
  assert.equal(choice.crossed, true);
});

test("sem alinhado em lugar nenhum, o torto da própria região ainda serve (fileira de baixo mais curta)", () => {
  const rows = [
    cand("a3", rect(500, 100, 200, 300), "content"),
    cand("b1", rect(0, 500, 200, 300), "content"),
  ];
  assert.equal(next(rows, "a3", "down"), "b1");
});
