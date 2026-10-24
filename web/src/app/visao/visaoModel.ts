// Regras da Visão geral que não dependem de tela: quais fileiras existem, que consulta cada uma faz e como as
// páginas se juntam. Só tipos nas importações, para o teste rodar direto no Node.

import type { Collection } from "../../bridge/generated/Collection";
import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import type { LibraryListRequest } from "../../bridge/generated/LibraryListRequest";
import type { LibraryPage } from "../../bridge/generated/LibraryPage";

/** Cada fileira pede uma página pequena; as seguintes só vêm quando a rolagem chega perto do fim. */
export const SHELF_PAGE = 20;

export type ShelfKind = "continue" | "recent" | "collection";

export interface ShelfSpec {
  id: string;
  kind: ShelfKind;
  /** Nome da coleção; as fileiras fixas têm o título nos textos da tela. */
  name: string | null;
  request: LibraryListRequest;
  /** Só entram jogos que já foram jogados (o núcleo ordena por última partida, mas não filtra por ela). */
  playedOnly: boolean;
}

export interface ShelfState {
  spec: ShelfSpec;
  games: LibraryGame[];
  /** Quantos jogos do núcleo já foram lidos (inclui os que `playedOnly` deixou de fora). */
  fetched: number;
  total: number;
}

export const CONTINUE_ID = "continue";
export const RECENT_ID = "recent";

/** Coleção manual sem jogo não tem o que mostrar: nem consulta. As automáticas se resolvem no núcleo. */
function isEmptyManual(collection: Collection): boolean {
  return collection.criteria === null && collection.gameIds.length === 0;
}

/** As fileiras da Visão geral, na ordem da tela: continuar jogando, recentes e uma por coleção do usuário. */
export function shelfSpecs(collections: readonly Collection[]): ShelfSpec[] {
  return [
    {
      id: CONTINUE_ID,
      kind: "continue",
      name: null,
      request: { installed: true, sort: "recently_played" },
      playedOnly: true,
    },
    { id: RECENT_ID, kind: "recent", name: null, request: { sort: "recently_played" }, playedOnly: true },
    ...collections
      .filter((collection) => !isEmptyManual(collection))
      .map(
        (collection): ShelfSpec => ({
          id: `collection:${collection.id}`,
          kind: "collection",
          name: collection.name,
          request: { collection: collection.id, sort: "recently_played" },
          playedOnly: false,
        }),
      ),
  ];
}

/** Junta uma página à fileira (a primeira, com `state` nulo, ou a seguinte). Jogo repetido entra uma vez só. */
export function appendPage(state: ShelfState | null, spec: ShelfSpec, page: LibraryPage, offset: number): ShelfState {
  const rows = spec.playedOnly ? page.rows.filter((game) => game.lastPlayedAt !== null) : page.rows;
  const before = state?.games ?? [];
  const seen = new Set(before.map((game) => game.id));
  const games = [...before, ...rows.filter((game) => !seen.has(game.id))];
  // Ordenado por última partida, o primeiro nunca jogado encerra os jogados: o resto da biblioteca não interessa.
  const played = !spec.playedOnly || rows.length === page.rows.length;
  return { spec, games, fetched: played ? offset + page.rows.length : page.total, total: page.total };
}

export function hasMore(state: ShelfState): boolean {
  return state.fetched < state.total;
}

/** Fileira sem jogos não aparece. */
export function visibleShelves(shelves: readonly ShelfState[]): ShelfState[] {
  return shelves.filter((shelf) => shelf.games.length > 0);
}

/** Os jogos que a tela conhece por ID: o destaque troca para qualquer um deles. */
export function gamesById(shelves: readonly ShelfState[], first: LibraryGame | null): Map<string, LibraryGame> {
  const found = new Map<string, LibraryGame>();
  if (first) found.set(first.id, first);
  for (const shelf of shelves) for (const game of shelf.games) found.set(game.id, game);
  return found;
}
