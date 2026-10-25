import { useCallback } from "react";
import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import { imageUrl } from "../jogo/jogoModel";
import { useDevImages } from "../jogo/useDevImages";

/**
 * A URL da capa de um jogo (`tako-img://`, pedida em pixels físicos) ou `undefined` sem capa. No navegador com o
 * hospedeiro falso vale a arte de exemplo, que só ele serve.
 */
export function useCapas(width: number, height: number): (game: LibraryGame) => string | undefined {
  const devImage = useDevImages();
  return useCallback(
    (game) => {
      if (devImage) return devImage("cover", game.id, game.title);
      return game.hasCover ? imageUrl("cover", game.id, width * 2, height * 2) : undefined;
    },
    [devImage, width, height],
  );
}
