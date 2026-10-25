import assert from "node:assert/strict";
import { test } from "node:test";
import { dialogState, dialogView, formatMb, indicator, menuLine, transition } from "./atualizacao/atualizacaoModel.ts";

const MB = 1024 ** 2;

/** Um `t()` mínimo: devolve a chave e os parâmetros, para o teste ver o que foi pedido. */
const t = (key, vars) => (vars ? `${key}(${Object.entries(vars).map(([k, v]) => `${k}=${String(v)}`).join(",")})` : key);

const OFFER = {
  version: "0.3.0",
  notes: ["Um", "Dois"],
  pageUrl: "https://github.com/lippdev/tako-launcher/releases",
  installable: true,
  size: Math.round(18.2 * MB),
};

function status(patch = {}) {
  return {
    state: "idle",
    current: "0.2.0",
    offer: null,
    progress: 0,
    downloadedBytes: 0,
    remainingSeconds: null,
    gameRunning: false,
    ...patch,
  };
}

const flags = (patch = {}) => ({ explicit: false, dialogOpen: false, ...patch });

test("megabytes com uma casa, vírgula em português", () => {
  assert.equal(formatMb(18.2 * MB), "18,2");
  assert.equal(formatMb(18.2 * MB, false), "18.2");
  assert.equal(formatMb(0), "0,0");
});

test("o diálogo só existe com uma versão achada e traduz cada estado do núcleo", () => {
  assert.equal(dialogState(status({ state: "downloading" })), null);
  assert.equal(dialogState(status({ state: "current" })), null);
  const pairs = [
    ["available", "disponivel"],
    ["downloading", "baixando"],
    ["ready", "pronta"],
    ["installing", "pronta"],
    ["download_failed", "erro"],
  ];
  for (const [state, estado] of pairs) assert.equal(dialogState(status({ state, offer: OFFER })), estado, state);
  assert.equal(dialogState(status({ state: "searching", offer: OFFER })), null);
});

test("baixando mostra a versão, o quanto já veio e o tempo restante, como o Slint", () => {
  const view = dialogView(
    status({ state: "downloading", offer: OFFER, progress: 0.68, downloadedBytes: Math.round(12.4 * MB), remainingSeconds: 5 }),
    t,
  );
  assert.equal(view.estado, "baixando");
  assert.equal(view.progresso, 68);
  assert.equal(view.subtitulo, "0.3.0 · updates.desktop.progress(done=12,4,total=18,2) · updates.desktop.remaining(seconds=5)");
  assert.equal(view.observacao, undefined);
  // Sem tamanho nem estimativa, só a versão.
  const bare = dialogView(status({ state: "downloading", offer: { ...OFFER, size: 0 } }), t);
  assert.equal(bare.subtitulo, "0.3.0");
});

test("pronta lembra que o Depois instala ao sair e, com jogo aberto, que o reinício espera", () => {
  const ready = dialogView(status({ state: "ready", offer: OFFER, progress: 1 }), t);
  assert.equal(ready.estado, "pronta");
  assert.equal(ready.subtitulo, "0.3.0 updates.desktop.size(size=18,2)");
  assert.equal(ready.observacao, "updates.desktop.help_ready");
  assert.deepEqual(ready.novidades, ["Um", "Dois"]);
  const game = dialogView(status({ state: "ready", offer: OFFER, progress: 1, gameRunning: true }), t);
  assert.equal(game.observacao, "updates.desktop.help_ready updates.finish_game");
});

test("sem arquivo conferível, o botão só leva à página", () => {
  const page = dialogView(status({ state: "available", offer: { ...OFFER, installable: false, size: 0 } }), t);
  assert.equal(page.soPagina, true);
  assert.equal(page.subtitulo, "0.3.0");
  assert.equal(dialogView(status({ state: "ready", offer: OFFER }), t).soPagina, false);
});

test("procura pedida: em dia e falha viram aviso, versão achada abre o diálogo", () => {
  const searching = status({ state: "searching" });
  const current = transition(searching, status({ state: "current" }), flags({ explicit: true }), t);
  assert.deepEqual(current.notice, { tone: "success", text: "updates.desktop.current_toast(version=0.2.0)" });
  assert.equal(current.explicit, false);
  const failed = transition(searching, status({ state: "check_failed" }), flags({ explicit: true }), t);
  assert.deepEqual(failed.notice, { tone: "error", text: "updates.desktop.failed_toast" });
  const found = transition(searching, status({ state: "downloading", offer: OFFER }), flags({ explicit: true }), t);
  assert.equal(found.dialogOpen, true);
  assert.equal(found.explicit, false);
  assert.equal(found.notice, null);
  const page = transition(searching, status({ state: "available", offer: OFFER }), flags({ explicit: true }), t);
  assert.equal(page.dialogOpen, true);
});

test("procura automática ao abrir: só o download que a gente viu começar abre o diálogo", () => {
  const seen = transition(status({ state: "searching" }), status({ state: "downloading", offer: OFFER }), flags(), t);
  assert.equal(seen.dialogOpen, true);
  // Abriu a interface com o download já andando, ou sem versão nova: nada de diálogo nem aviso.
  assert.equal(transition(null, status({ state: "downloading", offer: OFFER }), flags(), t).dialogOpen, false);
  const step = status({ state: "downloading", offer: OFFER, progress: 0.4 });
  assert.equal(transition(status({ state: "downloading", offer: OFFER }), step, flags(), t).dialogOpen, false);
  const none = transition(status({ state: "searching" }), status({ state: "current" }), flags(), t);
  assert.deepEqual([none.dialogOpen, none.notice], [false, null]);
  // Só a página (sem arquivo): acende a barra, sem abrir nada sozinho.
  assert.equal(transition(status({ state: "searching" }), status({ state: "available", offer: OFFER }), flags(), t).dialogOpen, false);
});

test("pronta não abre diálogo nem avisa; falha com o diálogo fechado avisa", () => {
  const downloading = status({ state: "downloading", offer: OFFER, progress: 0.9 });
  const ready = transition(downloading, status({ state: "ready", offer: OFFER, progress: 1 }), flags(), t);
  assert.deepEqual([ready.dialogOpen, ready.notice], [false, null]);
  const failed = transition(downloading, status({ state: "download_failed", offer: OFFER }), flags(), t);
  assert.deepEqual(failed.notice, { tone: "error", text: "updates.desktop.download_failed" });
  // Com o diálogo aberto, o próprio diálogo vira o de erro: sem aviso por cima.
  const open = transition(downloading, status({ state: "download_failed", offer: OFFER }), flags({ dialogOpen: true }), t);
  assert.equal(open.notice, null);
  assert.equal(open.dialogOpen, true);
});

test("o botão da barra aparece só com algo a mostrar", () => {
  assert.equal(indicator(null, t), null);
  assert.equal(indicator(status({ state: "current" }), t), null);
  assert.equal(indicator(status({ state: "searching" }), t), null);
  assert.deepEqual(indicator(status({ state: "downloading", offer: OFFER }), t), {
    icon: "download",
    label: "updates.desktop.downloading",
    attention: false,
  });
  assert.deepEqual(indicator(status({ state: "ready", offer: OFFER }), t), {
    icon: "refresh",
    label: "desktop.window.update_ready",
    attention: true,
  });
  assert.equal(indicator(status({ state: "download_failed", offer: OFFER }), t).icon, "badge-alert");
  assert.equal(indicator(status({ state: "available", offer: OFFER }), t).label, "updates.desktop.found(version=0.3.0)");
});

test("a linha do menu muda no lugar conforme o estado", () => {
  const idle = menuLine(status(), null, "Procurar atualizações", t);
  assert.deepEqual(idle, { label: "Procurar atualizações", icon: "refresh", disabled: false });
  assert.deepEqual(menuLine(status({ state: "searching" }), null, "x", t), {
    label: "updates.desktop.searching",
    icon: "refresh",
    disabled: true,
  });
  assert.deepEqual(menuLine(status({ state: "current" }), "current", "x", t), {
    label: "updates.desktop.current",
    icon: "check",
    disabled: false,
  });
  assert.deepEqual(menuLine(status({ state: "check_failed" }), "failed", "x", t), {
    label: "updates.desktop.check_failed",
    icon: "badge-alert",
    disabled: false,
  });
  // Passados os 4 s, volta ao texto de sempre.
  assert.equal(menuLine(status({ state: "current" }), null, "Procurar", t).label, "Procurar");
});
