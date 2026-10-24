import { useEffect, useRef, useState } from "react";
import { CampoDeBusca } from "../components/CampoDeBusca/CampoDeBusca";
import { useEscapeLayer } from "../lib/escapeLayers";
import { useFocusTrap } from "../lib/focusTrap";
import { Portal } from "../lib/Portal";
import { usePresence } from "../lib/presence";
import { strings } from "./strings";
import styles from "./BuscaGeral.module.css";

interface BuscaGeralProps {
  open: boolean;
  onClose: () => void;
  /** Cada Ctrl + K aumenta o número: com a busca já aberta, devolve o foco ao campo. */
  focusToken: number;
}

/**
 * Marcador da busca geral (Ctrl + K). TODO: a busca de verdade (jogos, configurações, ações) é de outra tarefa;
 * por enquanto é só o campo da linha de 58 num painel, com Esc e clique fora fechando.
 */
export function BuscaGeral({ open, onClose, focusToken }: BuscaGeralProps) {
  const veil = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const { mounted, visible, animating } = usePresence(open, veil);

  useEscapeLayer(open, onClose);
  useFocusTrap(open, box);

  useEffect(() => {
    if (open) input.current?.focus({ preventScroll: true });
  }, [open, focusToken]);

  if (!mounted) return null;

  return (
    <Portal>
      <div
        ref={veil}
        data-visible={visible}
        data-animating={animating}
        className={styles["veil"]}
        onMouseDown={(event) => {
          if (event.target === event.currentTarget) onClose();
        }}
      >
        <div ref={box} role="dialog" aria-modal="true" aria-label={strings.search.title} tabIndex={-1} className={styles["panel"]}>
          <CampoDeBusca
            variant="geral"
            inputRef={input}
            value={query}
            onChange={setQuery}
            placeholder={strings.search.placeholder}
            label={strings.search.label}
            hint={strings.search.escape}
          />
          <p className={styles["soon"]}>{strings.search.soon}</p>
        </div>
      </div>
    </Portal>
  );
}
