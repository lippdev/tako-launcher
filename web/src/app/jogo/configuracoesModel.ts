// Do núcleo (`game.settings`) para o painel ConfiguracoesDoJogo e de volta (`game.settings.set`). Só dados e funções
// puras, sem importações em execução (os textos entram por `labels`), para o teste rodar direto no Node.

import type { GameChoice } from "../../bridge/generated/GameChoice";
import type { GameSettingChange } from "../../bridge/generated/GameSettingChange";
import type { GameSettingsView } from "../../bridge/generated/GameSettingsView";
import type {
  ChaveConfiguracaoJogo,
  DescricoesConfiguracoesDoJogo,
  OpcaoDeSelecao,
  OpcoesConfiguracoesDoJogo,
  ValorConfiguracaoJogo,
  ValoresConfiguracoesDoJogo,
} from "../../components/ConfiguracoesDoJogo/ConfiguracoesDoJogo";

/** Os textos que o painel monta com dados do núcleo (vêm de `strings.ts`). */
export interface PanelLabels {
  noScreen: string;
  keepCurrent: string;
  keepAudio: string;
  on: string;
  off: string;
  disconnected: (name: string) => string;
  /** "1920 × 1080 · 144 Hz" para a chave `1920x1080@144`. */
  mode: (key: string) => string;
  screenNote: string;
  whichScreenNote: string;
  launchNote: string;
  environmentNote: string;
  commandsNote: string;
  commandsBefore: string;
  commandsAfter: string;
  vrrUnsupported: string;
  audioLocked: string;
}

/** O que o painel recebe para desenhar: já no formato do componente. */
export interface PanelModel {
  valores: ValoresConfiguracoesDoJogo;
  padroes: ValoresConfiguracoesDoJogo;
  opcoes: OpcoesConfiguracoesDoJogo;
  descricoes: DescricoesConfiguracoesDoJogo;
  somenteLeitura: ChaveConfiguracaoJogo[];
  mudancas: number;
}

/** Valores das opções que não são um id do núcleo. */
export const KEEP = "keep";
const OUT = "out:";
const ON = "on";
const OFF = "off";

/** A opção `value` entra na lista se ainda não estiver (a escolhida ou o padrão de uma tela que já não a tem). */
function withOption(
  options: OpcaoDeSelecao<string>[],
  value: string,
  label: (value: string) => string,
): OpcaoDeSelecao<string>[] {
  return options.some((o) => o.value === value) ? options : [...options, { value, label: label(value) }];
}

function audioValue(device: { id: string } | null): string {
  return device ? `${OUT}${device.id}` : KEEP;
}

export function panelModel(view: GameSettingsView, labels: PanelLabels): PanelModel {
  const valores: ValoresConfiguracoesDoJogo = {};
  const padroes: ValoresConfiguracoesDoJogo = {};
  const opcoes: OpcoesConfiguracoesDoJogo = {};
  const descricoes: DescricoesConfiguracoesDoJogo = {};
  const somenteLeitura: ChaveConfiguracaoJogo[] = [];
  const onOff = (on: boolean) => (on ? ON : OFF);

  // Tela: usar a tela de jogo, qual tela, resolução, HDR e VRR.
  valores.usarTela = view.useScreen.effective;
  padroes.usarTela = view.useScreen.default;
  descricoes.usarTela = labels.screenNote;

  if (view.screen.options.length > 0) {
    const defaultName = view.screen.default?.name ?? labels.noScreen;
    valores.monitor = view.screen.saved ?? "";
    padroes.monitor = "";
    opcoes.monitor = [
      { value: "", label: defaultName },
      ...view.screen.options.map((o) => ({ value: o.id, label: o.connected ? o.name : labels.disconnected(o.name) })),
    ];
    descricoes.monitor = labels.whichScreenNote;
  }

  const keys = view.mode;
  if (keys.options.length > 0 || keys.saved.kind !== "default") {
    const modeLabel = (key: string) => (key === KEEP ? labels.keepCurrent : labels.mode(key));
    const value = keys.effective ?? KEEP;
    const fallback = keys.default ?? KEEP;
    valores.resolucaoETaxa = value;
    padroes.resolucaoETaxa = fallback;
    opcoes.resolucaoETaxa = withOption(
      withOption(
        [KEEP, ...keys.options].map((key) => ({ value: key, label: modeLabel(key) })),
        value,
        modeLabel,
      ),
      fallback,
      modeLabel,
    );
  }

  valores.hdr = onOff(view.hdr.effective);
  padroes.hdr = onOff(view.hdr.default);
  opcoes.hdr = [
    { value: ON, label: labels.on },
    { value: OFF, label: labels.off },
  ];

  if (view.vrr.unsupported) {
    // Como as Configurações: a linha fica à vista, desligada, com o motivo.
    valores.vrr = false;
    padroes.vrr = false;
    descricoes.vrr = view.vrr.limitation ?? labels.vrrUnsupported;
    somenteLeitura.push("vrr");
  } else {
    valores.vrr = view.vrr.effective;
    padroes.vrr = view.vrr.default;
  }

  // Som.
  const outputs = view.audio.options.map((o) => ({
    value: `${OUT}${o.id}`,
    label: o.connected ? o.name : labels.disconnected(o.name),
  }));
  const outputLabel = (value: string) => {
    if (value === KEEP) return labels.keepAudio;
    const id = value.slice(OUT.length);
    return [view.audio.effective, view.audio.default].find((d) => d?.id === id)?.name ?? id;
  };
  const audio = audioValue(view.audio.effective);
  const audioDefault = audioValue(view.audio.default);
  valores.saidaDeAudio = audio;
  padroes.saidaDeAudio = audioDefault;
  opcoes.saidaDeAudio = withOption(
    withOption([{ value: KEEP, label: labels.keepAudio }, ...outputs], audio, outputLabel),
    audioDefault,
    outputLabel,
  );
  if (!view.audio.canSwitch) {
    descricoes.saidaDeAudio = labels.audioLocked;
    somenteLeitura.push("saidaDeAudio");
  }

  // Abrir o jogo.
  valores.argumentos = view.arguments.saved ?? "";
  padroes.argumentos = "";
  descricoes.argumentos = labels.launchNote;
  valores.pastaDeTrabalho = view.workingDirectory.saved ?? "";
  padroes.pastaDeTrabalho = "";
  valores.variaveisDeAmbiente = view.environment.saved ?? "";
  padroes.variaveisDeAmbiente = "";
  descricoes.variaveisDeAmbiente = labels.environmentNote;
  valores.comandosAntesEDepois = [
    view.beforeCommand.saved ? `${labels.commandsBefore}: ${view.beforeCommand.saved}` : null,
    view.afterCommand.saved ? `${labels.commandsAfter}: ${view.afterCommand.saved}` : null,
  ]
    .filter(Boolean)
    .join(" · ");
  descricoes.comandosAntesEDepois = labels.commandsNote;
  somenteLeitura.push("comandosAntesEDepois");

  return { valores, padroes, opcoes, descricoes, somenteLeitura, mudancas: view.changes };
}

/** O texto de um campo livre: em branco volta ao padrão (`null`). */
function textOrNull(value: string): string | null {
  return value.trim() === "" ? null : value;
}

/** Ligado/desligado igual ao padrão não é uma escolha do jogo: volta ao padrão em vez de gravar o mesmo valor. */
function flag(value: boolean, standard: boolean): boolean | null {
  return value === standard ? null : value;
}

/**
 * A mudança que a ponte grava para o que o usuário fez numa linha. Escolher o que já é o padrão apaga a escolha do
 * jogo (a linha deixa de aparecer como mudada). `null` quando o valor não faz sentido para a linha.
 */
export function toChange(
  key: ChaveConfiguracaoJogo,
  value: ValorConfiguracaoJogo,
  view: GameSettingsView,
): GameSettingChange | null {
  if (typeof value === "boolean") {
    if (key === "usarTela") return { field: "use_screen", value: flag(value, view.useScreen.default) };
    if (key === "vrr") return { field: "vrr", value: flag(value, view.vrr.default) };
    return null;
  }
  switch (key) {
    case "monitor":
      return { field: "screen", value: value === "" ? null : value };
    case "resolucaoETaxa": {
      const choice: GameChoice =
        value === (view.mode.default ?? KEEP)
          ? { kind: "default" }
          : value === KEEP
            ? { kind: "keep" }
            : { kind: "pick", id: value };
      return { field: "mode", value: choice };
    }
    case "hdr":
      return value === ON || value === OFF ? { field: "hdr", value: flag(value === ON, view.hdr.default) } : null;
    case "saidaDeAudio": {
      const choice: GameChoice =
        value === audioValue(view.audio.default)
          ? { kind: "default" }
          : value === KEEP
            ? { kind: "keep" }
            : value.startsWith(OUT)
              ? { kind: "pick", id: value.slice(OUT.length) }
              : { kind: "default" };
      return { field: "audio", value: choice };
    }
    case "argumentos":
      return { field: "arguments", value: textOrNull(value) };
    case "pastaDeTrabalho":
      return { field: "working_directory", value: textOrNull(value) };
    case "variaveisDeAmbiente":
      return { field: "environment", value: textOrNull(value) };
    default:
      return null;
  }
}
