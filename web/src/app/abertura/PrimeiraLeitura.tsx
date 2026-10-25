import mark from "../../assets/brand/tako-mark.png";
import { useT } from "../../i18n";
import { scanView, type ScanProgress } from "./model";
import styles from "./PrimeiraLeitura.module.css";

/**
 * A tela da primeira leitura das lojas (TelaCarregamento do Slint): a marca, o texto de estado, a porcentagem e a
 * barra com o progresso de verdade. A barra só anima `transform`; sem número ela corre, e com "Reduzir animações"
 * fica parada (o `base.css` zera as durações).
 */
export function PrimeiraLeitura({ scan }: { scan: ScanProgress }) {
  const t = useT();
  const view = scanView(scan);
  const status = t(view.textKey);
  return (
    <div className={styles["tela"]}>
      <img className={styles["marca"]} src={mark} alt="" />
      <div className={styles["estado"]}>
        <div className={styles["linha"]}>
          <span className={styles["texto"]}>{status}</span>
          <span className={styles["porcentagem"]}>{view.percent}</span>
        </div>
        <div
          className={styles["trilho"]}
          role="progressbar"
          aria-label={status}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={view.progress === null ? undefined : Math.round(view.progress * 100)}
        >
          {view.progress === null ? (
            <div className={styles["corre"]} />
          ) : (
            <div className={styles["cheio"]} style={{ transform: `scaleX(${String(view.progress)})` }} />
          )}
        </div>
      </div>
    </div>
  );
}
