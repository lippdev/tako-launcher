// Regras puras dos modos de exibição da lista da biblioteca (WEB-11c): os três modos, o agrupamento das Categorias e
// o pedido de cada fileira. Sem importações em execução, para o teste rodar direto no Node. Espelha o Slint
// (`crates/tako-ui/src/shell/desktop.rs`: `View`, `GroupBy`, `groups`).

import type { Collection } from "../../bridge/generated/Collection";
import type { GameStatus } from "../../bridge/generated/GameStatus";
import type { Store } from "../../bridge/generated/Store";
import type { Tag } from "../../bridge/generated/Tag";
import type { LibraryCriteria } from "../uiState";

/** Chaves das configurações do núcleo, as mesmas do Slint (`VIEW_KEY` e `GROUP_KEY`). */
export const VIEW_KEY = "desktop.view";
export const GROUP_KEY = "desktop.group_by";

export const LIST_VIEWS = ["list", "grid", "categories"] as const;
export type ListView = (typeof LIST_VIEWS)[number];

/** Os agrupamentos das Categorias, como no Slint. */
export const GROUP_BYS = ["status", "collection", "tag", "store", "install", "genre"] as const;
export type GroupBy = (typeof GROUP_BYS)[number];

/** Ordem das situações nas Categorias (a do Slint). */
export const GROUP_STATUS_ORDER: readonly GameStatus[] = ["playing", "wishlist", "paused", "beaten", "abandoned"];

/** Capas pedidas por página de uma fileira (no Slint a fileira corta em 24; aqui a rolagem pede mais). */
export const ROW_PAGE = 24;

/** Valor guardado → modo. Ausente, vazio ou desconhecido é a Lista. */
export function parseListView(raw: string | undefined): ListView {
  return LIST_VIEWS.find((view) => view === raw) ?? "list";
}

/** Valor guardado → agrupamento. Ausente ou desconhecido é a Situação, o padrão do Slint. */
export function parseGroupBy(raw: string | undefined): GroupBy {
  return GROUP_BYS.find((group) => group === raw) ?? "status";
}

/** O que muda no pedido de `library.list` para listar só uma fileira. */
export type GroupRequest = Pick<LibraryCriteria, "statuses" | "noStatus" | "tags" | "stores" | "collection" | "installed"> & { genres?: string[] };

/** Uma fileira das Categorias, antes de ter jogos. */
export interface GroupSpec {
  /** Estável entre pedidos: serve de chave da fileira. */
  id: string;
  kind: GroupBy;
  /** O valor do grupo: situação, id da coleção, etiqueta, loja ou `installed` / `not_installed`. */
  value: string;
  /** Nome da fileira; `null` quando o texto vem do catálogo (instalação) ou das tabelas do app (situação, loja). */
  title: string | null;
  /** O pedido da fileira: os critérios da tela com o do grupo por cima. */
  request: GroupRequest;
}

export interface GroupSources {
  collections: readonly Collection[];
  tags: readonly Tag[];
  stores: readonly Store[];
}

const pick = (c: LibraryCriteria): GroupRequest => ({
  statuses: c.statuses,
  noStatus: c.noStatus,
  tags: c.tags,
  stores: c.stores,
  collection: c.collection,
  installed: c.installed,
});

/**
 * As fileiras do agrupamento, na ordem do Slint, já respeitando o que a tela filtra (uma fileira que o filtro esvazia
 * nem é pedida). Fileira vazia depois do pedido (coleção sem jogo, filtro de busca) o chamador esconde.
 */
export function planGroups(by: GroupBy, criteria: LibraryCriteria, sources: GroupSources): GroupSpec[] {
  const base = pick(criteria);
  const spec = (value: string, title: string | null, patch: Partial<GroupRequest>): GroupSpec => ({
    id: `${by}:${value}`,
    kind: by,
    value,
    title,
    request: { ...base, ...patch },
  });
  switch (by) {
    case "genre":
      // A fila de fileiras consulta os gêneros dos jogos filtrados antes de pedir as capas.
      return [spec("*", null, {})];
    case "status":
      // Filtro "sem situação" sozinho não deixa jogo com situação; com situações escolhidas, só elas.
      return GROUP_STATUS_ORDER.filter((status) =>
        criteria.statuses.length > 0 ? criteria.statuses.includes(status) : !criteria.noStatus,
      ).map((status) => spec(status, null, { statuses: [status], noStatus: false }));
    case "collection":
      // Com uma coleção aberta só ela tem o que mostrar: a lista vale uma coleção por vez.
      return sources.collections
        .filter((c) => criteria.collection === null || c.id === criteria.collection)
        .map((c) => spec(c.id, c.name, { collection: c.id }));
    case "tag":
      return sources.tags
        .filter((tag) => tag.count > 0)
        .map((tag) => spec(tag.name, tag.name, { tags: [...new Set([...criteria.tags, tag.name])] }));
    case "store":
      return sources.stores
        .filter((store) => criteria.stores.length === 0 || criteria.stores.includes(store))
        .map((store) => spec(store, null, { stores: [store] }));
    case "install":
      return [true, false]
        .filter((installed) => criteria.installed === null || criteria.installed === installed)
        .map((installed) => spec(installed ? "installed" : "not_installed", null, { installed }));
  }
}
