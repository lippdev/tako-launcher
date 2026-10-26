import { useCallback, useEffect, useRef, useState } from "react";
import { call, on } from "../../bridge";
import type { ControlCenterState } from "../../bridge/generated/ControlCenterState";
import type { ControlCenterAction } from "../../bridge/generated/ControlCenterAction";

/** Só a última leitura do núcleo; a intenção de abrir sobrevive à recriação do CEF. */
export function useControlCenter() {
  const [snapshot, setSnapshot] = useState<ControlCenterState | null>(null);
  const [error, setError] = useState(false);
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const changing = useRef(false);
  const refresh = useCallback(() => {
    const current = ++generation.current;
    return call("control_center.state").then((state) => {
      if (current === generation.current) setSnapshot(state);
    }, (failure: unknown) => {
      console.error("control_center.state", failure);
      if (current === generation.current) setError(true);
    });
  }, []);
  useEffect(() => {
    void refresh();
    const stopOpen = on("control_center.open", (open) => {
      setSnapshot((prev) => (prev ? { ...prev, open } : null));
      void refresh();
    });
    const stopEnd = on("game.ended", () => {
      generation.current += 1;
      setSnapshot(null);
    });
    return () => { generation.current += 1; stopOpen(); stopEnd(); };
  }, [refresh]);
  const open = snapshot?.open ?? false;
  useEffect(() => {
    if (!open) return;
    const timer = setInterval(() => { if (!changing.current) void refresh(); }, 2000);
    return () => { clearInterval(timer); };
  }, [open, refresh]);
  const change = useCallback(async (action: ControlCenterAction) => {
    if (changing.current) return;
    changing.current = true;
    setBusy(true);
    setError(false);
    const current = ++generation.current;
    try {
      const state = await call("control_center.set", action);
      if (current === generation.current) setSnapshot(state);
    } catch (failure) {
      console.error("control_center.set", failure);
      if (current === generation.current) setError(true);
    } finally {
      changing.current = false;
      setBusy(false);
    }
  }, []);
  return { snapshot, open, error, busy, change };
}
