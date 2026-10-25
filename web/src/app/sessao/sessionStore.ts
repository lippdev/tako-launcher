// O estado da sessão de jogo que a página do jogo, a Visão geral, a lista e as camadas compartilham (WEB-21). A
// ponte entra por `SessionBridge`, para o teste usar uma de mentira; a ligada à ponte de verdade está em
// `useGameSession.ts`. Sem importações em execução, para o teste rodar direto no Node.

import type { GameEnded } from "../../bridge/generated/GameEnded";
import type { GameStarted } from "../../bridge/generated/GameStarted";
import type { RunningGame } from "../../bridge/generated/RunningGame";
import { errorCode, initialSession, needsPolling, reduceSession, type SessionEvent, type SessionState } from "./sessionModel.ts";

export interface SessionBridge {
  running(): Promise<RunningGame | null>;
  play(gameId: string): Promise<void>;
  resume(): Promise<void>;
  end(force: boolean): Promise<void>;
  /** Escuta `game.started`; devolve a função que para. */
  onStarted(handler: (started: GameStarted) => void): () => void;
  onEnded(handler: (ended: GameEnded) => void): () => void;
}

export interface SessionStoreOptions {
  bridge: SessionBridge;
  now?: () => number;
  /** De quanto em quanto perguntar `game.running` enquanto abre ou fecha; 0 desliga (os testes). */
  pollMs?: number;
}

/** O que `play` conta a quem pediu para abrir. */
export type PlayResult = "ok" | "conflict" | "error";
/** O que `end` conta a quem pediu para encerrar: `conflict` = a janela do jogo ainda não existe, ofereça forçar. */
export type EndResult = "ok" | "conflict" | "error";

export interface SessionStore {
  getState(): SessionState;
  subscribe(listener: () => void): () => void;
  /** Liga os ouvintes e lê `game.running`. Nesta ordem: o fim não visto chega logo depois da resposta. */
  start(): void;
  stop(): void;
  play(gameId: string, title: string): Promise<PlayResult>;
  resume(): Promise<void>;
  end(force?: boolean): Promise<EndResult>;
  dismissClosing(): void;
  dismissNotice(): void;
  dismissWarnings(): void;
}

export function createSessionStore({ bridge, now = Date.now, pollMs = 2000 }: SessionStoreOptions): SessionStore {
  let state: SessionState = initialSession;
  const listeners = new Set<() => void>();
  let started = false;
  let stops: (() => void)[] = [];
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** Sobe a cada evento do núcleo: uma leitura que partiu antes de um evento não vale mais quando volta. */
  let version = 0;
  let generation = 0;

  const dispatch = (event: SessionEvent) => {
    const next = reduceSession(state, event);
    if (next === state) return;
    state = next;
    listeners.forEach((listener) => {
      listener();
    });
    schedulePoll();
  };

  const read = async () => {
    const asked = version;
    const current = generation;
    try {
      const running = await bridge.running();
      // Parou, ou um evento chegou no meio: a resposta é de antes.
      if (current !== generation || asked !== version) return;
      dispatch({ type: "running", running, now: now() });
    } catch (failure) {
      console.error("game.running", failure);
    }
  };

  function schedulePoll() {
    if (timer !== undefined || !started || pollMs <= 0 || !needsPolling(state)) return;
    timer = setTimeout(() => {
      timer = undefined;
      void read().finally(schedulePoll);
    }, pollMs);
  }

  return {
    getState: () => state,
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    start() {
      if (started) return;
      started = true;
      generation += 1;
      stops = [
        bridge.onStarted((event) => {
          version += 1;
          dispatch({ type: "started", started: event, now: now() });
        }),
        bridge.onEnded((event) => {
          version += 1;
          dispatch({ type: "ended", ended: event });
        }),
      ];
      void read();
    },
    stop() {
      started = false;
      generation += 1;
      stops.forEach((stop) => {
        stop();
      });
      stops = [];
      clearTimeout(timer);
      timer = undefined;
    },
    async play(gameId, title) {
      // Abrindo já no clique. Com outro jogo aberto não mexe no que se mostra: o núcleo vai recusar (`conflict`).
      const optimistic = state.running === null;
      if (optimistic) dispatch({ type: "play", gameId, title, now: now() });
      try {
        await bridge.play(gameId);
        return "ok";
      } catch (failure) {
        if (optimistic) dispatch({ type: "play-failed", gameId });
        if (errorCode(failure) === "conflict") {
          // Se havia outro jogo aberto, a tela vai dizer qual: relê para saber.
          void read();
          return "conflict";
        }
        console.error("game.play", failure);
        return "error";
      }
    },
    async resume() {
      try {
        await bridge.resume();
      } catch (failure) {
        // `conflict`: nenhum jogo aberto (acabou agora): a leitura acerta a tela.
        if (errorCode(failure) !== "conflict") console.error("game.resume", failure);
        void read();
      }
    },
    async end(force = false) {
      dispatch(force ? { type: "force-requested", now: now() } : { type: "end-requested", now: now() });
      try {
        await bridge.end(force);
        return "ok";
      } catch (failure) {
        void read();
        if (errorCode(failure) === "conflict") {
          dispatch({ type: "end-conflict" });
          return "conflict";
        }
        console.error("game.end", failure);
        return "error";
      }
    },
    dismissClosing: () => {
      dispatch({ type: "dismiss-closing" });
    },
    dismissNotice: () => {
      dispatch({ type: "dismiss-notice" });
    },
    dismissWarnings: () => {
      dispatch({ type: "dismiss-warnings" });
    },
  };
}
