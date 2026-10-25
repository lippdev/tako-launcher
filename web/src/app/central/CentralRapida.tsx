import { useRef, useState } from "react";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { CampoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { ControleDeslizante } from "../../components/ControleDeslizante/ControleDeslizante";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { useT } from "../../i18n";
import { useEscapeLayer } from "../../lib/escapeLayers";
import { useFocusTrap } from "../../lib/focusTrap";
import { Portal } from "../../lib/Portal";
import { useGameSession } from "../sessao/useGameSession";
import { centralStrings } from "../strings";
import { useControlCenter } from "./useControlCenter";
import styles from "./CentralRapida.module.css";

/** Barra e painéis rápidos da Central Slint; a sessão e os dispositivos continuam no Rust. */
export function CentralRapida() {
  const t = useT();
  const central = useControlCenter();
  const session = useGameSession();
  const [panel, setPanel] = useState<"audio" | "screen" | "performance" | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [endError, setEndError] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const open = central.open && session.running !== null && session.running.phase !== "closing";
  useFocusTrap(open, box);
  useEscapeLayer(open, () => {
    if (panel) setPanel(null);
    else void session.resume();
  });
  const state = central.snapshot;
  const audio = state?.audio;

  if (!open || !state || !session.running) return null;

  return (
    <Portal>
      <div className={styles["veil"]}>
        <div ref={box} className={styles["central"]} role="dialog" aria-modal="true" aria-label={centralStrings.title} tabIndex={-1}>
          <header className={styles["game"]}>
            <div>
              <h2>{session.running.title}</h2>
              <p>{t("cc.playing")}{session.elapsed ? ` · ${session.elapsed}` : ""}</p>
            </div>
            {state.controller.connected && <p>{state.controller.battery === null ? state.controller.name : t("cc.pad_battery", { percent: String(state.controller.battery) })}</p>}
          </header>

          {panel === "audio" && <section className={styles["panel"]} aria-label={t("cc.audio")}>
            {audio ? <>
              <label>{t("cc.volume")}
                <ControleDeslizante value={Math.round((audio.volume ?? 0) * 100)} step={5} disabled={central.busy || audio.volume === null} aria-label={t("cc.volume")} format={(v) => `${String(v)}%`} onChange={(value) => { void central.change({ action: "volume", value: value / 100 }); }} />
              </label>
              <BotaoDesktop variant="outlined" disabled={central.busy} aria-pressed={audio.muted} onClick={() => { void central.change({ action: "mute", muted: !audio.muted }); }}>{t("cc.mute")}</BotaoDesktop>
              <CampoDeSelecao label={t("cc.output")} value={audio.defaultId ?? ""} disabled={central.busy || !audio.outputs.canSwitch} options={audio.outputs.outputs.map((o) => ({ value: o.id, label: o.name }))} onChange={(id) => { void central.change({ action: "audio_output", id }); }} />
            </> : <p>{t("cc.audio_unavailable")}</p>}
          </section>}

          {panel === "screen" && <section className={styles["panel"]} aria-label={t("cc.screen")}>
            {state.displays.length === 0 && <p>{t("cc.screen_unavailable")}</p>}
            {state.displays.map((display) => <CampoDeSelecao key={display.id} label={display.name} value={display.currentMode ?? ""} disabled={central.busy || state.displayManaged} options={display.modes.map((mode) => ({ value: mode, label: mode }))} onChange={(mode) => { void central.change({ action: "display_mode", id: display.id, mode }); }} />)}
          </section>}

          {panel === "performance" && <section className={styles["panel"]} aria-label={t("settings.performance.title")}>
            <p>{t("settings.performance.supervisor")}</p>
            <dl>
              <div><dt>{t("settings.performance.memory")}</dt><dd>{state.memoryMib === null ? t("settings.performance.unknown") : centralStrings.memory(state.memoryMib)}</dd></div>
              <div><dt>{centralStrings.cpuTime}</dt><dd>{state.cpuSeconds === null ? t("settings.performance.unknown") : centralStrings.seconds(state.cpuSeconds)}</dd></div>
            </dl>
          </section>}

          {(central.error || endError) && <p role="alert">{centralStrings.actionError}</p>}
          <nav className={styles["actions"]} aria-label={centralStrings.title}>
            <BotaoDesktop icon="play" onClick={() => { void session.resume(); }}>{t("cc.back_to_game")}</BotaoDesktop>
            <BotaoDesktop variant="outlined" aria-expanded={panel === "audio"} onClick={() => { setPanel(panel === "audio" ? null : "audio"); }}>{t("cc.audio")}</BotaoDesktop>
            <BotaoDesktop variant="outlined" aria-expanded={panel === "screen"} onClick={() => { setPanel(panel === "screen" ? null : "screen"); }}>{t("cc.screen")}</BotaoDesktop>
            <BotaoDesktop variant="outlined" aria-expanded={panel === "performance"} onClick={() => { setPanel(panel === "performance" ? null : "performance"); }}>{t("settings.performance.title")}</BotaoDesktop>
            <BotaoDesktop variant="outlined" disabled={session.running.phase === "closing"} onClick={() => { setConfirm(true); }}>{t("cc.end")}</BotaoDesktop>
          </nav>
        </div>
      </div>
      <Dialogo open={confirm} onClose={() => { setConfirm(false); }} title={t("cc.end_confirm")} closeLabel={t("common.close")} actions={<>
        <BotaoDesktop variant="outlined" onClick={() => { setConfirm(false); }}>{t("common.cancel")}</BotaoDesktop>
        <BotaoDesktop onClick={() => {
          setConfirm(false);
          void session.end().then((result) => { setEndError(result === "error"); });
        }}>{t("cc.end")}</BotaoDesktop>
      </>}><p>{session.running.title}</p></Dialogo>
    </Portal>
  );
}
