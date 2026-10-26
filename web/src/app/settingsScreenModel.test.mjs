import assert from "node:assert/strict";
import { test } from "node:test";
import { DEFAULTS, KEYS, hideOthersText, modeLabel, readHideOthers, readPercent } from "./settings/model.ts";

test("volume guardado vira número de 0 a 100; o resto vale o padrão", () => {
  assert.equal(readPercent({}, KEYS.navigationVolume, DEFAULTS.navigationVolume), 0);
  assert.equal(readPercent({}, KEYS.themeMusicVolume, DEFAULTS.themeMusicVolume), 70);
  assert.equal(readPercent({ ThemeMusicVolume: "35" }, KEYS.themeMusicVolume, 70), 35);
  assert.equal(readPercent({ ThemeMusicVolume: "0" }, KEYS.themeMusicVolume, 70), 0);
  assert.equal(readPercent({ ThemeMusicVolume: "100" }, KEYS.themeMusicVolume, 70), 100);
  for (const bad of ["101", "-1", "abc", "", "1.5", "999", " 5"])
    assert.equal(readPercent({ ThemeMusicVolume: bad }, KEYS.themeMusicVolume, 70), 70, bad);
});

test("desligar as outras telas vem ligado e só o desligamento explícito vale, no formato da ponte ou do app antigo", () => {
  assert.equal(readHideOthers({}), true);
  assert.equal(readHideOthers({ GameScreenHideOthers: "Disconnect" }), true);
  assert.equal(readHideOthers({ GameScreenHideOthers: "None" }), false);
  assert.equal(hideOthersText(true), "Disconnect");
  assert.equal(hideOthersText(false), "None");
});

test("a chave do modo vira o texto da tela; chave de outro formato aparece como veio", () => {
  assert.equal(modeLabel("3840x2160@120"), "3840 × 2160 · 120 Hz");
  assert.equal(modeLabel("1280x720@60"), "1280 × 720 · 60 Hz");
  assert.equal(modeLabel("1920x1080"), "1920x1080");
  assert.equal(modeLabel(""), "");
});
