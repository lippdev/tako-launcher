import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { call } from "../bridge";
import { COMMAND_EVENT } from "../focus";
import { Abas, type Aba } from "../components/Abas/Abas";
import { Aviso } from "../components/Aviso/Aviso";
import { CampoDeBusca } from "../components/CampoDeBusca/CampoDeBusca";
import { Icon } from "../components/Icon/Icon";
import { BibliotecaGrade } from "../components/BibliotecaGrade/BibliotecaGrade";
import { ListaDeJogos } from "../components/ListaDeJogos/ListaDeJogos";
import { cx } from "../lib/cx";
import { useRunningGameId, useSessionActions } from "./sessao/useGameSession";
import { withRunning } from "./libraryRow";
import { listLabels, listViewStrings, strings, libraryStrings as s } from "./strings";
import { hasSavableCriteria, smartRuleFromCriteria } from "./colecoes/colecoesModel";
import { OrganizarColecoes } from "./colecoes/OrganizarColecoes";
import { SalvarColecao } from "./colecoes/SalvarColecao";
import { LibraryFilters, filterCount } from "./LibraryFilters";
import { LibraryBulkActions } from "./LibraryBulkActions";
import { AgruparPor, SeletorDeModo } from "./lista/ModoDaLista";
import { CategoriasDaLista } from "./lista/CategoriasDaLista";
import { GROUP_KEY, VIEW_KEY, parseGroupBy, parseListView, planGroups, type GroupBy, type ListView } from "./lista/listaModel";
import { useLibraryGroups } from "./lista/useLibraryGroups";
import { useSettings } from "./settings/useSettings";
import { STORE_IDS, type LibraryCriteria } from "./uiState";
import { useLibrary } from "./useLibrary";
import styles from "./ColunaDaLista.module.css";
import controls from "./LibraryControls.module.css";

const TABS: readonly Aba<LibraryCriteria["filter"]>[] = [
  { id: "all", label: strings.list.tabs.all },
  { id: "favorites", label: strings.list.tabs.favorites },
  { id: "installed", label: strings.list.tabs.installed },
];
/** O modo de exibição e o agrupamento das Categorias: as chaves do núcleo que o Slint também usa. */
const MODE_KEYS = [VIEW_KEY, GROUP_KEY];

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
  const runningId = useRunningGameId();
  const sessionActions = useSessionActions();
  // O jogo aberto ganha o ponto "Jogando" na lista e na grade (o núcleo diz qual; a lista só pinta).
  const rows = useMemo(() => withRunning(library.rows, runningId), [library.rows, runningId]);
  const prefs = useSettings(MODE_KEYS);
  // Recolhida a coluna só tem os ícones: a grade e as categorias esperam ela abrir (como o Slint).
  const view: ListView = collapsed ? "list" : parseListView(prefs.values[VIEW_KEY]);
  const groupBy = parseGroupBy(prefs.values[GROUP_KEY]);
  const specs = useMemo(
    () => planGroups(groupBy, library.request, { collections: library.collections, tags: library.tags, stores: STORE_IDS }),
    [groupBy, library.request, library.collections, library.tags],
  );
  const groups = useLibraryGroups(view === "categories", library.request, specs);
  const chooseView = (next: ListView) => {
    void prefs.save(VIEW_KEY, next);
  };
  const chooseGroup = (next: GroupBy) => {
    void prefs.save(GROUP_KEY, next);
  };
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
  const asideRef = useRef<HTMLElement>(null);


  const update = (patch: Partial<LibraryCriteria>) => {
    setSelection(new Set());
    onCriteriaChange({ ...criteria, ...patch });
  };
  const toggleLabel = collapsed ? strings.list.expand : strings.list.collapse;
  const count = filterCount(criteria);
  const collection = library.collections.find((c) => c.id === criteria.collection);
  const rule = smartRuleFromCriteria(criteria, collection?.criteria ? collection : null);
  // O ▶ da grade: abre o jogo; no que já está aberto, volta a ele.
  const play = async (id: string) => {
    if (id === runningId) {
      void sessionActions.resume();
      return;
    }
    const game = knownGame(id);
    const feedback = await sessionActions.play({ id, title: game?.title ?? "" });
    // O aviso de "abrindo" é do próprio botão (Abrindo…); só o que deu errado precisa de texto aqui.
    if (feedback.tone === "error") collectionDone(true, feedback.text);
  };
  // Os jogos que a tela conhece: a página da lista e as capas das fileiras (a seleção pode vir de qualquer uma).
  const knownGame = useCallback(
    (id: string) =>
      library.games.find((g) => g.id === id) ??
      [...groups.groups.values()].flatMap((g) => g.games).find((g) => g.id === id),
    [library.games, groups.groups],
  );
  const collectionDone = useCallback((error: boolean, message: string) => {
    setNotice({ error, key: Date.now(), message });
  }, []);
  useEffect(() => {
    const el = asideRef.current;
    if (!el) return;
    const onCommand = (e: Event) => {
      const ce = e as CustomEvent<{ command: "favorite" | "menu" }>;
      const targetIds = selection.size > 0 ? [...selection] : activeId ? [activeId] : [];
      if (ce.detail?.command === "favorite") {
        if (targetIds.length === 0) return;
        ce.preventDefault();
        const allFav = targetIds.every((id) => knownGame(id)?.favorite);
        void call("game.favorite.set", { gameIds: targetIds, favorite: !allFav })
          .then(() => {
            library.refresh();
          })
          .catch(() => {
            collectionDone(true, s.loadError);
          });
      }
      // Menu na linha: no Slint abre o Gerenciar daquele jogo (`Action::More`); a linha da web ainda não tem esse
      // menu. TODO(WEB-05b): ligar quando houver.

    };
    el.addEventListener(COMMAND_EVENT, onCommand);
    return () => el.removeEventListener(COMMAND_EVENT, onCommand);
  }, [selection, activeId, library, knownGame, collectionDone]);
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
        ref={asideRef}
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
            <div className={controls["countLine"]}>
              <div className={controls["count"]} role="status">
                {selection.size ? s.selected(selection.size, library.total) : s.count(library.total)}
                {library.pending && !library.loading ? ` · ${s.busy}` : ""}
              </div>
              <SeletorDeModo view={view} onViewChange={chooseView} />
            </div>
          )}
          {view === "categories" && <AgruparPor value={groupBy} onChange={chooseGroup} />}
          {library.error && (
            <div className={controls["count"]} role="alert">
              {s.loadError}
              <button className={controls["retry"]} onClick={library.refresh}>
                {s.retry}
              </button>
            </div>
          )}
          {!prefs.ready ? (
            <div className={cx(styles["games"])} />
          ) : view === "grid" ? (
            <BibliotecaGrade
              key={library.queryKey}
              className={cx(styles["games"])}
              aria-label={strings.list.label}
              rows={rows}
              labels={{ ...listLabels, empty, play: listViewStrings.play }}
              loading={library.loading}
              activeId={activeId}
              selection={selection}
              onActivate={onActiveChange}
              onSelectionChange={setSelection}
              onPlay={(id) => void play(id)}
              resolveCover={library.resolveCover}
              resolveIcon={library.resolveIcon}
              onNearEnd={library.loadMore}
            />
          ) : view === "categories" ? (
            <CategoriasDaLista
              className={cx(styles["games"])}
              specs={specs}
              groups={groups.groups}
              loading={groups.loading}
              downloading={library.downloading}
              devCover={library.devCover}
              activeId={activeId}
              selection={selection}
              onSelectionChange={setSelection}
              onActivate={onActiveChange}
              onNearEnd={groups.loadMore}
              empty={empty}
            />
          ) : (
            <ListaDeJogos
              key={library.queryKey}
              className={cx(styles["games"])}
              aria-label={strings.list.label}
              collapsed={collapsed}
              rows={rows}
              labels={{ ...listLabels, empty }}
              loading={library.loading}
              activeId={activeId}
              selection={selection}
              onActivate={onActiveChange}
              onSelectionChange={setSelection}
              resolveIcon={library.resolveIcon}
              onNearEnd={library.loadMore}
            />
          )}
          {selection.size > 0 && !collapsed && (
            <LibraryBulkActions
              selection={selection}
              allFavorite={[...selection].every((id) => knownGame(id)?.favorite)}
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
