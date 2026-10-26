import { call } from "../../bridge";
import type { AudioOutputList } from "../../bridge/generated/AudioOutputList";
import type { DisplayInfo } from "../../bridge/generated/DisplayInfo";
import type { OpcaoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
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
  const t = useT();
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
    screenId ? (values[KEYS.screenName] ?? screenId) : t("settings.screen_web.choose_display"),
  );
  const displayDescription =
    displays.status === "failed"
      ? t("settings.screen_web.display_error")
      : list && list.displays.length === 0
        ? t("settings.screen.no_displays")
        : t("settings.screen.display_desc_desktop");

  return (
    <>
      <GrupoDeConfiguracao title={t("settings.screen_web.screen_header")}>
        {managed && (
          <LinhaDeConfiguracao
            title={t("settings.screen.managed")}
            description={t("settings.screen.managed_desc")}
            control={<span>{t("settings.screen.managed_value")}</span>}
          />
        )}
        <LinhaChave
          title={t("settings.screen.enabled")}
          description={t("settings.screen.enabled_desc")}
          checked={readBool(values, KEYS.screenEnabled, DEFAULTS.screenEnabled)}
          disabled={managed}
          onChange={(on) => void save(KEYS.screenEnabled, boolText(on))}
        />
        <LinhaSelecao
          title={t("settings.screen.display")}
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
          title={t("settings.screen.hide")}
          description={t("settings.screen.hide_desc")}
          checked={readHideOthers(values)}
          disabled={managed}
          onChange={(on) => void save(KEYS.screenHideOthers, hideOthersText(on))}
        />
        {selected && (
          <LinhaSelecao
            title={t("settings.screen.mode")}
            description={t("settings.screen.mode_desc")}
            options={withSaved(
              [
                { value: "", label: t("settings.screen.keep_mode") },
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
            title={t("settings.screen.hdr")}
            description={t("settings.screen.hdr_desc")}
            checked={readBool(values, KEYS.screenHdr, DEFAULTS.screenHdr)}
            disabled={managed}
            onChange={(on) => void save(KEYS.screenHdr, boolText(on))}
          />
        )}
        {/* O VRR aparece mesmo sem suporte, desligado e com o motivo (no Windows falta API pública). */}
        {selected && (
          <LinhaChave
            title={t("vrr.toggle")}
            description={
              selected.vrr === "unsupported" ? (list?.vrrLimitation ?? t("vrr.unsupported")) : t("vrr.toggle_desc")
            }
            checked={selected.vrr !== "unsupported" && readBool(values, KEYS.screenVrr, DEFAULTS.screenVrr)}
            disabled={managed || selected.vrr === "unsupported"}
            onChange={(on) => void save(KEYS.screenVrr, boolText(on))}
          />
        )}
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={t("settings.screen.audio_header")}>
        <AudioRow outputs={outputs.status === "ready" ? outputs.data : null} failed={outputs.status === "failed"} values={values} save={save} />
      </GrupoDeConfiguracao>
      <GrupoDeConfiguracao title={t("settings.screen_web.during_header")}>
        <LinhaChave
          title={t("settings.screen.restore")}
          description={t("settings.screen.restore_desc")}
          checked={readBool(values, KEYS.restoreEnvironment, DEFAULTS.restoreEnvironment)}
          onChange={(on) => void save(KEYS.restoreEnvironment, boolText(on))}
        />
        <LinhaChave
          title={t("settings.screen.cursor")}
          description={t("settings.screen.cursor_desc")}
          checked={readBool(values, KEYS.hideCursor, DEFAULTS.hideCursor)}
          onChange={(on) => void save(KEYS.hideCursor, boolText(on))}
        />
        <LinhaChave
          title={t("settings.screen.mouse")}
          description={t("settings.screen.mouse_desc")}
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
  const t = useT();
  const audioId = values[KEYS.audioId] ?? "";
  const known = outputs?.outputs.find((output) => sameId(output.id, audioId));
  const options = withSaved(
    [
      { value: "", label: t("settings.screen.audio_keep") },
      ...(outputs?.outputs ?? []).map((output) => ({ value: output.id, label: output.name })),
    ],
    known?.id ?? audioId,
    values[KEYS.audioName] || audioId,
  );
  const locked = outputs !== null && !outputs.canSwitch;
  return (
    <LinhaSelecao
      title={t("settings.screen.audio")}
      description={failed ? t("settings.screen_web.audio_output_error") : locked ? t("settings.screen.audio_locked") : t("settings.screen.audio_desc")}
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
