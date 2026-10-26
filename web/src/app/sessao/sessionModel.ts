// A regra da sessão de jogo na interface (WEB-21), sem tela nem ponte: o estado a partir das respostas de
// `game.running` e dos eventos `game.started` e `game.ended`, o que o usuário pede (Jogar, Encerrar, forçar) e quando
// um fim vira "bem-vindo de volta" ou o diálogo de erro (SESSION-003). O núcleo é a fonte da verdade (D-10): aqui
// só mora a última leitura dele. Sem importações em execução, para o teste rodar direto no Node.

import type { GameEnded } from "../../bridge/generated/GameEnded";
import type { GameStarted } from "../../bridge/generated/GameStarted";
import type { RunningGame } from "../../bridge/generated/RunningGame";
import type { RunningState } from "../../bridge/generated/RunningState";

/** Depois disso o botão vira "Ainda abrindo…" e aparece o cancelar (`LAUNCH_SLOW` do Slint). */
export const LAUNCH_SLOW_MS = 20_000;
/** Quanto esperar o jogo fechar sozinho antes de oferecer forçar (`PATIENCE` do "Fechando o jogo…" do Slint). */
export const CLOSING_PATIENCE_MS = 10_000;
/**
 * Depois de `game.play` o núcleo ainda abre o jogo (`HostRequest::Launch`), então um `game.running` que volta `null`
 * logo em seguida não quer dizer que o jogo não abriu: nesse prazo a resposta vazia é ignorada.
 */
export const PENDING_GRACE_MS = 4_000;

/** O jogo que está aberto (ou abrindo, ou fechando). */
export interface RunningSession {
  gameId: string;
  title: string;
  /** ISO-8601; no "abrindo" otimista, o instante do clique. */
  startedAt: string;
  phase: RunningState;
  /** Quando esta fase começou aqui (ms): o "fechando" conta a paciência a partir dele. */
  phaseSince: number;
  /** O clique foi daqui e o núcleo ainda não confirmou (ms do clique); `null` quando ele já respondeu. */
  pendingSince: number | null;
}

/** O que mostrar quando um jogo terminou. */
export type EndNotice =
  | { kind: "welcome"; id: number; gameId: string; title: string; playedSeconds: number }
  | {
      kind: "error";
      id: number;
      gameId: string;
      title: string;
      playedSeconds: number;
      end: "quick" | "never_started" | "failed" | "launch_failed";
      exitCode: number | null;
      exitCodeText: string | null;
      message: string | null;
    };

export interface SessionState {
  running: RunningSession | null;
  /** O núcleo respondeu `conflict` ao pedido de fechar (a janela do jogo ainda não existe): oferecer forçar já. */
  forceOffered: boolean;
  /** O usuário fechou o diálogo "Fechando o jogo…" sem esperar. */
  closingDismissed: boolean;
  notice: EndNotice | null;
  /** Avisos da abertura (`game.started`), já no idioma do usuário. */
  warnings: readonly string[];
  /** Conta os avisos e os fins para dar um `id` a cada um (o aviso novo reinicia a contagem de quem o mostra). */
  counter: number;
}

export const initialSession: SessionState = {
  running: null,
  forceOffered: false,
  closingDismissed: false,
  notice: null,
  warnings: [],
  counter: 0,
};

export type SessionEvent =
  | { type: "play"; gameId: string; title: string; now: number }
  | { type: "play-failed"; gameId: string }
  | { type: "running"; running: RunningGame | null; now: number }
  | { type: "started"; started: GameStarted; now: number }
  | { type: "ended"; ended: GameEnded }
  | { type: "end-requested"; now: number }
  | { type: "force-requested"; now: number }
  | { type: "end-conflict" }
  | { type: "dismiss-closing" }
  | { type: "dismiss-notice" }
  | { type: "dismiss-warnings" };

/** O estado a seguir. Função pura: o relógio entra no evento. */
export function reduceSession(state: SessionState, event: SessionEvent): SessionState {
  switch (event.type) {
    case "play":
      // Abrindo já no clique, sem esperar o `game.started`: o botão não pode voltar a "Jogar" no meio do caminho.
      return {
        ...state,
        running: {
          gameId: event.gameId,
          title: event.title,
          startedAt: new Date(event.now).toISOString(),
          phase: "launching",
          phaseSince: event.now,
          pendingSince: event.now,
        },
        forceOffered: false,
        closingDismissed: false,
      };
    case "play-failed":
      return state.running?.gameId === event.gameId && state.running.pendingSince !== null
        ? { ...state, running: null }
        : state;
    case "running": {
      const before = state.running;
      if (event.running === null) {
        // Logo depois do clique o núcleo pode ainda não ter registrado o jogo.
        if (before?.pendingSince != null && event.now - before.pendingSince < PENDING_GRACE_MS) return state;
        return { ...state, running: null, forceOffered: false, closingDismissed: false };
      }
      const same = before?.gameId === event.running.gameId && before.phase === event.running.state;
      const keepClosing = same && event.running.state === "closing";
      return {
        ...state,
        running: {
          gameId: event.running.gameId,
          title: event.running.title,
          startedAt: event.running.startedAt,
          phase: event.running.state,
          phaseSince: same && before ? before.phaseSince : event.now,
          pendingSince: null,
        },
        forceOffered: keepClosing && state.forceOffered,
        closingDismissed: keepClosing && state.closingDismissed,
      };
    }
    case "started": {
      const { started } = event;
      const warnings = started.notices.length > 0 ? started.notices : state.warnings;
      // Só abriu, sem acompanhar (loja por URI): não há sessão para mostrar.
      if (!started.tracked) {
        const mine = state.running?.gameId === started.gameId;
        return { ...state, running: mine ? null : state.running, warnings, counter: state.counter + 1 };
      }
      return {
        ...state,
        running: {
          gameId: started.gameId,
          title: started.title,
          startedAt: started.startedAt,
          phase: "launching",
          phaseSince: event.now,
          pendingSince: null,
        },
        forceOffered: false,
        closingDismissed: false,
        warnings,
        counter: state.counter + 1,
      };
    }
    case "ended": {
      const { ended } = event;
      const mine = state.running === null || state.running.gameId === ended.gameId;
      return {
        ...state,
        running: mine ? null : state.running,
        forceOffered: mine ? false : state.forceOffered,
        closingDismissed: mine ? false : state.closingDismissed,
        notice: noticeFor(ended, state.counter + 1),
        counter: state.counter + 1,
      };
    }
    case "end-requested":
      return state.running
        ? {
            ...state,
            running: { ...state.running, phase: "closing", phaseSince: event.now },
            forceOffered: false,
            closingDismissed: false,
          }
        : state;
    case "force-requested":
      // Volta a esperar o fim (pelo acompanhamento), como o "Forçar encerramento" do Slint.
      return state.running
        ? { ...state, running: { ...state.running, phase: "closing", phaseSince: event.now }, forceOffered: false }
        : state;
    case "end-conflict":
      return state.running ? { ...state, forceOffered: true } : state;
    case "dismiss-closing":
      return { ...state, closingDismissed: true };
    case "dismiss-notice":
      return { ...state, notice: null };
    case "dismiss-warnings":
      return { ...state, warnings: [] };
  }
}

/**
 * Um fim normal é o "bem-vindo de volta". Fechou logo ao abrir, com erro, nem abriu ou não pôde abrir: o diálogo de
 * erro (SESSION-003). Como o Slint, que só deixa passar o `normal`.
 */
export function noticeFor(ended: GameEnded, id: number): EndNotice {
  if (ended.end === "normal") {
    return { kind: "welcome", id, gameId: ended.gameId, title: ended.title, playedSeconds: ended.playedSeconds };
  }
  return {
    kind: "error",
    id,
    gameId: ended.gameId,
    title: ended.title,
    playedSeconds: ended.playedSeconds,
    end: ended.end,
    exitCode: ended.exitCode,
    exitCodeText: ended.exitCodeText,
    message: ended.message,
  };
}

/** Em que ponto está um jogo da biblioteca: `none` quando ele não é o que está aberto. */
export type GamePhase = "none" | RunningState;

export function phaseOf(state: SessionState, gameId: string): GamePhase {
  return state.running?.gameId === gameId ? state.running.phase : "none";
}

/** Abrindo há tempo demais: o botão vira "Ainda abrindo…" e aparece o cancelar. */
export function isLaunchSlow(running: RunningSession | null, now: number): boolean {
  if (running?.phase !== "launching") return false;
  const since = Date.parse(running.startedAt);
  return Number.isFinite(since) && now - since >= LAUNCH_SLOW_MS;
}

/** O jogo não fechou: oferecer forçar (o núcleo recusou o pedido ou a paciência acabou). */
export function isClosingStuck(state: SessionState, now: number): boolean {
  const running = state.running;
  if (running?.phase !== "closing") return false;
  return state.forceOffered || now - running.phaseSince >= CLOSING_PATIENCE_MS;
}

/** O diálogo "Fechando o jogo…" fica na tela enquanto fecha, a menos que o usuário o tenha dispensado. */
export function isClosingDialogOpen(state: SessionState): boolean {
  return state.running?.phase === "closing" && !state.closingDismissed;
}

/** Os estados em que vale perguntar ao núcleo de tempos em tempos (o fim por si só chega por evento). */
export function needsPolling(state: SessionState): boolean {
  const phase = state.running?.phase;
  return phase === "launching" || phase === "closing";
}

/** "Sessão atual", em minutos inteiros e nunca menos de 1 (como o Slint): `42 min`, `1 h 20 min`. */
export function elapsedMinutes(startedAt: string, now: number): number | null {
  const since = Date.parse(startedAt);
  if (!Number.isFinite(since)) return null;
  return Math.max(1, Math.floor(Math.max(0, now - since) / 60_000));
}

export type Translate = (key: string, vars?: Record<string, string | number>) => string;

/** O tempo da sessão como o núcleo escreve (`presentation.session.*`): "42 min jogados", "1 h 20 min jogados". */
export function playedText(t: Translate, seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  if (minutes < 1) return t("presentation.session.less_than_minute");
  if (minutes < 60) return t("presentation.session.minutes", { n: minutes });
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (rest === 0) return t("presentation.session.hours", { h: hours });
  return t("presentation.session.hours_minutes", { h: hours, m: String(rest).padStart(2, "0") });
}

/** "Bem-vindo de volta · Jogo · 42 min jogados"; sem tempo (zero), sem a última parte. */
export function welcomeText(t: Translate, notice: Extract<EndNotice, { kind: "welcome" }>): string {
  if (notice.playedSeconds <= 0) return t("session.welcome_back", { game: notice.title });
  return t("session.welcome_back_played", { game: notice.title, time: playedText(t, notice.playedSeconds) });
}

/** Os textos do diálogo de erro: o título, e a causa com o código explicado antes, quando há. */
export interface ErrorTexts {
  title: string;
  /** Duas partes separadas por linha em branco: o código de saída e a sugestão. */
  message: string;
  /** Quanto durou, para o subtítulo; `null` quando o jogo nem abriu. */
  subtitle: string | null;
  /** O código de saída como número, para o "Detalhes técnicos". */
  exitCode: number | null;
}

export function errorTexts(t: Translate, notice: Extract<EndNotice, { kind: "error" }>): ErrorTexts {
  const game = notice.title;
  const [title, text] =
    notice.end === "failed"
      ? ([t("session.ended_error", { game }), t("session.ended_error_text")])
      : notice.end === "quick"
        ? ([t("session.ended_quick", { game }), t("session.ended_quick_text")])
        : ([t("session.never_started", { game }), t("session.never_started_text")]);
  // Não abriu por um motivo que o núcleo já escreveu: o motivo vale mais que a sugestão genérica.
  const cause = notice.end === "launch_failed" && notice.message ? notice.message : text;
  const message = notice.exitCodeText ? `${notice.exitCodeText}.\n\n${cause}` : cause;
  return {
    title,
    message,
    subtitle:
      notice.playedSeconds > 0 && notice.end !== "launch_failed"
        ? t("session.closed_after", { time: playedText(t, notice.playedSeconds) })
        : null,
    exitCode: notice.exitCode,
  };
}

/** Lê o `code` de um erro da ponte sem importar o cliente (o `CallError` guarda o `BridgeError` em `error`). */
export function errorCode(failure: unknown): string | null {
  if (typeof failure !== "object" || failure === null || !("error" in failure)) return null;
  const inner = failure.error;
  if (typeof inner !== "object" || inner === null || !("code" in inner)) return null;
  const code = inner.code;
  return typeof code === "string" ? code : null;
}
