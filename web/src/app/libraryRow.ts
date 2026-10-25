import type { LibraryGame } from "../bridge/generated/LibraryGame";
import type { GameRow } from "../components/ListaDeJogos/ListaDeJogos";

const statuses = {
  playing: "playing",
  beaten: "completed",
  wishlist: "queued",
  paused: "paused",
  abandoned: "dropped",
} as const;

/** Só campos conhecidos: situação pessoal não significa processo em execução. */
export function libraryGameToRow(
  game: LibraryGame,
  storeName: string = game.store,
  downloadPercent: number | null = null,
): GameRow {
  return {
    id: game.id,
    title: game.title,
    store: storeName,
    installed: game.installed,
    favorite: game.favorite,
    status: game.status ? statuses[game.status] : "none",
    running: false,
    downloadPercent,
    updateAvailable: false,
  };
}

/** Marca como aberto (`running`) a linha do jogo que o núcleo diz estar aberto; as outras ficam como estão. */
export function withRunning(rows: readonly GameRow[], runningId: string | null): readonly GameRow[] {
  if (runningId === null || !rows.some((row) => row.id === runningId)) return rows;
  return rows.map((row) => (row.id === runningId ? { ...row, running: true } : row));
}

export function libraryGameIcon(game: LibraryGame, px: number): string | null {
  return game.hasIcon ? `tako-img://icon?id=${encodeURIComponent(game.id)}&w=${px}&h=${px}` : null;
}
