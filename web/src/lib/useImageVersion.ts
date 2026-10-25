import { useSyncExternalStore } from "react";
import { on } from "../bridge";
import { artChanged, artRevision, imageVersion, subscribeArt } from "./imageVersions";

// Uma assinatura por página, independente de quais telas ou imagens estão montadas.
on("art.changed", ({ gameId }) => artChanged(gameId));

export function useImageVersion(id: string | undefined): number {
  return useSyncExternalStore(subscribeArt, () => imageVersion(id ?? ""));
}

export function useArtRevision(): number {
  return useSyncExternalStore(subscribeArt, artRevision);
}
