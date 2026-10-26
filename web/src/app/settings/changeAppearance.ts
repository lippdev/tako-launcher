import { setAppearance, type Appearance } from "../../appearance/appearance";
import type { CategoryProps } from "./Linhas";
import { appearanceFromSettings } from "./model";

/**
 * Muda a aparência na hora (`setAppearance`) e grava a chave pela ponte. Se a gravação falhar, a página volta à
 * aparência que ficou nas configurações. Aparência e Tema usam a mesma função, então mostram sempre o mesmo valor.
 */
export function changeAppearance({ save, read }: Pick<CategoryProps, "save" | "read">, patch: Partial<Appearance>, key: string, value: string) {
  setAppearance(patch);
  void save(key, value).then((ok) => {
    if (!ok) setAppearance(appearanceFromSettings(read()));
  });
}
