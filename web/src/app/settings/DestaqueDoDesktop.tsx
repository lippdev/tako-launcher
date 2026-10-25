import { useState } from "react";
import { ACCENT_CHOICES, accentOrDefault, presetOf } from "../../appearance/desktopLook";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { useT } from "../../i18n";
import { cx } from "../../lib/cx";
import { parseAccent } from "./model";
import styles from "./DestaqueDoDesktop.module.css";

interface DestaqueDoDesktopProps {
  /** A cor gravada (`#RRGGBB`), ou `null` quando só vale a do tema. */
  value: string | null;
  theme: string;
  onChange: (hex: string) => void;
}

/**
 * Cor de destaque (README da TelaTema): as nove cores prontas e uma cor própria em `#RRGGBB`, como o app Slint. A
 * escolha vale na hora; um código incompleto ou inválido mostra o erro no campo e não grava nada.
 */
export function DestaqueDoDesktop({ value, theme, onChange }: DestaqueDoDesktopProps) {
  const t = useT();
  const current = accentOrDefault(value, theme);
  const preset = presetOf(current);
  // O que está sendo digitado; `null` = o campo mostra a cor em uso.
  const [draft, setDraft] = useState<string | null>(null);
  const text = draft ?? current;
  const invalid = draft !== null && draft.trim() !== "" && parseAccent(draft) === null;

  function type(next: string) {
    const hex = parseAccent(next);
    if (hex) {
      setDraft(null);
      if (hex !== current) onChange(hex);
    } else {
      setDraft(next);
    }
  }

  const accentName = (id: string) => {
    switch (id) {
      case "stone":
        return t("settings.appearance.desk_accent_stone");
      case "orange":
        return t("settings.appearance.desk_accent_orange");
      case "gold":
        return t("settings.appearance.desk_accent_gold");
      case "green":
        return t("settings.appearance.desk_accent_green");
      case "cyan":
        return t("settings.appearance.desk_accent_cyan");
      case "blue":
        return t("settings.appearance.desk_accent_blue");
      case "violet":
        return t("settings.appearance.desk_accent_violet");
      case "pink":
        return t("settings.appearance.desk_accent_pink");
      case "white":
        return t("settings.appearance.desk_accent_white");
      default:
        return id;
    }
  };

  return (
    <>
      <div className={styles["swatches"]} role="group" aria-label={t("settings.theme_web.accent_cards")}>
        {ACCENT_CHOICES.map((choice) => (
          <button
            key={choice.id}
            type="button"
            className={styles["swatch"]}
            style={{ background: choice.hex }}
            aria-label={accentName(choice.id)}
            title={accentName(choice.id)}
            aria-pressed={choice.id === preset}
            onClick={() => {
              setDraft(null);
              if (choice.hex !== current) onChange(choice.hex);
            }}
          />
        ))}
      </div>
      <div className={styles["custom"]}>
        <CampoDeTexto
          className={cx(styles["field"])}
          label={preset ? t("settings.appearance.desk_accent_custom") : t("settings.theme.custom")}
          value={text}
          help={t("settings.appearance.desk_accent_custom_hint")}
          error={invalid ? t("settings.appearance.desk_accent_invalid") : undefined}
          maxLength={7}
          spellCheck={false}
          autoComplete="off"
          onChange={type}
        />
      </div>
    </>
  );
}
