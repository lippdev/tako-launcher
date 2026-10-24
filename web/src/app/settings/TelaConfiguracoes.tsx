import { useState, type KeyboardEvent } from "react";
import { Aviso } from "../../components/Aviso/Aviso";
import { Icon } from "../../components/Icon/Icon";
import { useT } from "../../i18n";
import { settingsStrings } from "../strings";
import { SETTINGS_CATEGORY_IDS, type SettingsCategoryId } from "../uiState";
import { AparenciaCategoria } from "./AparenciaCategoria";
import { BibliotecaCategoria } from "./BibliotecaCategoria";
import { CategoriaDeConfiguracao } from "./CategoriaDeConfiguracao";
import { DownloadsCategoria } from "./DownloadsCategoria";
import { GeralCategoria } from "./GeralCategoria";
import { IntegracoesCategoria } from "./IntegracoesCategoria";
import type { CategoryProps } from "./Linhas";
import { SistemaCategoria } from "./SistemaCategoria";
import { SobreCategoria } from "./SobreCategoria";
import { SomCategoria } from "./SomCategoria";
import { TelaCategoria } from "./TelaCategoria";
import { TemaCategoria } from "./TemaCategoria";
import { CATEGORY_ICONS } from "./categories";
import { APPEARANCE_KEYS, KEYS } from "./model";
import { useSettings } from "./useSettings";
import styles from "./TelaConfiguracoes.module.css";

/** Tudo o que as categorias lêem de uma vez com `settings.get`. */
const SETTINGS_KEYS: readonly string[] = [
  ...APPEARANCE_KEYS,
  KEYS.language,
  KEYS.openIn,
  KEYS.askBigPicture,
  KEYS.restoreEnvironment,
  KEYS.artColors,
  KEYS.disabledStores,
  KEYS.navigationVolume,
  KEYS.themeMusic,
  KEYS.themeMusicVolume,
  KEYS.screenEnabled,
  KEYS.screenHideOthers,
  KEYS.screenId,
  KEYS.screenName,
  KEYS.screenMode,
  KEYS.screenHdr,
  KEYS.screenVrr,
  KEYS.audioId,
  KEYS.audioName,
  KEYS.hideCursor,
  KEYS.controllerMouse,
  KEYS.discord,
  KEYS.sunshineSync,
  KEYS.developer,
  KEYS.downloadsEnabled,
  KEYS.downloadsAccepted,
  KEYS.downloadsUpload,
];

const PANEL_ID = "configuracoes-painel";

interface TelaConfiguracoesProps {
  category: SettingsCategoryId;
  onCategoryChange: (category: SettingsCategoryId) => void;
}

function Soon() {
  return <p className={styles["soon"]}>{settingsStrings.soon}</p>;
}

/** O conteúdo de uma categoria. As que ainda não têm tela mostram o "em breve". */
function Conteudo({ category, ...props }: CategoryProps & { category: SettingsCategoryId }) {
  switch (category) {
    case "general":
      return <GeralCategoria {...props} />;
    case "appearance":
      return <AparenciaCategoria {...props} />;
    case "library":
      return <BibliotecaCategoria {...props} />;
    case "theme":
      return <TemaCategoria {...props} />;
    case "sound":
      return <SomCategoria {...props} />;
    case "display":
      return <TelaCategoria {...props} />;
    case "integrations":
      return <IntegracoesCategoria {...props} />;
    case "system":
      return <SistemaCategoria {...props} />;
    case "about":
      return <SobreCategoria />;
    case "controllers":
      // TODO(WEB-13b): Controles espera o foco e o controle (FOCO/INPUT): lista de controles, botões e atalhos da
      // Central de Controle.
      return <Soon />;
    case "covers":
      // TODO(WEB-13b): Capas e dados espera os segredos dos provedores (IGDB, ScreenScraper, SteamGridDB), que não
      // passam pela ponte, e o cache de capas.
      return <Soon />;
    case "downloads":
      return <DownloadsCategoria {...props} />;
  }
}

/**
 * A vista Configurações (README da TelaConfiguracoes): as categorias à esquerda (292) e o conteúdo à direita
 * (até 900). Cada mudança grava na hora pela ponte; se ela falhar, a tela volta ao valor de antes e avisa.
 */
export function TelaConfiguracoes({ category, onCategoryChange }: TelaConfiguracoesProps) {
  const { values, ready, loadFailed, failures, save, read } = useSettings(SETTINGS_KEYS);
  const [loadNoticeClosed, setLoadNoticeClosed] = useState(false);
  const [saveNoticeClosedAt, setSaveNoticeClosedAt] = useState(0);
  const t = useT();
  // Prova do WEB-06a: o título de Geral já vem do catálogo do núcleo (as outras categorias migram na WEB-06b).
  const titleOf = (id: SettingsCategoryId) =>
    id === "general" ? t("settings.general.title") : settingsStrings.categories[id].title;
  const text = settingsStrings.categories[category];

  // ↑ ↓ Home End movem o foco pela lista; Enter e Espaço abrem (são botões). Tab percorre todos. Nas pontas a seta
  // não é tratada aqui: segue para o navegador de foco (↑ sobe para a barra, ↓ não tem para onde).
  function onNavKeyDown(event: KeyboardEvent<HTMLElement>) {
    const items = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>("button"));
    const index = items.findIndex((item) => item === document.activeElement);
    const target =
      event.key === "ArrowDown"
        ? items[Math.min(items.length - 1, index + 1)]
        : event.key === "ArrowUp"
          ? items[Math.max(0, index - 1)]
          : event.key === "Home"
            ? items[0]
            : event.key === "End"
              ? items[items.length - 1]
              : undefined;
    if (!target || target === document.activeElement) return;
    event.preventDefault();
    target.focus();
  }

  return (
    <div className={styles["screen"]}>
      <nav
        className={styles["nav"]}
        aria-label={settingsStrings.nav}
        data-focus-region="categories"
        data-focus-sections="content"
        onKeyDown={onNavKeyDown}
      >
        <p className={styles["meta"]}>{settingsStrings.count(SETTINGS_CATEGORY_IDS.length)}</p>
        {SETTINGS_CATEGORY_IDS.map((id) => (
          <CategoriaDeConfiguracao
            key={id}
            icon={CATEGORY_ICONS[id]}
            title={titleOf(id)}
            current={id === category}
            controls={PANEL_ID}
            onSelect={() => {
              onCategoryChange(id);
            }}
          />
        ))}
      </nav>
      <div className={styles["main"]}>
        {/* `key` reinicia a entrada escalonada a cada categoria. */}
        <div key={category} id={PANEL_ID} className={styles["body"]} aria-labelledby={`${PANEL_ID}-titulo`}>
          <header className={styles["head"]}>
            <span className={styles["headIcon"]}>
              <Icon name={CATEGORY_ICONS[category]} />
            </span>
            <div>
              <h1 id={`${PANEL_ID}-titulo`} className={styles["title"]}>
                {titleOf(category)}
              </h1>
              <p className={styles["description"]}>{text.description}</p>
            </div>
          </header>
          {ready && (
            <div className={styles["sections"]}>
              <Conteudo category={category} values={values} save={save} read={read} />
            </div>
          )}
        </div>
      </div>
      <Aviso
        open={loadFailed && !loadNoticeClosed}
        onClose={() => {
          setLoadNoticeClosed(true);
        }}
        tone="error"
      >
        {settingsStrings.loadError}
      </Aviso>
      <Aviso
        open={failures > saveNoticeClosedAt}
        onClose={() => {
          setSaveNoticeClosedAt(failures);
        }}
        noticeKey={failures}
        tone="error"
      >
        {settingsStrings.saveError}
      </Aviso>
    </div>
  );
}
