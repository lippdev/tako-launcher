import { CampoDeSelecao } from "../../components/CampoDeSelecao/CampoDeSelecao";
import { Icon, type IconName } from "../../components/Icon/Icon";
import { useT } from "../../i18n";
import { cx } from "../../lib/cx";
import { GROUP_BYS, LIST_VIEWS, type GroupBy, type ListView } from "./listaModel";
import styles from "./ModoDaLista.module.css";

const VIEW_ICONS: Record<ListView, IconName> = { list: "list", grid: "grid", categories: "books" };

interface ModoDaListaProps {
  view: ListView;
  onViewChange: (view: ListView) => void;
}

/** Os três modos da lista (Lista · Grade · Categorias), um grupo de botões de ícone com o atual marcado. */
export function SeletorDeModo({ view, onViewChange }: ModoDaListaProps) {
  const t = useT();
  return (
    <div className={styles["modes"]} role="radiogroup" aria-label={t("desktop.view.menu")}>
      {LIST_VIEWS.map((mode) => {
        const label = t(`desktop.view.${mode}`);
        return (
          <button
            key={mode}
            type="button"
            role="radio"
            aria-checked={mode === view}
            aria-label={label}
            title={label}
            className={cx(styles["mode"], mode === view && styles["on"])}
            onClick={() => {
              onViewChange(mode);
            }}
          >
            <Icon name={VIEW_ICONS[mode]} size="sm" />
          </button>
        );
      })}
    </div>
  );
}

interface AgruparPorProps {
  value: GroupBy;
  onChange: (group: GroupBy) => void;
}

/** "Agrupar por" das Categorias: Situação, Coleção, Etiqueta, Loja ou Instalação. */
export function AgruparPor({ value, onChange }: AgruparPorProps) {
  const t = useT();
  return (
    <div className={styles["groupBy"]}>
      <span>{t("desktop.group.by_prefix")}</span>
      <CampoDeSelecao
        label={t("desktop.group.by_prefix")}
        value={value}
        options={GROUP_BYS.map((group) => ({ value: group, label: t(`desktop.group.${group}`) }))}
        onChange={onChange}
      />
    </div>
  );
}
