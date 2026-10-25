import { useState } from "react";
import { ControleDeslizante } from "./ControleDeslizante";

export const title = "ControleDeslizante";
export const preview = "ControleDeslizante";

export function Demo() {
  const [volume, setVolume] = useState(60);
  const [deadzone, setDeadzone] = useState(12);
  const [size, setSize] = useState(28);
  const [opacity, setOpacity] = useState(0);
  return (
    <div style={{ display: "grid", gap: "var(--space-24)", justifyContent: "start" }}>
      <ControleDeslizante value={volume} onChange={setVolume} format={(v) => `${v}%`} aria-label="Volume" />
      <ControleDeslizante value={deadzone} onChange={setDeadzone} max={50} format={(v) => `${v}%`} aria-label="Zona morta" />
      <ControleDeslizante value={size} onChange={setSize} min={12} max={48} format={(v) => `${v}px`} aria-label="Tamanho" />
      <ControleDeslizante value={opacity} onChange={setOpacity} step={5} format={(v) => `${v}%`} aria-label="Opacidade (no mínimo)" />
      <ControleDeslizante value={40} onChange={() => undefined} disabled format={(v) => `${v}%`} aria-label="Desabilitado" />
    </div>
  );
}
