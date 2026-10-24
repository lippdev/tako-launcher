// A parte pura da busca geral (Ctrl + K): juntar jogos e atalhos, ordenar, mover a seleção. Sem React e sem ponte
// (só tipos), para o teste rodar direto no Node. O que entra nos resultados segue `shell/global_search.rs` do app
// atual (jogos da biblioteca inteira, sem os filtros da lista), mais os atalhos de tela pedidos no WEB-14a.

import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import type { SettingsCategoryId, ViewId } from "../uiState";

/** Quantos jogos a busca pede à ponte: a página é pequena, nunca a biblioteca inteira. */
export const GAME_LIMIT = 8;
/** Quantos atalhos de tela entram junto, no máximo. */
export const SHORTCUT_LIMIT = 5;

/** Para onde um atalho leva. */
export interface ShortcutTarget {
  view: ViewId;
  /** Só na vista Configurações: a categoria que abre. */
  settingsCategory?: SettingsCategoryId;
}

/** Um atalho que o catálogo conhece (título e grupo já traduzidos). */
export interface Shortcut {
  id: string;
  title: string;
  /** De onde é (ex.: "Configurações" para uma categoria); fica na linha, sob o título. */
  group: string;
  target: ShortcutTarget;
}

export interface GameResult {
  kind: "game";
  id: string;
  title: string;
  /** Loja e situação de instalação: "Steam · Instalado". */
  meta: string;
  game: LibraryGame;
}

export interface ShortcutResult {
  kind: "shortcut";
  id: string;
  title: string;
  meta: string;
  target: ShortcutTarget;
}

export type SearchResult = GameResult | ShortcutResult;

/** Minúsculas e sem acento: "configuracoes" encontra "Configurações". */
export function normalize(text: string): string {
  return text
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLocaleLowerCase("pt-BR")
    .trim();
}

export interface ShortcutLabels {
  /** Título de cada vista. */
  views: Record<ViewId, string>;
  /** Título de cada categoria das Configurações. */
  categories: Record<SettingsCategoryId, string>;
  /** O grupo das categorias ("Configurações"). */
  settingsGroup: string;
  /** O grupo das vistas ("Ir para"). */
  viewsGroup: string;
}

// A vista "downloads" ficou sem tela (os downloads vivem no painel da barra e em Configurações › Downloads).
const VIEW_ORDER: readonly ViewId[] = ["biblioteca", "configuracoes", "perfil"];
const CATEGORY_ORDER: readonly SettingsCategoryId[] = [
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
];

/** Todos os atalhos: as vistas e, depois, cada categoria das Configurações, na ordem da lista delas. */
export function buildShortcuts(labels: ShortcutLabels): Shortcut[] {
  return [
    ...VIEW_ORDER.map((view) => ({
      id: `view:${view}`,
      title: labels.views[view],
      group: labels.viewsGroup,
      target: { view },
    })),
    ...CATEGORY_ORDER.map((category) => ({
      id: `settings:${category}`,
      title: labels.categories[category],
      group: labels.settingsGroup,
      target: { view: "configuracoes" as const, settingsCategory: category },
    })),
  ];
}

/**
 * Os atalhos que combinam com o texto, melhores primeiro: título que começa com o texto, depois título que o
 * contém, depois grupo + título ("configuracoes tema"). Dentro de cada grau vale a ordem do catálogo. Texto vazio
 * não traz atalho: a busca vazia mostra só os jogos, como o app atual.
 */
export function matchShortcuts(shortcuts: readonly Shortcut[], query: string, limit: number = SHORTCUT_LIMIT): Shortcut[] {
  const needle = normalize(query);
  if (needle === "") return [];
  const ranked: { shortcut: Shortcut; rank: number; order: number }[] = [];
  shortcuts.forEach((shortcut, order) => {
    const title = normalize(shortcut.title);
    let rank: number;
    if (title.startsWith(needle)) rank = 0;
    else if (title.includes(needle)) rank = 1;
    else if (normalize(`${shortcut.group} ${shortcut.title}`).includes(needle)) rank = 2;
    else return;
    ranked.push({ shortcut, rank, order });
  });
  ranked.sort((a, b) => a.rank - b.rank || a.order - b.order);
  return ranked.slice(0, limit).map((r) => r.shortcut);
}

export interface ResultLabels {
  stores: Record<LibraryGame["store"], string>;
  installed: string;
  notInstalled: string;
}

export function gameResult(game: LibraryGame, labels: ResultLabels): GameResult {
  return {
    kind: "game",
    id: game.id,
    title: game.title,
    meta: `${labels.stores[game.store]} · ${game.installed ? labels.installed : labels.notInstalled}`,
    game,
  };
}

/** Os resultados na ordem da tela: primeiro os jogos que a ponte devolveu (já ordenados por ela), depois os atalhos. */
export function buildResults(
  games: readonly LibraryGame[],
  shortcuts: readonly Shortcut[],
  gameLabels: ResultLabels,
): SearchResult[] {
  return [
    ...games.map((game) => gameResult(game, gameLabels)),
    ...shortcuts.map<ShortcutResult>((shortcut) => ({
      kind: "shortcut",
      id: shortcut.id,
      title: shortcut.title,
      meta: shortcut.group,
      target: shortcut.target,
    })),
  ];
}

/** ↑ e ↓ andam uma linha e param nas pontas (sem dar a volta), como no app atual. */
export function moveSelection(index: number, delta: -1 | 1, count: number): number {
  if (count <= 0) return 0;
  return Math.min(Math.max(index + delta, 0), count - 1);
}

/** A seleção continua válida quando a lista muda de tamanho. */
export function clampSelection(index: number, count: number): number {
  return count <= 0 ? 0 : Math.min(Math.max(index, 0), count - 1);
}

/** A inicial do jogo, para quando não há imagem. */
export function initial(title: string): string {
  const first = [...title.trim()][0];
  return first ? first.toLocaleUpperCase("pt-BR") : "";
}
