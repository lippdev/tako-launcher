import { useCallback, useEffect, useRef, useState } from "react";
import { call } from "../../bridge";
import type { SettingsMap } from "./model";

/** Grava uma chave. Devolve `true` se o núcleo aceitou; se falhou, a tela já voltou ao valor de antes. */
export type SaveSetting = (key: string, value: string) => Promise<boolean>;

/**
 * As configurações da tela, lidas com `settings.get` ao abrir e gravadas na hora com `settings.set` (sem botão
 * "Salvar"). A tela muda antes da resposta; se a ponte falhar, volta ao valor anterior e `failures` sobe, para a
 * tela mostrar o erro. `ready` vira `true` depois da leitura, ou da falha dela (valem os padrões).
 */
export function useSettings(keys: readonly string[]) {
  const [values, setValues] = useState<SettingsMap>({});
  const [ready, setReady] = useState(false);
  const [loadFailed, setLoadFailed] = useState(false);
  const [failures, setFailures] = useState(0);
  // O valor mais novo, para a volta de uma gravação que falhou não pisar numa mudança que veio depois.
  const latest = useRef<SettingsMap>({});

  useEffect(() => {
    let cancelled = false;
    call("settings.get", { keys: [...keys] })
      .then((data) => {
        if (cancelled) return;
        latest.current = { ...data.values, ...latest.current };
        setValues(latest.current);
      })
      .catch((error: unknown) => {
        console.error("settings.get falhou, valem os padrões", error);
        if (!cancelled) setLoadFailed(true);
      })
      .finally(() => {
        if (!cancelled) setReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, [keys]);

  const save = useCallback<SaveSetting>(async (key, value) => {
    const previous = latest.current[key];
    latest.current = { ...latest.current, [key]: value };
    setValues(latest.current);
    try {
      await call("settings.set", { key, value });
      return true;
    } catch (error) {
      console.error(`settings.set ${key} falhou`, error);
      if (latest.current[key] === value) {
        const reverted = { ...latest.current };
        if (previous === undefined) delete reverted[key];
        else reverted[key] = previous;
        latest.current = reverted;
        setValues(latest.current);
      }
      setFailures((n) => n + 1);
      return false;
    }
  }, []);

  /** O valor mais novo, mesmo dentro de uma função que já foi criada. */
  const read = useCallback(() => latest.current, []);

  return { values, ready, loadFailed, failures, save, read };
}
