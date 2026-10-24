import { useMemo, useRef, type KeyboardEvent, type MouseEvent } from "react";
import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import {
  BibliotecaCategorias,
  type CategoryGame,
  type GameCategoryGroup,
} from "../../components/BibliotecaCategorias/BibliotecaCategorias";
import { useT } from "../../i18n";
import { cx } from "../../lib/cx";
import { libraryStrings as s, listLabels, listViewStrings as v } from "../strings";
import type { GroupSpec } from "./listaModel";
import type { GroupData } from "./useLibraryGroups";
import styles from "./ModoDaLista.module.css";

interface CategoriasDaListaProps {
  specs: readonly GroupSpec[];
  groups: ReadonlyMap<string, GroupData>;
  loading: boolean;
  /** Porcentagem de download por jogo ("Baixando · n%"). */
  downloading: ReadonlyMap<string, number>;
  /** Capa de exemplo do hospedeiro falso; no app de verdade as capas vêm do núcleo. */
  devCover: ((id: string, title: string) => string) | null;
  activeId: string | null;
  selection: ReadonlySet<string>;
  onSelectionChange: (ids: Set<string>) => void;
  onActivate: (id: string) => void;
  onNearEnd: (groupId: string) => void;
  /** Mensagem do estado vazio quando nenhuma fileira tem jogo. */
  empty: string;
  className?: string;
}

/** O título da fileira: situação e loja saem das tabelas do app, instalação do catálogo, o resto é o nome dado. */
function titleOf(spec: GroupSpec, installedLabel: string, notInstalledLabel: string): string {
  switch (spec.kind) {
    case "status":
      return s.statuses[spec.value as keyof typeof s.statuses];
    case "store":
      return s.stores[spec.value as keyof typeof s.stores];
    case "install":
      return spec.value === "installed" ? installedLabel : notInstalledLabel;
    default:
      return spec.title ?? spec.value;
  }
}

/**
 * As Categorias na coluna: uma fileira de capas por grupo (BibliotecaCategorias). Clique e Enter abrem a página do
 * jogo; Ctrl e Shift + clique marcam como na lista e na grade (a fileira só abre, então o clique com tecla é
 * tomado aqui antes). Esc limpa a seleção e Ctrl + A marca as capas que já chegaram.
 */
export function CategoriasDaLista({
  specs,
  groups,
  loading,
  downloading,
  devCover,
  activeId,
  selection,
  onSelectionChange,
  onActivate,
  onNearEnd,
  empty,
  className,
}: CategoriasDaListaProps) {
  const t = useT();
  const anchor = useRef<string | null>(null);
  const installedLabel = t("desktop.group.installed");
  const notInstalledLabel = t("desktop.group.not_installed");

  const rows = useMemo(() => {
    const toGame = (game: LibraryGame): CategoryGame => {
      const percent = downloading.get(game.id);
      return {
        id: game.id,
        title: game.title,
        hint: percent === undefined ? null : listLabels.downloading(percent),
        accentColor: game.accent,
        hasArt: devCover ? true : game.hasCover,
        coverUrl: devCover ? devCover(game.id, game.title) : null,
        active: game.id === activeId,
        selected: selection.has(game.id),
      };
    };
    return specs.flatMap<GameCategoryGroup>((spec) => {
      const data = groups.get(spec.id);
      if (!data || data.total === 0) return [];
      return [
        {
          id: spec.id,
          title: titleOf(spec, installedLabel, notInstalledLabel),
          countLabel: s.count(data.total),
          games: data.games.map(toGame),
        },
      ];
    });
  }, [specs, groups, downloading, devCover, activeId, selection, installedLabel, notInstalledLabel]);

  // A ordem em que as capas aparecem (sem repetir o jogo que está em mais de uma fileira): base do Shift + clique.
  const order = useMemo(() => [...new Set(rows.flatMap((row) => row.games.map((game) => game.id)))], [rows]);
  const selecting = selection.size > 0;

  const onClickCapture = (event: MouseEvent<HTMLDivElement>) => {
    const id = (event.target as Element).closest("li[data-id]")?.getAttribute("data-id");
    if (!id) return;
    const additive = event.ctrlKey || event.metaKey;
    if (!additive && !event.shiftKey) {
      // Clique simples: sai da seleção e deixa a fileira abrir o jogo.
      if (selecting) onSelectionChange(new Set());
      anchor.current = id;
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const next = new Set(selection);
    if (event.shiftKey) {
      const from = order.indexOf(anchor.current ?? activeId ?? id);
      const to = order.indexOf(id);
      const [low, high] = from < 0 ? [to, to] : from <= to ? [from, to] : [to, from];
      if (!additive) next.clear();
      for (const rangeId of order.slice(low, high + 1)) next.add(rangeId);
    } else {
      if (!selecting && activeId !== null) next.add(activeId);
      if (!next.delete(id)) next.add(id);
      anchor.current = id;
    }
    onSelectionChange(next);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape" && selecting) {
      onSelectionChange(new Set());
    } else if ((event.key === "a" || event.key === "A") && (event.ctrlKey || event.metaKey)) {
      onSelectionChange(new Set(order));
    } else {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
  };

  return (
    <div
      className={cx(styles["categories"], className)}
      aria-busy={loading || undefined}
      onClickCapture={onClickCapture}
      onKeyDown={onKeyDown}
    >
      {loading && rows.length === 0 ? (
        <p className={styles["status"]} role="status">
          {v.loadingGroups}
        </p>
      ) : (
        <BibliotecaCategorias
          groups={rows}
          labels={{
            previous: v.previous,
            next: v.next,
            gamesCount: s.count,
            empty,
            region: v.categoriesRegion,
          }}
          loading={loading}
          onOpen={onActivate}
          onNearEnd={onNearEnd}
        />
      )}
    </div>
  );
}
