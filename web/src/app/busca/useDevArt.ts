import { useEffect, useState } from "react";

interface DevArt {
  cover: (id: string, title: string) => string;
  icon: (id: string, px: number) => string;
}

/**
 * Só no navegador com o hospedeiro falso (`npm run dev`): capa e ícone de exemplo no lugar de `tako-img://`, que
 * só o Tako serve. No Tako, e no build de produção, devolve `null` e a arte vem do núcleo.
 */
export function useDevArt(): DevArt | null {
  const [art, setArt] = useState<DevArt | null>(null);
  useEffect(() => {
    if (!import.meta.env.DEV || window.location.protocol.startsWith("tako")) return;
    void import("../../bridge/fake").then(({ fakeGameIcon, fakeGameImage, isFakeHost }) => {
      if (isFakeHost()) setArt({ cover: (id, title) => fakeGameImage("cover", id, title), icon: fakeGameIcon });
    });
  }, []);
  return art;
}
