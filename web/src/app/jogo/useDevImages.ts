import { useEffect, useState } from "react";

type DevImage = (kind: "hero" | "logo" | "cover", id: string, title: string) => string;

/**
 * Só no navegador com o hospedeiro falso (`npm run dev`): arte de exemplo no lugar de `tako-img://`, que só o Tako
 * serve. No Tako, e no build de produção, devolve `null` e a arte vem do núcleo.
 */
export function useDevImages(): DevImage | null {
  const [resolver, setResolver] = useState<DevImage | null>(null);
  useEffect(() => {
    if (!import.meta.env.DEV || window.location.protocol.startsWith("tako")) return;
    void import("../../bridge/fake").then(({ fakeGameImage, isFakeHost }) => {
      if (isFakeHost()) setResolver(() => fakeGameImage);
    });
  }, []);
  return resolver;
}
