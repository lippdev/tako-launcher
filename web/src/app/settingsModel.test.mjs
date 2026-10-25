import assert from "node:assert/strict";
import { test } from "node:test";
import {
  appearanceFromSettings,
  isStoreDisabled,
  readBool,
  readLanguage,
  readOpenIn,
  themeFromStored,
  themeToStored,
  withStoreEnabled,
} from "./settings/model.ts";

test("sem nada guardado a aparência é a padrão", () => {
  assert.deepEqual(appearanceFromSettings({}), {
    theme: "tako",
    reduceMotion: false,
    glassEffects: true,
    uiScale: 100,
    accent: null,
    glassOpacity: 28,
    glassPill: true,
    glassAmbient: true,
  });
});
test("a aparência guardada vira a aparência da página; vidro é o ReducedTransparency invertido", () => {
  assert.deepEqual(
    appearanceFromSettings({ DesktopTheme: "glass", ReducedMotion: "true", ReducedTransparency: "true", TextSizePercent: "125" }),
    { theme: "vidro", reduceMotion: true, glassEffects: false, uiScale: 125, accent: null, glassOpacity: 28, glassPill: true, glassAmbient: true },
  );
  assert.equal(appearanceFromSettings({ ReducedTransparency: "false" }).glassEffects, true);
});
test("valor ilegível cai no padrão, campo a campo", () => {
  assert.deepEqual(
    appearanceFromSettings({ DesktopTheme: "lixo", ReducedMotion: "sim", ReducedTransparency: "", TextSizePercent: "90" }),
    { theme: "tako", reduceMotion: false, glassEffects: true, uiScale: 100, accent: null, glassOpacity: 28, glassPill: true, glassAmbient: true },
  );
  assert.equal(appearanceFromSettings({ TextSizePercent: "abc" }).uiScale, 100);
  assert.equal(appearanceFromSettings({ DesktopTheme: "toString" }).theme, "tako");
});
test("o tema vai e volta pelo nome que o núcleo guarda", () => {
  for (const theme of ["tako", "escuro", "meia-noite", "oled", "vidro"])
    assert.equal(themeFromStored(themeToStored(theme)), theme);
  assert.equal(themeToStored("escuro"), "dark");
  assert.equal(themeToStored("vidro"), "glass");
});
test("chaves de sim ou não usam o padrão quando faltam ou estão quebradas", () => {
  assert.equal(readBool({}, "A", true), true);
  assert.equal(readBool({ A: "false" }, "A", true), false);
  assert.equal(readBool({ A: "talvez" }, "A", false), false);
});
test("idioma e 'Abrir em'", () => {
  assert.equal(readLanguage({}), "pt-BR");
  assert.equal(readLanguage({ Language: "en" }), "en-US");
  assert.equal(readLanguage({ Language: "EN_us" }), "en-US");
  assert.equal(readLanguage({ Language: "pt-PT" }), "pt-BR");
  assert.equal(readOpenIn({}), "desktop");
  assert.equal(readOpenIn({ OpenIn: "big-picture" }), "big-picture");
  assert.equal(readOpenIn({ OpenIn: "outro" }), "desktop");
});
test("DisabledStores liga e desliga a loja sem tocar nas outras", () => {
  assert.equal(withStoreEnabled(undefined, "epic", false), "Epic");
  assert.equal(withStoreEnabled("Epic", "battle_net", false), "Epic,BattleNet");
  assert.equal(withStoreEnabled("Epic,BattleNet", "epic", true), "BattleNet");
  assert.equal(withStoreEnabled("Ea,Amazon", "ea", true), "Amazon");
  assert.equal(withStoreEnabled("Epic", "epic", false), "Epic");
  assert.equal(withStoreEnabled("", "steam", true), "");
  // Nome escrito em outra caixa, espaços e nomes desconhecidos.
  assert.equal(withStoreEnabled(" epic , Futuro ", "epic", true), "Futuro");
  assert.equal(withStoreEnabled(" epic , Futuro ", "gog", false), "epic,Futuro,Gog");
});
test("isStoreDisabled ignora maiúsculas e espaços", () => {
  assert.equal(isStoreDisabled("Ea, amazon", "amazon"), true);
  assert.equal(isStoreDisabled("Ea,Amazon", "steam"), false);
  assert.equal(isStoreDisabled(undefined, "steam"), false);
  assert.equal(isStoreDisabled("BATTLENET", "battle_net"), true);
});
