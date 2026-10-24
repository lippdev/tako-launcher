import { useState, type KeyboardEvent } from "react";
import { Aviso } from "../../components/Aviso/Aviso";
import { Icon } from "../../components/Icon/Icon";
import { settingsStrings } from "../strings";
import { SETTINGS_CATEGORY_IDS, type SettingsCategoryId } from "../uiState";
import { AparenciaCategoria } from "./AparenciaCategoria";
import { BibliotecaCategoria } from "./BibliotecaCategoria";
import { CategoriaDeConfiguracao } from "./CategoriaDeConfiguracao";
import { GeralCategoria } from "./GeralCategoria";
import type { CategoryProps } from "./Linhas";
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
];

const PANEL_ID = "configuracoes-painel";

interface TelaConfiguracoesProps {
  category: SettingsCategoryId;
  onCategoryChange: (category: SettingsCategoryId) => void;
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
    default:
      // TODO(WEB-13b): Tema, Som, Tela, Controles, Capas e dados, Downloads, Integrações, Sistema e Sobre.
      return <p className={styles["soon"]}>{settingsStrings.soon}</p>;
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
  const text = settingsStrings.categories[category];

  // ↑ ↓ Home End movem o foco pela lista; Enter e Espaço abrem (são botões). Tab percorre todos.
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
    if (!target) return;
    event.preventDefault();
    target.focus();
  }

  return (
    <div className={styles["screen"]}>
      <nav className={styles["nav"]} aria-label={settingsStrings.nav} onKeyDown={onNavKeyDown}>
        <p className={styles["meta"]}>{settingsStrings.count(SETTINGS_CATEGORY_IDS.length)}</p>
        {SETTINGS_CATEGORY_IDS.map((id) => (
          <CategoriaDeConfiguracao
            key={id}
            icon={CATEGORY_ICONS[id]}
            title={settingsStrings.categories[id].title}
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
                {text.title}
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
