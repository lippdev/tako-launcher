import { useMemo } from "react";
import { MenuDeContexto, type MenuCloseReason, type MenuEntry } from "../components/MenuDeContexto/MenuDeContexto";
import { useT } from "../i18n";
import { menuLine } from "./atualizacao/atualizacaoModel";
import { checkForUpdates, useUpdates } from "./atualizacao/atualizacaoStore";
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
  const t = useT();
  const { status, transient } = useUpdates();
  // A linha de atualizações muda no lugar conforme o estado, como no Slint (WEB-33).
  const updates = menuLine(status, transient, strings.menu.checkUpdates, t);
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
        label: updates.label,
        icon: updates.icon,
        disabled: updates.disabled,
        onSelect: () => {
          void checkForUpdates();
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
    [onNavigate, onAddGame, updates.label, updates.icon, updates.disabled],
  );

  return <MenuDeContexto open={open} onClose={onClose} anchor={anchor} entries={entries} label={strings.topbar.menu} align="start" />;
}
