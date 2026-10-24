import { useState, type MouseEvent } from "react";
import { Abas, type Aba } from "../components/Abas/Abas";
import { Icon } from "../components/Icon/Icon";
import { Marca } from "../components/Marca/Marca";
import { cx } from "../lib/cx";
import { BarraDaJanela } from "./BarraDaJanela";
import { DownloadsNaBarra } from "./DownloadsNaBarra";
import { MenuDoTako } from "./MenuDoTako";
import { strings } from "./strings";
import type { ViewId } from "./uiState";
import { windowAction } from "./windowActions";
import styles from "./BarraSuperior.module.css";

/** Só a biblioteca: não existe loja (decisão do mantenedor), então a barra não tem LOJA. */
const TABS: readonly Aba<"biblioteca">[] = [{ id: "biblioteca", label: strings.topbar.library }];

interface BarraSuperiorProps {
  view: ViewId;
  onNavigate: (view: ViewId) => void;
  onSearch: () => void;
  /** Abre a página de um jogo na biblioteca (o painel de downloads concluídos). */
  onOpenGame: (gameId: string) => void;
  /** Abre Configurações › Downloads. */
  onOpenDownloadSettings: () => void;
  /** Nome e situação do perfil. */
  profile: { name: string; online: boolean };
}

/** A barra de cima de borda a borda (README da BarraSuperior), que também é a barra da janela (arrasta e duplo clique maximiza). */
export function BarraSuperior({ view, onNavigate, onOpenGame, onOpenDownloadSettings, profile, onSearch }: BarraSuperiorProps) {
  const [brand, setBrand] = useState<HTMLButtonElement | null>(null);
  const [menuOpen, setMenuOpen] = useState(false);

  /** Duplo clique no vazio da barra maximiza ou restaura; nos controles não. */
  const onDoubleClick = (event: MouseEvent<HTMLElement>) => {
    if (event.target instanceof Element && event.target.closest("button, input, [role='tab']")) return;
    windowAction("toggle_maximize");
  };

  return (
    <header className={styles["bar"]} data-focus-region="top" onDoubleClick={onDoubleClick}>
      <button
        ref={setBrand}
        type="button"
        className={styles["brand"]}
        aria-label={strings.topbar.menu}
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onClick={() => {
          setMenuOpen((open) => !open);
        }}
      >
        <Marca />
        <span className={styles["chevron"]}>
          <Icon name="chevron-down" size="sm" />
        </span>
      </button>
      <MenuDoTako
        open={menuOpen}
        anchor={brand}
        onClose={() => {
          setMenuOpen(false);
        }}
        onNavigate={onNavigate}
      />

      <Abas
        variant="topo"
        className={cx(styles["nav"])}
        label={strings.topbar.nav}
        tabs={TABS}
        // Fora da biblioteca nenhuma aba fica marcada (README: "Nas Configurações nenhuma aba fica marcada").
        value={view === "biblioteca" ? "biblioteca" : ""}
        onChange={() => {
          onNavigate("biblioteca");
        }}
      />

      <div className={styles["right"]}>
        <button
          type="button"
          className={styles["action"]}
          aria-label={strings.search.label}
          aria-keyshortcuts="Control+K"
          title={strings.search.label}
          onClick={onSearch}
        >
          <Icon name="search" />
        </button>
        <DownloadsNaBarra onOpenGame={onOpenGame} onOpenSettings={onOpenDownloadSettings} />

        <button
          type="button"
          className={cx(styles["action"], styles["profile"], view === "perfil" && styles["on"])}
          aria-label={strings.topbar.profile}
          title={strings.topbar.profile}
          {...(view === "perfil" ? { "aria-current": "page" as const } : {})}
          onClick={() => {
            // TODO(WEB-14): a TelaPerfil; por enquanto abre a vista vazia.
            onNavigate("perfil");
          }}
        >
          <span
            className={cx(styles["avatar"], profile.online && styles["online"])}
            {...(profile.online ? { role: "img", "aria-label": strings.topbar.online } : { "aria-hidden": true })}
          >
            {Array.from(profile.name)[0]?.toUpperCase() ?? "?"}
          </span>
          {profile.name}
          <span className={styles["chevron"]}>
            <Icon name="chevron-down" size="sm" />
          </span>
        </button>

        <button
          type="button"
          className={styles["action"]}
          aria-label={strings.topbar.bigPicture}
          title={strings.topbar.bigPicture}
          onClick={() => {
            windowAction("big_picture");
          }}
        >
          <Icon name="gamepad" />
        </button>

        <BarraDaJanela />
      </div>
    </header>
  );
}
