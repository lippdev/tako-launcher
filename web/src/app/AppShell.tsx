import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { AvisosDoControle, useSpatialNavigation } from "../focus";
import { useT } from "../i18n";
import { BarraSuperior } from "./BarraSuperior";
import { BuscaGeral } from "./BuscaGeral";
import { ColunaDaLista } from "./ColunaDaLista";
import { PaginaDoJogo } from "./jogo/PaginaDoJogo";
import { TelaPerfil } from "./perfil/TelaPerfil";
import { TelaConfiguracoes } from "./settings/TelaConfiguracoes";
import { CamadasDaSessao } from "./sessao/CamadasDaSessao";
import { CentralRapida } from "./central/CentralRapida";
import { useSavedAppearance } from "./settings/useSavedAppearance";
import { VisaoGeral } from "./visao/VisaoGeral";
import { VIEW_IDS, defaultLibraryCriteria, type ViewId } from "./uiState";
import { useScreenState } from "./useScreenState";
import { Vista } from "./Vista";
import { windowAction } from "./windowActions";
import styles from "./AppShell.module.css";

const VIEW_TITLE_KEYS: Record<ViewId, string> = {
  biblioteca: "desktop.library",
  configuracoes: "desktop.settings",
  perfil: "profile.tab",
  downloads: "downloads.title",
};

/**
 * O app desktop: barra de cima de borda a borda, a lista da biblioteca à esquerda e a área das vistas
 * (README do design system, "A referência: TelaBiblioteca"). Perfil e Downloads são contêineres vazios por enquanto.
 */
export function AppShell() {
  const t = useT();
  const { state, ready: screenReady, update } = useScreenState();
  // A aparência salva (tema, animações, vidro, tamanho) é lida na abertura, antes de mostrar a janela.
  const appearanceReady = useSavedAppearance();
  const ready = screenReady && appearanceReady;
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchFocus, setSearchFocus] = useState(0);
  const mainRef = useRef<HTMLElement>(null);

  const { view, listCollapsed, activeGame } = state;
  const listShown = view === "biblioteca";

  const profile = { name: t("profile.tab"), online: true };

  const navigate = useCallback(
    (next: ViewId) => {
      update({ view: next });
    },
    [update],
  );
  // Escolher um jogo na lista abre a página dele; a Visão geral (ou um jogo que saiu da biblioteca) volta ao conteúdo da vista.
  const openGame = useCallback(
    (id: string | null) => {
      update({ activeGame: id });
    },
    [update],
  );
  const toggleList = useCallback(() => {
    update({ listCollapsed: !listCollapsed });
  }, [update, listCollapsed]);

  // Navegação por direção (teclado hoje, controle depois). "Voltar" sem camada nem campo: da página do jogo
  // para a Visão geral; das outras vistas para a biblioteca; na Visão geral não há para onde.
  useSpatialNavigation(
    useCallback(() => {
      if (view !== "biblioteca") navigate("biblioteca");
      else if (activeGame !== null) openGame(null);
      else return false;
      return true;
    }, [view, activeGame, navigate, openGame]),
  );

  // A largura da lista muda de uma vez; o conteúdo parte da posição antiga e desliza até a nova por `transform`
  // (FLIP): a largura nunca é animada. A transição dura `--tempo-lista` (500 ms); "Reduzir animações" zera.
  const mainLeft = useRef<number | null>(null);
  const listKey = !listShown ? "fora" : listCollapsed ? "recolhida" : "aberta";
  useLayoutEffect(() => {
    const main = mainRef.current;
    if (!main) return;
    const left = main.offsetLeft;
    const before = mainLeft.current;
    mainLeft.current = left;
    if (before === null || before === left) return;
    main.style.transition = "none";
    main.style.transform = `translateX(${String(before - left)}px)`;
    void main.offsetWidth; // aplica o ponto de partida antes de soltar a transição
    main.style.transition = "";
    main.style.transform = "";
  }, [listKey, ready]);

  // Atalhos (README, "Entradas"). O Esc é da pilha de camadas (`escapeLayers`), não daqui.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const mod = (event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey;
      const key = event.key.toLowerCase();
      if (mod && key === "k") {
        event.preventDefault();
        setSearchOpen(true);
        setSearchFocus((n) => n + 1);
      } else if (mod && key === "b") {
        // A lista só existe na biblioteca.
        if (listShown) {
          event.preventDefault();
          toggleList();
        }
      } else if (event.key === "F11" && !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey) {
        event.preventDefault();
        windowAction("big_picture");
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [listShown, toggleList]);

  // Antes de ler o `ui.state` só o fundo: a tela não aparece no padrão para depois pular para a vista guardada.
  if (!ready) return <div className={styles["window"]} />;

  return (
    <div className={styles["window"]}>
      <BarraSuperior
        view={view}
        onNavigate={navigate}
        profile={profile}
        onOpenGame={(id) => {
          update({ view: "biblioteca", activeGame: id });
        }}
        onOpenDownloadSettings={() => {
          update({ view: "configuracoes", settingsCategory: "downloads" });
        }}
        onSearch={() => {
          setSearchOpen(true);
          setSearchFocus((n) => n + 1);
        }}
      />
      <div className={styles["body"]}>
        <ColunaDaLista
          shown={listShown}
          collapsed={listCollapsed}
          onToggleCollapsed={toggleList}
          criteria={state.library}
          onCriteriaChange={(library) => { update({ library }); }}
          activeId={activeGame}
          onActiveChange={openGame}
        />
        <main ref={mainRef} className={styles["main"]} data-focus-region="content" data-focus-initial="">
          {VIEW_IDS.map((id) => {
            const gamePage = id === "biblioteca" && activeGame !== null;
            return (
              <Vista
                key={id}
                title={t(VIEW_TITLE_KEYS[id])}
                active={id === view}
                bare={id === "configuracoes" || id === "biblioteca" || id === "perfil"}
                flush={id === "biblioteca"}
              >
                {gamePage && (
                  <PaginaDoJogo
                    gameId={activeGame}
                    onMissing={() => {
                      openGame(null);
                    }}
                  />
                )}
                {id === "biblioteca" && !gamePage && <VisaoGeral onOpenGame={openGame} />}
                {id === "perfil" && (
                  <TelaPerfil
                    onOpenGame={(gameId) => {
                      update({ view: "biblioteca", activeGame: gameId });
                    }}
                    onOpenCollection={(collection) => {
                      update({ view: "biblioteca", activeGame: null, library: { ...defaultLibraryCriteria, collection } });
                    }}
                  />
                )}
                {id === "configuracoes" && (
                  <TelaConfiguracoes
                    category={state.settingsCategory}
                    onCategoryChange={(settingsCategory) => {
                      update({ settingsCategory });
                    }}
                  />
                )}
              </Vista>
            );
          })}
        </main>
      </div>
      <CamadasDaSessao />
      <CentralRapida />
      <AvisosDoControle />
      <BuscaGeral
        open={searchOpen}
        focusToken={searchFocus}
        onClose={() => {
          setSearchOpen(false);
        }}
        onOpenGame={(id) => {
          update({ view: "biblioteca", activeGame: id });
        }}
        onOpenShortcut={({ view: next, settingsCategory }) => {
          update(settingsCategory ? { view: next, settingsCategory } : { view: next });
        }}
      />
    </div>
  );
}
