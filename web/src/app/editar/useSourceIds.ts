import { useCallback, useEffect, useRef, useState } from "react";
import { CallError, call, on } from "../../bridge";
import type { GameSourceIds } from "../../bridge/generated/GameSourceIds";
import type { SourceIdChoice } from "../../bridge/generated/SourceIdChoice";
import type { SourceIdResults } from "../../bridge/generated/SourceIdResults";

/** A conferência de um id digitado: esperando a fonte, a fonte conhece (nome para confirmar) ou não conhece. */
export type Check =
  | { source: string; state: "checking" }
  | { source: string; state: "found"; id: string; name: string }
  | { source: string; state: "unknown" }
  /** O texto não tem cara de id daquela fonte; `message` é a explicação do núcleo, no idioma atual. */
  | { source: string; state: "invalid"; message: string };

export type SetResult = { ok: true; view: GameSourceIds } | { ok: false };

/**
 * "Fontes e IDs" de um jogo: as fontes ligadas e o id em cada uma (`game.source_ids`), a busca pelo nome em todas
 * ao mesmo tempo e a conferência de um id digitado. Busca e conferência demoram (rede): a chamada responde na hora e
 * o resultado vem por evento. Lê de novo cada vez que `active` liga.
 */
export function useSourceIds(gameId: string, active: boolean) {
  const [view, setView] = useState<GameSourceIds | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  const [results, setResults] = useState<readonly SourceIdResults[]>([]);
  const [term, setTerm] = useState<string | null>(null);
  const [searching, setSearching] = useState(false);
  const [searchFailed, setSearchFailed] = useState(false);
  const [check, setCheck] = useState<Check | null>(null);
  const searchRequest = useRef<number | null>(null);
  const checkRequest = useRef<number | null>(null);

  // Cada abertura começa do zero (ajuste de estado durante o desenho, sem quadro com a sobra da anterior).
  const [wasActive, setWasActive] = useState(active);
  if (active !== wasActive) {
    setWasActive(active);
    if (active) {
      setView(null);
      setLoadFailed(false);
      setResults([]);
      setTerm(null);
      setSearching(false);
      setSearchFailed(false);
      setCheck(null);
    }
  }

  const search = useCallback(
    (text: string) => {
      const wanted = text.trim();
      if (wanted === "") return;
      setTerm(wanted);
      setSearching(true);
      setSearchFailed(false);
      searchRequest.current = null;
      call("game.source_ids.search", { gameId, term: wanted }).then(
        (job) => {
          searchRequest.current = job.requestId;
        },
        (failure: unknown) => {
          console.error("game.source_ids.search", failure);
          setSearching(false);
          setSearchFailed(true);
        },
      );
    },
    [gameId],
  );

  // Cada abertura lê as fontes e já busca pelo nome do jogo.
  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    call("game.source_ids", { gameId }).then(
      (loaded) => {
        if (cancelled) return;
        setView(loaded);
        search(loaded.title);
      },
      (failure: unknown) => {
        console.error("game.source_ids", failure);
        if (!cancelled) setLoadFailed(true);
      },
    );
    return () => {
      cancelled = true;
      searchRequest.current = null;
      checkRequest.current = null;
    };
  }, [active, gameId, search]);

  useEffect(() => {
    if (!active) return;
    const stopFound = on("game.source_ids.found", (found) => {
      if (found.gameId !== gameId) return;
      if (searchRequest.current !== null && searchRequest.current !== found.requestId) return;
      searchRequest.current = null;
      setResults(found.results);
      setSearching(false);
    });
    const stopChecked = on("game.source_ids.checked", (checked) => {
      if (checked.gameId !== gameId) return;
      if (checkRequest.current !== null && checkRequest.current !== checked.requestId) return;
      checkRequest.current = null;
      setCheck(
        checked.found
          ? { source: checked.source, state: "found", id: checked.found.id, name: checked.found.name }
          : { source: checked.source, state: "unknown" },
      );
    });
    return () => {
      stopFound();
      stopChecked();
    };
  }, [active, gameId]);

  /** Confere o texto na fonte; o resultado (ou o motivo de não valer) aparece em `check`. */
  const verify = useCallback(
    (source: string, text: string) => {
      setCheck({ source, state: "checking" });
      checkRequest.current = null;
      call("game.source_ids.check", { gameId, source, text }).then(
        (job) => {
          checkRequest.current = job.requestId;
        },
        (failure: unknown) => {
          console.error("game.source_ids.check", failure);
          const message = failure instanceof CallError && failure.error.code === "bad_request" ? failure.error.message : "";
          setCheck({ source, state: "invalid", message });
        },
      );
    },
    [gameId],
  );

  /** Fixa o id da fonte, diz "nenhum" ou volta ao automático. */
  const choose = useCallback(
    async (source: string, choice: SourceIdChoice): Promise<SetResult> => {
      try {
        const next = await call("game.source_ids.set", { gameId, source, choice });
        setView(next);
        setCheck(null);
        return { ok: true, view: next };
      } catch (failure) {
        console.error("game.source_ids.set", failure);
        return { ok: false };
      }
    },
    [gameId],
  );

  return { view, loadFailed, results, term, searching, searchFailed, check, setCheck, search, verify, choose };
}
