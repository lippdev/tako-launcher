// Paridade com `tako-ui/src/shell/first_run.rs`: regras sem React nem ponte.
import type { Store } from "../../bridge/generated/Store";
import { withStoreEnabled } from "../settings/model.ts";

export const FIRST_RUN_KEY = "FirstRunCompleted";
// A ponte expõe só `enabled`; o JSON com as credenciais fica no cofre do Slint.
export const IGDB_KEY = "metadata.igdb";
export type Step = 1 | 2 | 3 | 4 | 5;
export type OpenMode = "desktop" | "big-picture";
export type Settings = Readonly<Record<string, string>>;

export function decideFirstRun(values: Settings, libraryCount: number, reopening: boolean): "show" | "skip" | "complete-existing" {
  if (reopening || values["SafeMode"] === "true" || values[FIRST_RUN_KEY] === "true") return "skip";
  // A projeção do cofre não é uma configuração do SQLite: não conta para `Settings::is_empty`.
  if (Object.keys(values).some((key) => key !== IGDB_KEY) || libraryCount > 0) return "complete-existing";
  return "show";
}

export function nextStep(step: Step, hasEmulators: boolean): Step {
  return step === 1 && !hasEmulators ? 3 : Math.min(5, step + 1) as Step;
}

export function previousStep(step: Step, hasEmulators: boolean): Step | null {
  if (step === 1) return null; // Esc nas lojas equivale a "Pular configuração" no Slint.
  return step === 3 && !hasEmulators ? 1 : step - 1 as Step;
}

export function storeChoice(disabled: string | undefined, store: Store, enabled: boolean) {
  return { key: "DisabledStores", value: withStoreEnabled(disabled, store, enabled) };
}

export function coverChoice(enabled: boolean) {
  return { key: IGDB_KEY, value: String(enabled) };
}

export function modeChoice(mode: OpenMode) {
  return { key: "OpenIn", value: mode };
}

export function finishChoice() {
  return { key: FIRST_RUN_KEY, value: "true" };
}

export function initialMode(values: Settings, controllersConnected: boolean): OpenMode {
  const saved = values["OpenIn"];
  return saved === "big-picture" || (saved !== "desktop" && controllersConnected) ? "big-picture" : "desktop";
}
