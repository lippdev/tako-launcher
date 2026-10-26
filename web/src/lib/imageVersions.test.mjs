import assert from "node:assert/strict";
import { test } from "node:test";
import { artChanged, imageUrl, imageVersion, subscribeArt, versionedImageUrl } from "./imageVersions.ts";

test("art.changed invalida só o jogo, em todos os tipos e tamanhos, mesmo após desmontar", () => {
  const id = "gog:32 espaço&";
  const other = imageUrl("cover", "steam:1", 200, 300);
  let updates = 0;
  const unsubscribe = subscribeArt(() => updates++);
  const old = imageUrl("cover", id, 200, 300);
  artChanged(id);
  for (const kind of ["cover", "hero", "logo", "icon"]) {
    const url = new URL(imageUrl(kind, id, 80, 120));
    assert.equal(url.searchParams.get("id"), id);
    assert.equal(url.searchParams.get("v"), "1");
    assert.ok(imageUrl(kind, id, 80, 120).includes("%20"));
  }
  assert.equal(new URL(versionedImageUrl(old)).searchParams.get("v"), "1");
  assert.equal(imageUrl("cover", "steam:1", 200, 300), other);
  assert.equal(updates, 1);
  unsubscribe();
  artChanged(id);
  assert.equal(imageVersion(id), 2);
  assert.equal(updates, 1);
  assert.equal(new URL(imageUrl("cover", id, 200, 300)).searchParams.get("v"), "2");
  assert.equal(versionedImageUrl("data:image/png,exemplo"), "data:image/png,exemplo");
});
