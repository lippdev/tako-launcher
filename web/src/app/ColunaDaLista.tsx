import { useMemo, useState } from "react";
import { Abas, type Aba } from "../components/Abas/Abas";
import { CampoDeBusca } from "../components/CampoDeBusca/CampoDeBusca";
import { Icon } from "../components/Icon/Icon";
import { ListaDeJogos, type GameRow } from "../components/ListaDeJogos/ListaDeJogos";
import { cx } from "../lib/cx";
import { listLabels, strings } from "./strings";
import styles from "./ColunaDaLista.module.css";

type Filter = "all" | "favorites" | "installed";

const TABS: readonly Aba<Filter>[] = [
  { id: "all", label: strings.list.tabs.all },
  { id: "favorites", label: strings.list.tabs.favorites },
  { id: "installed", label: strings.list.tabs.installed },
];

interface ColunaDaListaProps {
  /** Só a biblioteca mostra a lista; nas outras vistas a coluna some (largura 0). */
  shown: boolean;
  /** Recolhida (76, só ícones) em vez de aberta (292). */
  collapsed: boolean;
  onToggleCollapsed: () => void;
  rows: readonly GameRow[];
  resolveIcon?: (id: string, px: number) => string | null;
}

/**
 * A coluna da esquerda da biblioteca (README da TelaBiblioteca): abas, busca com o botão de recolher, "Visão
 * geral" e a ListaDeJogos. Por enquanto o estado é só dela: abrir um jogo, jogar e ligar à biblioteca de verdade
 * são de outras tarefas.
 */
export function ColunaDaLista({ shown, collapsed, onToggleCollapsed, rows, resolveIcon }: ColunaDaListaProps) {
  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [activeId, setActiveId] = useState<string | null>(null);
  const [selection, setSelection] = useState<ReadonlySet<string>>(new Set());

  const visibleRows = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase("pt-BR");
    return rows.filter(
      (row) =>
        (filter === "all" || (filter === "favorites" ? row.favorite : row.installed)) &&
        row.title.toLocaleLowerCase("pt-BR").includes(needle),
    );
  }, [rows, filter, query]);

  const toggleLabel = collapsed ? strings.list.expand : strings.list.collapse;

  return (
    <aside
      // Escondida, a coluna sai da ordem do Tab, do clique e do leitor de tela, mas fica no DOM até o fade acabar.
      inert={!shown}
      className={cx(styles["side"], collapsed && styles["collapsed"], !shown && styles["hidden"])}
      aria-label={strings.list.label}
    >
      <div className={styles["inner"]}>
        {!collapsed && (
          <Abas variant="lista" label={strings.list.tabsLabel} tabs={TABS} value={filter} onChange={setFilter} />
        )}
        <div className={styles["top"]}>
          {!collapsed && (
            <CampoDeBusca
              variant="lista"
              className={cx(styles["search"])}
              value={query}
              onChange={setQuery}
              placeholder={strings.list.searchPlaceholder}
              label={strings.list.searchLabel}
            />
          )}
          <button
            type="button"
            className={styles["toggle"]}
            aria-label={toggleLabel}
            aria-keyshortcuts="Control+B"
            title={`${toggleLabel} · ${strings.list.shortcut}`}
            onClick={onToggleCollapsed}
          >
            <Icon name="panel" size="sm" />
          </button>
        </div>
        <button
          type="button"
          className={cx(styles["home"], activeId === null && styles["active"])}
          title={strings.list.overview}
          aria-label={strings.list.overview}
          aria-current={activeId === null ? "true" : undefined}
          onClick={() => {
            setActiveId(null);
            setSelection(new Set());
          }}
        >
          <span className={styles["homeIcon"]}>
            <Icon name="home" size="sm" />
          </span>
          {!collapsed && <span className={styles["homeLabel"]}>{strings.list.overview}</span>}
        </button>
        <ListaDeJogos
          className={cx(styles["games"])}
          aria-label={strings.list.label}
          collapsed={collapsed}
          rows={visibleRows}
          labels={listLabels}
          activeId={activeId}
          selection={selection}
          onActivate={setActiveId}
          onSelectionChange={setSelection}
          {...(resolveIcon ? { resolveIcon } : {})}
        />
      </div>
    </aside>
  );
}
