import assert from "node:assert/strict";
import { test } from "node:test";
import { libraryGameIcon, libraryGameToRow } from "./libraryRow.ts";
const game = {
  id: "heroic:a b&x",
  title: "Jogo",
  store: "epic",
  installed: true,
  favorite: true,
  status: "beaten",
  hasIcon: true,
};
test("mapeia a situação sem inventar download ou processo aberto", () => {
  assert.deepEqual(libraryGameToRow(game), {
    id: game.id,
    title: game.title,
    store: "epic",
    installed: true,
    favorite: true,
    status: "completed",
    running: false,
    downloadPercent: null,
    updateAvailable: false,
  });
  for (const [status, expected] of [
    [null, "none"],
    ["playing", "playing"],
    ["wishlist", "queued"],
    ["paused", "paused"],
    ["abandoned", "dropped"],
  ])
    assert.equal(libraryGameToRow({ ...game, status }).status, expected);
});
test("ícone só quando existe e com ID escapado", () => {
  assert.equal(libraryGameIcon(game, 48), "tako-img://icon?id=heroic%3Aa%20b%26x&w=48&h=48");
  assert.equal(libraryGameIcon({ ...game, hasIcon: false }, 48), null);
});
