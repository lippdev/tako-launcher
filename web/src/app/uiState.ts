// Estado de tela que sobrevive ao fechamento da interface (D-10, docs/web/ponte.md §6): guardado no núcleo
// com `ui.state.set` e lido com `ui.state`. Este módulo é só o formato e a leitura; não conhece a ponte
// (e não importa nada, para o teste rodar direto no Node).

/** As vistas do app. Cada uma é um contêiner por enquanto (as telas são de outras tarefas). */
export const VIEW_IDS = ["biblioteca", "configuracoes", "perfil", "downloads"] as const;
export type ViewId = (typeof VIEW_IDS)[number];

export interface ScreenState {
  view: ViewId;
  /** Lista da esquerda recolhida (76) em vez de aberta (292). */
  listCollapsed: boolean;
}

/** Versão do formato. Quem mudar o formato de um jeito que quebra a leitura sobe este número. */
export const SCREEN_STATE_VERSION = 1;

export const defaultScreenState: ScreenState = { view: "biblioteca", listCollapsed: false };

export function isViewId(value: unknown): value is ViewId {
  return typeof value === "string" && (VIEW_IDS as readonly string[]).includes(value);
}

/**
 * Lê o JSON guardado. Nunca lança: ausente, quebrado, de outra versão ou com campo desconhecido cai no padrão
 * (campo a campo: um valor ruim não derruba os bons).
 */
export function parseScreenState(json: string | null | undefined): ScreenState {
  if (!json) return defaultScreenState;
  let raw: unknown;
  try {
    raw = JSON.parse(json);
  } catch {
    return defaultScreenState;
  }
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return defaultScreenState;
  const data = raw as Record<string, unknown>;
  if (data["v"] !== SCREEN_STATE_VERSION) return defaultScreenState;
  return {
    view: isViewId(data["view"]) ? data["view"] : defaultScreenState.view,
    listCollapsed: typeof data["listCollapsed"] === "boolean" ? data["listCollapsed"] : defaultScreenState.listCollapsed,
  };
}

export function serializeScreenState(state: ScreenState): string {
  return JSON.stringify({ v: SCREEN_STATE_VERSION, view: state.view, listCollapsed: state.listCollapsed });
}
