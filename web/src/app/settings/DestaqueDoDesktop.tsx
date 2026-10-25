import { useState } from "react";
import { ACCENT_CHOICES, accentOrDefault, presetOf } from "../../appearance/desktopLook";
import { CampoDeTexto } from "../../components/CampoDeTexto/CampoDeTexto";
import { cx } from "../../lib/cx";
import { settingsStrings } from "../strings";
import { parseAccent } from "./model";
import styles from "./DestaqueDoDesktop.module.css";

const s = settingsStrings.theme.accent;

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

  return (
    <>
      <div className={styles["swatches"]} role="group" aria-label={s.cards}>
        {ACCENT_CHOICES.map((choice) => (
          <button
            key={choice.id}
            type="button"
            className={styles["swatch"]}
            style={{ background: choice.hex }}
            aria-label={s.names[choice.id]}
            title={s.names[choice.id]}
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
          label={preset ? s.custom.label : s.custom.title}
          value={text}
          help={s.custom.hint}
          error={invalid ? s.custom.invalid : undefined}
          maxLength={7}
          spellCheck={false}
          autoComplete="off"
          onChange={type}
        />
      </div>
    </>
  );
}
