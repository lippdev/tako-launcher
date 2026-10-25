import { useEffect, useRef, type RefObject } from "react";

/** Guarda o valor mais recente numa ref, para efeitos e ouvintes que não devem se refazer a cada render. */
export function useLatest<T>(value: T): RefObject<T> {
  const ref = useRef(value);
  useEffect(() => {
    ref.current = value;
  });
  return ref;
}
