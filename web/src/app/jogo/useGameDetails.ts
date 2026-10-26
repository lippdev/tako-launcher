import { useCallback, useEffect, useState } from "react";
import { CallError, call, on } from "../../bridge";
import type { ErrorCode } from "../../bridge/generated/ErrorCode";
import type { GameDetails } from "../../bridge/generated/GameDetails";

interface Answer {
  /** De que jogo é esta resposta. */
  id: string;
  details: GameDetails | null;
  error: ErrorCode | null;
}

/**
 * Os detalhes de um jogo, da ponte (`game.details`). Lê de novo a cada `library.changed` (favorito, situação,
 * etiquetas, leitura das lojas) sem apagar o que já está na tela; uma falha de leitura guarda o código e mantém
 * os detalhes anteriores do mesmo jogo. De outro jogo, nunca devolve nada: quem chama espera `details` do jogo pedido.
 */
export function useGameDetails(gameId: string) {
  const [answer, setAnswer] = useState<Answer | null>(null);
  const [revision, setRevision] = useState(0);

  const refresh = useCallback(() => {
    setRevision((n) => n + 1);
  }, []);
  useEffect(() => on("library.changed", refresh), [refresh]);

  useEffect(() => {
    let cancelled = false;
    call("game.details", { gameId })
      .then((details) => {
        if (!cancelled) setAnswer({ id: gameId, details, error: null });
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        console.error("game.details", error);
        const code: ErrorCode = error instanceof CallError ? error.error.code : "internal";
        setAnswer((before) => ({ id: gameId, details: before?.id === gameId ? before.details : null, error: code }));
      });
    return () => {
      cancelled = true;
    };
  }, [gameId, revision]);

  const current = answer?.id === gameId ? answer : null;
  return { details: current?.details ?? null, error: current?.error ?? null, loading: current === null, refresh };
}
