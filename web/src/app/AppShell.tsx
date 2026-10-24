import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { BarraSuperior } from "./BarraSuperior";
import { BuscaGeral } from "./BuscaGeral";
import { ColunaDaLista } from "./ColunaDaLista";
import { strings } from "./strings";
import { VIEW_IDS, type ViewId } from "./uiState";
import { useLibrary } from "./useLibrary";
import { useScreenState } from "./useScreenState";
import { Vista } from "./Vista";
import { windowAction } from "./windowActions";
import styles from "./AppShell.module.css";

/** TODO(WEB-14): nome e situação vêm do perfil; por enquanto o marcador. */
const PROFILE = { name: strings.topbar.profile, online: true };

/**
 * O app desktop: barra de cima de borda a borda, a lista da biblioteca à esquerda e a área das vistas
 * (README do design system, "A referência: TelaBiblioteca"). As vistas são contêineres vazios por enquanto.
 */
export function AppShell() {
  const { state, ready, update } = useScreenState();
  const library = useLibrary();
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchFocus, setSearchFocus] = useState(0);
  const mainRef = useRef<HTMLElement>(null);

  const { view, listCollapsed } = state;
  const listShown = view === "biblioteca";

  const navigate = useCallback(
    (next: ViewId) => {
      update({ view: next });
    },
    [update],
  );
  const toggleList = useCallback(() => {
    update({ listCollapsed: !listCollapsed });
  }, [update, listCollapsed]);

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
        profile={PROFILE}
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
          rows={library.rows}
          {...(library.resolveIcon ? { resolveIcon: library.resolveIcon } : {})}
        />
        <main ref={mainRef} className={styles["main"]}>
          {VIEW_IDS.map((id) => (
            <Vista key={id} title={strings.views[id]} active={id === view} />
          ))}
        </main>
      </div>
      <BuscaGeral
        open={searchOpen}
        focusToken={searchFocus}
        onClose={() => {
          setSearchOpen(false);
        }}
      />
    </div>
  );
}
