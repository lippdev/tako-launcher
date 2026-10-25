import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { call, on } from "../../bridge";
import type { AvisoTone } from "../../components/Aviso/Aviso";
import { useT } from "../../i18n";
import { formatDuration } from "../jogo/jogoModel";
import { gameStrings } from "../strings";
import {
  elapsedMinutes,
  isClosingDialogOpen,
  isClosingStuck,
  isLaunchSlow,
  phaseOf,
  type GamePhase,
  type SessionState,
} from "./sessionModel";
import { createSessionStore, type EndResult } from "./sessionStore";

/** A sessão da interface inteira: um só estado, ligado à ponte (`game.running`, `game.started`, `game.ended`). */
const store = createSessionStore({
  bridge: {
    running: () => call("game.running"),
    play: async (gameId) => {
      await call("game.play", { gameId });
    },
    resume: async () => {
      await call("game.resume");
    },
    end: async (force) => {
      await call("game.end", { force });
    },
    onStarted: (handler) => on("game.started", handler),
    onEnded: (handler) => on("game.ended", handler),
  },
});

/** O primeiro a escutar liga os ouvintes e lê `game.running` (o fim não visto chega logo depois da resposta). */
function subscribe(listener: () => void): () => void {
  store.start();
  return store.subscribe(listener);
}

function useSessionState(): SessionState {
  return useSyncExternalStore(subscribe, () => store.getState());
}

/** Só o jogo aberto: quem apenas pinta um indicador (a lista) não desenha de novo a cada passo da sessão. */
export function useRunningGameId(): string | null {
  return useSyncExternalStore(subscribe, () => store.getState().running?.gameId ?? null);
}

/**
 * Relógio da sessão: a cada segundo enquanto abre ou fecha (os prazos de "Ainda abrindo…" e de forçar), a cada meio
 * minuto jogando ("Jogando · 42 min"); parado sem jogo. O instante de cada fase entra no estado, então o relógio nunca
 * fica antes do começo dela.
 */
function useClock(state: SessionState): number {
  const [tick, setTick] = useState(0);
  const phase = state.running?.phase;
  const interval = phase === undefined ? null : phase === "playing" ? 30_000 : 1_000;
  useEffect(() => {
    if (interval === null) return;
    const timer = setInterval(() => {
      setTick(Date.now());
    }, interval);
    return () => {
      clearInterval(timer);
    };
  }, [interval]);
  return Math.max(tick, state.running?.phaseSince ?? 0);
}

export interface PlayFeedback {
  tone: AvisoTone;
  text: string;
}

/**
 * O que o usuário pode pedir, sem ler o estado: não desenha de novo a cada passo da sessão (a lista só precisa disto
 * e do `useRunningGameId`).
 */
export function useSessionActions() {
  const t = useT();
  return useMemo(
    () => ({
      /** Pede para abrir; devolve o aviso que a tela mostra (abrindo, outro jogo aberto, não pode abrir, falha). */
      async play(game: { id: string; title: string }): Promise<PlayFeedback> {
        const other = store.getState().running;
        const result = await store.play(game.id, game.title);
        if (result === "ok") return { tone: "tako", text: gameStrings.notices.opening(game.title) };
        if (result === "conflict" && other && other.gameId !== game.id) {
          return { tone: "error", text: t("session.other_running", { game: other.title }) };
        }
        return { tone: "error", text: result === "conflict" ? gameStrings.notices.cannotPlay : gameStrings.notices.playError };
      },
      /** Volta para a janela do jogo (a interface fecha e o jogo fica na frente). */
      resume: () => store.resume(),
      /** Encerra (ou força); `conflict` = a janela do jogo ainda não existe e o núcleo oferece forçar. */
      end: (force = false): Promise<EndResult> => store.end(force),
      dismissClosing: () => {
        store.dismissClosing();
      },
      dismissNotice: () => {
        store.dismissNotice();
      },
      dismissWarnings: () => {
        store.dismissWarnings();
      },
    }),
    [t],
  );
}

/**
 * A sessão de jogo para a tela: em que ponto está cada jogo e o que o usuário pode pedir (Jogar, Voltar ao jogo,
 * Encerrar). A página do jogo, a Visão geral, a lista e as camadas usam este mesmo lugar.
 */
export function useGameSession() {
  const state = useSessionState();
  const now = useClock(state);
  const actions = useSessionActions();
  return useMemo(() => {
    const { running } = state;
    const minutes = running && running.phase !== "launching" ? elapsedMinutes(running.startedAt, now) : null;
    return {
      ...actions,
      state,
      running,
      /** Em que ponto está este jogo; `none` se ele não é o que está aberto. */
      phaseOf: (gameId: string): GamePhase => phaseOf(state, gameId),
      /** Abrindo há tempo demais: "Ainda abrindo…" e o cancelar. */
      launchSlow: isLaunchSlow(running, now),
      /** Fechando e o jogo não saiu: oferecer forçar. */
      closingStuck: isClosingStuck(state, now),
      closingDialogOpen: isClosingDialogOpen(state),
      /** "42 min", "1 h 20 min": o tempo da sessão em curso; `null` enquanto abre. */
      elapsed: minutes === null ? null : formatDuration(minutes * 60),
    };
  }, [state, now, actions]);
}
