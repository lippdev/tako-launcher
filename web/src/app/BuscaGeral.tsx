import { useEffect, useId, useMemo, useRef, useState } from "react";
import mark from "../assets/brand/tako-mark.png";
import { Aviso } from "../components/Aviso/Aviso";
import { CampoDeBusca } from "../components/CampoDeBusca/CampoDeBusca";
import { Icon, type IconName } from "../components/Icon/Icon";
import { useEscapeLayer } from "../lib/escapeLayers";
import { useFocusTrap } from "../lib/focusTrap";
import { Portal } from "../lib/Portal";
import { usePresence } from "../lib/presence";
import { useImageVersion } from "../lib/useImageVersion";
import { useT } from "../i18n";
import { imageUrl } from "../lib/imageVersions";
import {
  buildResults,
  buildShortcuts,
  clampSelection,
  initial,
  matchShortcuts,
  moveSelection,
  type SearchResult,
  type ShortcutTarget,
} from "./busca/buscaModel";
import { CATEGORY_ICONS } from "./settings/categories";
import { useBusca } from "./busca/useBusca";
import { useDevArt } from "./busca/useDevArt";
import type { ViewId } from "./uiState";
import styles from "./BuscaGeral.module.css";

interface BuscaGeralProps {
  open: boolean;
  onClose: () => void;
  /** Cada Ctrl + K aumenta o número: com a busca já aberta, devolve o foco ao campo. */
  focusToken: number;
  /** Enter ou clique num jogo: abre a página dele. A busca já foi fechada. */
  onOpenGame: (id: string) => void;
  /** Enter ou clique num atalho: leva à vista (e à categoria, nas Configurações). A busca já foi fechada. */
  onOpenShortcut: (target: ShortcutTarget) => void;
}

/** `tako-img://` do núcleo (docs/web/ponte.md §4), em pixels físicos (o dobro do que se vê). */
function artUrl(kind: "cover" | "icon", id: string, w: number, h: number): string {
  return imageUrl(kind, id, w, h);
}

const VIEW_ICONS: Record<ViewId, IconName> = { biblioteca: "home", configuracoes: "gear", perfil: "user", downloads: "download" };

/** A arte da linha: o ícone do destino num atalho; num jogo, capa, senão ícone, senão a inicial (também se a imagem não carrega). */
function Arte({ result, art }: { result: SearchResult; art: ReturnType<typeof useDevArt> }) {
  const version = useImageVersion(result.kind === "game" ? result.id : undefined);
  const key = `${result.id}:${String(version)}`;
  const [failed, setFailed] = useState<string | null>(null);
  if (result.kind === "shortcut") {
    const { view, settingsCategory } = result.target;
    return (
      <span className={styles["art"]} data-fallback="" aria-hidden="true">
        <Icon name={settingsCategory ? CATEGORY_ICONS[settingsCategory] : VIEW_ICONS[view]} />
      </span>
    );
  }
  if (failed !== key) {
    const { game } = result;
    const src = game.hasCover
      ? (art?.cover(game.id, game.title) ?? artUrl("cover", game.id, 80, 108))
      : game.hasIcon
        ? (art?.icon(game.id, 80) ?? artUrl("icon", game.id, 80, 80))
        : null;
    if (src) {
      return (
        <img
          className={styles["art"]}
          src={src}
          alt=""
          width={40}
          height={54}
          draggable={false}
          onError={() => {
            setFailed(key);
          }}
        />
      );
    }
  }
  return (
    <span className={styles["art"]} data-fallback="" aria-hidden="true">
      {initial(result.title)}
    </span>
  );
}

/**
 * A busca geral (Ctrl + K), independente dos filtros e da aba da biblioteca: jogos (uma página pequena da ponte, com
 * debounce) e atalhos para as telas e categorias das Configurações. ↑ ↓ movem a seleção, Enter abre, Esc fecha.
 * O foco fica no campo; a seleção é anunciada por `aria-activedescendant`.
 */
export function BuscaGeral({ open, onClose, focusToken, onOpenGame, onOpenShortcut }: BuscaGeralProps) {
  const t = useT();
  const veil = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const listId = useId();
  const [query, setQuery] = useState("");
  const [rawSelected, setRawSelected] = useState(0);
  const { mounted, visible, animating } = usePresence(open, veil);
  const art = useDevArt();
  const { games, loading, error } = useBusca(open, query);

  const shortcuts = useMemo(
    () =>
      buildShortcuts({
        views: {
          biblioteca: t("desktop.library"),
          configuracoes: t("desktop.settings"),
          perfil: t("profile.tab"),
          downloads: t("downloads.title"),
        },
        categories: {
          general: t("settings.general.title"),
          theme: t("settings.theme.title"),
          appearance: t("settings.appearance.title"),
          sound: t("settings.sound.title"),
          display: t("settings.screen.title"),
          controllers: t("settings.controllers.title"),
          library: t("settings.library.title"),
          covers: t("settings.covers.title"),
          downloads: t("downloads.title"),
          integrations: t("settings.integrations.title"),
          system: t("settings.system.title"),
          about: t("settings.about.title"),
        },
        settingsGroup: t("desktop.settings"),
        viewsGroup: t("desktop.quick_search.views_group"),
      }),
    [t],
  );

  const resultLabels = useMemo(
    () => ({
      stores: {
        steam: t("sources.store.steam"),
        epic: t("sources.store.epic"),
        gog: t("sources.store.gog"),
        xbox: t("sources.store.xbox"),
        ubisoft: t("sources.store.ubisoft"),
        ea: t("sources.store.ea"),
        battle_net: t("sources.store.battle_net"),
        amazon: t("sources.store.amazon"),
        heroic: t("sources.store.heroic"),
        lutris: t("sources.store.lutris"),
        bottles: t("sources.store.bottles"),
        itch: t("sources.store.itch"),
        emulators: t("sources.store.emulators"),
        manual: t("sources.store.manual"),
      },
      installed: t("game_page.installed"),
      notInstalled: t("game_page.not_installed"),
    }),
    [t],
  );

  // Cada abertura começa do zero, como o app atual (uma busca nova a cada Ctrl + K).
  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setQuery("");
      setRawSelected(0);
    }
  }

  const results = useMemo(() => buildResults(games, matchShortcuts(shortcuts, query), resultLabels), [games, shortcuts, query, resultLabels]);
  const selected = clampSelection(rawSelected, results.length);
  const optionId = (index: number) => `${listId}-${String(index)}`;

  useEscapeLayer(open, onClose);
  useFocusTrap(open, box);

  useEffect(() => {
    if (open) input.current?.focus({ preventScroll: true });
  }, [open, focusToken]);

  // A linha selecionada fica à vista quando a lista rola.
  useEffect(() => {
    if (!open) return;
    document.getElementById(`${listId}-${String(selected)}`)?.scrollIntoView({ block: "nearest" });
  }, [open, selected, listId, results]);

  const pick = (result: SearchResult | undefined) => {
    if (!result) return;
    onClose();
    if (result.kind === "game") onOpenGame(result.id);
    else onOpenShortcut(result.target);
  };

  if (!mounted) return null;

  const settled = !loading && !error;
  const showEmpty = settled && results.length === 0;

  return (
    <Portal>
      <div
        ref={veil}
        data-visible={visible}
        data-animating={animating}
        className={styles["veil"]}
        onMouseDown={(event) => {
          if (event.target === event.currentTarget) onClose();
        }}
      >
        <div ref={box} role="dialog" aria-modal="true" aria-label={t("desktop.quick_search.title")} tabIndex={-1} className={styles["panel"]}>
          <CampoDeBusca
            variant="geral"
            inputRef={input}
            value={query}
            onChange={(value) => {
              setQuery(value);
              setRawSelected(0);
            }}
            placeholder={t("desktop.quick_search.placeholder")}
            label={t("desktop.quick_search.label")}
            hint={t("desktop.search_esc")}
            keyShortcuts={t("desktop.quick_search.key_shortcuts")}
            role="combobox"
            aria-autocomplete="list"
            aria-expanded={results.length > 0}
            aria-controls={listId}
            {...(results.length > 0 ? { "aria-activedescendant": optionId(selected) } : {})}
            onKeyDown={(event) => {
              if (event.nativeEvent.isComposing) return;
              if (event.key === "ArrowDown" || event.key === "ArrowUp") {
                event.preventDefault();
                setRawSelected(moveSelection(selected, event.key === "ArrowDown" ? 1 : -1, results.length));
              } else if (event.key === "Enter") {
                event.preventDefault();
                pick(results[selected]);
              }
            }}
          />
          <div id={listId} role="listbox" aria-label={t("desktop.quick_search.results")} className={styles["results"]} hidden={results.length === 0}>
            {results.map((result, index) => (
              <div
                key={result.id}
                id={optionId(index)}
                role="option"
                aria-selected={index === selected}
                data-selected={index === selected}
                className={styles["result"]}
                onMouseDown={(event) => {
                  // O foco fica no campo.
                  event.preventDefault();
                }}
                onMouseMove={() => {
                  if (index !== selected) setRawSelected(index);
                }}
                onClick={() => {
                  pick(result);
                }}
              >
                <Arte result={result} art={art} />
                <span className={styles["text"]}>
                  <strong className={styles["title"]}>{result.title}</strong>
                  <span className={styles["meta"]}>{result.meta}</span>
                </span>
                <em className={styles["tag"]}>{result.kind === "game" ? t("desktop.global_search_library") : t("desktop.quick_search.tag_shortcut")}</em>
              </div>
            ))}
          </div>
          {showEmpty && (
            <p className={styles["empty"]}>
              <img className={styles["emptyMark"]} src={mark} alt="" />
              {t("desktop.global_search_empty")}
            </p>
          )}
          <p role="status" className={styles["live"]}>
            {showEmpty ? t("desktop.global_search_empty") : settled ? t("desktop.quick_search.count", { n: results.length }) : ""}
          </p>
        </div>
      </div>
      <Aviso open={open && error} onClose={() => {}} duration={null} tone="error">
        {t("desktop.quick_search.error")}
      </Aviso>
    </Portal>
  );
}
