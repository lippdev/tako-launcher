import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../bridge";
import { defaultScreenState, parseScreenState, serializeScreenState, type ScreenState } from "./uiState";

/** Espera depois da última mudança antes de guardar no núcleo. */
const SAVE_DELAY_MS = 300;

/**
 * O estado de tela que sobrevive (vista atual, lista recolhida). Lê com `ui.state` ao abrir e guarda com
 * `ui.state.set` quando muda, com debounce. `ready` só vira `true` depois da leitura (ou da falha dela), para a
 * tela não aparecer no padrão e pular para o estado guardado.
 */
export function useScreenState() {
  const [state, setState] = useState<ScreenState>(defaultScreenState);
  const [ready, setReady] = useState(false);
  // O último texto que o núcleo já tem: evita guardar de novo o que acabou de ser lido.
  const saved = useRef<string>(serializeScreenState(defaultScreenState));
  const pending = useRef<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    call("ui.state")
      .then((data) => {
        if (cancelled) return;
        const restored = parseScreenState(data.json);
        saved.current = serializeScreenState(restored);
        setState(restored);
      })
      .catch((error: unknown) => {
        console.error("ui.state: não deu para ler, vale o padrão", error);
      })
      .finally(() => {
        if (!cancelled) setReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const flush = useCallback(() => {
    const json = pending.current;
    pending.current = null;
    if (json === null || json === saved.current) return;
    saved.current = json;
    call("ui.state.set", { json }).catch((error: unknown) => {
      console.error("ui.state.set falhou", error);
    });
  }, []);

  useEffect(() => {
    if (!ready) return;
    pending.current = serializeScreenState(state);
    const timer = window.setTimeout(flush, SAVE_DELAY_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [ready, state, flush]);

  // Janela fechando antes do debounce: manda o que falta (a interface é descartável, o núcleo não).
  useEffect(() => {
    window.addEventListener("pagehide", flush);
    return () => {
      window.removeEventListener("pagehide", flush);
    };
  }, [flush]);

  const update = useCallback((patch: Partial<ScreenState>) => {
    setState((current) => ({ ...current, ...patch }));
  }, []);

  return { state, ready, update };
}
