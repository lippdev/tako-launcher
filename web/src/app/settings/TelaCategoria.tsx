import { call } from "../../bridge";
import type { AudioOutputList } from "../../bridge/generated/AudioOutputList";
import type { DisplayInfo } from "../../bridge/generated/DisplayInfo";
import type { OpcaoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { settingsStrings } from "../strings";
import { LinhaChave, LinhaSelecao, type CategoryProps } from "./Linhas";
import {
  DEFAULTS,
  KEYS,
  boolText,
  modeLabel,
  readBool,
  readHideOthers,
  hideOthersText,
  type SettingsMap,
} from "./model";
import { useSystemList } from "./useSystemList";

const s = settingsStrings.screen;

const readDisplays = () => call("system.displays");
const readOutputs = () => call("system.audio_outputs");

const sameId = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

/** Uma opção para o valor guardado, quando ele não está (mais) na lista do sistema: a tela some ao desligar o cabo. */
function withSaved(options: OpcaoDeSelecao<string>[], value: string, label: string): OpcaoDeSelecao<string>[] {
  return options.some((option) => option.value === value) ? options : [{ value, label }, ...options];
}

/**
 * Configurações › Tela e áudio do jogo: a tela onde o jogo aparece (com resolução, HDR e VRR), a saída de áudio e o
 * que muda durante a partida. As telas e as saídas vêm do sistema (`system.displays`, `system.audio_outputs`); linha
 * sem dado some. Ao trocar de tela, resolução, HDR e VRR voltam ao padrão (o núcleo guarda os da tela anterior).
 */
export function TelaCategoria({ values, save, read }: CategoryProps) {
  const displays = useSystemList(readDisplays);
  const outputs = useSystemList(readOutputs);
  const list = displays.status === "ready" ? displays.data : null;
  const managed = list?.managedBySession ?? false;

  const screenId = values[KEYS.screenId] ?? "";
  const selected = list?.displays.find((display) => sameId(display.id, screenId));
  const mode = values[KEYS.screenMode] ?? "";

  async function pickDisplay(display: DisplayInfo) {
    if (selected && sameId(selected.id, display.id)) return;
    if (!(await save(KEYS.screenId, display.id))) return;
    const now: SettingsMap = read();
    await Promise.all([
      save(KEYS.screenName, display.name),
      ...(now[KEYS.screenMode] ? [save(KEYS.screenMode, "")] : []),
      ...(readBool(now, KEYS.screenHdr, DEFAULTS.screenHdr) ? [save(KEYS.screenHdr, "false")] : []),
      ...(readBool(now, KEYS.screenVrr, DEFAULTS.screenVrr) ? [save(KEYS.screenVrr, "false")] : []),
    ]);
  }

  const displayOptions = withSaved(
    (list?.displays ?? []).map((display) => ({ value: display.id, label: display.name })),
    selected?.id ?? screenId,
    screenId ? (values[KEYS.screenName] ?? screenId) : s.display.choose,
  );
  const displayDescription =
    displays.status === "failed"
      ? s.display.error
      : list && list.displays.length === 0
        ? s.display.none
        : s.display.description;

  return (
    <>
      <GrupoDeConfiguracao title={s.screen}>
        {managed && (
          <LinhaDeConfiguracao
            title={s.managed.title}
            description={s.managed.description}
            control={<span>{s.managed.value}</span>}
          />
        )}
        <LinhaChave
          title={s.enabled.title}
          description={s.enabled.description}
          checked={readBool(values, KEYS.screenEnabled, DEFAULTS.screenEnabled)}
          disabled={managed}
          onChange={(on) => void save(KEYS.screenEnabled, boolText(on))}
        />
        <LinhaSelecao
          title={s.display.title}
          description={displayDescription}
          options={displayOptions}
          value={selected?.id ?? screenId}
          disabled={managed || !list || list.displays.length === 0}
          onChange={(id) => {
            const display = list?.displays.find((candidate) => candidate.id === id);
            if (display) void pickDisplay(display);
          }}
        />
        <LinhaChave
          title={s.hide.title}
          description={s.hide.description}
          checked={readHideOthers(values)}
          disabled={managed}
          onChange={(on) => void save(KEYS.screenHideOthers, hideOthersText(on))}
        />
        {selected && (
          <LinhaSelecao
            title={s.mode.title}
            description={s.mode.description}
            options={withSaved(
              [
                { value: "", label: s.mode.keep },
                ...selected.modes.map((key) => ({ value: key, label: modeLabel(key) })),
              ],
              mode,
              modeLabel(mode),
            )}
            value={mode}
            disabled={managed}
            onChange={(value) => void save(KEYS.screenMode, value)}
          />
        )}
        {selected && selected.hdr !== "unsupported" && (
          <LinhaChave
            title={s.hdr.title}
            description={s.hdr.description}
            checked={readBool(values, KEYS.screenHdr, DEFAULTS.screenHdr)}
            disabled={managed}
            onChange={(on) => void save(KEYS.screenHdr, boolText(on))}
          />
        )}
        {/* O VRR aparece mesmo sem suporte, desligado e com o motivo (no Windows falta API pública). */}
        {selected && (
          <LinhaChave
            title={s.vrr.title}
            description={
              selected.vrr === "unsupported" ? (list?.vrrLimitation ?? s.vrr.unsupported) : s.vrr.description
            }
            checked={selected.vrr !== "unsupported" && readBool(values, KEYS.screenVrr, DEFAULTS.screenVrr)}
            disabled={managed || selected.vrr === "unsupported"}
            onChange={(on) => void save(KEYS.screenVrr, boolText(on))}
          />
        )}
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={s.audio}>
        <AudioRow outputs={outputs.status === "ready" ? outputs.data : null} failed={outputs.status === "failed"} values={values} save={save} />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={s.during}>
        <LinhaChave
          title={s.restore.title}
          description={s.restore.description}
          checked={readBool(values, KEYS.restoreEnvironment, DEFAULTS.restoreEnvironment)}
          onChange={(on) => void save(KEYS.restoreEnvironment, boolText(on))}
        />
        <LinhaChave
          title={s.cursor.title}
          description={s.cursor.description}
          checked={readBool(values, KEYS.hideCursor, DEFAULTS.hideCursor)}
          onChange={(on) => void save(KEYS.hideCursor, boolText(on))}
        />
        <LinhaChave
          title={s.mouse.title}
          description={s.mouse.description}
          checked={readBool(values, KEYS.controllerMouse, DEFAULTS.controllerMouse)}
          onChange={(on) => void save(KEYS.controllerMouse, boolText(on))}
        />
      </GrupoDeConfiguracao>
    </>
  );
}

interface AudioRowProps extends Pick<CategoryProps, "values" | "save"> {
  /** `null` enquanto lê ou se a leitura falhou. */
  outputs: AudioOutputList | null;
  failed: boolean;
}

/** "Saída de áudio do jogo": "Não alterar" ou uma das saídas; desligada se o sistema não deixa trocar. */
function AudioRow({ outputs, failed, values, save }: AudioRowProps) {
  const audioId = values[KEYS.audioId] ?? "";
  const known = outputs?.outputs.find((output) => sameId(output.id, audioId));
  const options = withSaved(
    [
      { value: "", label: s.audioOutput.keep },
      ...(outputs?.outputs ?? []).map((output) => ({ value: output.id, label: output.name })),
    ],
    known?.id ?? audioId,
    values[KEYS.audioName] || audioId,
  );
  const locked = outputs !== null && !outputs.canSwitch;
  return (
    <LinhaSelecao
      title={s.audioOutput.title}
      description={failed ? s.audioOutput.error : locked ? s.audioOutput.locked : s.audioOutput.description}
      options={options}
      value={known?.id ?? audioId}
      disabled={outputs === null || locked}
      onChange={(id) => {
        const output = outputs?.outputs.find((candidate) => candidate.id === id);
        void save(KEYS.audioId, output?.id ?? "").then((ok) => {
          if (ok) void save(KEYS.audioName, output?.name ?? "");
        });
      }}
    />
  );
}
