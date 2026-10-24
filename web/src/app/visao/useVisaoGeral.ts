import { useCallback, useEffect, useRef, useState } from "react";
import { call, on } from "../../bridge";
import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import { RECENT_ID, SHELF_PAGE, appendPage, hasMore, shelfSpecs, type ShelfState } from "./visaoModel";

interface Overview {
  status: "loading" | "ready" | "error";
  /** Quantos jogos a biblioteca tem (sem filtro). */
  total: number;
  /** O jogo jogado por último; sem nenhum jogado, o primeiro da biblioteca. */
  first: LibraryGame | null;
  shelves: ShelfState[];
}

/**
 * Os dados da Visão geral, da ponte: uma página pequena (`SHELF_PAGE`) por fileira, e as seguintes só quando a rolagem
 * da fileira chega perto do fim. Em `library.changed` relê as fileiras sem apagar o que está na tela; respostas de
 * uma leitura antiga nunca substituem as atuais.
 */
export function useVisaoGeral() {
  const [overview, setOverview] = useState<Overview>({ status: "loading", total: 0, first: null, shelves: [] });
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);
  const busy = useRef(new Set<string>());
  const shelves = useRef<ShelfState[]>([]);

  const refresh = useCallback(() => {
    setRevision((n) => n + 1);
  }, []);
  useEffect(() => on("library.changed", refresh), [refresh]);

  useEffect(() => {
    const current = ++generation.current;
    busy.current.clear();
    void (async () => {
      const specs = shelfSpecs(await call("collections.list"));
      const pages = await Promise.all(
        specs.map((spec) => call("library.list", { ...spec.request, offset: 0, limit: SHELF_PAGE })),
      );
      if (current !== generation.current) return;
      const recent = specs.findIndex((spec) => spec.id === RECENT_ID);
      const next = specs.map((spec, i) => appendPage(null, spec, pages[i] ?? { rows: [], total: 0 }, 0));
      shelves.current = next;
      setOverview({
        status: "ready",
        total: pages[recent]?.total ?? 0,
        first: pages[recent]?.rows[0] ?? null,
        shelves: next,
      });
    })().catch((error: unknown) => {
      if (current !== generation.current) return;
      console.error("visão geral", error);
      // Falha numa releitura mantém o que já estava na tela; só a primeira vira erro.
      setOverview((before) => (before.status === "ready" ? before : { ...before, status: "error" }));
    });
    return () => {
      generation.current = current + 1;
    };
  }, [revision]);

  /** A fileira chegou perto do fim: pede a página seguinte dela (uma por vez). */
  const loadMore = useCallback((id: string) => {
    const state = shelves.current.find((shelf) => shelf.spec.id === id);
    if (!state || !hasMore(state) || busy.current.has(id)) return;
    busy.current.add(id);
    const current = generation.current;
    void call("library.list", { ...state.spec.request, offset: state.fetched, limit: SHELF_PAGE })
      .then((page) => {
        if (current !== generation.current) return;
        shelves.current = shelves.current.map((shelf) =>
          shelf.spec.id === id ? appendPage(shelf, shelf.spec, page, shelf.fetched) : shelf,
        );
        setOverview((before) => ({ ...before, shelves: shelves.current }));
      })
      .catch((error: unknown) => {
        console.error("library.list", error);
      })
      .finally(() => {
        busy.current.delete(id);
      });
  }, []);

  const retry = useCallback(() => {
    setOverview((before) => ({ ...before, status: "loading" }));
    refresh();
  }, [refresh]);

  return { ...overview, loadMore, retry };
}
