import { Icon } from "../components/Icon/Icon";
import { cx } from "../lib/cx";
import { strings } from "./strings";
import { windowAction } from "./windowActions";
import styles from "./BarraDaJanela.module.css";

/** Os botões da janela sem moldura, na ponta da BarraSuperior (README da BarraDaJanela). */
export function BarraDaJanela() {
  return (
    <div className={styles["controls"]} role="group" aria-label={strings.window.group}>
      <button
        type="button"
        className={styles["button"]}
        aria-label={strings.window.minimize}
        title={strings.window.minimize}
        onClick={() => {
          windowAction("minimize");
        }}
      >
        <Icon name="minus" size="sm" />
      </button>
      <button
        type="button"
        className={styles["button"]}
        aria-label={strings.window.toggleMaximize}
        title={strings.window.toggleMaximize}
        onClick={() => {
          windowAction("toggle_maximize");
        }}
      >
        <Icon name="square" size="sm" />
      </button>
      <button
        type="button"
        className={cx(styles["button"], styles["close"])}
        aria-label={strings.window.close}
        title={strings.window.close}
        onClick={() => {
          windowAction("close");
        }}
      >
        <Icon name="x" size="sm" />
      </button>
    </div>
  );
}
