import assert from "node:assert/strict";
import { test } from "node:test";
import {
  CLOSING_PATIENCE_MS,
  LAUNCH_SLOW_MS,
  PENDING_GRACE_MS,
  elapsedMinutes,
  errorCode,
  errorTexts,
  initialSession,
  isClosingDialogOpen,
  isClosingStuck,
  isLaunchSlow,
  needsPolling,
  noticeFor,
  phaseOf,
  playedText,
  reduceSession,
  welcomeText,
} from "./sessao/sessionModel.ts";

const T0 = 1_000_000;

const running = (patch = {}) => ({
  gameId: "steam:1",
  title: "Elden Ring",
  startedAt: new Date(T0).toISOString(),
  state: "playing",
  ...patch,
});
const ended = (patch = {}) => ({
  gameId: "steam:1",
  title: "Elden Ring",
  playedSeconds: 2520,
  end: "normal",
  exitCode: null,
  exitCodeText: null,
  message: null,
  ...patch,
});
const started = (patch = {}) => ({
  gameId: "steam:1",
  title: "Elden Ring",
  startedAt: new Date(T0).toISOString(),
  tracked: true,
  notices: [],
  ...patch,
});
const run = (state, ...events) => events.reduce(reduceSession, state);

/** Um `t()` que mostra a chave e os parâmetros, para o teste ver o que foi pedido. */
const t = (key, vars) => (vars ? `${key}(${Object.entries(vars).map(([k, v]) => `${k}=${v}`).join(",")})` : key);

test("Jogar abre já no clique, e uma falha desfaz", () => {
  const opening = run(initialSession, { type: "play", gameId: "steam:1", title: "Elden Ring", now: T0 });
  assert.equal(opening.running?.phase, "launching");
  assert.equal(phaseOf(opening, "steam:1"), "launching");
  assert.equal(phaseOf(opening, "steam:2"), "none");
  assert.equal(run(opening, { type: "play-failed", gameId: "steam:1" }).running, null);
  // Falha de outro jogo não mexe no que está abrindo.
  assert.equal(run(opening, { type: "play-failed", gameId: "steam:2" }).running?.gameId, "steam:1");
});

test("logo depois do clique, game.running vazio é ignorado; depois do prazo, vale", () => {
  const opening = run(initialSession, { type: "play", gameId: "steam:1", title: "Elden Ring", now: T0 });
  assert.equal(run(opening, { type: "running", running: null, now: T0 + 500 }).running?.phase, "launching");
  assert.equal(run(opening, { type: "running", running: null, now: T0 + PENDING_GRACE_MS }).running, null);
});

test("game.running traz o estado do núcleo e game.started marca abrindo", () => {
  const playing = run(initialSession, { type: "running", running: running(), now: T0 });
  assert.equal(playing.running?.phase, "playing");
  assert.equal(playing.running?.pendingSince, null);
  const opening = run(initialSession, { type: "started", started: started(), now: T0 });
  assert.equal(opening.running?.phase, "launching");
  assert.equal(opening.running?.startedAt, new Date(T0).toISOString());
  // A mesma fase não recomeça a contagem de quem espera nela.
  const again = run(playing, { type: "running", running: running(), now: T0 + 60_000 });
  assert.equal(again.running?.phaseSince, T0);
});

test("jogo aberto sem acompanhamento não vira sessão, e os avisos da abertura ficam guardados", () => {
  const optimistic = run(initialSession, { type: "play", gameId: "steam:1", title: "Elden Ring", now: T0 });
  const untracked = run(optimistic, {
    type: "started",
    started: started({ tracked: false, notices: ["Não abriu OBS"] }),
    now: T0 + 100,
  });
  assert.equal(untracked.running, null);
  assert.deepEqual(untracked.warnings, ["Não abriu OBS"]);
  assert.deepEqual(run(untracked, { type: "dismiss-warnings" }).warnings, []);
});

test("o fim normal é o bem-vindo de volta e limpa a sessão", () => {
  const playing = run(initialSession, { type: "running", running: running(), now: T0 });
  const after = run(playing, { type: "ended", ended: ended() });
  assert.equal(after.running, null);
  assert.deepEqual(after.notice, {
    kind: "welcome",
    id: 1,
    gameId: "steam:1",
    title: "Elden Ring",
    playedSeconds: 2520,
  });
  assert.equal(run(after, { type: "dismiss-notice" }).notice, null);
});

test("interface reaberta depois do jogo: game.running vazio e o game.ended não visto que vem em seguida", () => {
  // O núcleo responde `null` e manda o último fim logo depois: o aviso aparece.
  const opened = run(initialSession, { type: "running", running: null, now: T0 }, { type: "ended", ended: ended() });
  assert.equal(opened.running, null);
  assert.equal(opened.notice?.kind, "welcome");
  // Com erro, o diálogo.
  const failed = run(
    initialSession,
    { type: "running", running: null, now: T0 },
    { type: "ended", ended: ended({ end: "failed", exitCode: 3221225477, exitCodeText: "Código de saída 0xC0000005" }) },
  );
  assert.equal(failed.notice?.kind, "error");
});

test("só o fim normal é bem-vindo: quick, never_started, failed e launch_failed avisam como erro", () => {
  assert.equal(noticeFor(ended(), 1).kind, "welcome");
  for (const end of ["quick", "never_started", "failed", "launch_failed"]) {
    const notice = noticeFor(ended({ end, exitCode: end === "failed" ? 5 : null }), 7);
    assert.equal(notice.kind, "error", end);
    assert.equal(notice.end, end);
    assert.equal(notice.id, 7);
  }
});

test("um fim de outro jogo não derruba o que está aberto", () => {
  const playing = run(initialSession, { type: "running", running: running({ gameId: "steam:2" }), now: T0 });
  const after = run(playing, { type: "ended", ended: ended({ gameId: "steam:1", end: "launch_failed" }) });
  assert.equal(after.running?.gameId, "steam:2");
  assert.equal(after.notice?.kind, "error");
});

test("encerrar: fechando, a paciência de 10 s e o conflict que oferece forçar na hora", () => {
  const playing = run(initialSession, { type: "running", running: running(), now: T0 });
  const closing = run(playing, { type: "end-requested", now: T0 + 1000 });
  assert.equal(closing.running?.phase, "closing");
  assert.equal(isClosingDialogOpen(closing), true);
  assert.equal(isClosingStuck(closing, T0 + 1000 + CLOSING_PATIENCE_MS - 1), false);
  assert.equal(isClosingStuck(closing, T0 + 1000 + CLOSING_PATIENCE_MS), true);
  // O núcleo recusou (a janela do jogo ainda não existe): forçar já.
  const refused = run(closing, { type: "end-conflict" });
  assert.equal(isClosingStuck(refused, T0 + 1001), true);
  // Forçar volta a esperar o fim.
  const forced = run(refused, { type: "force-requested", now: T0 + 2000 });
  assert.equal(isClosingStuck(forced, T0 + 2001), false);
  // Dispensar o diálogo não muda a fase; pedir de novo o traz de volta.
  const dismissed = run(closing, { type: "dismiss-closing" });
  assert.equal(isClosingDialogOpen(dismissed), false);
  assert.equal(dismissed.running?.phase, "closing");
  assert.equal(isClosingDialogOpen(run(dismissed, { type: "end-requested", now: T0 + 3000 })), true);
});

test("a leitura do núcleo confirma 'fechando' sem apagar a oferta de forçar nem recomeçar a espera", () => {
  const closing = run(
    initialSession,
    { type: "running", running: running(), now: T0 },
    { type: "end-requested", now: T0 + 1000 },
    { type: "end-conflict" },
  );
  const read = run(closing, { type: "running", running: running({ state: "closing" }), now: T0 + 3000 });
  assert.equal(read.forceOffered, true);
  assert.equal(read.running?.phaseSince, T0 + 1000);
  // O jogo voltou a jogar (o usuário voltou a ele e respondeu): sem diálogo nem oferta.
  const back = run(read, { type: "running", running: running(), now: T0 + 4000 });
  assert.equal(back.forceOffered, false);
  assert.equal(back.closingDismissed, false);
});

test("'Ainda abrindo…' só depois de 20 s e só abrindo", () => {
  const opening = run(initialSession, { type: "started", started: started(), now: T0 });
  assert.equal(isLaunchSlow(opening.running, T0 + LAUNCH_SLOW_MS - 1), false);
  assert.equal(isLaunchSlow(opening.running, T0 + LAUNCH_SLOW_MS), true);
  const playing = run(opening, { type: "running", running: running(), now: T0 + 1 });
  assert.equal(isLaunchSlow(playing.running, T0 + LAUNCH_SLOW_MS * 5), false);
  assert.equal(isLaunchSlow(null, T0), false);
});

test("só abrindo e fechando precisam perguntar ao núcleo", () => {
  assert.equal(needsPolling(initialSession), false);
  assert.equal(needsPolling(run(initialSession, { type: "started", started: started(), now: T0 })), true);
  assert.equal(needsPolling(run(initialSession, { type: "running", running: running(), now: T0 })), false);
  assert.equal(needsPolling(run(initialSession, { type: "running", running: running({ state: "closing" }), now: T0 })), true);
});

test("a sessão atual conta em minutos inteiros e nunca menos de 1", () => {
  const start = new Date(T0).toISOString();
  assert.equal(elapsedMinutes(start, T0), 1);
  assert.equal(elapsedMinutes(start, T0 - 5000), 1);
  assert.equal(elapsedMinutes(start, T0 + 42 * 60_000 + 59_000), 42);
  assert.equal(elapsedMinutes("não é data", T0), null);
});

test("o tempo jogado usa os textos do núcleo, com plural e horas", () => {
  assert.equal(playedText(t, 30), "presentation.session.less_than_minute");
  assert.equal(playedText(t, 60), "presentation.session.minutes(n=1)");
  assert.equal(playedText(t, 42 * 60 + 10), "presentation.session.minutes(n=42)");
  assert.equal(playedText(t, 2 * 3600), "presentation.session.hours(h=2)");
  assert.equal(playedText(t, 3600 + 5 * 60), "presentation.session.hours_minutes(h=1,m=05)");
});

test("bem-vindo de volta leva o tempo, e sem tempo fica só o jogo", () => {
  const welcome = noticeFor(ended({ playedSeconds: 2520 }), 1);
  assert.equal(
    welcomeText(t, welcome),
    "session.welcome_back_played(game=Elden Ring,time=presentation.session.minutes(n=42))",
  );
  assert.equal(welcomeText(t, noticeFor(ended({ playedSeconds: 0 }), 1)), "session.welcome_back(game=Elden Ring)");
});

test("o erro do jogo põe o código de saída que veio antes da sugestão", () => {
  const failed = noticeFor(
    ended({
      end: "failed",
      playedSeconds: 600,
      exitCode: 0xc0000005,
      exitCodeText: "Código de saída 0xC0000005 (3221225477): memória",
    }),
    1,
  );
  const texts = errorTexts(t, failed);
  assert.equal(texts.title, "session.ended_error(game=Elden Ring)");
  assert.equal(texts.message, "Código de saída 0xC0000005 (3221225477): memória.\n\nsession.ended_error_text");
  assert.equal(texts.exitCode, 0xc0000005);
  assert.equal(texts.subtitle, "session.closed_after(time=presentation.session.minutes(n=10))");

  const quick = errorTexts(t, noticeFor(ended({ end: "quick", playedSeconds: 3 }), 1));
  assert.equal(quick.title, "session.ended_quick(game=Elden Ring)");
  assert.equal(quick.message, "session.ended_quick_text");
  assert.equal(quick.exitCode, null);

  const never = errorTexts(t, noticeFor(ended({ end: "never_started", playedSeconds: 0 }), 1));
  assert.equal(never.title, "session.never_started(game=Elden Ring)");
  assert.equal(never.subtitle, null);
});

test("não abriu por um motivo que o núcleo escreveu: o motivo no lugar da sugestão, sem tempo", () => {
  const texts = errorTexts(
    t,
    noticeFor(ended({ end: "launch_failed", playedSeconds: 0, message: "O comando antes do jogo falhou" }), 1),
  );
  assert.equal(texts.message, "O comando antes do jogo falhou");
  assert.equal(texts.subtitle, null);
  assert.equal(texts.title, "session.never_started(game=Elden Ring)");
});

test("errorCode lê o código do erro da ponte e nada mais", () => {
  assert.equal(errorCode({ error: { code: "conflict", message: "x" } }), "conflict");
  assert.equal(errorCode(new Error("x")), null);
  assert.equal(errorCode(null), null);
  assert.equal(errorCode({ error: "conflict" }), null);
});
