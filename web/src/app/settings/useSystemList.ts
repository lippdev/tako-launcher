import { useEffect, useState } from "react";

/** Uma lista do sistema (telas, saídas de áudio): ainda lendo, lida ou com a leitura falhada. */
export type SystemList<T> = { status: "loading" } | { status: "ready"; data: T } | { status: "failed" };

/**
 * Lê uma lista do sistema pela ponte quando a categoria abre. `read` tem que ser estável (uma função de módulo):
 * a leitura roda uma vez por abertura, não a cada desenho.
 */
export function useSystemList<T>(read: () => Promise<T>): SystemList<T> {
  const [list, setList] = useState<SystemList<T>>({ status: "loading" });
  useEffect(() => {
    let cancelled = false;
    read()
      .then((data) => {
        if (!cancelled) setList({ status: "ready", data });
      })
      .catch((error: unknown) => {
        console.error("leitura do sistema falhou", error);
        if (!cancelled) setList({ status: "failed" });
      });
    return () => {
      cancelled = true;
    };
  }, [read]);
  return list;
}
