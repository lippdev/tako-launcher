import { useMemo } from "react";
import { MenuDeContexto, type MenuCloseReason, type MenuEntry } from "../components/MenuDeContexto/MenuDeContexto";
import { addGameStrings, strings } from "./strings";
import type { ViewId } from "./uiState";
import { windowAction } from "./windowActions";

interface MenuDoTakoProps {
  open: boolean;
  onClose: (reason: MenuCloseReason) => void;
  /** A marca, que abriu o menu. */
  anchor: HTMLElement | null;
  onNavigate: (view: ViewId) => void;
  /** Escolheu "Adicionar jogo…". */
  onAddGame: () => void;
}

/** O menu da marca (README do MenuDoTako): Adicionar jogo, Configurações, Procurar atualizações, separador e Sair. Sem itens de loja. */
export function MenuDoTako({ open, onClose, anchor, onNavigate, onAddGame }: MenuDoTakoProps) {
  const entries = useMemo<MenuEntry[]>(
    () => [
      {
        id: "add-game",
        label: addGameStrings.menu,
        icon: "plus",
        onSelect: onAddGame,
      },
      {
        id: "settings",
        label: strings.menu.settings,
        icon: "gear",
        onSelect: () => {
          onNavigate("configuracoes");
        },
      },
      {
        id: "check-updates",
        label: strings.menu.checkUpdates,
        icon: "refresh",
        onSelect: () => {
          // TODO: ainda não há chamada na ponte para procurar atualização do Tako.
        },
      },
      { type: "separator" },
      {
        id: "quit",
        label: strings.menu.quit,
        icon: "x",
        danger: true,
        onSelect: () => {
          // TODO: a ponte só tem `window.action`; "close" fecha a janela. Sair de vez (encerrar o núcleo) pede uma chamada própria.
          windowAction("close");
        },
      },
    ],
    [onNavigate, onAddGame],
  );

  return <MenuDeContexto open={open} onClose={onClose} anchor={anchor} entries={entries} label={strings.topbar.menu} align="start" />;
}
