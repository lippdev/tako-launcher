import assert from "node:assert/strict";
import { test } from "node:test";
import {
  ACCENT_CHOICES,
  GLASS_OPACITY_DEFAULT,
  accentOrDefault,
  glassAlpha,
  isLightAccent,
  lookVariables,
  presetOf,
} from "../appearance/desktopLook.ts";
import { DEFAULTS, KEYS, APPEARANCE_KEYS, appearanceFromSettings, parseAccent } from "./settings/model.ts";

const base = { theme: "vidro", accent: null, glassEffects: true, glassOpacity: GLASS_OPACITY_DEFAULT, glassPill: false, glassAmbient: true };

test("o padrão do deslizante é o mesmo no modelo e no vidro", () => {
  assert.equal(DEFAULTS.glassOpacity, GLASS_OPACITY_DEFAULT);
  // 28 → 7%, o branco 7% (#ffffff12) do Liquid Glass no tokens.css.
  assert.equal(glassAlpha(GLASS_OPACITY_DEFAULT), 7);
});

test("a cor de destaque só aceita #RRGGBB e é gravada em maiúsculas, como o Slint", () => {
  assert.equal(parseAccent("#ff8a3d"), "#FF8A3D");
  assert.equal(parseAccent("  #FF8A3D "), "#FF8A3D");
  for (const bad of [undefined, "", "FF8A3D", "#FF8A3", "#FF8A3DD", "#GG8A3D", "laranja", "#FF8A3D;x"])
    assert.equal(parseAccent(bad), null, String(bad));
});

test("as nove cores prontas são as do Slint e se reconhecem sem olhar a caixa", () => {
  assert.equal(ACCENT_CHOICES.length, 9);
  assert.equal(ACCENT_CHOICES[0].hex, "#D8C8BC");
  assert.equal(presetOf("#ff8a3d"), "orange");
  assert.equal(presetOf("#123456"), null);
  for (const choice of ACCENT_CHOICES) assert.equal(parseAccent(choice.hex), choice.hex);
});

test("sem escolha o destaque é a pedra no TAKO e o azul nos outros temas", () => {
  assert.equal(accentOrDefault(null, "tako"), "#D8C8BC");
  assert.equal(accentOrDefault(null, "oled"), "#4D9BFF");
  assert.equal(accentOrDefault("#FF8A3D", "escuro"), "#FF8A3D");
});

test("a opacidade do vidro vai de 2% a 20%, com a conta inteira do Slint", () => {
  assert.equal(glassAlpha(0), 2);
  assert.equal(glassAlpha(100), 20);
  assert.equal(glassAlpha(50), 11);
  assert.equal(glassAlpha(-5), 2);
  assert.equal(glassAlpha(250), 20);
});

test("texto sobre o destaque: escuro nas cores claras, claro nas outras (luma > 0,6)", () => {
  for (const id of ["stone", "orange", "gold", "white"]) assert.equal(isLightAccent(ACCENT_CHOICES.find((c) => c.id === id).hex), true, id);
  assert.equal(isLightAccent("#4D9BFF"), false);
  assert.equal(isLightAccent("#000000"), false);
});

test("o destaque escolhido vale fora do TAKO e nunca no TAKO", () => {
  const dark = lookVariables({ ...base, theme: "escuro", accent: "#FF8A3D" });
  assert.equal(dark.set["--accent"], "#FF8A3D");
  assert.equal(dark.set["--on-accent"], "var(--on-primary)");
  const blue = lookVariables({ ...base, theme: "oled", accent: "#4D9BFF" });
  assert.equal(blue.set["--on-accent"], "var(--text)");
  const tako = lookVariables({ ...base, theme: "tako", accent: "#FF8A3D" });
  assert.deepEqual(Object.keys(tako.set), []);
  assert.ok(tako.clear.includes("--accent") && tako.clear.includes("--on-accent"));
  // Sem escolha, quem manda é o tema.
  assert.ok(lookVariables({ ...base, theme: "escuro" }).clear.includes("--accent"));
});

test("as opções do vidro só mexem no Liquid Glass", () => {
  for (const theme of ["tako", "escuro", "meia-noite", "oled"]) {
    const { set } = lookVariables({ ...base, theme, glassOpacity: 90, glassPill: true, glassAmbient: false });
    assert.deepEqual(Object.keys(set), [], theme);
  }
});

test("opacidade: no padrão nada muda; fora dele troca só as superfícies do vidro, relativas ao padrão", () => {
  assert.deepEqual(Object.keys(lookVariables(base).set), []);
  const { set } = lookVariables({ ...base, glassOpacity: 100 });
  assert.equal(set["--glass-panel"], "rgb(255 255 255 / 20%)");
  assert.equal(set["--surface"], set["--glass-panel"]);
  assert.equal(set["--group"], set["--glass-panel"]);
  assert.equal(set["--tile"], "rgb(255 255 255 / 14.3%)");
  assert.equal(set["--glass-control"], "rgb(255 255 255 / 28.6%)");
  // Sem os efeitos de vidro o Liquid Glass é opaco: a opacidade não conta.
  assert.deepEqual(Object.keys(lookVariables({ ...base, glassOpacity: 100, glassEffects: false }).set), []);
});

test("pílula troca os raios dos controles; luz ambiente desligada põe a aurora na cor de destaque", () => {
  const pill = lookVariables({ ...base, glassPill: true }).set;
  assert.equal(pill["--radius-control"], "var(--radius-full)");
  assert.equal(pill["--radius-button"], "var(--radius-full)");
  const aurora = lookVariables({ ...base, glassAmbient: false }).set;
  assert.match(aurora["--ambient-1"], /var\(--accent\)/);
  assert.equal(lookVariables(base).set["--ambient-1"], undefined);
});

test("as chaves novas fazem parte da aparência lida na abertura e viram a aparência da página", () => {
  for (const key of [KEYS.accent, KEYS.glassOpacity, KEYS.glassPill, KEYS.glassAmbient]) assert.ok(APPEARANCE_KEYS.includes(key), key);
  const a = appearanceFromSettings({
    DesktopTheme: "glass",
    DesktopAccent: "#ff8a3d",
    DesktopGlassOpacity: "60",
    DesktopGlassPill: "false",
    DesktopGlassAmbient: "false",
  });
  assert.equal(a.accent, "#FF8A3D");
  assert.equal(a.glassOpacity, 60);
  assert.equal(a.glassPill, false);
  assert.equal(a.glassAmbient, false);
  // Valores quebrados caem nos padrões, campo a campo.
  const broken = appearanceFromSettings({ DesktopAccent: "vermelho", DesktopGlassOpacity: "250", DesktopGlassPill: "sim" });
  assert.equal(broken.accent, null);
  assert.equal(broken.glassOpacity, 28);
  assert.equal(broken.glassPill, true);
});
