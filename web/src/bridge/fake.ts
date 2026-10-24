import type { Call, Event } from "./index";
import type { BridgeError } from "./generated/BridgeError";
import type { ErrorCode } from "./generated/ErrorCode";
import type { GameDetails } from "./generated/GameDetails";
import type { GameRequest } from "./generated/GameRequest";
import type { UiState } from "./generated/UiState";
import type { GameRow } from "../components/ListaDeJogos/ListaDeJogos";
import type { LibraryGame } from "./generated/LibraryGame";
import type { LibraryListRequest } from "./generated/LibraryListRequest";
import type { Collection } from "./generated/Collection";
import type { CollectionGames } from "./generated/CollectionGames";
import type { GameFavoriteSet } from "./generated/GameFavoriteSet";
import type { GameStatusSet } from "./generated/GameStatusSet";
import type { GameTagsSet } from "./generated/GameTagsSet";
import type { LibrarySourceReport } from "./generated/LibrarySourceReport";
import type { LibrarySources } from "./generated/LibrarySources";
import type { Setting } from "./generated/Setting";
import type { SettingsGetRequest } from "./generated/SettingsGetRequest";
import type { SettingsValues } from "./generated/SettingsValues";
import type { Store } from "./generated/Store";
import type { AudioOutputList } from "./generated/AudioOutputList";
import type { DisplayList } from "./generated/DisplayList";
import type { DownloadAdd } from "./generated/DownloadAdd";
import type { DownloadCancel } from "./generated/DownloadCancel";
import type { DownloadEntry } from "./generated/DownloadEntry";
import type { DownloadRequest } from "./generated/DownloadRequest";
import type { DownloadSource } from "./generated/DownloadSource";
import type { DownloadSources } from "./generated/DownloadSources";
import type { DownloadsSummary } from "./generated/DownloadsSummary";

import type { GameChoice } from "./generated/GameChoice";
import type { GameSettingsSet } from "./generated/GameSettingsSet";
import type { GameSettingsView } from "./generated/GameSettingsView";
// Hospedeiro falso para rodar a interface no navegador (Vite) sem o Tako: responde com dados de exemplo.

// Só para desenvolvimento; o build de produção não instala isto.

/** Devolve o `data` da resposta (ou nada, para chamadas sem retorno). */
type Handler = (payload: unknown) => unknown;
export type FakeHandlers = Partial<Record<Call["name"], Handler>>;

/** Erro com o código do núcleo (`conflict`, `not_found`…): o falso responde com ele em vez de `internal`. */
export class FakeError extends Error {
  readonly code: ErrorCode;
  constructor(code: ErrorCode, message: string) {
    super(message);
    this.code = code;
  }
}

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
                error: { code: e instanceof FakeError ? e.code : "internal", message: String(e) },
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
/** Última partida e horas jogadas por posição em `FAKE_NAMES`; quem não está aqui nunca foi jogado (a Visão geral). */
const PLAYED: Record<number, [lastPlayedAt: string, hours: number]> = {
  0: ["2026-10-24T16:17:00Z", 7],
  1: ["2026-10-24T04:57:00Z", 24],
  3: ["2026-10-23T22:10:00Z", 112],
  5: ["2026-10-22T21:03:00Z", 31],
  4: ["2026-10-20T19:40:00Z", 58],
  6: ["2026-10-18T14:00:00Z", 203],
  7: ["2026-10-10T20:00:00Z", 46],
  8: ["2026-09-30T18:30:00Z", 190],
  2: ["2026-08-12T10:00:00Z", 12],
  9: ["2026-07-02T15:00:00Z", 19],
};
const games: LibraryGame[] = fakeGames().map((row, i) => ({
  id: row.id,
  title: row.title,
  store: row.store === "Epic" ? "epic" : row.store === "GOG" ? "gog" : "steam",
  installed: row.installed,
  favorite: row.favorite,
  status: i === 0 ? "playing" : null,
  lastPlayedAt: PLAYED[i]?.[0] ?? null,
  playTimeSeconds: (PLAYED[i]?.[1] ?? 0) * 3600,
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
  {
    id: "souls",
    name: "Desafio e atmosfera",
    gameIds: ["steam:1003", "steam:1007", "steam:1009", "steam:1010", "steam:1005"],
    criteria: null,
  },
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

// ---- Configurações e lojas de exemplo (WEB-13a) ----
// Guardadas em memória e em `sessionStorage` (a aparência volta igual ao recarregar a página).

/** As chaves que o núcleo aceita em `settings.set` (`crates/tako-app/src/settings.rs`). */
const KNOWN_SETTINGS = new Set([
  "DynamicColor",
  "DesktopArtColors",
  "ReducedMotion",
  "ReducedTransparency",
  "RestoreEnvironmentAfterGame",
  "DeveloperOptions",
  "AskBigPictureWithController",
  "DisabledStores",
  "DesktopTheme",
  "Language",
  "OpenIn",
  "TextSizePercent",
]);
const BOOL_SETTINGS = new Set([
  "DynamicColor",
  "DesktopArtColors",
  "ReducedMotion",
  "ReducedTransparency",
  "RestoreEnvironmentAfterGame",
  "DeveloperOptions",
  "AskBigPictureWithController",
]);
const SETTINGS_STORAGE = "tako.dev.settings";

const settings = new Map<string, string>(
  Object.entries(JSON.parse(sessionStorage.getItem(SETTINGS_STORAGE) ?? "{}") as Record<string, string>),
);

function getSettings(payload: unknown): SettingsValues {
  const { keys } = payload as SettingsGetRequest;
  if (keys?.some((key) => !KNOWN_SETTINGS.has(key))) throw new Error("bad_request: chaves de configuração inválidas");
  return { values: Object.fromEntries([...settings].filter(([key]) => !keys || keys.includes(key))) };
}

function setSetting(payload: unknown) {
  const { key, value } = payload as Setting;
  const valid =
    KNOWN_SETTINGS.has(key) &&
    (!BOOL_SETTINGS.has(key) || value === "true" || value === "false") &&
    (key !== "TextSizePercent" || ["100", "110", "125"].includes(value)) &&
    (key !== "OpenIn" || value === "desktop" || value === "big-picture");
  if (!valid) throw new Error("bad_request: configuração inválida");
  settings.set(key, value);
  sessionStorage.setItem(SETTINGS_STORAGE, JSON.stringify(Object.fromEntries(settings)));
}

/** As lojas de exemplo, com o que achariam; `notice` mostra o aviso do núcleo. */
const SAMPLE_STORES: { store: Store; name: string; found: number; client: boolean; notice?: string }[] = [
  { store: "steam", name: "Steam", found: 8, client: true },
  { store: "epic", name: "Epic", found: 2, client: true },
  { store: "gog", name: "GOG", found: 3, client: true },
  { store: "xbox", name: "Xbox", found: 0, client: false },
  { store: "ubisoft", name: "Ubisoft", found: 0, client: true, notice: "Não deu para ler a biblioteca da Ubisoft" },
  { store: "ea", name: "EA", found: 0, client: false },
  { store: "battle_net", name: "Battle.net", found: 0, client: false },
  { store: "amazon", name: "Amazon", found: 0, client: false },
  { store: "emulators", name: "Emuladores", found: 0, client: true },
];

let sources: LibrarySources = {
  scanning: false,
  completed: 0,
  total: SAMPLE_STORES.length,
  measuringSizes: false,
  sources: [],
  error: null,
};

function emitScanProgress() {
  window.__takoDeliver?.(JSON.stringify({ v: 1, event: { name: "library.scan.progress", payload: sources } }));
}

function isStoreOff(store: Store): boolean {
  const key = store.replace("_", "").toLowerCase();
  return (settings.get("DisabledStores") ?? "")
    .split(",")
    .some((name) => name.trim().toLowerCase() === key);
}

function sampleReport(sample: (typeof SAMPLE_STORES)[number], index: number, readAt: Date): LibrarySourceReport {
  const enabled = !isStoreOff(sample.store);
  return {
    store: sample.store,
    name: sample.name,
    enabled,
    found: enabled ? sample.found : 0,
    clientInstalled: enabled && sample.client,
    error: enabled ? (sample.notice ?? null) : null,
    lastScannedAt: enabled ? readAt.toISOString() : null,
    elapsedMs: enabled ? 120 + index * 30 : null,
  };
}

/** Lê as lojas devagar (400 ms cada) para a tela mostrar o andamento; respeita `DisabledStores` da hora. */
function scanSources() {
  if (sources.scanning) return;
  sources = { ...sources, scanning: true, completed: 0, measuringSizes: false, error: null };
  emitScanProgress();
  const step = (index: number) => {
    const sample = SAMPLE_STORES[index];
    if (!sample) {
      sources = { ...sources, measuringSizes: true };
      emitScanProgress();
      setTimeout(() => {
        sources = { ...sources, scanning: false, measuringSizes: false };
        emitScanProgress();
        changed();
      }, 600);
      return;
    }
    const reports = [...sources.sources];
    reports[index] = sampleReport(sample, index, new Date());
    sources = { ...sources, completed: index + 1, sources: reports };
    emitScanProgress();
    setTimeout(() => {
      step(index + 1);
    }, 400);
  };
  setTimeout(() => {
    step(0);
  }, 200);
}

Object.assign(defaults, {
  "settings.get": getSettings,
  "settings.set": setSetting,
  "library.sources": () => sources,
  "library.scan": scanSources,
});
// A primeira leitura já aconteceu quando a interface abre (como no Tako, que lê ao iniciar).
sources = {
  ...sources,
  completed: SAMPLE_STORES.length,
  sources: SAMPLE_STORES.map((sample, index) => sampleReport(sample, index, new Date(Date.now() - 3 * 60_000))),
};

// ---- Página do jogo (WEB-12a) ----
// Detalhes de exemplo: uns jogos completos, outros com campos `null` (o bloco some), um sem arte, um sem logo,
// um que sai em breve e um que dá `conflict` ao jogar.

interface SampleGame {
  /** `accent` do jogo (a cor do EstadoSemArte); sem ele vale a do tema. */
  accent: string | null;
  hero: boolean;
  logo: boolean;
  details: Partial<Omit<GameDetails, "game">>;
}

const LONG_TEXT =
  "Da Santa Monica Studio, trazido ao PC com a Jetpack Interactive, é uma jornada épica e emocionante de Kratos e Atreus entre segurar e deixar ir.\n\n" +
  "Continuação do aclamado capítulo anterior, a história começa com o Fimbulwinter já avançado. Kratos e Atreus percorrem os Nove Reinos atrás de respostas enquanto as forças de Odin se preparam para a batalha profetizada.\n\n" +
  "No caminho, exploram paisagens míticas e enfrentam deuses e monstros nórdicos.";

const SAMPLE_GAMES: Record<string, SampleGame> = {
  // Completo: arte, logo, todos os dados.
  "steam:1000": {
    accent: "#6b8cae",
    hero: true,
    logo: true,
    details: {
      platform: "Windows",
      description: LONG_TEXT,
      developer: "Santa Monica Studios",
      publisher: "PlayStation Publishing LLC",
      releaseDate: "2024-09-19",
      genres: ["Ação", "Aventura"],
      metacriticScore: 94,
      criticScore: 94,
      recentSessions: [
        { startedAt: "2026-10-24T16:17:00Z", durationSeconds: 2520 },
        { startedAt: "2026-10-22T21:03:00Z", durationSeconds: 7 * 3600 + 1200 },
        { startedAt: "2026-10-20T19:40:00Z", durationSeconds: 5400 },
      ],
      sizeBytes: 25_231_000_000,
      sizeLabel: "23.5 GB",
    },
  },
  // Sem logo (título em texto), poucos dados.
  "steam:1001": {
    accent: "#c27a3d",
    hero: true,
    logo: false,
    details: {
      platform: "Windows",
      description: "Um RPG de ação de mundo aberto em que você escolhe a vocação, recruta peões e caça monstros.",
      developer: "Capcom",
      genres: ["RPG"],
      sizeLabel: "71.2 GB",
    },
  },
  // Não instalado, sem arte, com a cor do jogo.
  "steam:1002": {
    accent: "#2f8f83",
    hero: false,
    logo: false,
    details: { platform: null, developer: "Ubisoft Montreal", releaseDate: "2016-11-15", genres: ["Ação", "Mundo aberto"] },
  },
  // Arte e logo, nota mista, sem sessões.
  "steam:1004": {
    accent: "#d7c33a",
    hero: true,
    logo: true,
    details: {
      platform: "Windows",
      description: "Uma aventura de ação e RPG de mundo aberto em Night City.",
      developer: "CD Projekt Red",
      publisher: "CD Projekt",
      releaseDate: "2020-12-10",
      metacriticScore: 62,
      sizeLabel: "68.4 GB",
    },
  },
  // Jogados por último depois do primeiro: arte e logo, para o destaque da Visão geral trocar entre jogos completos.
  "steam:1003": {
    accent: "#c9a24a",
    hero: true,
    logo: true,
    details: {
      platform: "Windows",
      description: "Um RPG de ação em um mundo aberto criado por Hidetaka Miyazaki e George R. R. Martin.",
      developer: "FromSoftware",
      publisher: "Bandai Namco",
      releaseDate: "2022-02-25",
      genres: ["RPG", "Ação"],
      metacriticScore: 96,
      sizeLabel: "49.8 GB",
    },
  },
  "steam:1006": {
    accent: "#8a64d6",
    hero: true,
    logo: false,
    details: {
      platform: "Windows",
      description: "Reúna seu grupo e volte aos Reinos Esquecidos numa história de companheirismo e traição.",
      developer: "Larian Studios",
      releaseDate: "2023-08-03",
      genres: ["RPG"],
      metacriticScore: 96,
      sizeLabel: "122 GB",
    },
  },
  "steam:1008": {
    accent: "#9a7a45",
    hero: true,
    logo: true,
    details: {
      platform: "Windows",
      developer: "CD Projekt Red",
      releaseDate: "2015-05-19",
      genres: ["RPG", "Mundo aberto"],
      metacriticScore: 93,
    },
  },
  // Em breve.
  "steam:1005": {
    accent: "#9b4dca",
    hero: true,
    logo: false,
    details: {
      upcoming: true,
      description: "A continuação do roguelike de ação sobre a princesa das sombras.",
      developer: "Supergiant Games",
      releaseDate: "2026-12-01",
      genres: ["Roguelike"],
    },
  },
  // Joga e dá `conflict`: o jogo saiu do disco depois da última leitura.
  "steam:1009": {
    accent: "#c24a4a",
    hero: true,
    logo: true,
    details: { platform: "Windows", developer: "Shift Up", metacriticScore: 81, sizeLabel: "52.0 GB" },
  },
  // O mínimo: o núcleo só sabe o título.
  "steam:1007": { accent: null, hero: false, logo: false, details: {} },
};

// A arte e a cor de cada jogo de exemplo entram na lista de jogos do núcleo falso.
for (const game of games) {
  const sample = SAMPLE_GAMES[game.id];
  Object.assign(game, {
    accent: sample?.accent ?? null,
    hasHero: sample?.hero ?? false,
    hasLogo: sample?.logo ?? false,
    // Dois jogos sem capa, para ver o EstadoSemArte nas fileiras.
    hasCover: game.id !== "steam:1002" && game.id !== "steam:1011",
  });
}

/** O título em linhas de até 14 letras, em caixa alta e escapado para SVG, para a capa de exemplo. */
function coverLines(title: string): string[] {
  const lines: string[] = [];
  for (const word of title.toUpperCase().split(/\s+/)) {
    const last = lines[lines.length - 1];
    if (last !== undefined && last.length + word.length < 14) lines[lines.length - 1] = `${last} ${word}`;
    else lines.push(word);
  }
  return lines.slice(0, 4).map((line) => line.replace(/&/g, "&amp;").replace(/</g, "&lt;"));
}

/** Arte de exemplo (SVG num data URI) no lugar de `tako-img://hero`, `logo` e `cover`, que só o Tako serve. */
export function fakeGameImage(kind: "hero" | "logo" | "cover", id: string, title: string): string {
  const n = Number(id.split(":")[1]);
  const hue = HUES[n % HUES.length] ?? 210;
  const lines = coverLines(title);
  const svg =
    kind === "cover"
      ? `<svg xmlns="http://www.w3.org/2000/svg" width="600" height="900" viewBox="0 0 600 900"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${String(hue)} 55% 46%)"/><stop offset="1" stop-color="hsl(${String((hue + 60) % 360)} 55% 14%)"/></linearGradient><radialGradient id="r" cx=".7" cy=".28" r=".55"><stop offset="0" stop-color="#fff" stop-opacity=".4"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient></defs><rect width="600" height="900" fill="url(#g)"/><rect width="600" height="900" fill="url(#r)"/><path d="M0 700 L150 500 L270 620 L420 420 L600 640 V900 H0Z" fill="#000" fill-opacity=".38"/>${lines.map((line, i) => `<text x="40" y="${String(860 - (lines.length - i) * 56)}" font-family="Poppins, sans-serif" font-weight="800" font-size="44" fill="#f4f1f0">${line}</text>`).join("")}</svg>`
      : kind === "hero"
      ? `<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="620" viewBox="0 0 1920 620"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${String(hue)} 55% 52%)"/><stop offset="1" stop-color="hsl(${String((hue + 60) % 360)} 55% 16%)"/></linearGradient><radialGradient id="r" cx=".72" cy=".3" r=".5"><stop offset="0" stop-color="#fff" stop-opacity=".45"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></radialGradient></defs><rect width="1920" height="620" fill="url(#g)"/><rect width="1920" height="620" fill="url(#r)"/><path d="M0 520 L360 330 L620 470 L980 250 L1320 440 L1620 300 L1920 460 V620 H0Z" fill="#000" fill-opacity=".35"/></svg>`
      : `<svg xmlns="http://www.w3.org/2000/svg" width="440" height="140" viewBox="0 0 440 140"><text x="436" y="96" text-anchor="end" font-family="Poppins, sans-serif" font-weight="800" font-size="${String(Math.min(56, Math.floor(410 / (title.length * 0.72))))}" fill="#f4f1f0">${title
          .toUpperCase()
          .replace(/&/g, "&amp;")
          .replace(/</g, "&lt;")}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

function gameDetails(payload: unknown): GameDetails {
  const { gameId } = payload as GameRequest;
  const game = games.find((g) => g.id === gameId);
  if (!game) throw new FakeError("not_found", gameId);
  const sample = SAMPLE_GAMES[game.id]?.details ?? {};
  const tags = [...(gameTags.get(game.id) ?? [])];
  const upcoming = sample.upcoming ?? false;
  return {
    game: { ...game },
    canPlay: game.installed && !upcoming,
    upcoming,
    platform: null,
    description: null,
    developer: null,
    publisher: null,
    releaseDate: null,
    genres: null,
    metacriticScore: null,
    criticScore: null,
    recentSessions: null,
    sizeBytes: null,
    sizeLabel: null,
    ...sample,
    tags: tags.length > 0 ? tags : null,
  };
}

function playGame(payload: unknown) {
  const { gameId } = payload as GameRequest;
  const game = games.find((g) => g.id === gameId);
  if (!game) throw new FakeError("not_found", gameId);
  // Sem dado de execução na ponte: o falso só avisa que abriria.
  if (!game.installed || SAMPLE_GAMES[gameId]?.details.upcoming || gameId === "steam:1009")
    throw new FakeError("conflict", `${gameId} não pode ser jogado agora`);
  console.info("[hospedeiro falso] jogar:", game.title);
}

Object.assign(defaults, { "game.details": gameDetails, "game.play": playGame });

// ---- Configurações: Tema a Sobre (WEB-13b) ----
// As chaves novas, com a mesma validação do núcleo, e as telas e saídas de áudio de exemplo.

const MORE_BOOL_SETTINGS = [
  "ThemeMusicEnabled",
  "GameScreenEnabled",
  "GameScreenHdr",
  "GameScreenVrr",
  "HideCursorDuringGame",
  "ControllerMouseDuringGame",
  "SafeMode",
  "DiscordRichPresence",
  "SunshineSyncOnRescan",
];
const MORE_TEXT_SETTINGS = ["GameScreenHideOthers", "GameScreenId", "GameScreenName", "GameScreenMode", "GameAudioDeviceId", "GameAudioDeviceName"];
const PERCENT_SETTINGS = ["NavigationSoundVolume", "ThemeMusicVolume"];
for (const key of [...MORE_BOOL_SETTINGS, ...MORE_TEXT_SETTINGS, ...PERCENT_SETTINGS]) KNOWN_SETTINGS.add(key);
for (const key of MORE_BOOL_SETTINGS) BOOL_SETTINGS.add(key);

/** A saída "com defeito" mostra o aviso de gravação que falhou e a volta ao valor de antes. */
const BROKEN_AUDIO_ID = "broken";

const setBaseSetting = setSetting;
function setMoreSettings(payload: unknown) {
  const { key, value } = payload as Setting;
  if (PERCENT_SETTINGS.includes(key) && !(/^\d{1,3}$/.test(value) && Number(value) <= 100))
    throw new Error("bad_request: configuração inválida");
  if (key === "GameAudioDeviceId" && value === BROKEN_AUDIO_ID) throw new Error("internal: não foi possível gravar");
  setBaseSetting(payload);
}

const SAMPLE_DISPLAYS: DisplayList = {
  managedBySession: false,
  vrrLimitation: "O Windows não tem API pública para ligar o VRR. Use o painel do driver de vídeo (AMD, NVIDIA ou Intel).",
  displays: [
    {
      id: "DISPLAY1",
      name: "Monitor",
      primary: true,
      active: true,
      hdr: "unsupported",
      vrr: "unsupported",
      modes: ["1920x1080@144", "1920x1080@60", "1280x720@60"],
    },
    {
      id: "DISPLAY2",
      name: "TV",
      primary: false,
      active: false,
      hdr: "off",
      vrr: "off",
      modes: ["3840x2160@120", "3840x2160@60", "1920x1080@120"],
    },
  ],
};

const SAMPLE_AUDIO: AudioOutputList = {
  canSwitch: true,
  outputs: [
    { id: "speakers", name: "Alto-falantes", default: true },
    { id: "tv", name: "TV (HDMI)", default: false },
    { id: BROKEN_AUDIO_ID, name: "Saída com defeito (a gravação falha)", default: false },
  ],
};

Object.assign(defaults, {
  "settings.set": setMoreSettings,
  "system.displays": () => SAMPLE_DISPLAYS,
  "system.audio_outputs": () => SAMPLE_AUDIO,
});
// ---- Textos e idiomas (WEB-06a) ----
// Um catálogo pequeno de exemplo, no formato do núcleo (`tako_core::i18n::messages`): só o que as telas já migradas
// usam. O de verdade tem todas as chaves de `crates/tako-core/locales`.

const SAMPLE_CATALOGS: Record<"pt-BR" | "en-US", Record<string, string>> = {
  "pt-BR": {
    "settings.general.title": "Geral",
    "first_run.splash.scanning": "Procurando seus jogos…",
    "first_run.splash.starting": "Iniciando o Tako…",
    "common.games.one": "{n} jogo",
    "common.games.other": "{n} jogos",
    // Downloads (WEB-14b): os textos de `downloads.toml` e dois de `common.toml`.
    "downloads.title": "Downloads",
    "downloads.enable": "Downloads por torrent",
    "downloads.enable_desc": "Baixar jogos de fontes .json que você mesmo adiciona. Desligado até você ligar.",
    "downloads.privacy": "Privacidade",
    "downloads.privacy_desc": "Num torrent, o seu endereço IP fica visível para os outros participantes e para quem monitora a rede. O Tako não abre portas no roteador (sem UPnP) e para de enviar quando o download termina. O histórico fica no banco local até você remover os itens da lista; isso não apaga os arquivos.",
    "downloads.upload": "Limite de envio",
    "downloads.upload_desc": "Quanto o Tako envia para outros participantes enquanto baixa. Ao terminar, o envio para.",
    "downloads.upload_none": "Sem limite",
    "downloads.sources": "Fontes de download",
    "downloads.sources_desc": "{count} fontes adicionadas por você · a procedência aparece em cada item",
    "downloads.source_desc": "{count} itens · atualizada em {date}",
    "downloads.source_loading": "Lendo a fonte…",
    "downloads.tool": "O Tako é só a ferramenta",
    "downloads.tool_desc": "O Tako não hospeda, não indexa, não recomenda e não fiscaliza fontes nem conteúdo. Você escolhe as fontes e responde pelo que baixa.",
    "downloads.legal_title": "Antes de ligar os downloads",
    "downloads.legal_text": "O Tako fornece só a ferramenta: não traz, não sugere e não lista fontes, nem verifica o conteúdo delas. Baixar material protegido por direitos autorais sem autorização pode ser ilegal onde você mora, e a responsabilidade é sua.\n\nNum torrent, o seu endereço IP fica visível para os outros participantes e para quem monitora a rede. O Tako nunca abre nem executa o que for baixado: ao terminar, mostra só a pasta.",
    "downloads.legal_accept": "Entendi e aceito",
    "downloads.group_active": "Baixando",
    "downloads.group_queued": "Na fila",
    "downloads.group_done": "Concluídos",
    "downloads.empty": "Nenhum download",
    "downloads.n_active": "{n} baixando",
    "downloads.paused": "Pausado",
    "downloads.waiting": "espera",
    "downloads.metadata": "Buscando os dados do torrent…",
    "downloads.of": "{done} de {total}",
    "downloads.left": "faltam {time}",
    "downloads.failed": "Erro: {error}",
    "downloads.cancel_title": "Cancelar o download?",
    "downloads.cancel_title_game": "Cancelar o download de {title}?",
    "downloads.cancel_text": "O jogo deixa de mostrar o download. Os arquivos parciais podem ficar na pasta ou ser apagados.",
    "downloads.cancel_keep": "Continuar baixando",
    "downloads.cancel_keep_files": "Cancelar e manter os arquivos",
    "downloads.cancel_delete": "Cancelar e apagar",
    "downloads.pause": "Pausar",
    "downloads.resume": "Continuar",
    "downloads.cancel": "Cancelar",
    "common.cancel": "Cancelar",
    "common.close": "Fechar",
  },
  "en-US": {
    "settings.general.title": "General",
    "first_run.splash.scanning": "Looking for your games…",
    "first_run.splash.starting": "Starting Tako…",
    "common.games.one": "{n} game",
    "common.games.other": "{n} games",
    // Downloads (WEB-14b): os textos de `downloads.toml` e dois de `common.toml`.
    "downloads.title": "Downloads",
    "downloads.enable": "Torrent downloads",
    "downloads.enable_desc": "Download games from .json sources you add yourself. Off until you turn it on.",
    "downloads.privacy": "Privacy",
    "downloads.privacy_desc": "In a torrent, your IP address is visible to other participants and to anyone watching the network. Tako opens no router ports (no UPnP) and stops uploading when a download finishes. Download history stays in the local database until you remove items from the list; this does not delete the files.",
    "downloads.upload": "Upload limit",
    "downloads.upload_desc": "How much Tako uploads to other participants while downloading. Uploading stops when it finishes.",
    "downloads.upload_none": "No limit",
    "downloads.sources": "Download sources",
    "downloads.sources_desc": "{count} sources added by you · each item shows where it came from",
    "downloads.source_desc": "{count} items · updated {date}",
    "downloads.source_loading": "Reading the source…",
    "downloads.tool": "Tako is only the tool",
    "downloads.tool_desc": "Tako doesn't host, index, recommend or check sources or content. You choose the sources and are responsible for what you download.",
    "downloads.legal_title": "Before turning on downloads",
    "downloads.legal_text": "Tako only provides the tool: it doesn't ship, suggest or list sources, nor check what they contain. Downloading copyrighted material without permission may be illegal where you live, and you are responsible for it.\n\nIn a torrent, your IP address is visible to other participants and to anyone watching the network. Tako never opens or runs what you download: when it finishes, it only shows the folder.",
    "downloads.legal_accept": "I understand and accept",
    "downloads.group_active": "Downloading",
    "downloads.group_queued": "Queued",
    "downloads.group_done": "Completed",
    "downloads.empty": "No downloads",
    "downloads.n_active": "{n} downloading",
    "downloads.paused": "Paused",
    "downloads.waiting": "waiting",
    "downloads.metadata": "Getting the torrent's data…",
    "downloads.of": "{done} of {total}",
    "downloads.left": "{time} left",
    "downloads.failed": "Error: {error}",
    "downloads.cancel_title": "Cancel the download?",
    "downloads.cancel_title_game": "Cancel the download of {title}?",
    "downloads.cancel_text": "The game stops showing the download. Partial files can stay in the folder or be deleted.",
    "downloads.cancel_keep": "Keep downloading",
    "downloads.cancel_keep_files": "Cancel and keep files",
    "downloads.cancel_delete": "Cancel and delete",
    "downloads.pause": "Pause",
    "downloads.resume": "Resume",
    "downloads.cancel": "Cancel",
    "common.cancel": "Cancel",
    "common.close": "Close",
  },
};

/** O idioma que o núcleo usaria para este valor de `Language` (`pt…` e `en…`; qualquer outro é recusado). */
function fakeLang(value: string | undefined): "pt-BR" | "en-US" | null {
  const code = (value ?? "pt-BR").trim().toLowerCase().replace("_", "-");
  return code.startsWith("pt") ? "pt-BR" : code.startsWith("en") ? "en-US" : null;
}

function i18nCatalog() {
  const lang = fakeLang(settings.get("Language")) ?? "pt-BR";
  return { lang, messages: SAMPLE_CATALOGS[lang] };
}

/** Como o núcleo: gravar um `Language` diferente avisa a interface com `i18n.changed`. */
const setSettingBase = defaults["settings.set"];
function setSettingAndAnnounceLanguage(payload: unknown) {
  const { key, value } = payload as Setting;
  if (key !== "Language") return setSettingBase?.(payload);
  if (!fakeLang(value)) throw new Error("bad_request: configuração inválida");
  const before = fakeLang(settings.get("Language"));
  setSettingBase?.(payload);
  const lang = fakeLang(value);
  if (lang !== before)
    setTimeout(() => {
      window.__takoDeliver?.(JSON.stringify({ v: 1, event: { name: "i18n.changed", payload: { lang } } }));
    }, 0);
}

Object.assign(defaults, { "i18n.catalog": i18nCatalog, "settings.set": setSettingAndAnnounceLanguage });

// ---- Downloads (WEB-14b) ----
// Um motor de exemplo em memória: o progresso avança sozinho (a velocidade é a mostrada), `downloads.changed` avisa a
// cada passo e toda chamada dá `conflict` enquanto os downloads estão desligados, como o núcleo.

for (const key of ["DownloadsEnabled", "DownloadsLegalAccepted", "DownloadsUploadLimitKiB"]) KNOWN_SETTINGS.add(key);
BOOL_SETTINGS.add("DownloadsEnabled");
BOOL_SETTINGS.add("DownloadsLegalAccepted");

const GB = 1024 ** 3;
const MB = 1024 ** 2;
/** Quantos downloads o motor de exemplo baixa ao mesmo tempo; os outros esperam na fila. */
const MAX_PARALLEL = 2;

let nextDownloadId = 1;
let downloads: DownloadEntry[] = [];
let downloadTimer: ReturnType<typeof setInterval> | undefined;
/** O ritmo de cada download de exemplo enquanto baixa (bytes por segundo). */
const FAKE_SPEED = new Map<number, number>();

/** Como o núcleo: ligados só com a chave e o aviso legal aceito. */
function downloadsOn(): boolean {
  return settings.get("DownloadsEnabled") === "true" && settings.get("DownloadsLegalAccepted") === "true";
}

function requireDownloads() {
  if (!downloadsOn()) throw new FakeError("conflict", "os downloads estão desligados");
  // Recarregar a página com tudo ligado (a configuração fica na sessão) também começa com os de exemplo.
  seedDownloads();
  ensureDownloadTimer();
}

function announceDownloads() {
  window.__takoDeliver?.(JSON.stringify({ v: 1, event: { name: "downloads.changed" } }));
}

function newDownload(title: string, gameId: string | null, total: number, done: number): DownloadEntry {
  return {
    id: nextDownloadId++,
    title,
    gameId,
    source: "Minha fonte",
    state: "queued",
    error: null,
    done,
    total,
    progress: done / total,
    speed: 0,
    etaSeconds: null,
  };
}

/** Sobe os da fila até o limite de paralelos e recalcula velocidade, tempo restante e progresso. */
function schedule() {
  let running = downloads.filter((d) => d.state === "downloading").length;
  for (const d of downloads) {
    if (d.state === "queued" && running < MAX_PARALLEL) {
      d.state = "downloading";
      running += 1;
    }
  }
  for (const d of downloads) {
    d.speed = d.state === "downloading" ? (FAKE_SPEED.get(d.id) ?? 40 * MB) : 0;
    d.etaSeconds = d.speed > 0 && d.total !== null ? Math.ceil((d.total - d.done) / d.speed) : null;
    d.progress = d.total === null ? null : d.done / d.total;
  }
}

function tickDownloads() {
  for (const d of downloads) {
    if (d.state !== "downloading" || d.total === null) continue;
    d.done = Math.min(d.total, d.done + d.speed);
    if (d.done >= d.total) d.state = "completed";
  }
  schedule();
  if (!downloads.some((d) => d.state === "downloading")) stopDownloadTimer();
  announceDownloads();
}

function stopDownloadTimer() {
  if (downloadTimer === undefined) return;
  clearInterval(downloadTimer);
  downloadTimer = undefined;
}

function ensureDownloadTimer() {
  if (downloadTimer === undefined && downloads.some((d) => d.state === "downloading")) {
    downloadTimer = setInterval(tickDownloads, 1000);
  }
}

/** Os da primeira vez que se liga: ligados a jogos da lista que não estão instalados. */
let seeded = false;
function seedDownloads() {
  if (seeded) return;
  seeded = true;
  const samples: [title: string, gameId: string, total: number, done: number, speed: number, paused?: boolean][] = [
    ["Watch Dogs 2", "steam:1002", 6.2 * GB, 3.5 * GB, 48 * MB],
    ["Black Myth: Wukong", "steam:1010", 9.4 * GB, 1.1 * GB, 32 * MB],
    ["Death Stranding", "steam:1011", 4.8 * GB, 0, 24 * MB],
    ["Ghost of Tsushima Director's Cut", "steam:1012", 7.5 * GB, 2.3 * GB, 40 * MB, true],
  ];
  downloads = samples.map(([title, gameId, total, done, speed, paused]) => {
    const entry = newDownload(title, gameId, Math.round(total), Math.round(done));
    FAKE_SPEED.set(entry.id, speed);
    if (paused) entry.state = "paused";
    return entry;
  });
  schedule();
}

function downloadById(payload: unknown): DownloadEntry {
  const { downloadId } = payload as DownloadRequest;
  const entry = downloads.find((d) => d.id === downloadId);
  if (!entry) throw new FakeError("not_found", `download ${String(downloadId)}`);
  return entry;
}

function changeDownload(payload: unknown, from: DownloadEntry["state"][], to: DownloadEntry["state"]) {
  requireDownloads();
  const entry = downloadById(payload);
  if (!from.includes(entry.state)) throw new FakeError("conflict", `download ${String(entry.id)} está ${entry.state}`);
  entry.state = to;
  schedule();
  ensureDownloadTimer();
  announceDownloads();
}

const SAMPLE_DOWNLOAD_SOURCES: DownloadSource[] = [
  { id: "fonte-1", name: "Minha fonte", host: "exemplo.com.br", itemCount: 128, dropped: 2, updatedAt: "2026-10-20T12:00:00Z" },
];

Object.assign(defaults, {
  "downloads.list": () => {
    requireDownloads();
    return downloads.map((d) => ({ ...d }));
  },
  "downloads.summary": (): DownloadsSummary => {
    requireDownloads();
    const open = downloads.filter((d) => d.state === "downloading" || d.state === "queued");
    return {
      downloading: downloads.filter((d) => d.state === "downloading").length,
      queued: downloads.filter((d) => d.state === "queued").length,
      speed: downloads.reduce((sum, d) => sum + d.speed, 0),
      done: open.reduce((sum, d) => sum + d.done, 0),
      total: open.reduce((sum, d) => sum + (d.total ?? 0), 0),
    };
  },
  "downloads.pause": (payload: unknown) => {
    changeDownload(payload, ["downloading", "queued"], "paused");
  },
  "downloads.resume": (payload: unknown) => {
    changeDownload(payload, ["paused", "failed"], "queued");
  },
  "downloads.cancel": (payload: unknown) => {
    requireDownloads();
    const { deleteFiles } = payload as DownloadCancel;
    const entry = downloadById(payload);
    console.info("[hospedeiro falso] cancelar download:", entry.title, deleteFiles ? "(apagando os arquivos)" : "(mantendo os arquivos)");
    downloads = downloads.filter((d) => d.id !== entry.id);
    schedule();
    announceDownloads();
  },
  "downloads.sources": (): DownloadSources => {
    requireDownloads();
    return { loading: false, sources: SAMPLE_DOWNLOAD_SOURCES, items: [], total: 0 };
  },
  "downloads.add": (payload: unknown): DownloadEntry => {
    requireDownloads();
    const { title, gameId } = payload as DownloadAdd;
    const open = ["downloading", "queued", "paused"];
    if (gameId && downloads.some((d) => d.gameId === gameId && open.includes(d.state)))
      throw new FakeError("conflict", "este jogo já tem um download");
    const entry = newDownload(title, gameId ?? null, 5 * GB, 0);
    FAKE_SPEED.set(entry.id, 36 * MB);
    downloads.push(entry);
    schedule();
    ensureDownloadTimer();
    announceDownloads();
    return { ...entry };
  },
});

/** Por cima do que já valida: o limite só aceita as quatro escolhas do núcleo, e ligar põe o motor para andar. */
const setSettingBeforeDownloads = defaults["settings.set"];
Object.assign(defaults, {
  "settings.set": (payload: unknown) => {
    const { key, value } = payload as Setting;
    if (key === "DownloadsUploadLimitKiB" && !["0", "64", "256", "1024"].includes(value))
      throw new Error("bad_request: configuração inválida");
    setSettingBeforeDownloads?.(payload);
    if (key !== "DownloadsEnabled" && key !== "DownloadsLegalAccepted") return;
    if (downloadsOn()) {
      seedDownloads();
      ensureDownloadTimer();
    } else {
      // Desligar solta o motor: os downloads param e voltam pausados quando a função for ligada de novo.
      stopDownloadTimer();
      for (const d of downloads) if (d.state === "downloading" || d.state === "queued") d.state = "paused";
      schedule();
    }
  },
});

// ---- Abertura do app (WEB-35) ----
// Parâmetros da página para ver a abertura no navegador:
//   ?demora=2000  a `library.list` só responde depois desse tanto desde a abertura (a entrada espera a biblioteca chegar);
//   ?primeira     biblioteca vazia e leitura das lojas em andamento (a tela de primeira leitura);
//   ?limpar       esquece o `ui.state` guardado (sem ele a página abre como a primeira abertura).
// Sem `ui.state` guardado vale a abertura normal; com ele (como ao voltar de um jogo) o app aparece direto.
const openParams = typeof location === "undefined" ? new URLSearchParams() : new URLSearchParams(location.search);
if (openParams.has("limpar")) {
  sessionStorage.removeItem("tako.dev.ui.state");
  uiState = { json: null };
}
const openDelayMs = Number(openParams.get("demora")) || 0;
const firstRead = openParams.has("primeira");
if (openDelayMs > 0 || firstRead) {
  const listBase = defaults["library.list"];
  const readyAt = performance.now() + openDelayMs;
  Object.assign(defaults, {
    "library.list": async (payload: unknown) => {
      const wait = readyAt - performance.now();
      if (wait > 0) await new Promise((resolve) => setTimeout(resolve, wait));
      const page = listBase?.(payload) as { rows: unknown[]; total: number };
      return firstRead && sources.scanning ? { rows: [], total: 0 } : page;
    },
  });
}
if (firstRead) {
  sources = { ...sources, completed: 0, sources: [] };
  scanSources();
}

// ---- Configurações do jogo (WEB-12b) ----
// As escolhas de cada jogo ficam em memória; o padrão vem das configurações do Tako (`settings`), as telas e as saídas
// de áudio são as de exemplo acima. Mesma conta do núcleo (`crates/tako-app/src/game_settings.rs`) em menor escala.

interface FakeGameSettings {
  useScreen: boolean | null;
  screen: string | null;
  mode: GameChoice;
  hdr: boolean | null;
  vrr: boolean | null;
  audio: GameChoice;
  arguments: string | null;
  workingDirectory: string | null;
  environment: string | null;
  beforeCommand: string | null;
  afterCommand: string | null;
}

const emptyGameSettings = (): FakeGameSettings => ({
  useScreen: null,
  screen: null,
  mode: { kind: "default" },
  hdr: null,
  vrr: null,
  audio: { kind: "default" },
  arguments: null,
  workingDirectory: null,
  environment: null,
  beforeCommand: null,
  afterCommand: null,
});

/** Dois jogos já com escolhas: o primeiro na TV (o painel abre mostrando "Padrão: …"), o outro com comandos só de leitura. */
const gameSettingsStore = new Map<string, FakeGameSettings>([
  [
    "steam:1000",
    {
      ...emptyGameSettings(),
      screen: "DISPLAY2",
      mode: { kind: "pick", id: "3840x2160@60" },
      hdr: true,
      audio: { kind: "pick", id: "tv" },
      arguments: "-windowed",
    },
  ],
  ["steam:1003", { ...emptyGameSettings(), beforeCommand: "mount.bat", afterCommand: "umount.bat" }],
]);

const sameFakeId = (a: string, b: string) => a.trim().toLowerCase() === b.trim().toLowerCase();
const fakeFlag = (key: string) => settings.get(key)?.trim().toLowerCase() === "true";
const textOrNull = (value: string | null | undefined) => (value?.trim() ? value.trim() : null);
const fakeMode = (saved: GameChoice) => (saved.kind === "keep" ? "" : saved.kind === "pick" ? saved.id : null);

function fakeGameSettingsView(gameId: string): GameSettingsView {
  if (!games.some((g) => g.id === gameId)) throw new FakeError("not_found", gameId);
  const saved = gameSettingsStore.get(gameId) ?? emptyGameSettings();
  const takoScreen = (settings.get("GameScreenId") ?? "").trim();
  const screens = SAMPLE_DISPLAYS.displays;
  const own = saved.screen ? (screens.find((d) => sameFakeId(d.id, saved.screen ?? ""))?.id ?? saved.screen) : null;
  const screenId = own ?? takoScreen;
  const shown = screens.find((d) => sameFakeId(d.id, screenId));
  const takoMode = textOrNull(settings.get("GameScreenMode"));
  const defaultMode = sameFakeId(screenId, takoScreen) ? takoMode : null;
  const useDefault = own !== null || fakeFlag("GameScreenEnabled");
  const hdrDefault = fakeFlag("GameScreenHdr");
  const vrrDefault = fakeFlag("GameScreenVrr");

  const modeSaved = fakeMode(saved.mode);
  const modeOptions = [...(shown?.modes ?? [])];
  if (saved.mode.kind === "pick" && !modeOptions.includes(saved.mode.id)) modeOptions.push(saved.mode.id);

  const outputs = SAMPLE_AUDIO.outputs.map((o) => ({ id: o.id, name: o.name, connected: true }));
  const takoAudioId = (settings.get("GameAudioDeviceId") ?? "").trim();
  const takoAudio = takoAudioId
    ? { id: takoAudioId, name: settings.get("GameAudioDeviceName") || takoAudioId }
    : null;
  const savedAudio = saved.audio;
  const audioEffective =
    savedAudio.kind === "pick"
      ? { id: savedAudio.id, name: outputs.find((o) => o.id === savedAudio.id)?.name ?? savedAudio.id }
      : savedAudio.kind === "keep"
        ? null
        : takoAudio;
  const environment = textOrNull(saved.environment);
  const changes = [
    saved.useScreen !== null,
    own !== null,
    saved.mode.kind !== "default",
    saved.hdr !== null,
    saved.vrr !== null,
    saved.audio.kind !== "default",
    textOrNull(saved.arguments) !== null,
    textOrNull(saved.workingDirectory) !== null,
    environment !== null,
    textOrNull(saved.beforeCommand) !== null,
    textOrNull(saved.afterCommand) !== null,
  ].filter(Boolean).length;
  const text = (value: string | null) => {
    const clean = textOrNull(value);
    return { saved: clean, effective: clean };
  };

  return {
    gameId,
    changes,
    useScreen: { saved: saved.useScreen, default: useDefault, effective: saved.useScreen ?? useDefault },
    screen: {
      saved: own,
      default: takoScreen
        ? {
            id: takoScreen,
            name: settings.get("GameScreenName") || (screens.find((d) => sameFakeId(d.id, takoScreen))?.name ?? takoScreen),
          }
        : null,
      effective: (saved.useScreen ?? useDefault) && screenId ? screenId : null,
      options: screens.map((d) => ({ id: d.id, name: d.name, connected: true })),
      noScreenChosen: saved.useScreen === true && !screenId,
    },
    mode: {
      saved: saved.mode,
      default: defaultMode,
      effective: modeSaved === null ? defaultMode : modeSaved === "" ? null : modeSaved,
      options: modeOptions,
    },
    hdr: { saved: saved.hdr, default: hdrDefault, effective: saved.hdr ?? hdrDefault },
    vrr: {
      saved: saved.vrr,
      default: vrrDefault,
      effective: saved.vrr ?? vrrDefault,
      unsupported: shown?.vrr === "unsupported",
      limitation: SAMPLE_DISPLAYS.vrrLimitation,
    },
    audio: {
      saved: saved.audio,
      default: takoAudio,
      effective: audioEffective,
      options: outputs,
      canSwitch: SAMPLE_AUDIO.canSwitch,
    },
    arguments: text(saved.arguments),
    workingDirectory: text(saved.workingDirectory),
    environment: { saved: environment, effective: environment },
    beforeCommand: text(saved.beforeCommand),
    afterCommand: text(saved.afterCommand),
    commandsApproved:
      !textOrNull(saved.beforeCommand) && !textOrNull(saved.afterCommand) && !textOrNull(saved.arguments) && !environment,
  };
}

function badFake(message: string): never {
  throw new FakeError("bad_request", message);
}

function setFakeGameSettings(payload: unknown): GameSettingsView {
  const { gameId, change } = payload as GameSettingsSet;
  fakeGameSettingsView(gameId); // o jogo precisa existir
  const value: FakeGameSettings = { ...(gameSettingsStore.get(gameId) ?? emptyGameSettings()) };
  const screens = SAMPLE_DISPLAYS.displays;
  const screenIdOf = () => value.screen ?? (settings.get("GameScreenId") ?? "").trim();
  switch (change.field) {
    case "use_screen":
      value.useScreen = change.value;
      break;
    case "hdr":
      value.hdr = change.value;
      break;
    case "vrr": {
      const shown = screens.find((d) => sameFakeId(d.id, screenIdOf()));
      if (change.value !== null && shown?.vrr === "unsupported") throw new FakeError("conflict", "a tela não informa VRR");
      value.vrr = change.value;
      break;
    }
    case "screen": {
      if (change.value === null) {
        value.screen = null;
        value.mode = { kind: "default" };
        break;
      }
      const screen = screens.find((d) => sameFakeId(d.id, change.value ?? ""));
      if (!screen) throw new FakeError("not_found", "tela não encontrada");
      if (value.screen !== screen.id) value.mode = { kind: "default" };
      value.screen = screen.id;
      break;
    }
    case "mode": {
      if (change.value.kind === "pick") {
        const id = change.value.id;
        const shown = screens.find((d) => sameFakeId(d.id, screenIdOf()));
        if (!/^\d+x\d+@\d+$/.test(id)) badFake("resolução inválida");
        if (value.mode.kind !== "pick" || value.mode.id !== id) {
          if (!screenIdOf()) throw new FakeError("conflict", "nenhuma tela de jogo escolhida");
          if (!shown?.modes.includes(id)) throw new FakeError("not_found", "resolução não encontrada");
        }
      }
      value.mode = change.value;
      break;
    }
    case "audio": {
      if (change.value.kind === "pick") {
        const id = change.value.id;
        if (!SAMPLE_AUDIO.outputs.some((o) => o.id === id)) throw new FakeError("not_found", "saída não encontrada");
        // A saída "com defeito" mostra o aviso de gravação que falhou e a volta ao valor de antes.
        if (id === BROKEN_AUDIO_ID) throw new Error("internal: não foi possível gravar");
      }
      value.audio = change.value;
      break;
    }
    case "arguments": {
      const text = change.value;
      if (text !== null && (text.length > 2048 || /\p{Cc}/u.test(text))) badFake("argumentos inválidos");
      value.arguments = textOrNull(text);
      break;
    }
    case "working_directory": {
      const text = textOrNull(change.value);
      if (text !== null && (text.length > 1024 || /\p{Cc}/u.test(text))) badFake("pasta de trabalho inválida");
      // Não há disco aqui: um caminho com "inexistente" faz de conta que a pasta não existe.
      if (text?.toLowerCase().includes("inexistente")) throw new FakeError("not_found", "pasta de trabalho não encontrada");
      value.workingDirectory = text;
      break;
    }
    case "environment": {
      const items = (change.value ?? "")
        .split(/[\n;]/)
        .map((item) => item.trim())
        .filter(Boolean);
      if (items.length > 32 || items.some((item) => !/^[A-Za-z_][A-Za-z0-9_]*=/.test(item))) badFake("variáveis inválidas");
      value.environment = items.length > 0 ? items.join(";") : null;
      break;
    }
    case "reset_all":
      gameSettingsStore.delete(gameId);
      return fakeGameSettingsView(gameId);
  }
  gameSettingsStore.set(gameId, value);
  return fakeGameSettingsView(gameId);
}

Object.assign(defaults, {
  "game.settings": (payload: unknown) => fakeGameSettingsView((payload as GameRequest).gameId),
  "game.settings.set": setFakeGameSettings,
});
