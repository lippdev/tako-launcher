import assert from "node:assert/strict";
import { test } from "node:test";
import { limparNome, MAX_NOME, normalizar, sugerir } from "../components/EditorDeEtiquetas/sugestoes.ts";

const todas = [
  { nome: "Co-op", jogos: 3 },
  { nome: "Sofá", jogos: 2 },
  { nome: "Zerado", jogos: 5 },
];

test("os nomes se comparam sem caixa, sem acento e sem espaços repetidos", () => {
  assert.equal(normalizar("  SOFÁ  Co-op "), "sofa co-op");
  assert.equal(limparNome("  Para   zerar "), "Para zerar");
});

test("digitar filtra as que existem e oferece criar o que não existe", () => {
  const s = sugerir(todas, ["Zerado"], "SOFA");
  assert.deepEqual(s.existentes, [{ nome: "Sofá", jogos: 2 }]);
  // Já existe (sem acento): não oferece criar.
  assert.equal(s.criar, null);

  const nova = sugerir(todas, [], "Com mods");
  assert.deepEqual(nova.existentes, []);
  assert.equal(nova.criar, "Com mods");
  // O nome criado é o limpo.
  assert.equal(sugerir(todas, [], "  Com   mods ").criar, "Com mods");
});

test("campo vazio lista o que falta, e a já aplicada sai da lista", () => {
  const s = sugerir(todas, ["Co-op"], "");
  assert.deepEqual(
    s.existentes.map((e) => e.nome),
    ["Sofá", "Zerado"],
  );
  assert.equal(s.criar, null);
});

test("a etiqueta que o jogo já tem não é oferecida nem criada de novo", () => {
  const s = sugerir(todas, ["Zerado"], "zerado");
  assert.deepEqual(s.existentes, []);
  assert.equal(s.criar, null);
});

test("o nome novo respeita o limite de tamanho", () => {
  assert.equal(sugerir([], [], "a".repeat(MAX_NOME)).criar, "a".repeat(MAX_NOME));
  assert.equal(sugerir([], [], "a".repeat(MAX_NOME + 1)).criar, null);
  assert.equal(sugerir([], [], "   ").criar, null);
});
