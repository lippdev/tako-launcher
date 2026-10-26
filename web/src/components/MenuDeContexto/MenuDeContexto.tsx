import { useEffect, useId, useLayoutEffect, useRef, type KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import { useEscapeLayer } from "../../lib/escapeLayers";
import { Portal } from "../../lib/Portal";
import { usePresence } from "../../lib/presence";
import { placeMenu, rootZoom, type Anchor } from "../../lib/position";
import { useLatest } from "../../lib/useLatest";
import { Icon, type IconName } from "../Icon/Icon";
import styles from "./MenuDeContexto.module.css";

export type MenuItem = {
  type?: "item";
  id: string;
  label: string;
  /** Ícone de 15, à frente do texto. */
  icon?: IconName;
  /** Ponto de cor à frente do texto (qualquer cor CSS), no lugar do ícone. */
  color?: string;
  /** Perigo, em vermelho. */
  danger?: boolean;
  disabled?: boolean;
  /** `radio` (uma escolha, `menuitemradio`) ou `checkbox` (liga e desliga); com `checked` marca o ✓. */
  kind?: "item" | "radio" | "checkbox";
  checked?: boolean;
  onSelect: () => void;
};

export type MenuEntry = MenuItem | { type: "separator" } | { type: "header"; label: string };

export type MenuCloseReason = "escape" | "outside" | "select" | "tab";

interface MenuDeContextoProps {
  open: boolean;
  onClose: (reason: MenuCloseReason) => void;
  /** O botão que abriu o menu, ou o ponto do clique (botão direito). */
  anchor: HTMLElement | { x: number; y: number } | null;
  entries: readonly MenuEntry[];
  /** Cabeçalho em caixa alta no topo (também é o nome do menu para leitores de tela). */
  title?: string;
  /** Nome do menu quando não há `title`. */
  label?: string;
  /** `end` (padrão): borda direita do menu na do botão. */
  align?: "start" | "end";
  className?: string;
}

/**
 * Menu padrão (README do MenuDeContexto, movimento em MenusEDicas): portal no `body`, 6 abaixo do botão e
 * alinhado à direita, sobe se não couber. ↑ ↓ Home End navegam, Enter e Espaço escolhem, Esc e clique fora fecham.
 */
export function MenuDeContexto({ open, onClose, anchor, entries, title, label, align = "end", className }: MenuDeContextoProps) {
  const ref = useRef<HTMLDivElement>(null);
  const opener = useRef<HTMLElement | null>(null);
  const titleId = useId();
  const { mounted, visible, animating } = usePresence(open, ref);

  const latestOnClose = useLatest(onClose);
  const close = (reason: MenuCloseReason) => {
    latestOnClose.current(reason);
  };
  /** Devolve o foco a quem abriu antes de fechar: o que a escolha abrir em seguida parte daí. */
  const restoreFocus = () => {
    if (opener.current?.isConnected) opener.current.focus({ preventScroll: true });
  };

  useEscapeLayer(open, () => {
    restoreFocus();
    close("escape");
  });

  // Foco inicial: o item marcado, senão o primeiro habilitado.
  useEffect(() => {
    const menu = ref.current;
    if (!open || !menu) return;
    // Se o foco já está no menu (efeito refeito pelo StrictMode), mantém o botão anterior.
    const focused = document.activeElement;
    if (focused instanceof HTMLElement && !menu.contains(focused)) opener.current = focused;
    const items = [...menu.querySelectorAll<HTMLElement>('[role^="menuitem"]:not([aria-disabled="true"])')];
    const first = items.find((el) => el.getAttribute("aria-checked") === "true") ?? items[0];
    (first ?? menu).focus({ preventScroll: true });
  }, [open]);

  // Clique fora fecha. O próprio botão que abriu fica de fora: é ele quem alterna.
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target instanceof Node ? event.target : null;
      if (!target || ref.current?.contains(target)) return;
      if (anchor instanceof HTMLElement && anchor.contains(target)) return;
      latestOnClose.current("outside");
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
    };
  }, [open, anchor, latestOnClose]);

  // Posição: junto ao botão, dentro da janela, e de novo quando a janela muda ou algo rola.
  useLayoutEffect(() => {
    const menu = ref.current;
    if (!mounted || !menu || !anchor) return;
    const place = () => {
      const zoom = rootZoom();
      const rect: Anchor =
        anchor instanceof HTMLElement
          ? anchor.getBoundingClientRect()
          : { left: anchor.x, right: anchor.x, top: anchor.y, bottom: anchor.y };
      const margin = 8 * zoom;
      const viewport = { width: window.innerWidth, height: window.innerHeight };
      menu.style.maxHeight = "";
      const natural = { width: menu.offsetWidth * zoom, height: menu.offsetHeight * zoom };
      const maxHeight = viewport.height - 2 * margin;
      if (natural.height > maxHeight) {
        menu.style.maxHeight = `${maxHeight / zoom}px`;
        natural.height = maxHeight;
      }
      const edge = anchor instanceof HTMLElement ? align : "start";
      const spot = placeMenu(rect, natural, viewport, {
        align: edge,
        gap: 6 * zoom,
        margin,
      });
      menu.style.left = `${spot.left / zoom}px`;
      menu.style.top = `${spot.top / zoom}px`;
      menu.dataset["side"] = spot.side;
      // A animação cresce a partir do canto do botão: à esquerda quando o menu se alinha pelo início.
      menu.dataset["align"] = edge;
    };
    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
    };
  }, [mounted, anchor, align, entries]);

  const choose = (item: MenuItem) => {
    if (item.disabled) return;
    restoreFocus();
    close("select");
    item.onSelect();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const menu = ref.current;
    if (!menu) return;
    const items = [...menu.querySelectorAll<HTMLElement>('[role^="menuitem"]:not([aria-disabled="true"])')];
    const index = items.indexOf(document.activeElement as HTMLElement);
    let next: HTMLElement | undefined;
    switch (event.key) {
      case "ArrowDown":
        next = items[index < 0 ? 0 : (index + 1) % items.length];
        break;
      case "ArrowUp":
        next = items[index < 0 ? items.length - 1 : (index - 1 + items.length) % items.length];
        break;
      case "Home":
        next = items[0];
        break;
      case "End":
        next = items[items.length - 1];
        break;
      case "Tab":
        // O Tab sai do menu: o foco volta ao botão e o Tab segue dali.
        restoreFocus();
        close("tab");
        return;
      default:
        if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
          // Digitar a inicial vai ao próximo item que começa com ela.
          const letter = event.key.toLowerCase();
          const ordered = [...items.slice(index + 1), ...items.slice(0, index + 1)];
          next = ordered.find((el) => el.textContent.trim().toLowerCase().startsWith(letter));
          if (next) break;
        }
        return;
    }
    event.preventDefault();
    next?.focus({ preventScroll: true });
  };

  if (!mounted) return null;

  return (
    <Portal>
      <div
        ref={ref}
        role="menu"
        tabIndex={-1}
        aria-label={title ? undefined : label}
        aria-labelledby={title ? titleId : undefined}
        data-visible={visible}
        data-animating={animating}
        className={cx(styles["menu"], className)}
        onKeyDown={onKeyDown}
      >
        {title && (
          <div id={titleId} role="presentation" className={styles["header"]}>
            {title}
          </div>
        )}
        {entries.map((entry, i) => {
          if (entry.type === "separator") return <div key={`sep-${String(i)}`} role="separator" className={styles["separator"]} />;
          if (entry.type === "header")
            return (
              <div key={`head-${String(i)}`} role="presentation" className={styles["header"]}>
                {entry.label}
              </div>
            );
          const role = entry.kind === "radio" ? "menuitemradio" : entry.kind === "checkbox" ? "menuitemcheckbox" : "menuitem";
          return (
            <button
              key={entry.id}
              type="button"
              role={role}
              tabIndex={-1}
              {...(entry.kind && entry.kind !== "item" ? { "aria-checked": entry.checked ?? false } : {})}
              {...(entry.disabled ? { "aria-disabled": true } : {})}
              className={cx(styles["item"], entry.danger && styles["danger"])}
              onClick={() => {
                choose(entry);
              }}
              onMouseEnter={(event) => {
                if (!entry.disabled) event.currentTarget.focus({ preventScroll: true });
              }}
            >
              {entry.color ? (
                <span className={styles["dot"]} style={{ background: entry.color }} aria-hidden="true" />
              ) : (
                entry.icon && <Icon name={entry.icon} size="sm" />
              )}
              <span className={styles["label"]}>{entry.label}</span>
              {entry.checked && (
                <span className={styles["check"]}>
                  <Icon name="check" size="sm" />
                </span>
              )}
            </button>
          );
        })}
      </div>
    </Portal>
  );
}
