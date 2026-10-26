import assert from "node:assert/strict";
import { beforeEach, test } from "node:test";
import { reportInputSource, resetSourceReporter } from "./sourceReporter.ts";

beforeEach(() => {
  resetSourceReporter();
});

test("reportInputSource envia a primeira mudança imediatamente", () => {
  const sent = [];
  const sender = (pad) => {
    sent.push(pad);
  };

  reportInputSource(true, 1000, sender);
  assert.equal(sent.length, 1);
  assert.equal(sent[0], true);
});

test("reportInputSource limita a no máximo uma vez por segundo e entrega o estado final", async () => {
  const sent = [];
  const sender = (pad) => {
    sent.push(pad);
  };

  // 1. Início: controle ativo no t = 1000 -> enviado na hora
  reportInputSource(true, 1000, sender);
  assert.equal(sent.length, 1);
  assert.equal(sent[0], true);

  // 2. Teclado/mouse usado 300 ms depois (t = 1300) -> agendado, não enviado ainda
  reportInputSource(false, 1300, sender);
  assert.equal(sent.length, 1);

  // 3. Espera o temporizador disparar (espera 1100 ms)
  await new Promise((resolve) => setTimeout(resolve, 1100));

  // Agora deve ter sido enviado pad: false
  assert.equal(sent.length, 2);
  assert.equal(sent[1], false);
});

test("reportInputSource não envia se voltar para o mesmo estado antes do temporizador", async () => {
  const sent = [];
  const sender = (pad) => {
    sent.push(pad);
  };

  // 1. Controle ativado
  reportInputSource(true, 1000, sender);
  assert.equal(sent.length, 1);

  // 2. Mouse move no t = 1200
  reportInputSource(false, 1200, sender);
  assert.equal(sent.length, 1);

  // 3. Controle usado de novo no t = 1400 (volta ao estado reportado)
  reportInputSource(true, 1400, sender);
  assert.equal(sent.length, 1);

  await new Promise((resolve) => setTimeout(resolve, 1100));

  // Nenhuma chamada extra necessária pois voltou para pad: true
  assert.equal(sent.length, 1);
});
