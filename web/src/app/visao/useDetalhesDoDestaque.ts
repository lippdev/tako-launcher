import { useEffect, useState } from "react";
import { call, on } from "../../bridge";
import type { GameDetails } from "../../bridge/generated/GameDetails";

interface Entry {
  /** Em que leitura da biblioteca estes detalhes foram pedidos. */
  revision: number;
  /** `null`: o núcleo não respondeu; o destaque fica só com o que a lista já sabe. */
  details: GameDetails | null;
}

/**
 * Os detalhes (descrição, nota) do jogo que o destaque vai mostrar, da ponte (`game.details`). Cada jogo é lido uma
 * vez; em `library.changed` o que está em foco é lido de novo sem apagar o que já está na tela. Devolve a tabela
 * por ID: quem chama só troca o destaque depois que o jogo novo tem entrada (com ou sem detalhes).
 */
export function useDetalhesDoDestaque(gameId: string | null): Readonly<Record<string, Entry>> {
  const [entries, setEntries] = useState<Readonly<Record<string, Entry>>>({});
  const [revision, setRevision] = useState(0);
  useEffect(
    () =>
      on("library.changed", () => {
        setRevision((n) => n + 1);
      }),
    [],
  );

  const stale = gameId !== null && entries[gameId]?.revision !== revision;
  useEffect(() => {
    if (gameId === null || !stale) return;
    let cancelled = false;
    call("game.details", { gameId })
      .then((details) => {
        if (!cancelled) setEntries((before) => ({ ...before, [gameId]: { revision, details } }));
      })
      .catch((error: unknown) => {
        console.error("game.details", error);
        if (!cancelled)
          setEntries((before) => ({ ...before, [gameId]: { revision, details: before[gameId]?.details ?? null } }));
      });
    return () => {
      cancelled = true;
    };
  }, [gameId, revision, stale]);

  return entries;
}
