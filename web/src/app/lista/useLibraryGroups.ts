import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { call, on } from "../../bridge";
import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import type { LibraryCriteria } from "../uiState";
import { ROW_PAGE, type GroupSpec } from "./listaModel";

/** Quantas fileiras pedem a primeira página ao mesmo tempo: biblioteca com muitas etiquetas não enche a ponte. */
const CONCURRENCY = 4;

export interface GroupData {
  games: LibraryGame[];
  total: number;
}

/** A tela pede cada fileira de uma vez: o núcleo filtra (`library.list` com o critério da fileira), a tela só junta. */
function fetchRow(
  criteria: LibraryCriteria,
  spec: GroupSpec,
  offset: number,
  limit: number,
): Promise<{ rows: LibraryGame[]; total: number }> {
  return call("library.list", {
    filter: criteria.filter,
    search: criteria.search,
    sort: criteria.sort,
    reversed: criteria.reversed,
    ...spec.request,
    offset,
    limit,
  });
}

/**
 * Os jogos de cada fileira das Categorias. Não lê a biblioteca inteira: cada fileira pede ao núcleo só as suas
 * primeiras `ROW_PAGE` capas (com o total, para a contagem) e a rolagem horizontal pede as seguintes (`loadMore`).
 * Respostas de pedidos antigos nunca substituem as atuais; `library.changed` lê de novo o que já estava na tela.
 */
export function useLibraryGroups(enabled: boolean, criteria: LibraryCriteria, specs: readonly GroupSpec[]) {
  const [groups, setGroups] = useState<ReadonlyMap<string, GroupData>>(new Map());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);
  const groupsRef = useRef(groups);
  groupsRef.current = groups;
  const busy = useRef(new Set<string>());
  const lastKey = useRef("");

  useEffect(() => on("library.changed", () => setRevision((n) => n + 1)), []);

  const key = useMemo(() => JSON.stringify({ criteria, specs }), [criteria, specs]);

  useEffect(() => {
    if (!enabled) return;
    const current = ++generation.current;
    const same = lastKey.current === key;
    lastKey.current = key;
    busy.current.clear();
    const previous = same ? groupsRef.current : new Map<string, GroupData>();
    const queue = [...specs];
    const worker = async () => {
      for (let spec = queue.shift(); spec; spec = queue.shift()) {
        // Mesmo pedido relido (evento): refaz até onde a rolagem já tinha chegado.
        const want = Math.max(ROW_PAGE, previous.get(spec.id)?.games.length ?? 0);
        const games: LibraryGame[] = [];
        let total = 0;
        while (games.length < want) {
          const page = await fetchRow(criteria, spec, games.length, Math.min(200, want - games.length));
          if (current !== generation.current) return;
          games.push(...page.rows);
          total = page.total;
          if (page.rows.length === 0 || games.length >= total) break;
        }
        setGroups((map) => new Map(map).set(spec.id, { games, total }));
      }
    };
    void (async () => {
      if (!same) {
        setGroups(new Map());
        setLoading(true);
      }
      setError(false);
      await Promise.all(Array.from({ length: Math.min(CONCURRENCY, specs.length) }, worker));
    })()
      .catch((failure: unknown) => {
        if (current !== generation.current) return;
        console.error("library.list (categorias)", failure);
        setError(true);
      })
      .finally(() => {
        if (current === generation.current) setLoading(false);
      });
    return () => {
      generation.current = current + 1;
    };
    // `key` já resume `criteria` e `specs`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, key, revision]);

  /** A rolagem da fileira chegou perto do fim: a página seguinte dela. */
  const loadMore = useCallback(
    (id: string) => {
      const data = groupsRef.current.get(id);
      const spec = specs.find((s) => s.id === id);
      if (!data || !spec || busy.current.has(id) || data.games.length >= data.total) return;
      busy.current.add(id);
      const current = generation.current;
      fetchRow(criteria, spec, data.games.length, ROW_PAGE)
        .then((page) => {
          if (current !== generation.current) return;
          setGroups((map) => {
            const row = map.get(id);
            return row ? new Map(map).set(id, { games: [...row.games, ...page.rows], total: page.total }) : map;
          });
        })
        .catch((failure: unknown) => {
          console.error("library.list (categorias)", failure);
          if (current === generation.current) setError(true);
        })
        .finally(() => {
          busy.current.delete(id);
        });
    },
    [criteria, specs],
  );

  return { groups, loading, error, loadMore };
}
