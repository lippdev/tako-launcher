// Regras puras de "Editar jogo", "Adicionar jogo" e "Fontes e IDs" (WEB-12c): rascunhos, falhas e textos montados
// a partir do que a ponte devolve. Sem React nem ponte, para testar com `node --test`.

import type { ErrorCode } from "../../bridge/generated/ErrorCode";
import type { GameAddResult } from "../../bridge/generated/GameAddResult";
import type { GameEdit } from "../../bridge/generated/GameEdit";
import type { GameEditSet } from "../../bridge/generated/GameEditSet";
import type { GameTracking } from "../../bridge/generated/GameTracking";
import type { SourceIdMatch } from "../../bridge/generated/SourceIdMatch";
import type { SourceIdState } from "../../bridge/generated/SourceIdState";

/** A ordem do seletor "Fim do jogo" (a mesma do Slint). */
export const TRACKING_ORDER: readonly GameTracking[] = ["auto", "process", "install_folder", "none"];

/** O que o usuário preenche ao editar. */
export interface EditDraft {
  title: string;
  arguments: string;
  workingDirectory: string;
  tracking: GameTracking;
}

export function draftOf(edit: GameEdit): EditDraft {
  return {
    title: edit.title,
    arguments: edit.arguments,
    workingDirectory: edit.workingDirectory,
    tracking: edit.tracking,
  };
}

/** Mudou algo além de espaços nas pontas? Sem mudança o "Salvar" fica desligado. */
export function isDirty(draft: EditDraft, edit: GameEdit): boolean {
  return (
    draft.title.trim() !== edit.title ||
    draft.arguments.trim() !== edit.arguments ||
    draft.workingDirectory.trim() !== edit.workingDirectory ||
    draft.tracking !== edit.tracking
  );
}

export function toSet(gameId: string, draft: EditDraft): GameEditSet {
  return {
    gameId,
    title: draft.title.trim(),
    arguments: draft.arguments.trim(),
    workingDirectory: draft.workingDirectory.trim(),
    tracking: draft.tracking,
  };
}

/** Qual campo a falha da gravação acusa. */
export type EditProblem = "name" | "folder" | "invalid" | "manual" | "save";

/** O que o núcleo recusou: nome vazio (conferido antes), pasta que não existe, entrada fora do limite, jogo de loja. */
export function editProblem(draft: EditDraft, code: ErrorCode): EditProblem {
  if (draft.title.trim() === "") return "name";
  switch (code) {
    case "not_found":
      return "folder";
    case "bad_request":
      return "invalid";
    case "conflict":
      return "manual";
    default:
      return "save";
  }
}

/** Resultado de "Adicionar jogo" que fecha o diálogo, o que pede o sistema da ROM ou o que avisa e continua. */
export type AddOutcome =
  | { kind: "added"; title: string }
  | { kind: "ask-system"; fileName: string; systems: GameAddResult["systems"]; others: GameAddResult["others"] }
  | { kind: "already"; title: string }
  | { kind: "unreadable" };

export function addOutcome(result: GameAddResult): AddOutcome {
  switch (result.status) {
    case "added":
      return { kind: "added", title: result.title ?? "" };
    case "needs_system":
      return { kind: "ask-system", fileName: result.fileName ?? "", systems: result.systems, others: result.others };
    case "already":
      return { kind: "already", title: result.title ?? "" };
    case "unreadable":
      return { kind: "unreadable" };
  }
}

/** Qual aviso a falha de `library.add_game` pede. */
export type AddProblem = "empty" | "invalid" | "not-found" | "error";

export function addProblem(path: string, code: ErrorCode): AddProblem {
  if (path.trim() === "") return "empty";
  if (code === "bad_request") return "invalid";
  if (code === "not_found") return "not-found";
  return "error";
}

// ---- Fontes e IDs ----

export interface SourceLabels {
  noneChosen: string;
  noMatch: string;
  chosen: string;
  automatic: string;
}

/** "ID 113112 · Hades (2018) · escolhido por você", "Nenhum · escolhido por você" ou "Sem correspondência". */
export function describeSource(state: SourceIdState, labels: SourceLabels): string {
  if (state.none) return labels.noneChosen;
  if (state.id === null) return labels.noMatch;
  let text = `ID ${state.id}`;
  if (state.title) text += ` · ${state.title}`;
  if (state.year !== null) text += ` (${String(state.year)})`;
  return `${text} · ${state.pinned ? labels.chosen : labels.automatic}`;
}

/** ✓ = o id em uso; ★ = o mais provável; senão nada. */
export function matchMarker(match: SourceIdMatch, state: SourceIdState, best: string | null): "current" | "best" | null {
  if (state.id === match.id) return "current";
  if (best === match.id) return "best";
  return null;
}

/** "2018 · PC · ID 113112". */
export function matchDetail(match: SourceIdMatch): string {
  return [match.year === null ? null : String(match.year), match.platforms, `ID ${match.id}`]
    .filter((part): part is string => part !== null && part !== "")
    .join(" · ");
}

/** Texto do aviso depois de mudar uma fonte. */
export type SourceChoiceKind = "id" | "none" | "auto";
