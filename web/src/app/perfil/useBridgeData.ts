import { useCallback, useEffect, useRef, useState } from "react";
import { on, type EventName } from "../../bridge";

export interface BridgeData<T> {
  /** O que o núcleo respondeu por último; fica na tela enquanto uma releitura roda. */
  data: T | null;
  status: "loading" | "ready" | "error";
  /** Lê de novo (o botão "Tentar novamente", ou depois de uma mudança que o núcleo não avisa). */
  reload: () => void;
}

/** Eventos do núcleo depois dos quais os números da biblioteca (jogo, sessão, coleção) podem ter mudado. */
export const LIBRARY_EVENTS: readonly EventName[] = ["library.changed"];

/**
 * Lê uma consulta da ponte e a refaz quando `load` muda ou quando um dos `events` chega. Uma resposta de leitura
 * antiga nunca vale por cima da atual, e a falha de uma releitura mantém o que já estava na tela (só a primeira
 * leitura vira erro). `load` deve ser estável (`useCallback`): mudar de identidade é pedir outra consulta.
 */
export function useBridgeData<T>(load: () => Promise<T>, events: readonly EventName[] = LIBRARY_EVENTS): BridgeData<T> {
  const [state, setState] = useState<{ data: T | null; status: BridgeData<T>["status"] }>({
    data: null,
    status: "loading",
  });
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);

  const reload = useCallback(() => {
    setRevision((n) => n + 1);
  }, []);

  useEffect(() => {
    const offs = events.map((name) => on(name, reload));
    return () => {
      offs.forEach((off) => {
        off();
      });
    };
  }, [events, reload]);

  useEffect(() => {
    const current = ++generation.current;
    load().then(
      (data) => {
        if (current === generation.current) setState({ data, status: "ready" });
      },
      (error: unknown) => {
        if (current !== generation.current) return;
        console.error("perfil", error);
        setState((before) => (before.data === null ? { data: null, status: "error" } : before));
      },
    );
    return () => {
      generation.current = current + 1;
    };
  }, [load, revision]);

  return { ...state, reload };
}
