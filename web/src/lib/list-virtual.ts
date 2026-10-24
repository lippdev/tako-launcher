import { useCallback, useEffect, useState, type RefObject } from "react";

interface VirtualListOptions {
  /** Quantas linhas a lista tem no total. */
  count: number;
  /** Altura de cada linha somada ao espaço entre elas, em px (as linhas são todas iguais). */
  itemSize: number;
  /** Linhas extras fora da janela, para a rolagem não mostrar vazio. */
  overscan?: number;
  /** O elemento que rola. */
  viewportRef: RefObject<HTMLElement | null>;
}

export interface VirtualRange {
  /** Primeira linha no DOM. */
  start: number;
  /** Uma depois da última linha no DOM. */
  end: number;
  /** Altura total da lista inteira, para o espaçador (a barra de rolagem fica certa). */
  totalSize: number;
  /** Rola o mínimo para a linha ficar à vista. */
  scrollToIndex: (index: number) => void;
}

/**
 * Janela de uma lista de linhas de altura fixa: só o que está à vista (mais a folga) vai para o DOM. Sem biblioteca.
 * O chamador posiciona cada linha em `index * itemSize` com `transform` dentro de um espaçador de `totalSize`.
 */
export function useVirtualRange({ count, itemSize, overscan = 8, viewportRef }: VirtualListOptions): VirtualRange {
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(0);

  useEffect(() => {
    const el = viewportRef.current;
    if (!el) return;
    let frame = 0;
    const readScroll = () => {
      frame = 0;
      setScrollTop(el.scrollTop);
    };
    // Um quadro por rolagem: vários eventos no mesmo quadro viram uma leitura só.
    const onScroll = () => {
      frame ||= requestAnimationFrame(readScroll);
    };
    const observer = new ResizeObserver(() => {
      setHeight(el.clientHeight);
    });
    // O ResizeObserver avisa ao observar, então a altura inicial chega sem ler aqui.
    el.addEventListener("scroll", onScroll, { passive: true });
    observer.observe(el);
    return () => {
      el.removeEventListener("scroll", onScroll);
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [viewportRef]);

  const scrollToIndex = useCallback(
    (index: number) => {
      const el = viewportRef.current;
      if (!el) return;
      const top = index * itemSize;
      const bottom = top + itemSize;
      if (top < el.scrollTop) el.scrollTop = top;
      else if (bottom > el.scrollTop + el.clientHeight) el.scrollTop = bottom - el.clientHeight;
    },
    [viewportRef, itemSize],
  );

  const first = Math.floor(scrollTop / itemSize);
  const visible = Math.ceil(height / itemSize) + 1;
  return {
    start: Math.max(0, first - overscan),
    end: Math.min(count, first + visible + overscan),
    totalSize: count * itemSize,
    scrollToIndex,
  };
}
