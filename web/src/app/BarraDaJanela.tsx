import { Icon } from "../components/Icon/Icon";
import { useT } from "../i18n";
import { cx } from "../lib/cx";
import { windowAction } from "./windowActions";
import styles from "./BarraDaJanela.module.css";

/** Os botões da janela sem moldura, na ponta da BarraSuperior (README da BarraDaJanela). */
export function BarraDaJanela() {
  const t = useT();
  return (
    <div className={styles["controls"]} data-focus-skip="" role="group" aria-label={t("desktop.window.group")}>
      <button
        type="button"
        className={styles["button"]}
        aria-label={t("desktop.window.minimize")}
        title={t("desktop.window.minimize")}
        onClick={() => {
          windowAction("minimize");
        }}
      >
        <Icon name="minus" size="sm" />
      </button>
      <button
        type="button"
        className={styles["button"]}
        aria-label={t("desktop.window.toggle_maximize")}
        title={t("desktop.window.toggle_maximize")}
        onClick={() => {
          windowAction("toggle_maximize");
        }}
      >
        <Icon name="square" size="sm" />
      </button>
      <button
        type="button"
        className={cx(styles["button"], styles["close"])}
        aria-label={t("desktop.window.close")}
        title={t("desktop.window.close")}
        onClick={() => {
          windowAction("close");
        }}
      >
        <Icon name="x" size="sm" />
      </button>
    </div>
  );
}
