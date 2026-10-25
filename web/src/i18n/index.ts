// Textos e idiomas na interface (docs/web/ponte.md §8): `useT()` nos componentes, `t()` fora deles. O catálogo é lido
// do núcleo na primeira vez que alguém usa e de novo a cada `i18n.changed`.

import { useSyncExternalStore } from "react";
import { call, on } from "../bridge";
import { createI18n, type TFunction } from "./core";

export type { TFunction, Vars } from "./core";

const store = createI18n({
  fetchCatalog: () => call("i18n.catalog"),
  warn: import.meta.env.DEV ? (message) => void console.warn(message) : undefined,
});

let started = false;

/** Liga o carregador: pede o catálogo agora e de novo a cada `i18n.changed`. Chamar mais de uma vez não faz nada. */
export function startI18n(): void {
  if (started) return;
  started = true;
  on("i18n.changed", () => {
    void store.reload();
  });
  void store.reload();
}

store.subscribe(() => {
  const { lang } = store.getSnapshot();
  if (lang) document.documentElement.lang = lang;
});

function subscribe(listener: () => void): () => void {
  startI18n();
  return store.subscribe(listener);
}

/** O `t` do idioma atual, para usar fora de componentes (um handler, um aviso). Não re-renderiza ninguém. */
export const t: TFunction = (key, vars) => store.getSnapshot().t(key, vars);

/** O `t` do idioma atual; o componente desenha de novo quando o idioma muda. */
export function useT(): TFunction {
  return useSyncExternalStore(subscribe, () => store.getSnapshot().t);
}
