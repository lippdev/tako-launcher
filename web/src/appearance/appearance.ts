// Opções de aparência e acessibilidade aplicadas no `<html>` (README do design system, "Temas" e
// "Acessibilidade"). Aqui mora só o efeito na página; de onde o valor vem (as configurações do núcleo)
// é assunto das telas de configuração.

import { useSyncExternalStore } from "react";

export const THEMES = ["tako", "escuro", "meia-noite", "oled", "vidro"] as const;
export type Theme = (typeof THEMES)[number];

export const UI_SCALES = [90, 100, 110, 125] as const;
export type UiScale = (typeof UI_SCALES)[number];

export interface Appearance {
  theme: Theme;
  /** "Reduzir animações". */
  reduceMotion: boolean;
  /** "Efeitos de vidro": desligado = sem `backdrop-filter`. */
  glassEffects: boolean;
  /** "Tamanho da interface", em porcentagem. */
  uiScale: UiScale;
}

export const defaultAppearance: Appearance = {
  theme: "tako",
  reduceMotion: false,
  glassEffects: true,
  uiScale: 100,
};

/**
 * O valor de `data-theme`. Sem efeitos de vidro o Liquid Glass vira `solido` (superfícies opacas, como o
 * `tokens.css` define); os outros temas já são opacos e ficam como estão.
 */
export function effectiveTheme({ theme, glassEffects }: Pick<Appearance, "theme" | "glassEffects">): string {
  return theme === "vidro" && !glassEffects ? "solido" : theme;
}

/** Escreve a aparência nos atributos e na variável de escala do `<html>`. */
export function applyAppearance(a: Appearance, root: HTMLElement = document.documentElement): void {
  root.dataset["theme"] = effectiveTheme(a);
  root.dataset["reduceMotion"] = String(a.reduceMotion);
  root.dataset["glass"] = a.glassEffects ? "on" : "off";
  root.style.setProperty("--ui-scale", String(a.uiScale / 100));
}

let current = defaultAppearance;
const subscribers = new Set<() => void>();

export function setAppearance(patch: Partial<Appearance>): void {
  current = { ...current, ...patch };
  applyAppearance(current);
  subscribers.forEach((notify) => notify());
}

export function useAppearance(): Appearance {
  return useSyncExternalStore(
    (notify) => {
      subscribers.add(notify);
      return () => subscribers.delete(notify);
    },
    () => current,
  );
}
