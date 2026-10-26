import { useEffect, useRef, useState } from "react";
import { call, on } from "../../bridge";
import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import { GAME_LIMIT } from "./buscaModel";

/** Espera depois da última tecla antes de perguntar à ponte. */
const DEBOUNCE_MS = 150;

interface BuscaState {
  games: LibraryGame[];
  /** Há uma consulta em andamento ou esperando o debounce. */
  loading: boolean;
  error: boolean;
}

interface Answer {
  /** A consulta que esta resposta atende; `null` = ainda nenhuma desde que a busca abriu. */
  key: string | null;
  games: LibraryGame[];
  error: boolean;
}

/**
 * Os jogos que combinam com o texto, só enquanto `active`: uma página pequena de `library.list` sem os filtros da
 * lista (a busca geral é da biblioteca inteira). Respostas de textos antigos nunca substituem as atuais. A primeira
 * consulta (texto vazio, ao abrir) vai na hora; as seguintes esperam 150 ms.
 */
export function useBusca(active: boolean, query: string): BuscaState {
  const [answer, setAnswer] = useState<Answer>({ key: null, games: [], error: false });
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);
  // O texto já consultado desde que abriu: só o primeiro vai sem espera.
  const asked = useRef<string | null>(null);
  const key = `${query.trim()}#${String(revision)}`;

  // Cada abertura esquece a resposta da anterior (o painel não mostra jogos de uma busca velha).
  const [wasActive, setWasActive] = useState(active);
  if (active !== wasActive) {
    setWasActive(active);
    if (active) setAnswer({ key: null, games: [], error: false });
  }

  useEffect(() => {
    if (!active) {
      asked.current = null;
      return;
    }
    return on("library.changed", () => {
      setRevision((n) => n + 1);
    });
  }, [active]);

  useEffect(() => {
    if (!active) return;
    const current = ++generation.current;
    const immediate = asked.current === null;
    asked.current = query;
    const timer = setTimeout(
      () => {
        call("library.list", { search: query.trim(), offset: 0, limit: GAME_LIMIT })
          .then((page) => {
            if (current === generation.current) setAnswer({ key, games: page.rows, error: false });
          })
          .catch((error: unknown) => {
            console.error("busca geral: library.list", error);
            if (current === generation.current) setAnswer({ key, games: [], error: true });
          });
      },
      immediate ? 0 : DEBOUNCE_MS,
    );
    return () => {
      clearTimeout(timer);
      // Um texto novo (ou fechar) invalida a consulta em voo.
      generation.current = current + 1;
    };
  }, [active, query, key]);

  return { games: answer.games, loading: active && answer.key !== key, error: answer.error && answer.key === key };
}
