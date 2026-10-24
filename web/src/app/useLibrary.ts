import { useEffect, useState } from "react";
import type { GameRow } from "../components/ListaDeJogos/ListaDeJogos";

interface Library {
  rows: readonly GameRow[];
  /** URL do ícone; ausente = `tako-img://icon`, que o núcleo serve. */
  resolveIcon?: (id: string, px: number) => string | null;
}

const empty: Library = { rows: [] };

/**
 * Os jogos da lista da esquerda.
 *
 * TODO: trocar por `call("library.list")` e pelo evento `library.changed` quando a ponte tiver a biblioteca
 * (outra tarefa). Até lá, só em desenvolvimento, vêm os jogos de exemplo do hospedeiro falso; no build de
 * produção a lista fica vazia.
 */
export function useLibrary(): Library {
  const [library, setLibrary] = useState<Library>(empty);
  useEffect(() => {
    if (!import.meta.env.DEV) return;
    let cancelled = false;
    void import("../bridge/fake").then(({ fakeGames, fakeGameIcon }) => {
      if (!cancelled) setLibrary({ rows: fakeGames(), resolveIcon: fakeGameIcon });
    });
    return () => {
      cancelled = true;
    };
  }, []);
  return library;
}
