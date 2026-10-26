import { icons, type IconName } from "../../assets/icons/icons.generated";
import { cx } from "../../lib/cx";
import styles from "./Icon.module.css";

export type { IconName };
export const ICON_NAMES = Object.keys(icons) as IconName[];

interface IconProps {
  name: IconName;
  /** `md` = 18 px (padrão); `sm` = 15 px, para menus, chips e controles pequenos. */
  size?: "md" | "sm";
  /** Preenchido: só ▶, estrela e coração ligados. */
  filled?: boolean;
  /** Nome do ícone para leitores de tela. Sem ele o ícone é decorativo (o rótulo é do botão). */
  label?: string;
}

/** Ícone do design system na cor do texto (`currentColor`). Botão só de ícone leva `aria-label` no botão. */
export function Icon({ name, size = "md", filled = false, label }: IconProps) {
  const className = cx(styles["icon"], styles[size], filled && styles["filled"]);
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      focusable="false"
      {...(label ? { role: "img", "aria-label": label } : { "aria-hidden": true })}
      // Miolo vem de scripts/sync-design.mjs (SVG do design system), nunca de entrada de usuário.
      dangerouslySetInnerHTML={{ __html: icons[name] }}
    />
  );
}
