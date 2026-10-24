import { useState } from "react";
import { Abas, type Aba } from "../components/Abas/Abas";
import { Aviso } from "../components/Aviso/Aviso";
import { CampoDeBusca } from "../components/CampoDeBusca/CampoDeBusca";
import { Icon } from "../components/Icon/Icon";
import { ListaDeJogos } from "../components/ListaDeJogos/ListaDeJogos";
import { cx } from "../lib/cx";
import { listLabels, strings, libraryStrings as s } from "./strings";
import { hasSavableCriteria, smartRuleFromCriteria } from "./colecoes/colecoesModel";
import { OrganizarColecoes } from "./colecoes/OrganizarColecoes";
import { SalvarColecao } from "./colecoes/SalvarColecao";
import { LibraryFilters, filterCount } from "./LibraryFilters";
import { LibraryBulkActions } from "./LibraryBulkActions";
import type { LibraryCriteria } from "./uiState";
import { useLibrary } from "./useLibrary";
import styles from "./ColunaDaLista.module.css";
import controls from "./LibraryControls.module.css";

const TABS: readonly Aba<LibraryCriteria["filter"]>[] = [
  { id: "all", label: strings.list.tabs.all },
  { id: "favorites", label: strings.list.tabs.favorites },
  { id: "installed", label: strings.list.tabs.installed },
];
interface ColunaDaListaProps {
  shown: boolean;
  collapsed: boolean;
  onToggleCollapsed: () => void;
  criteria: LibraryCriteria;
  onCriteriaChange: (criteria: LibraryCriteria) => void;
  /** O jogo aberto (a página dele); `null` = a Visão geral. Vive no estado de tela, não aqui. */
  activeId: string | null;
  onActiveChange: (id: string | null) => void;
}

/** A coluna consulta a ponte; só critérios, seleção e páginas alcançadas vivem nesta tela. */
export function ColunaDaLista({
  shown,
  collapsed,
  onToggleCollapsed,
  criteria,
  onCriteriaChange,
  activeId,
  onActiveChange,
}: ColunaDaListaProps) {
  const library = useLibrary(criteria);
  const [selection, setSelection] = useState<ReadonlySet<string>>(new Set());
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [notice, setNotice] = useState<{
    error: boolean;
    key: number;
    /** Texto próprio do aviso; sem ele valem os da biblioteca. */
    message?: string;
  } | null>(null);
  // O formulário de "salvar como coleção" recomeça vazio a cada abertura (a chave troca).
  const [saveOpen, setSaveOpen] = useState<number | null>(null);
  const [lastSaveKey, setLastSaveKey] = useState(0);
  const [manageOpen, setManageOpen] = useState(false);
  const update = (patch: Partial<LibraryCriteria>) => {
    setSelection(new Set());
    onCriteriaChange({ ...criteria, ...patch });
  };
  const toggleLabel = collapsed ? strings.list.expand : strings.list.collapse;
  const count = filterCount(criteria);
  const collection = library.collections.find((c) => c.id === criteria.collection);
  const rule = smartRuleFromCriteria(criteria, collection?.criteria ? collection : null);
  const collectionDone = (error: boolean, message: string) => {
    setNotice({ error, key: Date.now(), message });
  };
  const empty =
    criteria.search.trim() || count > Number(criteria.collection !== null)
      ? listLabels.empty
      : criteria.collection
        ? s.emptyCollection
        : criteria.filter === "favorites"
          ? s.emptyFavorites
          : criteria.filter === "installed"
            ? s.emptyInstalled
            : s.emptyLibrary;
  return (
    <>
      <aside
        inert={!shown}
        className={cx(styles["side"], collapsed && styles["collapsed"], !shown && styles["hidden"])}
        aria-label={strings.list.label}
        data-focus-region="list"
      >
        <div className={styles["inner"]}>
          {!collapsed && (
            <Abas
              variant="lista"
              label={strings.list.tabsLabel}
              tabs={TABS}
              value={criteria.filter}
              onChange={(filter) => {
                update({ filter });
              }}
            />
          )}
          <div className={styles["top"]}>
            {!collapsed && (
              <CampoDeBusca
                variant="lista"
                className={cx(styles["search"])}
                value={criteria.search}
                onChange={(search) => {
                  update({ search });
                }}
                placeholder={strings.list.searchPlaceholder}
                label={strings.list.searchLabel}
                action={{
                  label: s.filterCount(count),
                  count,
                  active: count > 0,
                  expanded: filtersOpen,
                  popup: "dialog",
                  onClick: () => {
                    setFiltersOpen(true);
                  },
                }}
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
          <div className={cx(styles["homeLine"], criteria.collection !== null && !collapsed && styles["collectionLine"])}>
            <button
              type="button"
              className={cx(styles["home"], activeId === null && styles["active"])}
              title={collection?.name ?? strings.list.overview}
              aria-label={collection?.name ?? strings.list.overview}
              aria-current={activeId === null ? "true" : undefined}
              onClick={() => {
                onActiveChange(null);
                setSelection(new Set());
              }}
            >
              <span className={styles["homeIcon"]}>
                <Icon name={criteria.collection ? "folder" : "home"} size="sm" />
              </span>
              {!collapsed && (
                <span className={styles["homeLabel"]}>
                  {collection
                    ? s.collectionLabel(collection.name, collection.criteria ? null : collection.gameIds.length)
                    : strings.list.overview}
                </span>
              )}
            </button>
            {!collapsed && criteria.collection && (
              <button
                className={styles["toggle"]}
                aria-label={s.allCollections}
                title={s.allCollections}
                onClick={() => {
                  update({ collection: null });
                }}
              >
                <Icon name="x" size="sm" />
              </button>
            )}
          </div>
          {!collapsed && (
            <div className={controls["count"]} role="status">
              {selection.size ? s.selected(selection.size, library.total) : s.count(library.total)}
              {library.pending && !library.loading ? ` · ${s.busy}` : ""}
            </div>
          )}
          {library.error && (
            <div className={controls["count"]} role="alert">
              {s.loadError}
              <button className={controls["retry"]} onClick={library.refresh}>
                {s.retry}
              </button>
            </div>
          )}
          <ListaDeJogos
            key={library.queryKey}
            className={cx(styles["games"])}
            aria-label={strings.list.label}
            collapsed={collapsed}
            rows={library.rows}
            labels={{ ...listLabels, empty }}
            loading={library.loading}
            activeId={activeId}
            selection={selection}
            onActivate={onActiveChange}
            onSelectionChange={setSelection}
            resolveIcon={library.resolveIcon}
            onNearEnd={library.loadMore}
          />
          {selection.size > 0 && !collapsed && (
            <LibraryBulkActions
              selection={selection}
              allFavorite={[...selection].every((id) => library.games.find((g) => g.id === id)?.favorite)}
              collections={library.collections}
              tags={library.tags}
              clear={() => {
                setSelection(new Set());
              }}
              notify={(error) => {
                setNotice({ error, key: Date.now() });
              }}
            />
          )}
        </div>
      </aside>
      <LibraryFilters
        open={filtersOpen}
        onClose={() => {
          setFiltersOpen(false);
        }}
        criteria={criteria}
        update={update}
        collections={library.collections}
        tags={library.tags}
        canSaveCollection={hasSavableCriteria(criteria)}
        onSaveCollection={() => {
          setFiltersOpen(false);
          setLastSaveKey((n) => n + 1);
          setSaveOpen(lastSaveKey + 1);
        }}
        onManageCollections={() => {
          setFiltersOpen(false);
          setManageOpen(true);
        }}
      />
      <SalvarColecao
        key={saveOpen ?? lastSaveKey}
        open={saveOpen !== null}
        onClose={() => {
          setSaveOpen(null);
        }}
        collections={library.collections}
        rule={rule}
        onDone={collectionDone}
      />
      <OrganizarColecoes
        open={manageOpen}
        onClose={() => {
          setManageOpen(false);
        }}
        collections={library.collections}
        rule={rule}
        onDeleted={(id) => {
          if (criteria.collection === id) update({ collection: null });
        }}
        onDone={collectionDone}
      />
      <Aviso open={library.error} onClose={() => {}} duration={null} tone="error">
        {s.loadError}
      </Aviso>
      <Aviso
        open={notice !== null}
        onClose={() => {
          setNotice(null);
        }}
        noticeKey={notice?.key ?? 0}
        tone={notice?.error ? "error" : "success"}
      >
        {notice?.message ?? (notice?.error ? s.actionError : s.actionSuccess)}
      </Aviso>
    </>
  );
}
