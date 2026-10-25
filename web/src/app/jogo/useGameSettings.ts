import { useCallback, useEffect, useRef, useState } from "react";
import { CallError, call } from "../../bridge";
import type { ErrorCode } from "../../bridge/generated/ErrorCode";
import type { GameSettingChange } from "../../bridge/generated/GameSettingChange";
import type { GameSettingsView } from "../../bridge/generated/GameSettingsView";

export type SaveResult = { ok: true } | { ok: false; code: ErrorCode };

/**
 * As configurações de um jogo, da ponte (`game.settings`). Lê quando `active` liga (cada vez que o painel abre) e
 * mantém o que já tinha enquanto lê de novo. As gravações (`game.settings.set`) saem uma de cada vez, na ordem em
 * que o usuário as fez, e cada resposta traz as configurações já atualizadas.
 */
export function useGameSettings(gameId: string, active: boolean) {
  const [view, setView] = useState<GameSettingsView | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  const queue = useRef<Promise<unknown>>(Promise.resolve());

  // Cada abertura começa sem a falha da anterior (ajuste de estado durante o desenho, sem quadro com o valor velho).
  const [wasActive, setWasActive] = useState(active);
  if (active !== wasActive) {
    setWasActive(active);
    if (active) setLoadFailed(false);
  }

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    // Atrás de qualquer gravação em andamento: a leitura não pode chegar antes e ser pisada por ela.
    void queue.current.then(() => call("game.settings", { gameId })).then(
      (next) => {
        if (cancelled) return;
        setView(next);
        setLoadFailed(false);
      },
      (error: unknown) => {
        console.error("game.settings", error);
        if (!cancelled) setLoadFailed(true);
      },
    );
    return () => {
      cancelled = true;
    };
  }, [gameId, active]);

  const save = useCallback(
    (change: GameSettingChange): Promise<SaveResult> => {
      const run = queue.current.then(
        async (): Promise<SaveResult> => {
          try {
            setView(await call("game.settings.set", { gameId, change }));
            return { ok: true };
          } catch (failure) {
            console.error("game.settings.set", failure);
            return { ok: false, code: failure instanceof CallError ? failure.error.code : "internal" };
          }
        },
      );
      queue.current = run;
      return run;
    },
    [gameId],
  );

  return { view, loadFailed, save };
}
