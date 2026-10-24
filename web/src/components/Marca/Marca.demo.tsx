import { Marca } from "./Marca";

export const title = "Marca";
export const preview = "Marca";

export function Demo() {
  return (
    <div style={{ display: "flex", gap: 32, alignItems: "center" }}>
      <Marca />
      <Marca variant="horizontal" />
    </div>
  );
}
