import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { call, on } from "../bridge";
import type { Collection } from "../bridge/generated/Collection";
import type { Tag } from "../bridge/generated/Tag";
import type { LibraryGame } from "../bridge/generated/LibraryGame";
import { percentByGame } from "./downloadsModel";
import { useDownloads } from "./downloadsStore";
import { libraryGameIcon, libraryGameToRow } from "./libraryRow";
import type { LibraryCriteria } from "./uiState";
import { libraryStrings } from "./strings";

export const PAGE_SIZE = 40;

/** Consulta só as páginas alcançadas; respostas de critérios antigos nunca substituem as atuais. */
export function useLibrary(criteria: LibraryCriteria) {
  // Só a busca espera (~200 ms); abas, filtros, ordenação e coleção valem na hora.
  const [search, setSearch] = useState(criteria.search);
  useEffect(() => {
    if (criteria.search === search) return;
    const timer = setTimeout(() => {
      setSearch(criteria.search);
    }, 200);
    return () => {
      clearTimeout(timer);
    };
  }, [criteria.search, search]);
  const request = useMemo(() => ({ ...criteria, search }), [criteria, search]);

  const [data, setData] = useState<{
    games: LibraryGame[];
    total: number;
    loading: boolean;
    error: boolean;
  }>({ games: [], total: 0, loading: true, error: false });
  const [collections, setCollections] = useState<Collection[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);
  const loaded = useRef(0);
  const busy = useRef(false);
  const [pending, setPending] = useState(false);
  const refresh = useCallback(() => {
    setRevision((n) => n + 1);
  }, []);

  useEffect(() => on("library.changed", refresh), [refresh]);
  useEffect(() => {
    let cancelled = false;
    void Promise.all([call("collections.list"), call("tags.list")])
      .then(([c, t]) => {
        if (!cancelled) {
          setCollections(c);
          setTags(t);
        }
      })
      .catch((error: unknown) => {
        console.error("biblioteca: opções", error);
        if (!cancelled) setData((d) => ({ ...d, error: true }));
      });
    return () => {
      cancelled = true;
    };
  }, [revision]);

  const previousRequest = useRef(request);
  useEffect(() => {
    const current = ++generation.current;
    const changed = previousRequest.current !== request;
    previousRequest.current = request;
    const count = changed ? PAGE_SIZE : Math.max(PAGE_SIZE, loaded.current);
    busy.current = true;
    // A lista permanece montada durante eventos; novos critérios começam na primeira página.
    void (async () => {
      setPending(true);
      if (changed) setData({ games: [], total: 0, loading: true, error: false });
      const games: LibraryGame[] = [];
      let total = 0;
      for (let offset = 0; offset < count; offset += PAGE_SIZE) {
        const page = await call("library.list", {
          ...request,
          offset,
          limit: PAGE_SIZE,
        });
        if (current !== generation.current) return;
        games.push(...page.rows);
        total = page.total;
        if (!page.rows.length || games.length >= total) break;
      }
      loaded.current = games.length;
      setData({ games, total, loading: false, error: false });
    })()
      .catch((error: unknown) => {
        if (current !== generation.current) return;
        console.error("library.list", error);
        setData((d) => ({ ...d, loading: false, error: true }));
      })
      .finally(() => {
        if (current === generation.current) {
          busy.current = false;
          setPending(false);
        }
      });
    return () => {
      generation.current = current + 1;
    };
  }, [request, revision]);

  const loadMore = useCallback(() => {
    if (busy.current || data.error || data.games.length >= data.total) return;
    busy.current = true;
    setPending(true);
    const current = generation.current;
    void call("library.list", {
      ...request,
      offset: data.games.length,
      limit: PAGE_SIZE,
    })
      .then((page) => {
        if (current !== generation.current) return;
        loaded.current = data.games.length + page.rows.length;
        setData((d) => ({
          ...d,
          games: [...d.games, ...page.rows],
          total: page.total,
        }));
      })
      .catch((error: unknown) => {
        console.error("library.list", error);
        if (current === generation.current) setData((d) => ({ ...d, error: true }));
      })
      .finally(() => {
        if (current === generation.current) {
          busy.current = false;
          setPending(false);
        }
      });
  }, [data, request]);

  const [devIcon, setDevIcon] = useState<((id: string, px: number) => string) | null>(null);
  useEffect(() => {
    if (import.meta.env.DEV && !window.location.protocol.startsWith("tako")) {
      void import("../bridge/fake").then(({ fakeGameIcon, isFakeHost }) => {
        if (isFakeHost()) setDevIcon(() => fakeGameIcon);
      });
    }
  }, []);
  // "Baixando · n%" na linha do jogo que tem um download em aberto (só existe com os downloads ligados).
  const { entries } = useDownloads();
  const downloading = useMemo(() => percentByGame(entries), [entries]);
  const rows = useMemo(
    () =>
      data.games.map((game) =>
        libraryGameToRow(game, libraryStrings.stores[game.store], downloading.get(game.id) ?? null),
      ),
    [data.games, downloading],
  );
  const resolveIcon = useCallback(
    (id: string, px: number) => {
      const game = data.games.find((g) => g.id === id);
      return game?.hasIcon ? (devIcon?.(id, px) ?? libraryGameIcon(game, px)) : null;
    },
    [data.games, devIcon],
  );
  return {
    ...data,
    rows,
    collections,
    tags,
    loadMore,
    refresh,
    resolveIcon,
    pending,
    queryKey: JSON.stringify(request),
  };
}
