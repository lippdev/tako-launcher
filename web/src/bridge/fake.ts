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
    "common.games.one": "{n} jogo",
    "common.games.other": "{n} jogos",
  },
  "en-US": {
    "settings.general.title": "General",
    "common.games.one": "{n} game",
    "common.games.other": "{n} games",
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
