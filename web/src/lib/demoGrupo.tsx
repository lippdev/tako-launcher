import type { ReactNode } from "react";
import styles from "./demoGrupo.module.css";

/** Bloco com título e fileira, só para as demos da galeria `#/componentes`. */
export function DemoGrupo({ titulo, children }: { titulo: string; children: ReactNode }) {
  return (
    <section className={styles["grupo"]}>
      <h3 className={styles["titulo"]}>{titulo}</h3>
      <div className={styles["fileira"]}>{children}</div>
    </section>
  );
}
