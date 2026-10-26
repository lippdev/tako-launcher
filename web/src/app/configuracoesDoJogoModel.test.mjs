import assert from "node:assert/strict";
import { test } from "node:test";
import { KEEP, panelModel, toChange } from "./jogo/configuracoesModel.ts";

const labels = {
  noScreen: "Nenhuma",
  keepCurrent: "Manter a atual",
  keepAudio: "Não alterar",
  on: "Ligado",
  off: "Desligado",
  disconnected: (name) => `${name} (desconectada)`,
  mode: (key) => `modo ${key}`,
  screenNote: "nota da tela",
  whichScreenNote: "nota de qual tela",
  launchNote: "nota de abrir",
  environmentNote: "nota das variáveis",
  commandsNote: "nota dos comandos",
  commandsBefore: "Antes",
  commandsAfter: "Depois",
  vrrUnsupported: "Esta tela não informa VRR",
  audioLocked: "Sistema não deixa trocar",
};

const text = (saved = null) => ({ saved, effective: saved });

/** Um jogo que segue o Tako: tela de jogo do Tako na TV, saída de áudio da TV. */
function view(patch = {}) {
  return {
    gameId: "steam:1",
    changes: 0,
    useScreen: { saved: null, default: true, effective: true },
    screen: {
      saved: null,
      default: { id: "DISPLAY2", name: "TV" },
      effective: "DISPLAY2",
      options: [
        { id: "DISPLAY1", name: "Monitor", connected: true },
        { id: "DISPLAY2", name: "TV", connected: true },
      ],
      noScreenChosen: false,
    },
    mode: {
      saved: { kind: "default" },
      default: "3840x2160@120",
      effective: "3840x2160@120",
      options: ["3840x2160@120", "3840x2160@60"],
    },
    hdr: { saved: null, default: true, effective: true },
    vrr: { saved: null, default: false, effective: false, unsupported: false, limitation: null },
    audio: {
      saved: { kind: "default" },
      default: { id: "tv", name: "TV (HDMI)" },
      effective: { id: "tv", name: "TV (HDMI)" },
      options: [
        { id: "speakers", name: "Alto-falantes", connected: true },
        { id: "tv", name: "TV (HDMI)", connected: true },
      ],
      canSwitch: true,
    },
    arguments: text(),
    workingDirectory: text(),
    environment: text(),
    beforeCommand: text(),
    afterCommand: text(),
    commandsApproved: true,
    ...patch,
  };
}

test("um jogo sem escolhas mostra os valores do Tako e nenhuma linha como mudada", () => {
  const m = panelModel(view(), labels);
  assert.deepEqual(m.valores, {
    usarTela: true,
    monitor: "",
    resolucaoETaxa: "3840x2160@120",
    hdr: "on",
    vrr: false,
    saidaDeAudio: "out:tv",
    argumentos: "",
    pastaDeTrabalho: "",
    variaveisDeAmbiente: "",
    comandosAntesEDepois: "",
  });
  for (const key of Object.keys(m.padroes)) {
    if (key === "comandosAntesEDepois") continue;
    assert.deepEqual(m.valores[key], m.padroes[key], key);
  }
  assert.equal(m.mudancas, 0);
  assert.deepEqual(m.somenteLeitura, ["comandosAntesEDepois"]);
  // "Tela deste jogo": a primeira opção é a tela de jogo do Tako.
  assert.deepEqual(m.opcoes.monitor, [
    { value: "", label: "TV" },
    { value: "DISPLAY1", label: "Monitor" },
    { value: "DISPLAY2", label: "TV" },
  ]);
  assert.equal(m.opcoes.resolucaoETaxa[0].value, KEEP);
  assert.equal(m.opcoes.resolucaoETaxa[1].label, "modo 3840x2160@120");
});

test("o que o jogo muda aparece como valor próprio, com o padrão do Tako guardado ao lado", () => {
  const m = panelModel(
    view({
      changes: 3,
      screen: { ...view().screen, saved: "DISPLAY1", effective: "DISPLAY1" },
      mode: {
        saved: { kind: "pick", id: "3840x2160@60" },
        default: "3840x2160@120",
        effective: "3840x2160@60",
        options: ["3840x2160@60"],
      },
      hdr: { saved: false, default: true, effective: false },
      audio: { ...view().audio, saved: { kind: "keep" }, effective: null },
    }),
    labels,
  );
  assert.equal(m.valores.monitor, "DISPLAY1");
  assert.equal(m.valores.resolucaoETaxa, "3840x2160@60");
  assert.equal(m.padroes.resolucaoETaxa, "3840x2160@120");
  // O padrão que não está na lista desta tela continua escolhível e com nome.
  assert.ok(m.opcoes.resolucaoETaxa.some((o) => o.value === "3840x2160@120" && o.label === "modo 3840x2160@120"));
  assert.equal(m.valores.hdr, "off");
  assert.equal(m.padroes.hdr, "on");
  assert.equal(m.valores.saidaDeAudio, KEEP);
  assert.equal(m.padroes.saidaDeAudio, "out:tv");
  assert.equal(m.mudancas, 3);
});

test("a linha de resolução some sem nenhuma resolução para escolher", () => {
  const none = view({ mode: { saved: { kind: "default" }, default: null, effective: null, options: [] } });
  assert.equal("resolucaoETaxa" in panelModel(none, labels).valores, false);
  // Com uma escolha guardada ela fica, mesmo que a tela tenha sumido.
  const kept = view({ mode: { saved: { kind: "keep" }, default: null, effective: null, options: [] } });
  assert.equal(panelModel(kept, labels).valores.resolucaoETaxa, KEEP);
});

test("a linha da tela some sem nenhuma tela; a escolhida e desconectada fica com o aviso", () => {
  const none = view({ screen: { ...view().screen, default: null, effective: null, options: [] } });
  assert.equal("monitor" in panelModel(none, labels).valores, false);
  const gone = view({
    screen: {
      ...view().screen,
      saved: "DISPLAY7",
      options: [...view().screen.options, { id: "DISPLAY7", name: "Projetor", connected: false }],
    },
  });
  const m = panelModel(gone, labels);
  assert.equal(m.valores.monitor, "DISPLAY7");
  assert.deepEqual(m.opcoes.monitor.at(-1), { value: "DISPLAY7", label: "Projetor (desconectada)" });
  // Sem tela do Tako, a opção "padrão" diz que não há.
  const noTako = view({ screen: { ...view().screen, default: null } });
  assert.equal(panelModel(noTako, labels).opcoes.monitor[0].label, "Nenhuma");
});

test("VRR sem suporte e saída de áudio travada ficam só de leitura, com o motivo", () => {
  const m = panelModel(
    view({
      vrr: { saved: true, default: true, effective: true, unsupported: true, limitation: "Falta API no Windows" },
      audio: { ...view().audio, canSwitch: false },
    }),
    labels,
  );
  assert.deepEqual(m.somenteLeitura, ["vrr", "saidaDeAudio", "comandosAntesEDepois"]);
  assert.equal(m.valores.vrr, false);
  assert.equal(m.descricoes.vrr, "Falta API no Windows");
  assert.equal(m.descricoes.saidaDeAudio, "Sistema não deixa trocar");
  const noReason = panelModel(view({ vrr: { ...view().vrr, unsupported: true } }), labels);
  assert.equal(noReason.descricoes.vrr, "Esta tela não informa VRR");
});

test("os comandos do jogo aparecem juntos, só de leitura", () => {
  const m = panelModel(view({ beforeCommand: text("mount.bat"), afterCommand: text("umount.bat") }), labels);
  assert.equal(m.valores.comandosAntesEDepois, "Antes: mount.bat · Depois: umount.bat");
  const only = panelModel(view({ afterCommand: text("umount.bat") }), labels);
  assert.equal(only.valores.comandosAntesEDepois, "Depois: umount.bat");
});

test("uma saída guardada que o sistema não lista continua escolhível, com o nome do jogo", () => {
  const m = panelModel(
    view({
      audio: {
        ...view().audio,
        saved: { kind: "pick", id: "{fone}" },
        effective: { id: "{fone}", name: "Fone" },
        options: [...view().audio.options, { id: "{fone}", name: "Fone", connected: false }],
      },
    }),
    labels,
  );
  assert.equal(m.valores.saidaDeAudio, "out:{fone}");
  assert.deepEqual(m.opcoes.saidaDeAudio.at(-1), { value: "out:{fone}", label: "Fone (desconectada)" });
});

test("o que o usuário faz vira a chamada da ponte; escolher o padrão apaga a escolha do jogo", () => {
  const v = view();
  const c = (key, value) => toChange(key, value, v);
  // Ligar e desligar.
  assert.deepEqual(c("usarTela", false), { field: "use_screen", value: false });
  assert.deepEqual(c("usarTela", true), { field: "use_screen", value: null });
  assert.deepEqual(c("vrr", true), { field: "vrr", value: true });
  assert.deepEqual(c("vrr", false), { field: "vrr", value: null });
  assert.deepEqual(c("hdr", "off"), { field: "hdr", value: false });
  assert.deepEqual(c("hdr", "on"), { field: "hdr", value: null });
  // Tela.
  assert.deepEqual(c("monitor", "DISPLAY1"), { field: "screen", value: "DISPLAY1" });
  assert.deepEqual(c("monitor", ""), { field: "screen", value: null });
  // Resolução: a do padrão, "manter a atual" e uma escolhida.
  assert.deepEqual(c("resolucaoETaxa", "3840x2160@120"), { field: "mode", value: { kind: "default" } });
  assert.deepEqual(c("resolucaoETaxa", KEEP), { field: "mode", value: { kind: "keep" } });
  assert.deepEqual(c("resolucaoETaxa", "3840x2160@60"), {
    field: "mode",
    value: { kind: "pick", id: "3840x2160@60" },
  });
  // Com padrão "manter a atual", voltar a ele apaga a escolha.
  const keepByDefault = view({ mode: { ...v.mode, default: null } });
  assert.deepEqual(toChange("resolucaoETaxa", KEEP, keepByDefault), { field: "mode", value: { kind: "default" } });
  // Áudio.
  assert.deepEqual(c("saidaDeAudio", "out:tv"), { field: "audio", value: { kind: "default" } });
  assert.deepEqual(c("saidaDeAudio", "out:speakers"), { field: "audio", value: { kind: "pick", id: "speakers" } });
  assert.deepEqual(c("saidaDeAudio", KEEP), { field: "audio", value: { kind: "keep" } });
  // Texto: em branco volta ao padrão.
  assert.deepEqual(c("argumentos", "-dx11"), { field: "arguments", value: "-dx11" });
  assert.deepEqual(c("argumentos", "   "), { field: "arguments", value: null });
  assert.deepEqual(c("pastaDeTrabalho", "D:/Jogos"), { field: "working_directory", value: "D:/Jogos" });
  assert.deepEqual(c("variaveisDeAmbiente", ""), { field: "environment", value: null });
  // O que a interface não grava (comandos) ou o que não faz sentido não vira chamada.
  assert.equal(c("comandosAntesEDepois", "calc.exe"), null);
  assert.equal(c("hdr", "talvez"), null);
  assert.equal(c("monitor", true), null);
});

test("sem padrão do Tako, a saída de áudio do padrão é 'não alterar'", () => {
  const v = view({ audio: { ...view().audio, default: null, effective: null } });
  assert.equal(panelModel(v, labels).padroes.saidaDeAudio, KEEP);
  assert.deepEqual(toChange("saidaDeAudio", KEEP, v), { field: "audio", value: { kind: "default" } });
  assert.deepEqual(toChange("saidaDeAudio", "out:tv", v), { field: "audio", value: { kind: "pick", id: "tv" } });
});
