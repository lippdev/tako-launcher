import assert from "node:assert/strict";
import { test } from "node:test";
import {
  activityCount,
  entryLine,
  entryPercent,
  formatBytes,
  percentByGame,
  readUploadLimit,
  summaryPercent,
  toPanelItem,
} from "./downloadsModel.ts";

const GB = 1024 ** 3;
const MB = 1024 ** 2;

/** Um `t()` mínimo: devolve a chave e os parâmetros, para o teste ver o que foi pedido. */
const t = (key, vars) => (vars ? `${key}(${Object.values(vars).join(",")})` : key);

function entry(patch = {}) {
  return {
    id: 7,
    title: "Watch Dogs 2",
    gameId: "steam:1002",
    source: "Minha fonte",
    state: "downloading",
    error: null,
    done: 3 * GB,
    total: 6 * GB,
    progress: 0.5,
    speed: 48 * MB,
    etaSeconds: 3600,
    ...patch,
  };
}

test("o limite de envio guardado só vale com as quatro escolhas do núcleo", () => {
  assert.equal(readUploadLimit(undefined), 0);
  assert.equal(readUploadLimit("64"), 64);
  assert.equal(readUploadLimit("1024"), 1024);
  assert.equal(readUploadLimit("999"), 0);
  assert.equal(readUploadLimit("abc"), 0);
});

test("tamanhos como o núcleo: uma casa e vírgula em português", () => {
  assert.equal(formatBytes(512), "512 B");
  assert.equal(formatBytes(64 * 1024), "64 KB");
  assert.equal(formatBytes(1024 * 1024), "1,0 MB");
  assert.equal(formatBytes(1.5 * GB), "1,5 GB");
  assert.equal(formatBytes(1.5 * GB, false), "1.5 GB");
});

test("a porcentagem do download e a da barra", () => {
  assert.equal(entryPercent(entry()), 50);
  assert.equal(entryPercent(entry({ progress: null })), null);
  assert.equal(entryPercent(entry({ progress: 1.4 })), 100);
  const summary = { downloading: 1, queued: 1, speed: 1, done: 3, total: 12 };
  assert.equal(activityCount(summary), 2);
  assert.equal(summaryPercent(summary), 25);
  assert.equal(summaryPercent({ downloading: 0, queued: 0, speed: 0, done: 0, total: 0 }), null);
});

test("só o download em aberto vinculado a um jogo vira porcentagem na lista", () => {
  const map = percentByGame([
    entry(),
    entry({ id: 8, gameId: "steam:1010", state: "paused", progress: 0.2 }),
    entry({ id: 9, gameId: "steam:1011", state: "queued", progress: null }),
    entry({ id: 10, gameId: "steam:1012", state: "completed", progress: 1 }),
    entry({ id: 11, gameId: "steam:1013", state: "failed", progress: 0.3 }),
    entry({ id: 12, gameId: null }),
  ]);
  assert.deepEqual([...map], [
    ["steam:1002", 50],
    ["steam:1010", 20],
    ["steam:1011", 0],
  ]);
});

test("a linha do download tem os textos do app atual", () => {
  assert.equal(
    entryLine(entry(), t),
    "downloads.of(3,0 GB,6,0 GB) · downloads.left(1 h)",
  );
  assert.equal(entryLine(entry({ state: "paused", speed: 0 }), t), "downloads.paused · downloads.of(3,0 GB,6,0 GB)");
  assert.equal(entryLine(entry({ state: "queued", speed: 0 }), t), "downloads.of(3,0 GB,6,0 GB) · downloads.waiting");
  assert.equal(
    entryLine(entry({ total: null, progress: null, done: 0, speed: 0 }), t),
    "downloads.metadata",
  );
  assert.equal(entryLine(entry({ state: "failed", error: "sem espaço" }), t).startsWith("downloads.failed(sem espaço)"), true);
});

test("o painel recebe o download com id de texto, estado e progresso", () => {
  const item = toPanelItem(entry(), t, (id) => `capa:${id}`);
  assert.equal(item.id, "7");
  assert.equal(item.estado, "baixando");
  assert.equal(item.progresso, 50);
  assert.equal(item.capa, "capa:steam:1002");
  assert.equal(item.subtitulo, "downloads.of(3,0 GB,6,0 GB) · downloads.left(1 h)");
  assert.equal(toPanelItem(entry({ gameId: null }), t, () => "x").capa, null);
  assert.equal(toPanelItem(entry({ state: "failed" }), t, () => "x").estado, "erro");
  assert.equal(toPanelItem(entry({ state: "completed", speed: 0 }), t, () => "x").estado, "concluido");
});
