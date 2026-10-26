// Regras puras das coleções na lista da biblioteca (WEB-11b): validar o nome e montar a regra da coleção automática a
// partir dos filtros da tela. Sem importações em execução, para o teste rodar direto no Node. O núcleo repete as
// validações (`tako_core::library::collections`); aqui elas só evitam a ida e volta e dão o erro no campo.

import type { Collection } from "../../bridge/generated/Collection";
import type { SmartCriteria } from "../../bridge/generated/SmartCriteria";
import type { LibraryCriteria } from "../uiState";

/** Limite de letras do nome (`MAX_NAME_LENGTH` do núcleo). */
export const MAX_NAME_LENGTH = 40;

export type NameProblem = "empty" | "too_long" | "duplicate";

/** Nome limpo: espaços nas pontas e repetidos saem (como `clean_name` do núcleo). */
export function cleanName(name: string): string {
  return name
    .split(" ")
    .map((word) => word.trim())
    .filter((word) => word !== "")
    .join(" ");
}

/** Mesma coleção para o núcleo: sem caixa nem acento (`title_eq`). */
function nameKey(name: string): string {
  return name.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase();
}

/** O nome repete o de outra coleção do mesmo tipo (manual ou automática)? `exceptId` é a coleção renomeada. */
export function nameProblem(
  name: string,
  existing: readonly Pick<Collection, "id" | "name">[],
  exceptId: string | null = null,
): NameProblem | null {
  const clean = cleanName(name);
  if (clean === "") return "empty";
  if ([...clean].length > MAX_NAME_LENGTH) return "too_long";
  const key = nameKey(clean);
  return existing.some((c) => c.id !== exceptId && nameKey(c.name) === key) ? "duplicate" : null;
}

/** As coleções do mesmo tipo de `auto`: o núcleo guarda manuais e automáticas em listas separadas. */
export function ofKind(collections: readonly Collection[], auto: boolean): Collection[] {
  return collections.filter((c) => (c.criteria !== null) === auto);
}

const DEFAULT_SORT: LibraryCriteria["sort"] = "recently_played";

/** Há algo a salvar: busca, aba, filtros ou ordenação diferente do padrão (como o Slint, que só mostra o botão assim). */
export function hasSavableCriteria(c: LibraryCriteria): boolean {
  return (
    c.search.trim() !== "" ||
    c.filter !== "all" ||
    c.collection !== null ||
    c.stores.length > 0 ||
    c.statuses.length > 0 ||
    c.tags.length > 0 ||
    c.noStatus ||
    c.installed !== null ||
    c.sort !== DEFAULT_SORT ||
    c.reversed
  );
}

/**
 * A regra da coleção automática para o que a lista mostra agora. O núcleo não aceita uma coleção automática dentro de
 * outra: com uma aberta (`open`), a regra dela é a base e os filtros da tela valem por cima, campo a campo (o que a
 * tela não mexeu continua como estava). Sem coleção automática aberta, a regra sai só dos filtros; uma coleção manual
 * escolhida vira parte da regra.
 */
export function smartRuleFromCriteria(c: LibraryCriteria, open: Collection | null = null): SmartCriteria {
  const base = open?.criteria ?? null;
  const baseLibrary = base?.library;
  const search = c.search.trim();
  const sortUntouched = c.sort === DEFAULT_SORT && !c.reversed;
  const ownStatus = c.statuses.length > 0 || c.noStatus;
  const filter = c.filter !== "all" ? c.filter : (baseLibrary?.filter ?? "all");
  const collection = open ? (baseLibrary?.collection ?? null) : c.collection;
  return {
    library: {
      filter,
      search: search || (baseLibrary?.search ?? ""),
      sort: sortUntouched && baseLibrary?.sort ? baseLibrary.sort : c.sort,
      reversed: sortUntouched ? (baseLibrary?.reversed ?? false) : c.reversed,
      collection,
      tags: c.tags.length ? c.tags : (baseLibrary?.tags ?? []),
      statuses: ownStatus ? c.statuses : (baseLibrary?.statuses ?? []),
      noStatus: ownStatus ? c.noStatus : (baseLibrary?.noStatus ?? false),
      stores: c.stores.length ? c.stores : (baseLibrary?.stores ?? []),
      installed: c.installed ?? baseLibrary?.installed ?? null,
    },
    genres: base?.genres ?? [],
    playTime: base?.playTime ?? "any",
    playedOnly: base?.playedOnly ?? false,
    brokenOnly: base?.brokenOnly ?? false,
    favoritesOnly: base?.favoritesOnly ?? false,
    collections: collection ? [collection] : [],
    // O núcleo recusa filtro de loja fora da aba Todos.
    filterStore: filter === "all" ? (base?.filterStore ?? null) : null,
  };
}
