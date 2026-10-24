// Estado de tela que sobrevive ao fechamento da interface (D-10, docs/web/ponte.md §6): guardado no núcleo
// com `ui.state.set` e lido com `ui.state`. Este módulo é só o formato e a leitura; não conhece a ponte
// (sem importações em execução, para o teste rodar direto no Node).

/** As vistas do app. Cada uma é um contêiner por enquanto (as telas são de outras tarefas). */
export const VIEW_IDS = ["biblioteca", "configuracoes", "perfil", "downloads"] as const;
export type ViewId = (typeof VIEW_IDS)[number];

/** As categorias da vista Configurações, na ordem da lista (README da TelaConfiguracoes). */
export const SETTINGS_CATEGORY_IDS = [
  "general",
  "theme",
  "appearance",
  "sound",
  "display",
  "controllers",
  "library",
  "covers",
  "downloads",
  "integrations",
  "system",
  "about",
] as const;
export type SettingsCategoryId = (typeof SETTINGS_CATEGORY_IDS)[number];

import type { LibraryTab } from "../bridge/generated/LibraryTab";
import type { SortOrder } from "../bridge/generated/SortOrder";
import type { Store } from "../bridge/generated/Store";
import type { GameStatus } from "../bridge/generated/GameStatus";

export const STORE_IDS: readonly Store[] = [
  "steam",
  "epic",
  "gog",
  "xbox",
  "ubisoft",
  "ea",
  "battle_net",
  "amazon",
  "heroic",
  "lutris",
  "bottles",
  "itch",
  "emulators",
  "manual",
];
export const STATUS_IDS: readonly GameStatus[] = ["playing", "beaten", "wishlist", "paused", "abandoned"];
export const SORT_IDS: readonly SortOrder[] = [
  "recently_played",
  "name",
  "play_time",
  "recently_added",
  "rating",
  "size",
];
export interface LibraryCriteria {
  filter: LibraryTab;
  search: string;
  sort: SortOrder;
  reversed: boolean;
  collection: string | null;
  stores: Store[];
  statuses: GameStatus[];
  noStatus: boolean;
  tags: string[];
  installed: boolean | null;
}
export const defaultLibraryCriteria: LibraryCriteria = {
  filter: "all",
  search: "",
  sort: "recently_played",
  reversed: false,
  collection: null,
  stores: [],
  statuses: [],
  noStatus: false,
  tags: [],
  installed: null,
};

export function parseLibraryCriteria(raw: unknown): LibraryCriteria {
  const d = typeof raw === "object" && raw !== null && !Array.isArray(raw) ? (raw as Record<string, unknown>) : {};
  const array = <T extends string>(value: unknown, allowed: readonly T[]): T[] =>
    Array.isArray(value) ? allowed.filter((item) => value.includes(item)) : [];
  return {
    filter: d["filter"] === "favorites" || d["filter"] === "installed" ? d["filter"] : "all",
    search: typeof d["search"] === "string" ? d["search"].slice(0, 512) : "",
    sort: SORT_IDS.includes(d["sort"] as SortOrder) ? (d["sort"] as SortOrder) : "recently_played",
    reversed: d["reversed"] === true,
    collection: typeof d["collection"] === "string" ? d["collection"] : null,
    stores: array(d["stores"], STORE_IDS),
    statuses: array(d["statuses"], STATUS_IDS),
    noStatus: d["noStatus"] === true,
    tags: Array.isArray(d["tags"])
      ? [...new Set(d["tags"].filter((t): t is string => typeof t === "string" && t.length > 0))].slice(0, 100)
      : [],
    installed: typeof d["installed"] === "boolean" ? d["installed"] : null,
  };
}

/** O ID do jogo aberto: texto não vazio e de tamanho razoável; qualquer outra coisa é a Visão geral. */
export function parseActiveGame(raw: unknown): string | null {
  return typeof raw === "string" && raw.length > 0 && raw.length <= 512 ? raw : null;
}

export interface ScreenState {
  view: ViewId;
  /** Lista da esquerda recolhida (76) em vez de aberta (292). */
  listCollapsed: boolean;
  library: LibraryCriteria;
  /** O jogo aberto na biblioteca (a página dele); `null` = a Visão geral. */
  activeGame: string | null;
  /** A categoria aberta em Configurações. */
  settingsCategory: SettingsCategoryId;
}

/** Versão do formato. Quem mudar o formato de um jeito que quebra a leitura sobe este número. */
export const SCREEN_STATE_VERSION = 1;

export const defaultScreenState: ScreenState = {
  view: "biblioteca",
  listCollapsed: false,
  library: defaultLibraryCriteria,
  activeGame: null,
  settingsCategory: "general",
};

export function isSettingsCategoryId(value: unknown): value is SettingsCategoryId {
  return typeof value === "string" && (SETTINGS_CATEGORY_IDS as readonly string[]).includes(value);
}

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
    listCollapsed:
      typeof data["listCollapsed"] === "boolean" ? data["listCollapsed"] : defaultScreenState.listCollapsed,
    library: parseLibraryCriteria(data["library"]),
    activeGame: parseActiveGame(data["activeGame"]),
    settingsCategory: isSettingsCategoryId(data["settingsCategory"])
      ? data["settingsCategory"]
      : defaultScreenState.settingsCategory,
  };
}

export function serializeScreenState(state: ScreenState): string {
  return JSON.stringify({ v: SCREEN_STATE_VERSION, ...state });
}
