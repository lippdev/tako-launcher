// O estado dos downloads que a barra, a lista e as Configurações compartilham (WEB-14b). O núcleo é a fonte da
// verdade (D-10): aqui só mora a última resposta de `downloads.list` e `downloads.summary`. Com os downloads
// desligados nada chama o motor: a leitura de `DownloadsEnabled` vem antes e decide.

import { useSyncExternalStore } from "react";
import { call, on } from "../bridge";
import type { DownloadEntry } from "../bridge/generated/DownloadEntry";
import type { DownloadsSummary } from "../bridge/generated/DownloadsSummary";
import { readBool } from "./settings/model";

export interface DownloadsState {
  /** `null` até a primeira leitura de `DownloadsEnabled`. */
  enabled: boolean | null;
  entries: readonly DownloadEntry[];
  summary: DownloadsSummary;
}

const EMPTY_SUMMARY: DownloadsSummary = { downloading: 0, queued: 0, speed: 0, done: 0, total: 0 };
const OFF: DownloadsState = { enabled: null, entries: [], summary: EMPTY_SUMMARY };

let state: DownloadsState = OFF;
let started = false;
let generation = 0;
const listeners = new Set<() => void>();

function publish(next: DownloadsState) {
  state = next;
  listeners.forEach((listener) => {
    listener();
  });
}

/** Lê tudo de novo. Desligado: não pergunta nada ao motor. Cada leitura invalida a anterior que ainda não voltou. */
export async function refreshDownloads(): Promise<void> {
  const current = ++generation;
  try {
    const { values } = await call("settings.get", { keys: ["DownloadsEnabled", "DownloadsLegalAccepted"] });
    if (current !== generation) return;
    // Como o núcleo: ligados só com a chave e o aviso legal aceito.
    if (!readBool(values, "DownloadsEnabled", false) || !readBool(values, "DownloadsLegalAccepted", false)) {
      publish({ ...OFF, enabled: false });
      return;
    }
    const [entries, summary] = await Promise.all([call("downloads.list"), call("downloads.summary")]);
    if (current !== generation) return;
    publish({ enabled: true, entries, summary });
  } catch (error) {
    if (current !== generation) return;
    // `conflict` = o núcleo ainda os tem desligados; qualquer outra falha deixa a última lista e só registra.
    const code = (error as { error?: { code?: string } }).error?.code;
    if (code === "conflict") publish({ ...OFF, enabled: false });
    else console.error("downloads: leitura falhou", error);
  }
}

function start() {
  if (started) return;
  started = true;
  on("downloads.changed", () => {
    void refreshDownloads();
  });
  void refreshDownloads();
}

function subscribe(listener: () => void): () => void {
  start();
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Os downloads de agora; a tela desenha de novo a cada `downloads.changed`. */
export function useDownloads(): DownloadsState {
  return useSyncExternalStore(subscribe, () => state);
}
