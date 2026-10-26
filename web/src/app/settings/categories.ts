import type { IconName } from "../../components/Icon/Icon";
import type { SettingsCategoryId } from "../uiState";

/** O ícone de cada categoria (README da TelaConfiguracoes), no quadrado da lista e no cabeçalho. */
export const CATEGORY_ICONS: Record<SettingsCategoryId, IconName> = {
  general: "gear",
  theme: "brush",
  appearance: "palette",
  sound: "volume",
  display: "monitor",
  controllers: "gamepad",
  library: "grid",
  covers: "image",
  downloads: "download",
  integrations: "plug",
  system: "cpu",
  about: "info",
};
