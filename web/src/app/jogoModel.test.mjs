import assert from "node:assert/strict";
import { test } from "node:test";
import {
  chipToStatus,
  formatDuration,
  formatLastPlayed,
  formatPlayTime,
  formatReleaseDate,
  gameBlocks,
  imageUrl,
  metacriticTone,
  metacriticVerdict,
  paragraphs,
  primaryState,
  statusToChip,
  validScore,
} from "./jogo/jogoModel.ts";

const game = {
  id: "steam:1",
  title: "Jogo",
  store: "steam",
  installed: true,
  favorite: false,
  status: null,
  lastPlayedAt: null,
  playTimeSeconds: 0,
  accent: null,
  hasCover: false,
  hasIcon: false,
  hasHero: false,
  hasLogo: false,
};
const empty = {
  game,
  canPlay: true,
  upcoming: false,
  platform: null,
  description: null,
  developer: null,
  publisher: null,
  releaseDate: null,
  genres: null,
  metacriticScore: null,
  criticScore: null,
  recentSessions: null,
  tags: null,
  sizeBytes: null,
  sizeLabel: null,
};

test("situação: ida e volta entre o núcleo e o chip", () => {
  for (const status of ["playing", "beaten", "wishlist", "paused", "abandoned"]) {
    assert.equal(chipToStatus(statusToChip(status)), status);
  }
  assert.equal(statusToChip(null), "none");
  assert.equal(chipToStatus("none"), null);
});

test("duração em português", () => {
  assert.equal(formatDuration(5), "menos de 1 min");
  assert.equal(formatDuration(42 * 60), "42 min");
  assert.equal(formatDuration(7 * 3600 + 20 * 60), "7 h 20 min");
  assert.equal(formatDuration(7 * 3600), "7 h");
  assert.equal(formatDuration(40 * 3600 + 1800), "40 h");
});

test("tempo de jogo zero é sem dado", () => {
  assert.equal(formatPlayTime(0), null);
  assert.equal(formatPlayTime(3600), "1 h");
});

test("última partida: dia, mês e hora, com o ano só se não for o corrente", () => {
  const now = new Date("2026-10-24T12:00:00Z");
  assert.equal(formatLastPlayed("2026-10-23T16:17:00Z", now, "UTC"), "23/10, 16:17");
  assert.equal(formatLastPlayed("2025-03-02T09:05:00Z", now, "UTC"), "02/03/2025, 09:05");
  assert.equal(formatLastPlayed(null, now, "UTC"), null);
  assert.equal(formatLastPlayed("não é data", now, "UTC"), null);
});

test("data de lançamento", () => {
  assert.equal(formatReleaseDate("2024-09-19"), "19 set, 2024");
  assert.equal(formatReleaseDate("em breve"), "em breve");
  assert.equal(formatReleaseDate(null), null);
});

test("Metacritic: faixas de cor e veredito", () => {
  assert.deepEqual([100, 75, 74, 50, 49, 0].map(metacriticTone), ["good", "good", "mixed", "mixed", "bad", "bad"]);
  assert.deepEqual([94, 90, 89, 75, 74, 50, 49].map(metacriticVerdict), [
    "universal",
    "universal",
    "favorable",
    "favorable",
    "mixed",
    "mixed",
    "unfavorable",
  ]);
  assert.equal(validScore(94), 94);
  for (const score of [null, -1, 101, Number.NaN]) assert.equal(validScore(score), null);
});

test("imagem do núcleo: id com codificação", () => {
  assert.equal(
    imageUrl("hero", "heroic:legendary:Fortnite", 1308.4, 700),
    "tako-img://hero?id=heroic%3Alegendary%3AFortnite&w=1308&h=700",
  );
});

test("parágrafos: linhas em branco separam e o vazio some", () => {
  assert.deepEqual(paragraphs("a\n\n  b  \n\n\n"), ["a", "b"]);
  assert.deepEqual(paragraphs(null), []);
  assert.deepEqual(paragraphs("  \n "), []);
});

test("sem dado, o bloco some", () => {
  assert.deepEqual(gameBlocks(empty), {
    description: [],
    developer: null,
    publisher: null,
    release: null,
    genres: [],
    tags: [],
    size: null,
    lastPlayed: null,
    playTime: null,
    metacritic: null,
    sessions: [],
  });
  // Texto em branco também é sem dado.
  assert.equal(gameBlocks({ ...empty, developer: "  ", sizeLabel: "" }).developer, null);
  assert.deepEqual(gameBlocks({ ...empty, genres: [" ", ""] }).genres, []);
});

test("com dado, o bloco aparece formatado", () => {
  const now = new Date("2026-10-24T12:00:00Z");
  const blocks = gameBlocks(
    {
      ...empty,
      game: { ...game, lastPlayedAt: "2026-10-23T16:17:00Z", playTimeSeconds: 7 * 3600 },
      developer: "Santa Monica Studios",
      releaseDate: "2024-09-19",
      metacriticScore: 94,
      recentSessions: [{ startedAt: "2026-10-23T16:17:00Z", durationSeconds: 2520 }],
      sizeLabel: "23.5 GB",
    },
    now,
    "UTC",
  );
  assert.equal(blocks.developer, "Santa Monica Studios");
  assert.equal(blocks.release, "19 set, 2024");
  assert.equal(blocks.lastPlayed, "23/10, 16:17");
  assert.equal(blocks.playTime, "7 h");
  assert.equal(blocks.metacritic, 94);
  assert.equal(blocks.size, "23.5 GB");
  assert.deepEqual(blocks.sessions, [{ startedAt: "23/10, 16:17", duration: "42 min" }]);
});

test("o primário: jogar, em breve ou não instalado", () => {
  assert.equal(primaryState(empty), "play");
  assert.equal(primaryState({ ...empty, canPlay: false, upcoming: true }), "upcoming");
  assert.equal(primaryState({ ...empty, canPlay: false, game: { ...game, installed: false } }), "not-installed");
});
