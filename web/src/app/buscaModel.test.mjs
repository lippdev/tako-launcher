import assert from "node:assert/strict";
import { test } from "node:test";
import {
  buildResults,
  buildShortcuts,
  clampSelection,
  initial,
  matchShortcuts,
  moveSelection,
  normalize,
} from "./busca/buscaModel.ts";

const labels = {
  views: { biblioteca: "Biblioteca", configuracoes: "Configurações", perfil: "Perfil", downloads: "Downloads" },
  categories: {
    general: "Geral",
    theme: "Tema",
    appearance: "Aparência",
    sound: "Som",
    display: "Tela",
    controllers: "Controles",
    library: "Biblioteca e lojas",
    covers: "Capas e dados",
    downloads: "Downloads",
    integrations: "Integrações",
    system: "Sistema",
    about: "Sobre",
  },
  settingsGroup: "Configurações",
  viewsGroup: "Ir para",
};
const resultLabels = { stores: { steam: "Steam", gog: "GOG" }, installed: "Instalado", notInstalled: "Não instalado" };
const shortcuts = buildShortcuts(labels);
const game = (id, title, store, installed) => ({ id, title, store, installed });

test("o catálogo tem as três vistas e as doze categorias, nessa ordem", () => {
  assert.equal(shortcuts.length, 15);
  assert.deepEqual(
    shortcuts.slice(0, 3).map((s) => s.target),
    [{ view: "biblioteca" }, { view: "configuracoes" }, { view: "perfil" }],
  );
  assert.deepEqual(shortcuts[3]?.target, { view: "configuracoes", settingsCategory: "general" });
  assert.deepEqual(shortcuts[14]?.target, { view: "configuracoes", settingsCategory: "about" });
});

test("normaliza acento e caixa", () => {
  assert.equal(normalize("  Configurações "), "configuracoes");
  assert.equal(normalize("APARÊNCIA"), "aparencia");
});

test("texto vazio não traz atalho", () => {
  assert.deepEqual(matchShortcuts(shortcuts, ""), []);
  assert.deepEqual(matchShortcuts(shortcuts, "   "), []);
});

test("título que começa com o texto vem antes do que só o contém", () => {
  const ids = matchShortcuts(shortcuts, "bib").map((s) => s.id);
  assert.deepEqual(ids, ["view:biblioteca", "settings:library"]);
  // Só "Aparência" começa com "a"; os demais apenas contêm.
  const a = matchShortcuts(shortcuts, "a", 20).map((s) => s.id);
  assert.equal(a[0], "settings:appearance");
});

test("sem acento e pelo grupo: 'configuracoes tema' acha a categoria", () => {
  assert.deepEqual(matchShortcuts(shortcuts, "configuracoes tema").map((s) => s.id), ["settings:theme"]);
  const all = matchShortcuts(shortcuts, "configuracoes", 20).map((s) => s.id);
  assert.equal(all[0], "view:configuracoes");
  assert.equal(all.length, 13);
});

test("respeita o limite de atalhos", () => {
  assert.equal(matchShortcuts(shortcuts, "configuracoes").length, 5);
});

test("jogos primeiro, atalhos depois, com loja e situação na linha", () => {
  const results = buildResults(
    [game("steam:1", "Hades", "steam", true), game("gog:2", "Tunic", "gog", false)],
    matchShortcuts(shortcuts, "som"),
    resultLabels,
  );
  assert.deepEqual(
    results.map((r) => [r.kind, r.id, r.title, r.meta]),
    [
      ["game", "steam:1", "Hades", "Steam · Instalado"],
      ["game", "gog:2", "Tunic", "GOG · Não instalado"],
      ["shortcut", "settings:sound", "Som", "Configurações"],
    ],
  );
  assert.deepEqual(results[2]?.target, { view: "configuracoes", settingsCategory: "sound" });
});

test("a seleção para nas pontas e se ajusta quando a lista encolhe", () => {
  assert.equal(moveSelection(0, -1, 3), 0);
  assert.equal(moveSelection(0, 1, 3), 1);
  assert.equal(moveSelection(2, 1, 3), 2);
  assert.equal(moveSelection(0, 1, 0), 0);
  assert.equal(clampSelection(5, 3), 2);
  assert.equal(clampSelection(1, 0), 0);
  assert.equal(clampSelection(-1, 3), 0);
});

test("a inicial do jogo", () => {
  assert.equal(initial("  élan"), "É");
  assert.equal(initial(""), "");
});
