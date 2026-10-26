import { useEffect, useState } from "react";
import { setAppearance } from "../../appearance/appearance";
import { call } from "../../bridge";
import { APPEARANCE_KEYS, appearanceFromSettings } from "./model";

/**
 * Lê a aparência salva (`settings.get`) ao abrir o app e aplica no `<html>`. Devolve `true` quando terminou, com
 * sucesso ou não (se a ponte falhar, vale a aparência padrão): a janela só aparece depois, para não piscar no tema
 * errado.
 */
export function useSavedAppearance(): boolean {
  const [ready, setReady] = useState(false);
  useEffect(() => {
    let cancelled = false;
    call("settings.get", { keys: [...APPEARANCE_KEYS] })
      .then((data) => {
        if (!cancelled) setAppearance(appearanceFromSettings(data.values));
      })
      .catch((error: unknown) => {
        console.error("settings.get: não deu para ler a aparência, vale a padrão", error);
      })
      .finally(() => {
        if (!cancelled) setReady(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);
  return ready;
}
