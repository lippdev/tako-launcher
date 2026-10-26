// A atualização do app que a barra, o menu da marca, o diálogo e as Configurações › Sobre compartilham (WEB-33). O
// núcleo é a fonte da verdade (D-10): aqui mora a última resposta de `updates.status`/`updates.changed` e o que é só
// de tela (diálogo aberto, aviso na tela, o "Você está em dia" que dura 4 s no menu, como no Slint).

import { useSyncExternalStore } from "react";
import { call, CallError, on } from "../../bridge";
import type { UpdateStatus } from "../../bridge/generated/UpdateStatus";
import { t } from "../../i18n";
import { transition, type Notice } from "./atualizacaoModel";

/** Quanto o menu da marca mostra "Você está em dia" / "Não deu para procurar" depois da procura (como no Slint). */
const TRANSIENT_MS = 4000;

export interface UpdatesUi {
  /** `null` até a primeira leitura. */
  status: UpdateStatus | null;
  dialogOpen: boolean;
  notice: (Notice & { key: number }) | null;
  transient: "current" | "failed" | null;
}

let state: UpdatesUi = { status: null, dialogOpen: false, notice: null, transient: null };
let explicit = false;
let started = false;
let noticeKey = 0;
let transientTimer: ReturnType<typeof setTimeout> | undefined;
const listeners = new Set<() => void>();

function publish(next: UpdatesUi) {
  state = next;
  listeners.forEach((listener) => {
    listener();
  });
}

function showNotice(notice: Notice): UpdatesUi["notice"] {
  noticeKey += 1;
  return { ...notice, key: noticeKey };
}

function setTransient(kind: UpdatesUi["transient"]) {
  clearTimeout(transientTimer);
  if (kind !== null)
    transientTimer = setTimeout(() => {
      publish({ ...state, transient: null });
    }, TRANSIENT_MS);
}

/** Um estado novo do núcleo (evento ou resposta): decide o diálogo e o aviso e guarda. */
function apply(next: UpdateStatus) {
  const wasExplicit = explicit;
  const outcome = transition(state.status, next, { explicit, dialogOpen: state.dialogOpen }, t);
  explicit = outcome.explicit;
  let transient = state.transient;
  if (wasExplicit && next.state === "current") transient = "current";
  else if (wasExplicit && next.state === "check_failed") transient = "failed";
  else if (next.state !== "current" && next.state !== "check_failed") transient = null;
  if (transient !== state.transient) setTransient(transient);
  publish({
    status: next,
    // Uma atualização que sumiu (ou nunca existiu) não deixa o diálogo aberto sem conteúdo.
    dialogOpen: outcome.dialogOpen && next.offer !== null,
    notice: outcome.notice ? showNotice(outcome.notice) : state.notice,
    transient,
  });
}

function start() {
  if (started) return;
  started = true;
  on("updates.changed", apply);
  call("updates.status")
    .then(apply)
    .catch((error: unknown) => {
      console.error("atualização: leitura falhou", error);
    });
}

function subscribe(listener: () => void): () => void {
  start();
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** A atualização de agora; a tela desenha de novo a cada `updates.changed`. */
export function useUpdates(): UpdatesUi {
  return useSyncExternalStore(subscribe, () => state);
}

/** Liga a leitura já na abertura: o aviso da procura automática chega mesmo antes de alguém usar o hook. */
export function startUpdates(): void {
  start();
}

function fail(text: string, error: unknown, what: string) {
  console.error(`atualização: ${what} falhou`, error);
  publish({ ...state, notice: showNotice({ tone: "error", text }) });
}

/** "Procurar atualizações" (menu da marca e Configurações): o resultado vira aviso ou abre o diálogo. */
export async function checkForUpdates(): Promise<void> {
  start();
  explicit = true;
  try {
    const reply = await call("updates.check");
    // Já havia algo para mostrar (baixando, pronta, falha ou só a página): a procura nova não refaz nada.
    if (reply.offer !== null && reply.state !== "searching") {
      explicit = false;
      publish({ ...state, status: reply, dialogOpen: true });
      return;
    }
    apply(reply);
  } catch (error) {
    explicit = false;
    fail(t("updates.desktop.failed_toast"), error, "procurar");
  }
}

export function openUpdateDialog(): void {
  publish({ ...state, dialogOpen: true });
}

export function closeUpdateDialog(): void {
  publish({ ...state, dialogOpen: false });
}

export function dismissUpdateNotice(): void {
  publish({ ...state, notice: null });
}

/** "Tentar de novo" depois de uma falha no download; sem versão guardada (a troca falhou antes), procura de novo. */
export async function retryUpdateDownload(): Promise<void> {
  if (state.status?.offer == null) return checkForUpdates();
  try {
    apply(await call("updates.download"));
  } catch (error) {
    fail(t("updates.desktop.download_failed"), error, "baixar");
  }
}

/** "Reiniciar agora": o núcleo troca o executável e sai. Com um jogo aberto, ele recusa e a tela avisa. */
export async function applyUpdate(): Promise<void> {
  try {
    await call("updates.apply");
  } catch (error) {
    const conflict = error instanceof CallError && error.error.code === "conflict";
    fail(
      conflict && state.status?.gameRunning === true
        ? t("updates.finish_game")
        : t("updates.apply_failed", { error: error instanceof CallError ? error.error.message : String(error) }),
      error,
      "reiniciar",
    );
    // Recusado por estado velho (o arquivo foi descartado, o jogo abriu): lê o que o núcleo diz agora.
    call("updates.status")
      .then(apply)
      .catch((reason: unknown) => {
        console.error("atualização: leitura falhou", reason);
      });
  }
}
