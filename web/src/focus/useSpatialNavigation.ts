import { useEffect } from "react";
import { useLatest } from "../lib/useLatest";
import { shouldDropEcho } from "./echo";
import {
  dispatchNavAction,
  isTextEntry,
  needsClick,
  recoverFocus,
  remember,
  setBackHandler,
  setInputSource,
} from "./engine";
import { startController } from "./controller";
import { actionForKey } from "./keymap";

/** O cursor está na ponta do campo, para onde a seta quer ir? Antes disso a seta é do texto. */
function caretAtEdge(field: HTMLElement, key: string): boolean {
  if (!(field instanceof HTMLInputElement)) return false;
  const { selectionStart: start, selectionEnd: end, value } = field;
  if (start === null || start !== end) return false;
  return key === "ArrowLeft" ? start === 0 : start === value.length;
}

/**
 * Liga a navegação por direção ao app: teclado, controle (pela ponte), origem da entrada (mouse apaga o foco
 * visível), memória de foco por região e recuperação do foco quando a tela troca. Uma vez, no `AppShell`. `onBack` é
 * o "voltar" da tela (fechar a página do jogo, sair das Configurações): roda quando camada, campo e componente não
 * trataram o Esc.
 */
export function useSpatialNavigation(onBack: () => boolean): void {
  const back = useLatest(onBack);

  useEffect(() => setBackHandler(() => back.current()), [back]);

  useEffect(() => {
    // Origem da entrada: no capture, antes de qualquer componente (a lista para o Esc, por exemplo).
    const onKeySource = (event: KeyboardEvent) => {
      if (event.isTrusted) setInputSource("keyboard");
    };
    const onPointer = () => {
      setInputSource("mouse");
    };

    const onKeyDown = (event: KeyboardEvent) => {
      // `isTrusted`: as teclas que o próprio navegador oferece aos componentes (ver `offer`) não voltam para cá.
      if (!event.isTrusted || event.defaultPrevented) return;
      const target = event.target instanceof HTMLElement ? event.target : null;
      const typing = isTextEntry(target);
      const { key, ctrlKey, altKey, metaKey, shiftKey } = event;
      const action = actionForKey({ key, ctrlKey, altKey, metaKey, shiftKey, typing });
      if (!action) return;

      if (shouldDropEcho(action, "keyboard")) {
        setInputSource("controller");
        event.preventDefault();
        return;
      }

      if (typing && target) {
        // Num campo, ← e → são do texto até o cursor chegar à ponta; ↑ ↓ não têm uso no texto de uma linha.
        if (action.kind === "navigate") {
          if (target instanceof HTMLTextAreaElement || target.isContentEditable) return;
          const horizontal = action.direction === "left" || action.direction === "right";
          if (horizontal && !caretAtEdge(target, event.key)) return;
        }
        // O Esc de um campo com texto limpa o campo (busca); vazio, sai dele.
        if (action.kind === "back" && target instanceof HTMLInputElement && target.value !== "") return;
      }

      // Segurar a seta repete o movimento; as outras ações valem uma vez por aperto.
      if (event.repeat && action.kind !== "navigate") return;

      if (action.kind === "activate") {
        // Botão, link e campo ativam sozinhos; só o que tem papel de botão sem teclado próprio precisa do clique.
        if (target && needsClick(target)) {
          event.preventDefault();
          target.click();
        }
        return;
      }
      if (dispatchNavAction(action, "keyboard")) event.preventDefault();
    };

    const onFocusIn = (event: FocusEvent) => {
      if (event.target instanceof HTMLElement) remember(event.target);
    };

    // O foco some quando o elemento sai do DOM ou fica `inert` (vista que sai, camada que fecha): confere uma vez
    // por quadro, depois da rajada de mudanças do React.
    let frame = 0;
    const observer = new MutationObserver(() => {
      if (frame !== 0) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        recoverFocus();
      });
    });
    observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["inert"] });

    document.addEventListener("keydown", onKeySource, true);
    document.addEventListener("pointerdown", onPointer, true);
    window.addEventListener("keydown", onKeyDown);
    document.addEventListener("focusin", onFocusIn);
    // O controle (SDL3 no núcleo) chega pela ponte já como comando; o mapa de botões e a repetição são do núcleo.
    const stopController = startController();
    return () => {
      stopController();
      observer.disconnect();
      cancelAnimationFrame(frame);
      document.removeEventListener("keydown", onKeySource, true);
      document.removeEventListener("pointerdown", onPointer, true);
      window.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("focusin", onFocusIn);
    };
  }, []);
}
