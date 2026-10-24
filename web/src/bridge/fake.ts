import type { Call, Event } from "./index";
import type { BridgeError } from "./generated/BridgeError";
import type { UiState } from "./generated/UiState";
import type { GameRow } from "../components/ListaDeJogos/ListaDeJogos";
import type { LibraryGame } from "./generated/LibraryGame";
import type { LibraryListRequest } from "./generated/LibraryListRequest";
import type { Collection } from "./generated/Collection";
import type { CollectionGames } from "./generated/CollectionGames";
import type { GameFavoriteSet } from "./generated/GameFavoriteSet";
import type { GameStatusSet } from "./generated/GameStatusSet";
import type { GameTagsSet } from "./generated/GameTagsSet"; // Hospedeiro falso para rodar a interface no navegador (Vite) sem o Tako: responde com dados de exemplo.

// Só para desenvolvimento; o build de produção não instala isto.

/** Devolve o `data` da resposta (ou nada, para chamadas sem retorno). */
type Handler = (payload: unknown) => unknown;
export type FakeHandlers = Partial<Record<Call["name"], Handler>>;

let uiState: UiState = { json: sessionStorage.getItem("tako.dev.ui.state") };

const defaults: FakeHandlers = {
  "app.info": () => ({ version: "dev", bridgeVersion: 1 }),
  "window.action": (action) => console.info("[hospedeiro falso] janela:", action),
  "ui.state": () => uiState,
  "ui.state.set": (state) => {
    uiState = state as UiState;
    sessionStorage.setItem("tako.dev.ui.state", uiState.json ?? "");
  },
};

/** Instala o hospedeiro falso se não houver um de verdade. Devolve uma função para emitir eventos. */
export function installFakeHost(handlers: FakeHandlers = {}, latencyMs = 30) {
  const all = { ...defaults, ...handlers };
  const send = (wire: object) => setTimeout(() => window.__takoDeliver?.(JSON.stringify(wire)), latencyMs);

  if (!window.takoHost) {
    installedFake = true;
    window.takoHost = {
      post(message) {
        const req = JSON.parse(message) as {
          v: number;
          id: number;
          name: Call["name"];
          payload?: unknown;
        };
        const handler = all[req.name];
        if (!handler) {
          const error: BridgeError = {
            code: "unknown_call",
            message: req.name,
          };
          send({ v: 1, id: req.id, ok: false, error });
          return;
        }
        Promise.resolve()
          .then(() => handler(req.payload))
          .then(
            (data) =>
              send({
                v: 1,
                id: req.id,
                ok: true,
                reply: { name: req.name, data },
              }),
            (e: unknown) =>
              send({
                v: 1,
                id: req.id,
                ok: false,
                error: { code: "internal", message: String(e) },
              }),
          );
      },
    };
  }

  return (event: Event) => send({ v: 1, event });
}

// ---- Dados de exemplo da lista de jogos (WEB-10a) ----
// Fonte dos exemplos do hospedeiro; o app consulta library.list pela ponte.

const FAKE_NAMES: [title: string, store: string][] = [
  ["God of War: Ragnarök", "Steam"],
  ["Dragon's Dogma 2", "Steam"],
  ["Watch Dogs 2", "Epic"],
  ["Elden Ring", "Steam"],
  ["Cyberpunk 2077", "GOG"],
  ["Hades II", "Steam"],
  ["Baldur's Gate 3", "GOG"],
  ["Sekiro: Shadows Die Twice", "Steam"],
  ["The Witcher 3: Wild Hunt", "GOG"],
  ["Stellar Blade", "Epic"],
  ["Black Myth: Wukong", "Steam"],
  ["Death Stranding", "Steam"],
  ["Ghost of Tsushima Director's Cut", "Steam"],
];

/** Os 13 jogos da prévia da TelaBiblioteca, sempre iguais (nada de aleatório). */
export function fakeGames(): GameRow[] {
  return FAKE_NAMES.map(([title, store], i) => ({
    id: `steam:${String(1000 + i)}`,
    title,
    store,
    // Os mesmos "Não instalado" da prévia (Watch Dogs 2, Black Myth, Death Stranding, Ghost of Tsushima).
    installed: ![2, 10, 11, 12].includes(i),
    favorite: i === 0 || i === 3,
    status: "none",
    running: false,
    downloadPercent: null,
    updateAvailable: i === 4 || i === 6,
    ...(i < 2 ? { hint: i === 0 ? "Jogado hoje, 16:17" : "Jogado hoje, 04:57" } : {}),
  }));
}

const HUES = [210, 20, 340, 150, 45, 275];

/** Ícone de exemplo (SVG num data URI): um degradê por jogo. No lugar de `tako-img://icon`, que só o Tako serve. */
export function fakeGameIcon(id: string, px: number): string {
  const n = Number(id.split(":")[1]);
  const hue = HUES[n % HUES.length] ?? 210;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${String(px)}" height="${String(px)}"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${String(hue)} 45% 45%)"/><stop offset="1" stop-color="hsl(${String((hue + 40) % 360)} 50% 18%)"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

let installedFake = false;
export function isFakeHost() {
  return installedFake;
}
const games: LibraryGame[] = fakeGames().map((row, i) => ({
  id: row.id,
  title: row.title,
  store: row.store === "Epic" ? "epic" : row.store === "GOG" ? "gog" : "steam",
  installed: row.installed,
  favorite: row.favorite,
  status: i === 0 ? "playing" : null,
  lastPlayedAt: i < 2 ? "2026-10-24T16:17:00Z" : null,
  playTimeSeconds: i * 3600,
  accent: null,
  hasCover: false,
  hasIcon: i !== 12,
  hasHero: false,
  hasLogo: false,
}));
const collections: Collection[] = [
  {
    id: "favorites-night",
    name: "Para jogar à noite",
    gameIds: ["steam:1000", "steam:1003", "steam:1005"],
    criteria: null,
  },
  { id: "empty", name: "Próximas aventuras", gameIds: [], criteria: null },
];
const gameTags = new Map<string, Set<string>>([
  ["steam:1003", new Set(["RPG"])],
  ["steam:1005", new Set(["Roguelike"])],
]);
function changed() {
  window.__takoDeliver?.(JSON.stringify({ v: 1, event: { name: "library.changed" } }));
}
function mutate<T>(payload: unknown, action: (p: T) => void) {
  action(payload as T);
  changed();
}
function collectionMutation(payload: unknown, add: boolean) {
  mutate<CollectionGames>(payload, (p) => {
    const c = collections.find((c) => c.id === p.collectionId);
    if (!c || c.criteria) throw new Error("coleção indisponível");
    c.gameIds = add ? [...new Set([...c.gameIds, ...p.gameIds])] : c.gameIds.filter((id) => !p.gameIds.includes(id));
  });
}
function listLibrary(payload: unknown) {
  const p = payload as LibraryListRequest;
  const collection = p.collection ? collections.find((c) => c.id === p.collection) : null;
  const needle = p.search?.trim().toLocaleLowerCase("pt-BR") ?? "";
  const rows = games.filter(
    (g) =>
      (p.filter !== "favorites" || g.favorite) &&
      (p.filter !== "installed" || g.installed) &&
      g.title.toLocaleLowerCase("pt-BR").includes(needle) &&
      (!p.collection || !!collection?.gameIds.includes(g.id)) &&
      (!p.stores?.length || p.stores.includes(g.store)) &&
      (p.installed == null || p.installed === g.installed) &&
      ((!p.statuses?.length && !p.noStatus) || (g.status === null ? p.noStatus : p.statuses?.includes(g.status))) &&
      (!p.tags?.length || p.tags.every((tag) => gameTags.get(g.id)?.has(tag))),
  );
  rows.sort((a, b) => {
    if (p.sort === "name") return a.title.localeCompare(b.title, "pt-BR");
    if (p.sort === "play_time") return b.playTimeSeconds - a.playTimeSeconds;
    if (!p.sort || p.sort === "recently_played")
      return (b.lastPlayedAt ?? "").localeCompare(a.lastPlayedAt ?? "") || a.title.localeCompare(b.title);
    return 0;
  });
  if (p.reversed) rows.reverse();
  const offset = Math.max(0, p.offset ?? 0);
  return {
    total: rows.length,
    rows: rows.slice(offset, offset + Math.min(200, Math.max(1, p.limit ?? 40))),
  };
}
Object.assign(defaults, {
  "library.list": listLibrary,
  "collections.list": () => collections,
  "tags.list": () =>
    [...new Set([...gameTags.values()].flatMap((tags) => [...tags]))].map((name) => ({
      name,
      count: [...gameTags.values()].filter((tags) => tags.has(name)).length,
    })),
  "game.favorite.set": (p: unknown) =>
    mutate<GameFavoriteSet>(p, (p) => {
      for (const g of games) if (p.gameIds.includes(g.id)) g.favorite = p.favorite;
    }),
  "game.status.set": (p: unknown) =>
    mutate<GameStatusSet>(p, (p) => {
      for (const g of games) if (p.gameIds.includes(g.id)) g.status = p.status;
    }),
  "game.tags.set": (p: unknown) =>
    mutate<GameTagsSet>(p, (p) => {
      for (const id of p.gameIds) {
        const tags = gameTags.get(id) ?? new Set<string>();
        for (const tag of p.tags) {
          if (p.on) tags.add(tag);
          else tags.delete(tag);
        }
        gameTags.set(id, tags);
      }
    }),
  "collections.add_games": (p: unknown) => {
    collectionMutation(p, true);
  },
  "collections.remove_games": (p: unknown) => {
    collectionMutation(p, false);
  },
});
