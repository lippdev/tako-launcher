import { useState } from "react";
import { Chave } from "./Chave";

export const title = "Chave";
// Sem pasta própria no design system: a chave aparece nas prévias de LinhaDeConfiguracao.
export const preview = "LinhaDeConfiguracao";

export function Demo() {
  const [on, setOn] = useState(true);
  const [off, setOff] = useState(false);
  return (
    <div style={{ display: "flex", gap: "var(--space-24)", alignItems: "center" }}>
      <Chave checked={on} onChange={setOn} aria-label="Ligada" />
      <Chave checked={off} onChange={setOff} aria-label="Desligada" />
      <Chave checked disabled onChange={() => undefined} aria-label="Ligada e desabilitada" />
      <Chave checked={false} disabled onChange={() => undefined} aria-label="Desligada e desabilitada" />
    </div>
  );
}
