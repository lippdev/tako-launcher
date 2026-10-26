import type { TFunction } from "../../i18n";
import { MAX_NAME_LENGTH, type NameProblem } from "./colecoesModel";

/** O erro do nome com o texto do Slint (`library.collection.*`). */
export function problemText(t: TFunction, problem: NameProblem): string {
  switch (problem) {
    case "empty":
      return t("library.collection.name_empty");
    case "too_long":
      return t("library.collection.name_too_long", { n: MAX_NAME_LENGTH });
    case "duplicate":
      return t("library.collection.name_taken");
  }
}
