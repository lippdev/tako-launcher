import type { Call, Event } from "./index";
import type { ControlCenterState } from "./generated/ControlCenterState";
import type { ControlCenterAction } from "./generated/ControlCenterAction";
import type { BridgeError } from "./generated/BridgeError";
import type { ErrorCode } from "./generated/ErrorCode";
import type { GameDetails } from "./generated/GameDetails";
import type { GameRequest } from "./generated/GameRequest";
import type { GameEndRequest } from "./generated/GameEndRequest";
import type { GameEnded } from "./generated/GameEnded";
import type { RunningGame } from "./generated/RunningGame";
import type { RunningState } from "./generated/RunningState";
import type { UiState } from "./generated/UiState";
import type { ControllerInfo } from "./generated/ControllerInfo";
import type { ControllerStyle } from "./generated/ControllerStyle";
import type { InputAction } from "./generated/InputAction";
import type { InputNotice } from "./generated/InputNotice";
import type { GameRow } from "../components/ListaDeJogos/ListaDeJogos";
import type { LibraryGame } from "./generated/LibraryGame";
import type { LibraryListRequest } from "./generated/LibraryListRequest";
import type { Collection } from "./generated/Collection";
import type { CollectionCreate } from "./generated/CollectionCreate";
import type { CollectionGames } from "./generated/CollectionGames";
import type { CollectionRename } from "./generated/CollectionRename";
import type { CollectionRequest } from "./generated/CollectionRequest";
import type { SmartCollectionSet } from "./generated/SmartCollectionSet";
import type { GameFavoriteSet } from "./generated/GameFavoriteSet";
import type { GameStatus } from "./generated/GameStatus";
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
import type { EmulatorFolder } from "./generated/EmulatorFolder";
import type { EmulatorFolderSystem } from "./generated/EmulatorFolderSystem";
import type { EmulatorsView } from "./generated/EmulatorsView";
import type { DownloadCancel } from "./generated/DownloadCancel";
import type { DownloadEntry } from "./generated/DownloadEntry";
import type { DownloadRequest } from "./generated/DownloadRequest";
import type { DownloadSource } from "./generated/DownloadSource";
import type { DownloadSources } from "./generated/DownloadSources";
import type { DownloadsSummary } from "./generated/DownloadsSummary";

import type { GameChoice } from "./generated/GameChoice";
import type { GameSettingsSet } from "./generated/GameSettingsSet";
import type { GameSettingsView } from "./generated/GameSettingsView";
import type { ProfileCollection } from "./generated/ProfileCollection";
import type { ProfileCollections } from "./generated/ProfileCollections";
import type { ProfileCreate } from "./generated/ProfileCreate";
import type { ProfileEntry } from "./generated/ProfileEntry";
import type { ProfileRename } from "./generated/ProfileRename";
import type { ProfileRequest } from "./generated/ProfileRequest";
import type { ProfileSession } from "./generated/ProfileSession";
import type { ProfileSessions } from "./generated/ProfileSessions";
import type { ProfileSessionsRequest } from "./generated/ProfileSessionsRequest";
import type { ProfileSummary } from "./generated/ProfileSummary";
import type { ProfilesList } from "./generated/ProfilesList";
import type { GameAdd } from "./generated/GameAdd";
import type { GameAddResult } from "./generated/GameAddResult";
import type { GameEdit } from "./generated/GameEdit";
import type { GameEditSet } from "./generated/GameEditSet";
import type { GameSourceIdSet } from "./generated/GameSourceIdSet";
import type { GameSourceIds } from "./generated/GameSourceIds";
import type { GameSourceIdsCheck } from "./generated/GameSourceIdsCheck";
import type { GameSourceIdsSearch } from "./generated/GameSourceIdsSearch";
import type { GameTracking } from "./generated/GameTracking";
import type { SourceIdResults } from "./generated/SourceIdResults";
import type { SourceIdsChecked } from "./generated/SourceIdsChecked";
import type { SourceIdsFound } from "./generated/SourceIdsFound";
import type { SourceIdsJob } from "./generated/SourceIdsJob";
import type { UpdateOffer } from "./generated/UpdateOffer";
import type { UpdatePrefs } from "./generated/UpdatePrefs";
import type { UpdateStatus } from "./generated/UpdateStatus";
import type { ArtCandidateDto } from "./generated/ArtCandidateDto";
import type { ArtCandidatesView } from "./generated/ArtCandidatesView";
import type { ArtCandidatesReady } from "./generated/ArtCandidatesReady";
import type { ArtCandidatesRequest } from "./generated/ArtCandidatesRequest";
import type { ArtChooseRequest } from "./generated/ArtChooseRequest";
import type { ArtResetRequest } from "./generated/ArtResetRequest";
import type { ArtReviewItem } from "./generated/ArtReviewItem";
import type { ArtReviewView } from "./generated/ArtReviewView";
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
  "input.source": () => {},
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
// Situações de exemplo para as Categorias terem mais de uma fileira (o primeiro já joga).
for (const [i, status] of ([[1, "paused"], [3, "beaten"], [5, "wishlist"], [7, "abandoned"], [8, "beaten"]] as const)) {
  const sample = games[i];
  if (sample) sample.status = status;
}
// ?jogos=600 acrescenta jogos de exemplo (lojas, situações e instalação variadas) para ver a rolagem paginada.
const extraGames = Number(new URLSearchParams(typeof location === "undefined" ? "" : location.search).get("jogos")) || 0;
{
  const stores: Store[] = ["steam", "epic", "gog", "xbox", "ubisoft"];
  const statuses: (GameStatus | null)[] = [null, null, "playing", "beaten", "wishlist", "paused", "abandoned"];
  for (let i = 0; i < Math.min(5000, extraGames); i++) {
    games.push({
      id: `steam:${String(2000 + i)}`,
      title: `Jogo de exemplo ${String(i + 1)}`,
      store: stores[i % stores.length] ?? "steam",
      installed: i % 3 !== 0,
      favorite: i % 17 === 0,
      status: statuses[i % statuses.length] ?? null,
      lastPlayedAt: null,
      playTimeSeconds: 0,
      accent: null,
      hasCover: false,
      hasIcon: true,
      hasHero: false,
      hasLogo: false,
    });
  }
}
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
/** Critérios do pedido, menos a coleção (que o núcleo resolve à parte). */
function matchesRequest(g: LibraryGame, p: LibraryListRequest): boolean {
  const needle = p.search?.trim().toLocaleLowerCase("pt-BR") ?? "";
  return Boolean(
    (p.filter !== "favorites" || g.favorite) &&
    (p.filter !== "installed" || g.installed) &&
    g.title.toLocaleLowerCase("pt-BR").includes(needle) &&
    (!p.stores?.length || p.stores.includes(g.store)) &&
    (!p.genres?.length || p.genres.some((genre) => g.genres?.includes(genre))) &&
    (p.installed == null || p.installed === g.installed) &&
    ((!p.statuses?.length && !p.noStatus) || (g.status === null ? p.noStatus : p.statuses?.includes(g.status))) &&
    (!p.tags?.length || p.tags.every((tag) => gameTags.get(g.id)?.has(tag))),
  );
}
/** O jogo está na coleção: manual = os ids dela; automática = a regra salva, resolvida na hora (como o núcleo). */
function inCollection(g: LibraryGame, collectionId: string): boolean {
  const c = collections.find((c) => c.id === collectionId);
  if (!c) return false;
  if (!c.criteria) return c.gameIds.includes(g.id);
  const { library: rule = {}, favoritesOnly, filterStore } = c.criteria;
  return (
    matchesRequest(g, rule) &&
    (!favoritesOnly || g.favorite) &&
    (!filterStore || g.store === filterStore) &&
    (!rule.collection || inCollection(g, rule.collection))
  );
}
function listLibrary(payload: unknown) {
  const requested = payload as LibraryListRequest;
  // Coleção automática aberta com a ordenação padrão: vale a ordenação salva nela.
  const rule = collections.find((c) => c.id === requested.collection)?.criteria?.library;
  const p: LibraryListRequest =
    rule && (requested.sort ?? "recently_played") === "recently_played" && !requested.reversed
      ? { ...requested, sort: rule.sort ?? "recently_played", reversed: rule.reversed ?? false }
      : requested;
  const rows = games.filter((g) => matchesRequest(g, p) && (!p.collection || inCollection(g, p.collection)));
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
    genres: [...new Set(rows.flatMap((game) => game.genres ?? []))].sort(),
    rows: rows.slice(offset, offset + Math.min(200, Math.max(0, p.limit ?? 40))),
  };
}
let nextCollection = 1;
const cleanName = (name: string) => name.split(" ").map((w) => w.trim()).filter(Boolean).join(" ");
const nameKey = (name: string) => name.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase();
const ofKind = (list: Collection[], auto: boolean) => list.filter((c) => (c.criteria !== null) === auto);
/** Como o núcleo: o nome vale só entre as coleções do mesmo tipo (sem caixa nem acento) e o erro é `bad_request`. */
function checkCollectionName(name: string, auto: boolean, exceptId: string | null = null) {
  const clean = cleanName(name);
  const taken = ofKind(collections, auto).some((c) => c.id !== exceptId && nameKey(c.name) === nameKey(clean));
  if (!clean || [...clean].length > 40 || taken) throw new FakeError("bad_request", "nome inválido");
}
function findCollection(id: string) {
  const c = collections.find((c) => c.id === id);
  if (!c) throw new FakeError("not_found", `coleção ${id}`);
  return c;
}
function setSmartCollection(payload: unknown) {
  const p = payload as SmartCollectionSet;
  if (p.criteria.library?.offset) throw new FakeError("bad_request", "critérios inválidos");
  if (p.criteria.filterStore && (p.criteria.library?.filter ?? "all") !== "all")
    throw new FakeError("bad_request", "filtro de loja exige a aba todos");
  const criteria = JSON.parse(JSON.stringify(p.criteria)) as SmartCollectionSet["criteria"];
  let saved: Collection;
  if (p.collectionId) {
    saved = findCollection(p.collectionId);
    if (!saved.criteria) throw new FakeError("not_found", `coleção automática ${p.collectionId}`);
    checkCollectionName(p.name, true, saved.id);
    saved.name = cleanName(p.name);
    saved.criteria = criteria;
  } else {
    checkCollectionName(p.name, true);
    saved = { id: `smart-${String(nextCollection++)}`, name: cleanName(p.name), gameIds: [], criteria };
    collections.push(saved);
  }
  changed();
  return saved;
}
Object.assign(defaults, {
  "library.list": listLibrary,
  // Manuais primeiro, automáticas depois, como a lista do núcleo.
  "collections.list": () => [...ofKind(collections, false), ...ofKind(collections, true)],
  "collections.create": (p: unknown) => {
    const { name } = p as CollectionCreate;
    checkCollectionName(name, false);
    const created: Collection = { id: `manual-${String(nextCollection++)}`, name: cleanName(name), gameIds: [], criteria: null };
    collections.push(created);
    changed();
    return created;
  },
  "collections.rename": (p: unknown) =>
    mutate<CollectionRename>(p, (p) => {
      const c = findCollection(p.collectionId);
      checkCollectionName(p.name, c.criteria !== null, c.id);
      c.name = cleanName(p.name);
    }),
  "collections.delete": (p: unknown) =>
    mutate<CollectionRequest>(p, (p) => {
      collections.splice(collections.indexOf(findCollection(p.collectionId)), 1);
    }),
  "collections.smart.set": setSmartCollection,
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
  "desktop.view",
  "desktop.group_by",
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
const firstRunDemo = typeof location !== "undefined" && new URLSearchParams(location.search).has("primeiraVez");
// Cenário isolado: recarregar conserva a conclusão e não apaga as preferências da demo normal.
const SETTINGS_STORAGE = firstRunDemo ? "tako.dev.first-run.settings" : "tako.dev.settings";

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
    (key !== "OpenIn" || value === "desktop" || value === "big-picture") &&
    (key !== "desktop.view" || ["list", "grid", "categories"].includes(value)) &&
    (key !== "desktop.group_by" || ["status", "collection", "tag", "genre", "store", "install"].includes(value));
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
    genres: sample?.details.genres ?? null,
    publisher: sample?.details.publisher ?? null,
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
    // Coleções (WEB-11b): textos de `smart.toml`, `library.toml`, `ui.toml`, `desktop.toml` e `settings.toml`.
    "smart.save": "Salvar como coleção automática",
    "smart.save_desc": "Guarda esta busca, filtros e ordenação; os jogos se atualizam sozinhos.",
    "smart.saved": "Coleção automática salva",
    "smart.name_rule": "Use um nome único de até 40 letras.",
    "smart.delete_title": "Excluir {name}?",
    "smart.delete_text": "Só a coleção será excluída. Os jogos continuam na biblioteca.",
    "library.collection.name_empty": "Dê um nome à coleção.",
    "library.collection.name_too_long": "Use até {n} letras.",
    "library.collection.name_taken": "Já existe uma coleção com esse nome.",
    "collections.title": "Coleções",
    "collections.new": "Nova coleção",
    "collections.name_rule": "Até 40 caracteres, sem repetir o nome de outra coleção.",
    "desktop.collections_manage": "Organizar coleções",
    "desktop.collection_auto": "Auto",
    "desktop.no_collections": "Nenhuma coleção ainda",
    // Modos da lista (WEB-11c): textos de `desktop.toml`.
    "desktop.view.list": "Lista",
    "desktop.view.grid": "Grade",
    "desktop.view.categories": "Categorias",
    "desktop.view.menu": "Modo de exibição",
    "desktop.group.by_prefix": "Agrupar por",
    "desktop.group.status": "Status",
    "desktop.group.collection": "Coleção",
    "desktop.group.tag": "Etiqueta",
    "desktop.group.genre": "Gênero",
    "desktop.group.store": "Loja",
    "desktop.group.install": "Instalação",
    "desktop.group.installed": "Instalados",
    "desktop.group.not_installed": "Não instalados",
    "settings.library.collections_create": "Criar",
    "settings.library.rename": "Renomear",
    "settings.library.delete": "Excluir",
    "settings.library.delete_title": "Excluir \"{name}\"?",
    "settings.library.delete_text": "Os jogos continuam na biblioteca; só a coleção some.",
    "common.save": "Salvar",
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
    // Coleções (WEB-11b): textos de `smart.toml`, `library.toml`, `ui.toml`, `desktop.toml` e `settings.toml`.
    "smart.save": "Save as smart collection",
    "smart.save_desc": "Saves this search, filters and sort order; games update automatically.",
    "smart.saved": "Smart collection saved",
    "smart.name_rule": "Use a unique name of up to 40 characters.",
    "smart.delete_title": "Delete {name}?",
    "smart.delete_text": "Only the collection will be deleted. Games stay in your library.",
    "library.collection.name_empty": "Give the collection a name.",
    "library.collection.name_too_long": "Use up to {n} characters.",
    "library.collection.name_taken": "A collection with this name already exists.",
    "collections.title": "Collections",
    "collections.new": "New collection",
    "collections.name_rule": "Up to 40 characters, not the same as another collection.",
    "desktop.collections_manage": "Manage collections",
    "desktop.collection_auto": "Auto",
    "desktop.no_collections": "No collections yet",
    // Library view modes (WEB-11c): texts from `desktop.toml`.
    "desktop.view.list": "List",
    "desktop.view.grid": "Grid",
    "desktop.view.categories": "Categories",
    "desktop.view.menu": "View mode",
    "desktop.group.by_prefix": "Group by",
    "desktop.group.status": "Status",
    "desktop.group.collection": "Collection",
    "desktop.group.tag": "Tag",
    "desktop.group.genre": "Genre",
    "desktop.group.store": "Store",
    "desktop.group.install": "Install state",
    "desktop.group.installed": "Installed",
    "desktop.group.not_installed": "Not installed",
    "settings.library.collections_create": "Create",
    "settings.library.rename": "Rename",
    "settings.library.delete": "Delete",
    "settings.library.delete_title": "Delete \"{name}\"?",
    "settings.library.delete_text": "The games stay in the library; only the collection goes away.",
    "common.save": "Save",
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

// ---- Perfil e perfis locais (WEB-14c) ----
// Sessões de exemplo contadas a partir de hoje (o relógio é o do navegador, para a semana e os dias fazerem sentido),
// com os jogos e os tempos de `games`, e uma lista de perfis em memória com as mesmas regras do núcleo
// (`tako_core::profiles`): nome de 1 a 32 caracteres e sem repetir, no máximo 8 perfis, o principal e o em uso não se
// apagam. Trocar só grava o perfil da próxima abertura (`restartRequired`).

/** [posição em `games`, dias atrás, hora, minuto, duração em segundos]: da mais recente para a mais antiga. */
const SAMPLE_SESSIONS: [index: number, daysAgo: number, hour: number, minute: number, seconds: number][] = [
  [0, 0, 14, 2, 42 * 60],
  [1, 0, 9, 10, 72 * 60],
  [3, 1, 20, 40, 139 * 60],
  [5, 1, 19, 58, 12 * 60],
  [5, 1, 11, 5, 20],
  [0, 2, 21, 3, 95 * 60],
  [4, 3, 22, 10, 55 * 60],
  [6, 5, 15, 30, 3 * 3600 + 20 * 60],
  [3, 8, 20, 0, 2 * 3600],
  [7, 12, 18, 45, 46 * 60],
  [6, 19, 21, 0, 4 * 3600],
  [8, 33, 17, 15, 3 * 3600 + 10 * 60],
  [2, 70, 10, 0, 12 * 3600],
];

const pad2 = (n: number) => String(n).padStart(2, "0");
const dayText = (date: Date) =>
  `${String(date.getFullYear())}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`;

interface FakeSession {
  gameIndex: number;
  startedAt: Date;
  seconds: number;
}

function fakeSessions(): FakeSession[] {
  const now = new Date();
  return SAMPLE_SESSIONS.map(([gameIndex, daysAgo, hour, minute, seconds]) => ({
    gameIndex,
    startedAt: new Date(now.getFullYear(), now.getMonth(), now.getDate() - daysAgo, hour, minute),
    seconds,
  })).sort((a, b) => b.startedAt.getTime() - a.startedAt.getTime());
}

/** Menos de 1 minuto: "abriu e fechou", fora dos totais (como o núcleo). */
const counts = (s: FakeSession) => (s.seconds < 60 ? 0 : s.seconds);

function profileSession(s: FakeSession): ProfileSession {
  return {
    game: { ...games[s.gameIndex]! },
    startedAt: s.startedAt.toISOString(),
    durationSeconds: s.seconds,
    quick: s.seconds < 60,
  };
}

function monday(from: Date): Date {
  const start = new Date(from.getFullYear(), from.getMonth(), from.getDate());
  start.setDate(start.getDate() - ((start.getDay() + 6) % 7));
  return start;
}

let fakeProfiles: { id: string; name: string }[] = [
  { id: "main", name: "" },
  { id: "p3f9a1c27b40", name: "Ana" },
];
/** O perfil cujos dados esta "abertura" usa; o gravado para a próxima pode ser outro. */
const OPENED_PROFILE = "main";
let nextProfile = "main";
let askOnStart = false;
const MAX_PROFILES = 8;
const MAX_PROFILE_NAME = 32;

function profilesList(): ProfilesList {
  const profiles: ProfileEntry[] = fakeProfiles.map((p) => ({
    id: p.id,
    name: p.name,
    main: p.id === "main",
    inUse: p.id === OPENED_PROFILE,
    next: p.id === nextProfile,
  }));
  return {
    profiles,
    askOnStart,
    maxProfiles: MAX_PROFILES,
    maxNameChars: MAX_PROFILE_NAME,
    restartRequired: nextProfile !== OPENED_PROFILE,
  };
}

/** O nome como o núcleo aceita: sem espaços nas pontas, 1 a 32 caracteres, sem controle e sem repetir (sem olhar caixa). */
function checkProfileName(name: unknown, except: string | null): string {
  const clean = typeof name === "string" ? name.trim() : "";
  const length = [...clean].length;
  // eslint-disable-next-line no-control-regex
  if (length < 1 || length > MAX_PROFILE_NAME || /[\u0000-\u001f\u007f-\u009f]/.test(clean))
    throw new FakeError("bad_request", "nome de perfil inválido");
  if (fakeProfiles.some((p) => p.id !== except && p.name.toLowerCase() === clean.toLowerCase()))
    throw new FakeError("conflict", "já existe um perfil com esse nome");
  return clean;
}

function profileById(payload: unknown): { id: string; name: string } {
  const { profileId } = payload as ProfileRequest;
  if (typeof profileId !== "string" || profileId.trim() === "") throw new FakeError("bad_request", "id de perfil inválido");
  const found = fakeProfiles.find((p) => p.id === profileId);
  if (!found) throw new FakeError("not_found", "perfil não encontrado");
  return found;
}

function profileSummary(): ProfileSummary {
  const sessions = fakeSessions();
  const today = new Date();
  const start = monday(today);
  const week = Array.from({ length: 7 }, (_, i) => {
    const day = new Date(start.getFullYear(), start.getMonth(), start.getDate() + i);
    const text = dayText(day);
    return {
      day: text,
      seconds: sessions.filter((s) => dayText(s.startedAt) === text).reduce((sum, s) => sum + counts(s), 0),
    };
  });
  const opened = fakeProfiles.find((p) => p.id === OPENED_PROFILE) ?? fakeProfiles[0]!;
  const top = [...games].filter((g) => g.playTimeSeconds > 0).sort((a, b) => b.playTimeSeconds - a.playTimeSeconds);
  return {
    profile: { id: opened.id, name: opened.name, main: opened.id === "main", inUse: true, next: opened.id === nextProfile },
    lastPlayedAt: sessions[0] ? sessions[0].startedAt.toISOString() : null,
    totalPlaySeconds: games.reduce((sum, g) => sum + g.playTimeSeconds, 0),
    gameCount: games.length,
    today: dayText(today),
    week,
    topGames: top.slice(0, 3).map((g) => ({ ...g })),
    latestGames: [...games].reverse().slice(0, 3).map((g) => ({ ...g })),
    recentSessions: sessions.slice(0, 5).map(profileSession),
  };
}

function profileSessions(payload: unknown): ProfileSessions {
  const { period } = payload as ProfileSessionsRequest;
  const now = new Date();
  const since =
    period === "week" ? monday(now) : period === "month" ? new Date(now.getFullYear(), now.getMonth(), 1) : null;
  const inPeriod = fakeSessions().filter((s) => since === null || s.startedAt >= since);
  const days: ProfileSessions["days"] = [];
  for (const s of inPeriod.slice(0, 300)) {
    const day = dayText(s.startedAt);
    let group = days[days.length - 1];
    if (group?.day !== day) {
      group = { day, totalSeconds: 0, sessions: [] };
      days.push(group);
    }
    group.totalSeconds += counts(s);
    group.sessions.push(profileSession(s));
  }
  return {
    today: dayText(now),
    count: inPeriod.length,
    totalSeconds: inPeriod.reduce((sum, s) => sum + counts(s), 0),
    days,
  };
}

function profileCollections(): ProfileCollections {
  const manual: ProfileCollection[] = collections.map((c) => {
    const inside = games.filter((g) => c.gameIds.includes(g.id));
    return {
      id: c.id,
      name: c.name,
      automatic: false,
      gameCount: inside.length,
      installedCount: inside.filter((g) => g.installed).length,
      covers: inside.slice(0, 4).map((g) => ({ ...g })),
    };
  });
  // Uma automática de exemplo, para o selo "Automática"; ela não existe em `collections.list`.
  const installed = games.filter((g) => g.installed);
  const automatic: ProfileCollection = {
    id: "auto-installed",
    name: "Instalados",
    automatic: true,
    gameCount: installed.length,
    installedCount: installed.length,
    covers: installed.slice(0, 4).map((g) => ({ ...g })),
  };
  const distinct = new Set(collections.flatMap((c) => c.gameIds));
  return { collections: [...manual, automatic], gameCount: distinct.size };
}

Object.assign(defaults, {
  "profile.summary": profileSummary,
  "profile.sessions": profileSessions,
  "profile.collections": profileCollections,
  "profiles.list": profilesList,
  "profiles.create": (payload: unknown) => {
    const { name } = payload as ProfileCreate;
    const clean = checkProfileName(name, null);
    if (fakeProfiles.length >= MAX_PROFILES) throw new FakeError("conflict", "perfis demais");
    fakeProfiles.push({ id: `p${Math.random().toString(16).slice(2, 14).padEnd(12, "0")}`, name: clean });
    return profilesList();
  },
  "profiles.rename": (payload: unknown) => {
    const profile = profileById(payload);
    profile.name = checkProfileName((payload as ProfileRename).name, profile.id);
    return profilesList();
  },
  "profiles.delete": (payload: unknown) => {
    const profile = profileById(payload);
    if (profile.id === "main" || profile.id === OPENED_PROFILE || profile.id === nextProfile)
      throw new FakeError("conflict", "o perfil principal e o perfil em uso não podem ser apagados");
    fakeProfiles = fakeProfiles.filter((p) => p.id !== profile.id);
    return profilesList();
  },
  "profiles.ask.set": (payload: unknown) => {
    const { on } = payload as { on: unknown };
    if (typeof on !== "boolean") throw new FakeError("bad_request", "valor inválido");
    askOnStart = on;
    return profilesList();
  },
  "profiles.switch": (payload: unknown) => {
    nextProfile = profileById(payload).id;
    return profilesList();
  },
});

// Os textos de `profile.toml` e `profiles.toml` e os de outras telas que o Perfil usa, nos dois idiomas.
Object.assign(SAMPLE_CATALOGS["pt-BR"], {
  "profile.tab": "Perfil",
  "profile.main": "Perfil principal",
  "profile.playing": "Jogando {game} · {time}",
  "profile.playing_title": "Jogando {game}",
  "profile.playing_since": "Agora · desde {time}",
  "profile.last": "Jogou por último {when}",
  "profile.tab_overview": "Visão geral",
  "profile.tab_sessions": "Sessões",
  "profile.tab_collections": "Coleções",
  "profile.switch": "Trocar perfil",
  "profile.edit": "Editar perfil",
  "profile.tile_time": "Tempo jogado",
  "profile.tile_top": "Mais jogado",
  "profile.tile_games": "Jogos na biblioteca",
  "profile.this_week": "Esta semana",
  "profile.nothing": "nada",
  "profile.recent": "Sessões recentes",
  "profile.empty": "Nenhuma sessão ainda. Jogue pelo Tako e o tempo aparece aqui.",
  "profile.period_week": "Esta semana",
  "profile.period_month": "Este mês",
  "profile.period_all": "Tudo",
  "profile.summary.one": "{n} sessão · {time}",
  "profile.summary.other": "{n} sessões · {time}",
  "profile.quick": "Abriu e fechou",
  "profile.in_game": "{time} no jogo",
  "profile.collections.one": "{n} coleção",
  "profile.collections.other": "{n} coleções",
  "profile.installed.one": "{n} instalado",
  "profile.installed.other": "{n} instalados",
  "profile.no_games": "Nenhum jogo ainda",
  "profile.auto": "Automática",
  "profile.new_text": "Junte jogos do seu jeito",
  "profile.duration.minutes": "{m} min",
  "profile.duration.hours": "{h} h",
  "profile.duration.hours_minutes": "{h} h {m} min",
  "profile.weekday.today": "hoje",
  "profile.weekday.mon": "seg",
  "profile.weekday.tue": "ter",
  "profile.weekday.wed": "qua",
  "profile.weekday.thu": "qui",
  "profile.weekday.fri": "sex",
  "profile.weekday.sat": "sáb",
  "profile.weekday.sun": "dom",
  "profile.weekday_long.mon": "Segunda",
  "profile.weekday_long.tue": "Terça",
  "profile.weekday_long.wed": "Quarta",
  "profile.weekday_long.thu": "Quinta",
  "profile.weekday_long.fri": "Sexta",
  "profile.weekday_long.sat": "Sábado",
  "profile.weekday_long.sun": "Domingo",
  "profiles.title": "Perfil",
  "profiles.desc": "Cada pessoa com a própria biblioteca, favoritos, tempo de jogo e configurações",
  "profiles.text":
    "Cada perfil guarda a própria biblioteca, favoritos, status, tempo de jogo, coleções, imagens escolhidas, tema e configurações. Contas das lojas e chaves de API são da máquina e valem para todos.",
  "profiles.main": "Principal",
  "profiles.main_desc": "O perfil que já existia. Ele não pode ser apagado.",
  "profiles.in_use": "Em uso",
  "profiles.new": "Novo perfil",
  "profiles.new_desc": "Começa vazio, com o assistente da primeira execução",
  "profiles.name_hint": "Nome com até {max} caracteres",
  "profiles.use": "Usar este perfil",
  "profiles.rename": "Renomear",
  "profiles.remove": "Apagar perfil",
  "profiles.remove_title": "Apagar o perfil {name}?",
  "profiles.remove_text":
    "A biblioteca, os favoritos, o tempo de jogo e as configurações dele serão apagados. Os jogos instalados continuam no PC. Não dá para desfazer.",
  "profiles.ask": "Perguntar quem está jogando ao abrir",
  "profiles.ask_desc": "Ao abrir o Tako, escolha o perfil antes de começar",
  "profiles.invalid_name": "Digite um nome de 1 a 32 caracteres.",
  "profiles.duplicate_name": "Já existe um perfil com esse nome.",
  "profiles.too_many": "O Tako aceita até 8 perfis.",
  "profiles.save_failed": "Não foi possível gravar os perfis.",
  "desktop.day.today": "Hoje",
  "desktop.day.yesterday": "Ontem",
  "desktop.sessions.see_all": "Ver todas",
  "presentation.play_time.minutes": "{n} min",
  "presentation.play_time.hours": "{n} h",
  "presentation.date.short": "%d/%m",
  "collections.new": "Nova coleção",
  "collections.name_rule": "Até 40 caracteres, sem repetir o nome de outra coleção.",
  "common.save": "Salvar",
});
Object.assign(SAMPLE_CATALOGS["en-US"], {
  "profile.tab": "Profile",
  "profile.main": "Main profile",
  "profile.playing": "Playing {game} · {time}",
  "profile.playing_title": "Playing {game}",
  "profile.playing_since": "Now · since {time}",
  "profile.last": "Last played {when}",
  "profile.tab_overview": "Overview",
  "profile.tab_sessions": "Sessions",
  "profile.tab_collections": "Collections",
  "profile.switch": "Switch profile",
  "profile.edit": "Edit profile",
  "profile.tile_time": "Time played",
  "profile.tile_top": "Most played",
  "profile.tile_games": "Games in library",
  "profile.this_week": "This week",
  "profile.nothing": "none",
  "profile.recent": "Recent sessions",
  "profile.empty": "No sessions yet. Play through Tako and your time shows up here.",
  "profile.period_week": "This week",
  "profile.period_month": "This month",
  "profile.period_all": "All",
  "profile.summary.one": "{n} session · {time}",
  "profile.summary.other": "{n} sessions · {time}",
  "profile.quick": "Opened and closed",
  "profile.in_game": "{time} in game",
  "profile.collections.one": "{n} collection",
  "profile.collections.other": "{n} collections",
  "profile.installed.one": "{n} installed",
  "profile.installed.other": "{n} installed",
  "profile.no_games": "No games yet",
  "profile.auto": "Automatic",
  "profile.new_text": "Group games your way",
  "profile.duration.minutes": "{m} min",
  "profile.duration.hours": "{h} h",
  "profile.duration.hours_minutes": "{h} h {m} min",
  "profile.weekday.today": "today",
  "profile.weekday.mon": "Mon",
  "profile.weekday.tue": "Tue",
  "profile.weekday.wed": "Wed",
  "profile.weekday.thu": "Thu",
  "profile.weekday.fri": "Fri",
  "profile.weekday.sat": "Sat",
  "profile.weekday.sun": "Sun",
  "profile.weekday_long.mon": "Monday",
  "profile.weekday_long.tue": "Tuesday",
  "profile.weekday_long.wed": "Wednesday",
  "profile.weekday_long.thu": "Thursday",
  "profile.weekday_long.fri": "Friday",
  "profile.weekday_long.sat": "Saturday",
  "profile.weekday_long.sun": "Sunday",
  "profiles.title": "Profile",
  "profiles.desc": "Each person with their own library, favorites, play time and settings",
  "profiles.text":
    "Each profile keeps its own library, favorites, status, play time, collections, chosen images, settings and theme. Store accounts and API keys belong to the computer and work for everyone.",
  "profiles.main": "Main",
  "profiles.main_desc": "The profile that already existed. It can't be deleted.",
  "profiles.in_use": "In use",
  "profiles.new": "New profile",
  "profiles.new_desc": "Starts empty, with the first-run assistant",
  "profiles.name_hint": "Name with up to {max} characters",
  "profiles.use": "Use this profile",
  "profiles.rename": "Rename",
  "profiles.remove": "Delete profile",
  "profiles.remove_title": "Delete the {name} profile?",
  "profiles.remove_text":
    "Its library, favorites, play time and settings will be deleted. Installed games stay on the PC. This can't be undone.",
  "profiles.ask": "Ask who's playing when opening",
  "profiles.ask_desc": "When Tako opens, pick the profile before starting",
  "profiles.invalid_name": "Type a name with 1 to 32 characters.",
  "profiles.duplicate_name": "There's already a profile with that name.",
  "profiles.too_many": "Tako allows up to 8 profiles.",
  "profiles.save_failed": "Couldn't save the profiles.",
  "desktop.day.today": "Today",
  "desktop.day.yesterday": "Yesterday",
  "desktop.sessions.see_all": "See all",
  "presentation.play_time.minutes": "{n} min",
  "presentation.play_time.hours": "{n} h",
  "presentation.date.short": "%m/%d",
  "collections.new": "New collection",
  "collections.name_rule": "Up to 40 characters, not the same as another collection.",
  "common.save": "Save",
});

// ---- Editar jogo, adicionar jogo, remover e Fontes e IDs (WEB-12c) ----
// Em memória. Mesmas regras do núcleo (`crates/tako-app/src/edit.rs`) em menor escala: um caminho com "inexistente"
// faz de conta que o arquivo ou a pasta não existe, e ROM pergunta o sistema.

/** Como `FakeError`, mas `String(e)` dá só a mensagem (o núcleo não põe "Error: " na frente). */
class FakeMessageError extends FakeError {
  override toString() {
    return this.message;
  }
}

interface FakeManual {
  path: string;
  arguments: string;
  workingDirectory: string;
  tracking: GameTracking;
}

const manualGames = new Map<string, FakeManual>();
let nextManual = 1;

const FAKE_ROM_SYSTEMS: Record<string, { key: string; name: string }> = {
  sfc: { key: "snes", name: "SNES" },
  smc: { key: "snes", name: "SNES" },
  nes: { key: "nes", name: "NES" },
  gba: { key: "gba", name: "Game Boy Advance" },
  md: { key: "megadrive", name: "Mega Drive" },
};
const OTHER_ROM_SYSTEMS = [
  { key: "n64", name: "Nintendo 64" },
  { key: "ps2", name: "PlayStation 2" },
  { key: "psx", name: "PlayStation" },
  { key: "dreamcast", name: "Dreamcast" },
];

const fileOf = (path: string) => path.split(/[\\/]/).pop() ?? path;
const extensionOf = (path: string) => (fileOf(path).includes(".") ? (fileOf(path).split(".").pop() ?? "").toLowerCase() : "");
const guessFakeTitle = (path: string) => {
  const stem = fileOf(path).replace(/\.[^.]*$/, "");
  const title = stem.replace(/[_-]/g, " ").trim();
  return title === "" ? stem : title;
};
const romTitle = (path: string) =>
  fileOf(path)
    .replace(/\.[^.]*$/, "")
    .replace(/_/g, " ")
    .replace(/\s*[([][^)\]]*[)\]]/g, "")
    .trim();

function manualOf(payload: unknown): { game: LibraryGame; manual: FakeManual } {
  const { gameId } = payload as GameRequest;
  if (typeof gameId !== "string" || gameId.trim() === "") throw new FakeMessageError("bad_request", "id inválido");
  const game = games.find((g) => g.id === gameId);
  if (!game) throw new FakeMessageError("not_found", "jogo não encontrado");
  const manual = manualGames.get(gameId);
  if (!manual) throw new FakeMessageError("conflict", "só jogos adicionados à mão têm isto");
  return { game, manual };
}

function fakeEditView(game: LibraryGame, manual: FakeManual): GameEdit {
  const folder = manual.path.replace(/[\\/][^\\/]*$/, "");
  return {
    gameId: game.id,
    title: game.title,
    target: manual.path,
    arguments: manual.arguments,
    workingDirectory: manual.workingDirectory,
    defaultFolder: folder === manual.path ? null : folder,
    tracking: manual.tracking,
  };
}

function oneFakeLine(value: string, max: number, what: string) {
  // eslint-disable-next-line no-control-regex -- é justamente o que o núcleo recusa
  if ([...value].length > max || /[\u0000-\u001f\u007f]/.test(value)) throw new FakeMessageError("bad_request", `${what} inválido`);
}

function addFakeGame(payload: unknown): GameAddResult {
  const { path: given, system } = payload as GameAdd;
  const path = (given ?? "").trim();
  // eslint-disable-next-line no-control-regex -- idem
  if (path === "" || [...path].length > 1024 || /\u0000/.test(path)) throw new FakeMessageError("bad_request", "caminho inválido");
  if (path.toLowerCase().includes("inexistente")) throw new FakeMessageError("not_found", "arquivo não encontrado");
  const none: GameAddResult = { status: "added", gameId: null, title: null, fileName: null, systems: [], others: [], };
  const known = [...manualGames].find(([, m]) => m.path.toLowerCase() === path.toLowerCase());
  if (known) return { ...none, status: "already", title: games.find((g) => g.id === known[0])?.title ?? "" };
  const extension = extensionOf(path);
  const rom = FAKE_ROM_SYSTEMS[extension];
  if (extension === "desktop") return { ...none, status: "unreadable" };
  if (system != null && !rom) throw new FakeMessageError("bad_request", "sistema só vale para ROM");
  let title = guessFakeTitle(path);
  if (rom) {
    if (system == null) {
      const others = [...Object.values(FAKE_ROM_SYSTEMS), ...OTHER_ROM_SYSTEMS].filter(
        (item, i, all) => item.key !== rom.key && all.findIndex((x) => x.key === item.key) === i,
      );
      return { ...none, status: "needs_system", fileName: fileOf(path), systems: [rom], others };
    }
    const chosen = [...Object.values(FAKE_ROM_SYSTEMS), ...OTHER_ROM_SYSTEMS].find((item) => item.key === system);
    if (!chosen) throw new FakeMessageError("bad_request", "sistema desconhecido");
    title = romTitle(path);
  }
  const id = `manual:fake${String(nextManual++)}`;
  games.unshift({
    id,
    title,
    store: "manual",
    installed: true,
    favorite: false,
    status: null,
    lastPlayedAt: null,
    playTimeSeconds: 0,
    accent: null,
    hasCover: false,
    hasIcon: false,
    hasHero: false,
    hasLogo: false,
  });
  manualGames.set(id, { path, arguments: "", workingDirectory: "", tracking: "auto" });
  changed();
  return { ...none, status: "added", gameId: id, title };
}

function editFakeGame(payload: unknown): GameEdit {
  const request = payload as GameEditSet;
  const { game, manual } = manualOf(request);
  oneFakeLine(request.title, 200, "nome");
  oneFakeLine(request.arguments, 2048, "argumentos");
  oneFakeLine(request.workingDirectory, 1024, "pasta de trabalho");
  const title = request.title.trim();
  if (title === "") throw new FakeMessageError("bad_request", "o jogo precisa de um nome");
  const folder = request.workingDirectory.trim();
  if (folder.toLowerCase().includes("inexistente")) throw new FakeMessageError("not_found", `pasta não encontrada: ${folder}`);
  if (!["auto", "process", "install_folder", "none"].includes(request.tracking)) throw new FakeMessageError("bad_request", "fim do jogo inválido");
  game.title = title;
  manual.arguments = request.arguments.trim();
  manual.workingDirectory = folder === "" ? manual.path.replace(/[\\/][^\\/]*$/, "") : folder;
  manual.tracking = request.tracking;
  changed();
  return fakeEditView(game, manual);
}

function removeFakeGame(payload: unknown) {
  const { game } = manualOf(payload);
  games.splice(games.indexOf(game), 1);
  manualGames.delete(game.id);
  changed();
}

const FAKE_SOURCES = ["Steam", "IGDB", "SteamGridDB"];
const FAKE_SOURCE_EXAMPLES: Record<string, string> = {
  Steam: "Ex.: 1145360 ou store.steampowered.com/app/1145360",
  IGDB: "Ex.: 113112 ou igdb.com/games/hades",
  SteamGridDB: "Ex.: 5249 ou steamgriddb.com/game/5249",
};
/** O que o casamento automático acharia, por fonte; "" = nada. */
const FAKE_AUTO_IDS: Record<string, string> = { Steam: "", IGDB: "113112", SteamGridDB: "5249" };
/** Ids que o usuário fixou, por jogo e fonte (vazio = "nenhum"). */
const fakePins = new Map<string, Map<string, string>>();

function fakeGameOf(payload: unknown): LibraryGame {
  const { gameId } = payload as GameRequest;
  if (typeof gameId !== "string" || gameId.trim() === "") throw new FakeMessageError("bad_request", "id inválido");
  const game = games.find((g) => g.id === gameId);
  if (!game) throw new FakeMessageError("not_found", "jogo não encontrado");
  return game;
}

function fakeSourceIds(game: LibraryGame): GameSourceIds {
  const pins = fakePins.get(game.id);
  return {
    gameId: game.id,
    title: game.title,
    sources: FAKE_SOURCES.map((source) => {
      const pin = pins?.get(source);
      const none = pin === "";
      const auto = source === "Steam" ? (game.id.startsWith("steam:") ? game.id.slice(6) : "") : (FAKE_AUTO_IDS[source] ?? "");
      const id = none ? null : (pin ?? (auto === "" ? null : auto));
      return {
        source,
        id,
        title: id === null ? null : game.title,
        year: id === null ? null : 2018,
        pinned: pin !== undefined,
        none,
        example: FAKE_SOURCE_EXAMPLES[source] ?? "",
      };
    }),
  };
}

function emitFakeEvent(name: string, payload: unknown, delayMs: number) {
  setTimeout(() => window.__takoDeliver?.(JSON.stringify({ v: 1, event: { name, payload } })), delayMs);
}

let nextFakeJob = 1;

function searchFakeSourceIds(payload: unknown): SourceIdsJob {
  const request = payload as GameSourceIdsSearch;
  const game = fakeGameOf(request);
  const term = (request.term ?? "").trim();
  if (term === "") throw new FakeMessageError("bad_request", "nome vazio");
  if ([...term].length > 200) throw new FakeMessageError("bad_request", "nome inválido");
  const requestId = nextFakeJob++;
  const results: SourceIdResults[] = FAKE_SOURCES.map((source, i) => ({
    source,
    best: String(113112 + i),
    matches: [
      { id: String(113112 + i), title: term, year: 2018, platforms: "PC" },
      { id: String(7000 + i), title: `${term} II`, year: 2024, platforms: "PC" },
      { id: String(8000 + i), title: `${term}: Definitive Edition`, year: null, platforms: null },
    ],
  }));
  // A busca de verdade vai à rede: o "Buscando…" aparece por um instante.
  emitFakeEvent("game.source_ids.found", { requestId, gameId: game.id, term, results } satisfies SourceIdsFound, 450);
  return { requestId };
}

function checkFakeSourceId(payload: unknown): SourceIdsJob {
  const request = payload as GameSourceIdsCheck;
  const game = fakeGameOf(request);
  if (!FAKE_SOURCES.includes(request.source)) throw new FakeMessageError("bad_request", "fonte desconhecida");
  const text = (request.text ?? "").trim();
  const digits = /^\d+$/.test(text) ? text : (/(?:app|games|game|gameid=)\/?(\d+)/.exec(text)?.[1] ?? null);
  if (digits === null) {
    throw new FakeMessageError(
      "bad_request",
      `Não reconheci o jogo nesse texto. ${FAKE_SOURCE_EXAMPLES[request.source] ?? "Número do jogo nesta fonte"}.`,
    );
  }
  const requestId = nextFakeJob++;
  // No falso, só os ids de 1 a 999999 existem.
  const known = Number(digits) > 0 && Number(digits) < 1_000_000;
  const found = known ? { id: digits, name: digits === "113112" ? "Hades (2018)" : `Jogo ${digits} (2020)` } : null;
  emitFakeEvent("game.source_ids.checked", { requestId, gameId: game.id, source: request.source, found } satisfies SourceIdsChecked, 350);
  return { requestId };
}

function setFakeSourceId(payload: unknown): GameSourceIds {
  const request = payload as GameSourceIdSet;
  const game = fakeGameOf(request);
  if (!FAKE_SOURCES.includes(request.source)) throw new FakeMessageError("bad_request", "fonte desconhecida");
  const pins = fakePins.get(game.id) ?? new Map<string, string>();
  switch (request.choice.kind) {
    case "id": {
      const id = request.choice.value.trim();
      if (id === "") throw new FakeMessageError("bad_request", "id vazio");
      oneFakeLine(id, 128, "id");
      pins.set(request.source, id);
      break;
    }
    case "none":
      pins.set(request.source, "");
      break;
    case "auto":
      pins.delete(request.source);
      break;
    default:
      throw new FakeMessageError("bad_request", "escolha inválida");
  }
  fakePins.set(game.id, pins);
  emitFakeEvent("library.changed", undefined, 100);
  emitFakeEvent("art.changed", { gameId: game.id }, 200);
  return fakeSourceIds(game);
}

Object.assign(defaults, {
  "library.add_game": addFakeGame,
  "game.edit": (payload: unknown) => {
    const { game, manual } = manualOf(payload);
    return fakeEditView(game, manual);
  },
  "game.edit.set": editFakeGame,
  "game.remove": removeFakeGame,
  "game.source_ids": (payload: unknown) => fakeSourceIds(fakeGameOf(payload)),
  "game.source_ids.set": setFakeSourceId,
  "game.source_ids.search": searchFakeSourceIds,
  "game.source_ids.check": checkFakeSourceId,
});

// ---- Sessão de jogo (WEB-21) ----
// Uma sessão simulada de alguns segundos: Jogar → abrindo → jogando → fim. Faz o que o núcleo faz
// (`crates/tako-app/src/session.rs`): um jogo por vez (`conflict`), `game.end` que recusa (`conflict`) enquanto a
// janela do jogo não existe, o `game.ended` que espera a próxima leitura de `game.running` quando a interface estava
// fechada. Parâmetros da página para ver os estados no navegador:
//   ?falha[=quick|never_started|launch_failed]  o jogo termina com erro (sem valor: caiu com um código do Windows);
//   ?abrindo=25000  quanto o jogo demora para aparecer (1500 ms; passe de 20 s para ver "Ainda abrindo…" e o cancelar);
//   ?sessao=6000    quanto dura a sessão depois que o jogo aparece (6 s);
//   ?preso          o jogo ignora o pedido de fechar: só "Forçar encerramento" o encerra;
//   ?avisos         a abertura traz um aviso (ferramenta que não abriu);
//   ?fechada        o fim da sessão não chega por evento: fica guardado até a próxima `game.running` (recarregue a página);
//   ?voltou         a página abre com um fim de sessão não visto (com ?falha, o erro).
const sessionParams = typeof location === "undefined" ? new URLSearchParams() : new URLSearchParams(location.search);
const UNSEEN_KEY = "tako.dev.session.unseen";

interface FakeGameSession {
  gameId: string;
  title: string;
  launchedAt: number;
  state: RunningState;
  timers: ReturnType<typeof setTimeout>[];
}
let fakeSession: FakeGameSession | null = null;
let unseenEnd: GameEnded | null = (() => {
  const saved = typeof sessionStorage === "undefined" ? null : sessionStorage.getItem(UNSEEN_KEY);
  return saved ? (JSON.parse(saved) as GameEnded) : null;
})();

const FAILURES = {
  failed: {
    end: "failed",
    exitCode: 0xc0000005,
    exitCodeText:
      "Código de saída 0xC0000005 (3221225477): o jogo acessou uma memória que não podia e caiu (mods, sobreposições, driver de vídeo ou arquivos corrompidos)",
  },
  quick: {
    end: "quick",
    exitCode: 0xc0000135,
    exitCodeText:
      "Código de saída 0xC0000135 (3221225781): uma DLL que o jogo precisa não foi encontrada (instale o Visual C++ e o DirectX, ou repare os arquivos do jogo)",
  },
  never_started: { end: "never_started", exitCode: null, exitCodeText: null },
  launch_failed: { end: "launch_failed", exitCode: null, exitCodeText: null },
} as const;

function sessionFailure() {
  if (!sessionParams.has("falha")) return null;
  const kind = sessionParams.get("falha") ?? "";
  return FAILURES[kind in FAILURES ? (kind as keyof typeof FAILURES) : "failed"];
}

const LAUNCH_FAILED_MESSAGE = "O comando antes do jogo falhou: mount.bat não foi encontrado";

function sessionMs(name: string, fallback: number): number {
  const value = Number(sessionParams.get(name));
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

function deliverEvent(name: Event["name"], payload?: unknown) {
  window.__takoDeliver?.(JSON.stringify({ v: 1, event: payload === undefined ? { name } : { name, payload } }));
}

function endFakeSession(end: GameEnded["end"], requested = false) {
  const session = fakeSession;
  if (!session) return;
  session.timers.forEach(clearTimeout);
  fakeSession = null;
  const failure = sessionFailure();
  const seconds = Math.round((Date.now() - session.launchedAt) / 1000);
  const ended: GameEnded = {
    gameId: session.gameId,
    title: session.title,
    playedSeconds: end === "launch_failed" ? 0 : seconds,
    end,
    exitCode: failure?.end === end ? failure.exitCode : null,
    exitCodeText: failure?.end === end ? failure.exitCodeText : null,
    message: end === "launch_failed" ? LAUNCH_FAILED_MESSAGE : null,
  };
  if (end !== "launch_failed") {
    const game = games.find((g) => g.id === session.gameId);
    if (game) {
      game.lastPlayedAt = new Date().toISOString();
      game.playTimeSeconds += seconds;
    }
    changed();
  }
  if (sessionParams.has("fechada") && !requested) {
    // Sem interface aberta o evento se perde: o núcleo guarda o último fim e o manda depois do próximo `game.running`.
    unseenEnd = ended;
    sessionStorage.setItem(UNSEEN_KEY, JSON.stringify(ended));
  } else {
    deliverEvent("game.ended", ended);
  }
}

function startFakeSession(payload: unknown) {
  const { gameId } = payload as GameRequest;
  const game = games.find((g) => g.id === gameId);
  if (!game) throw new FakeError("not_found", gameId);
  if (!game.installed || SAMPLE_GAMES[gameId]?.details.upcoming || gameId === "steam:1009")
    throw new FakeError("conflict", `${gameId} não pode ser jogado agora`);
  if (fakeSession) throw new FakeError("conflict", "já há um jogo aberto");
  const session: FakeGameSession = { gameId, title: game.title, launchedAt: Date.now(), state: "launching", timers: [] };
  fakeSession = session;
  const failure = sessionFailure();
  const opening = sessionMs("abrindo", 1500);
  const playing = sessionMs("sessao", 6000);
  const after = (ms: number, run: () => void) => {
    session.timers.push(setTimeout(run, ms));
  };
  // O núcleo responde antes de abrir o jogo: o `game.started` vem logo depois da resposta.
  if (failure?.end === "launch_failed") {
    after(800, () => {
      endFakeSession("launch_failed");
    });
    return;
  }
  after(80, () => {
    deliverEvent("game.started", {
      gameId,
      title: game.title,
      startedAt: new Date(session.launchedAt).toISOString(),
      tracked: true,
      notices: sessionParams.has("avisos") ? ["Não foi possível abrir OBS Studio: o arquivo não foi encontrado."] : [],
    });
  });
  after(opening, () => {
    // A janela do jogo apareceu (no Tako de verdade a interface fecha aqui).
    if (fakeSession === session && session.state === "launching") session.state = "playing";
  });
  after(opening + playing, () => {
    endFakeSession(failure ? failure.end : "normal");
  });
}

function readRunningGame(): RunningGame | null {
  const running: RunningGame | null = fakeSession && {
    gameId: fakeSession.gameId,
    title: fakeSession.title,
    startedAt: new Date(fakeSession.launchedAt).toISOString(),
    state: fakeSession.state,
  };
  if (unseenEnd) {
    const end = unseenEnd;
    unseenEnd = null;
    sessionStorage.removeItem(UNSEEN_KEY);
    // Depois da resposta (que sai em ~30 ms), na mesma entrega.
    setTimeout(() => {
      deliverEvent("game.ended", end);
    }, 90);
  }
  return running;
}

function endFakeGame(payload: unknown) {
  const { force } = payload as GameEndRequest;
  const session = fakeSession;
  if (!session) throw new FakeError("conflict", "nenhum jogo aberto");
  const wasLaunching = session.state === "launching";
  session.state = "closing";
  const closeIn = (ms: number) => {
    session.timers.push(
      setTimeout(() => {
        endFakeSession("normal", true);
      }, ms),
    );
  };
  if (force) {
    closeIn(300);
    return;
  }
  // A janela do jogo ainda não existe: não há o que fechar agora (a sessão fica "fechando", como no núcleo).
  if (wasLaunching) throw new FakeError("conflict", "encerrar o jogo: a janela do jogo ainda não existe");
  if (!sessionParams.has("preso")) closeIn(2000);
}

Object.assign(defaults, {
  "game.play": startFakeSession,
  "game.running": readRunningGame,
  "game.resume": () => {
    if (!fakeSession) throw new FakeError("conflict", "nenhum jogo aberto");
    console.info("[hospedeiro falso] voltar ao jogo:", fakeSession.title);
  },
  "game.end": endFakeGame,
});

if (sessionParams.has("voltou") && !unseenEnd) {
  const failure = sessionFailure();
  unseenEnd = {
    gameId: "steam:1000",
    title: "God of War: Ragnarök",
    playedSeconds: failure ? (failure.end === "quick" ? 4 : failure.end === "launch_failed" ? 0 : 3720) : 2520,
    end: failure ? failure.end : "normal",
    exitCode: failure ? failure.exitCode : null,
    exitCodeText: failure ? failure.exitCodeText : null,
    message: failure?.end === "launch_failed" ? LAUNCH_FAILED_MESSAGE : null,
  };
  sessionStorage.setItem(UNSEEN_KEY, JSON.stringify(unseenEnd));
}

Object.assign(SAMPLE_CATALOGS["pt-BR"], {
  "session.closing": "Fechando {game}…",
  "session.closing_wait": "Esperando o jogo fechar.",
  "session.closing_stuck":
    "O jogo ainda não fechou. Ele pode estar perguntando se você quer salvar: volte ao jogo para responder, ou force o encerramento (o que não foi salvo se perde).",
  "session.force_close": "Forçar encerramento",
  "session.other_running": "{game} ainda está aberto",
  "session.welcome_back": "Bem-vindo de volta · {game}",
  "session.welcome_back_played": "Bem-vindo de volta · {game} · {time}",
  "session.ended_quick": "{game} fechou logo depois de abrir",
  "session.ended_quick_text":
    "O jogo fechou sozinho em poucos segundos. Costuma ser uma dependência faltando (DirectX, Visual C++), arquivos corrompidos ou o jogo pedindo para abrir pela loja. Tente de novo, use Reparar arquivos nas opções do jogo ou salve o relatório de diagnóstico para pedir ajuda.",
  "session.ended_error": "{game} fechou com um erro",
  "session.ended_error_text":
    "O jogo fechou sozinho com um código de erro. Tente de novo, use Reparar arquivos nas opções do jogo ou salve o relatório de diagnóstico para pedir ajuda.",
  "session.never_started": "{game} não abriu",
  "session.never_started_text":
    "O Tako esperou 2 minutos e o jogo não apareceu. A loja pode ter mostrado um aviso, pedido login ou uma atualização: confira na loja, tente de novo ou salve o relatório de diagnóstico.",
  "session.try_again": "Tentar de novo",
  "session.closed_after": "Fechou {time} depois de abrir.",
  "session.repair": "Reparar arquivos",
  "session.save_report": "Salvar relatório de diagnóstico",
  "cc.back_to_game": "Voltar ao jogo",
  "desktop.opening": "Abrindo…",
  "desktop.opening_slow": "Ainda abrindo…",
  "presentation.session.less_than_minute": "menos de 1 min jogado",
  "presentation.session.minutes.one": "{n} min jogado",
  "presentation.session.minutes.other": "{n} min jogados",
  "presentation.session.hours": "{h} h jogadas",
  "presentation.session.hours_minutes": "{h} h {m} min jogados",
});
Object.assign(SAMPLE_CATALOGS["en-US"], {
  "session.closing": "Closing {game}…",
  "session.closing_wait": "Waiting for the game to close.",
  "session.closing_stuck":
    "The game hasn't closed yet. It may be asking whether to save: go back to the game to answer, or force it to close (anything unsaved is lost).",
  "session.force_close": "Force close",
  "session.other_running": "{game} is still open",
  "session.welcome_back": "Welcome back · {game}",
  "session.welcome_back_played": "Welcome back · {game} · {time}",
  "session.ended_quick": "{game} closed right after opening",
  "session.ended_quick_text":
    "The game closed by itself within a few seconds. This is usually a missing dependency (DirectX, Visual C++), corrupted files or the game asking to be opened from its store. Try again, use Repair files in the game options or save the diagnostics report to ask for help.",
  "session.ended_error": "{game} closed with an error",
  "session.ended_error_text":
    "The game closed by itself with an error code. Try again, use Repair files in the game options or save the diagnostics report to ask for help.",
  "session.never_started": "{game} didn't open",
  "session.never_started_text":
    "Tako waited 2 minutes and the game never showed up. The store may have shown a notice, asked you to sign in or started an update: check the store, try again or save the diagnostics report.",
  "session.try_again": "Try again",
  "session.closed_after": "It closed {time} after opening.",
  "session.repair": "Repair files",
  "session.save_report": "Save diagnostics report",
  "cc.back_to_game": "Back to game",
  "desktop.opening": "Opening…",
  "desktop.opening_slow": "Still opening…",
  "presentation.session.less_than_minute": "less than 1 min played",
  "presentation.session.minutes.one": "{n} min played",
  "presentation.session.minutes.other": "{n} min played",
  "presentation.session.hours": "{h} h played",
  "presentation.session.hours_minutes": "{h} h {m} min played",
});

// ---- Aparência do desktop e emuladores (WEB-13c1) ----
// As chaves novas, com a mesma validação do núcleo (`crates/tako-app/src/settings.rs`), e `emulators.*` com estado em
// memória: pastas de ROMs que mudam a lista e a contagem do "Emuladores" na próxima leitura.

for (const key of ["DesktopAccent", "DesktopGlassOpacity", "DesktopGlassAmbient", "DesktopGlassPill"]) KNOWN_SETTINGS.add(key);
for (const key of ["DesktopGlassAmbient", "DesktopGlassPill"]) BOOL_SETTINGS.add(key);

const setSettingBeforeLook = defaults["settings.set"];
Object.assign(defaults, {
  "settings.set": (payload: unknown) => {
    const { key, value } = payload as Setting;
    // `#RRGGBB` e uma posição de 0 a 100, como o núcleo aceita.
    if (key === "DesktopAccent" && !/^#[0-9a-fA-F]{6}$/.test(value)) throw new Error("bad_request: configuração inválida");
    if (key === "DesktopGlassOpacity" && !(/^\d{1,3}$/.test(value) && Number(value) <= 100))
      throw new Error("bad_request: configuração inválida");
    setSettingBeforeLook?.(payload);
  },
});

/** Os emuladores de `Emulator::ALL`; os dois primeiros "achados" neste PC de exemplo. */
const SAMPLE_EMULATORS: EmulatorsView["emulators"] = [
  { key: "retroarch", name: "RetroArch", found: true, path: "C:\\Program Files\\RetroArch\\retroarch.exe", flatpak: null, chosen: false },
  { key: "dolphin", name: "Dolphin", found: true, path: "C:\\Program Files\\Dolphin\\Dolphin.exe", flatpak: null, chosen: false },
  { key: "pcsx2", name: "PCSX2", found: false, path: null, flatpak: null, chosen: false },
  { key: "duckstation", name: "DuckStation", found: false, path: null, flatpak: null, chosen: false },
  { key: "ppsspp", name: "PPSSPP", found: false, path: null, flatpak: null, chosen: false },
  { key: "ryujinx", name: "Ryujinx", found: false, path: null, flatpak: null, chosen: false },
  { key: "cemu", name: "Cemu", found: false, path: null, flatpak: null, chosen: false },
  { key: "rpcs3", name: "RPCS3", found: false, path: null, flatpak: null, chosen: false },
];
const SAMPLE_ROM_SYSTEMS: EmulatorsView["systems"] = [
  ["nes", "Nintendo Entertainment System"],
  ["snes", "Super Nintendo"],
  ["n64", "Nintendo 64"],
  ["gba", "Game Boy Advance"],
  ["gc", "GameCube"],
  ["psx", "PlayStation"],
  ["ps2", "PlayStation 2"],
  ["ps3", "PlayStation 3"],
  ["psp", "PlayStation Portable"],
  ["megadrive", "Mega Drive"],
  ["dreamcast", "Dreamcast"],
  ["arcade", "Arcade"],
].map(([id, name]) => ({ id: id as string, name: name as string }));
const MAX_ROM_FOLDERS = 256;
const ROM_FOLDERS_STORAGE = firstRunDemo ? "tako.dev.first-run.rom-folders" : "tako.dev.rom-folders";

const romFolders: EmulatorsView["romFolders"] = JSON.parse(
  sessionStorage.getItem(ROM_FOLDERS_STORAGE) ?? "[]",
) as EmulatorsView["romFolders"];

function emulatorsView(): EmulatorsView {
  return {
    romFolders: romFolders.map((folder) => ({ ...folder })),
    emulators: SAMPLE_EMULATORS.map((emulator) => ({ ...emulator })),
    systems: SAMPLE_ROM_SYSTEMS,
    maxFolders: MAX_ROM_FOLDERS,
  };
}

/** Uma mudança nas pastas: o falso guarda, conta as ROMs do "Emuladores" e lê as lojas de novo, como o núcleo. */
function romFoldersChanged() {
  sessionStorage.setItem(ROM_FOLDERS_STORAGE, JSON.stringify(romFolders));
  const emulatorStore = SAMPLE_STORES.find((sample) => sample.store === "emulators");
  if (emulatorStore) emulatorStore.found = romFolders.length * 12;
  if (firstRunDemo) syncFirstRunRoms();
  scanSources();
}

function folderIndex(path: string): number {
  return romFolders.findIndex((folder) => folder.path === path.trim());
}

Object.assign(defaults, {
  "emulators.list": emulatorsView,
  "emulators.folder.add": (payload: unknown) => {
    const path = (payload as EmulatorFolder).path.trim();
    if (!path || path.length > 4096 || path.includes("\0") || !/^([a-zA-Z]:[\\/]|\\\\|\/)/.test(path))
      throw new FakeError("bad_request", "caminho de pasta inválido");
    // Um caminho com "inexistente" faz o falso responder como o núcleo para uma pasta que não existe.
    if (path.toLowerCase().includes("inexistente")) throw new FakeError("bad_request", "a pasta não existe");
    if (folderIndex(path) >= 0) return emulatorsView();
    if (romFolders.length >= MAX_ROM_FOLDERS) throw new FakeError("conflict", "pastas de ROMs demais");
    romFolders.push({ path, system: null });
    romFoldersChanged();
    return emulatorsView();
  },
  "emulators.folder.remove": (payload: unknown) => {
    const index = folderIndex((payload as EmulatorFolder).path);
    if (index < 0) throw new FakeError("not_found", "pasta de ROMs não encontrada");
    romFolders.splice(index, 1);
    romFoldersChanged();
    return emulatorsView();
  },
  "emulators.folder.system.set": (payload: unknown) => {
    const { path, system } = payload as EmulatorFolderSystem;
    if (system !== null && !SAMPLE_ROM_SYSTEMS.some((sample) => sample.id === system))
      throw new FakeError("bad_request", "sistema inválido");
    const folder = romFolders[folderIndex(path)];
    if (!folder) throw new FakeError("not_found", "pasta de ROMs não encontrada");
    if (folder.system !== system) {
      folder.system = system;
      romFoldersChanged();
    }
    return emulatorsView();
  },
});
// ---- Atualização do app (WEB-33) ----
// O ciclo do Slint sem rede nenhuma: procurar, achar, baixar com progresso, pronta, reiniciar. O cenário vem de
// `?atualizacao=` na URL: nova (padrão) · em-dia · falha · sem-arquivo · falha-download · jogo-aberto. Com o parâmetro,
// a procura automática ao abrir também roda (a de verdade roda ao abrir o núcleo); sem ele, só o botão procura.

Object.assign(SAMPLE_CATALOGS["pt-BR"], {
  "desktop.window.update_ready": "Atualização pronta",
  "updates.finish_game": "Encerre o jogo antes de atualizar",
  "updates.apply_failed": "Não foi possível iniciar a atualização: {error}",
  "updates.desktop.searching": "Procurando…",
  "updates.desktop.current": "Você está em dia",
  "updates.desktop.check_failed": "Não deu para procurar",
  "updates.desktop.current_version": "Versão atual {version}",
  "updates.desktop.current_toast": "Você está em dia — {version}",
  "updates.desktop.failed_toast": "Não deu para procurar atualizações",
  "updates.desktop.found": "Atualização {version} encontrada",
  "updates.desktop.download_failed": "Não deu para atualizar",
  "updates.desktop.progress": "{done} de {total} MB",
  "updates.desktop.remaining": "{seconds} s restantes",
  "updates.desktop.hide": "Esconder",
  "updates.desktop.later": "Depois",
  "updates.desktop.restart": "Reiniciar agora",
  "updates.desktop.retry": "Tentar de novo",
  "updates.desktop.downloading": "Baixando",
  "updates.desktop.notes": "Novidades",
  "updates.desktop.all_notes": "Ver todas as notas ↗",
  "updates.desktop.view": "Ver",
  "updates.desktop.size": "· {size} MB",
  "updates.desktop.help_downloading": "Pode fechar: continua em segundo plano e avisa na barra superior.",
  "updates.desktop.help_ready": '"Depois": instala quando você sair do Tako.',
  "settings.general.updates_header": "Atualizações",
  "settings.general.updates": "Procurar atualizações",
  "settings.general.updates_desc": "Confere se há uma versão nova do Tako",
  "settings.general.updates_value": "Procurar",
  "settings.general.autoupdate": "Atualizar automaticamente",
  "settings.general.autoupdate_desc": "Procura e baixa a versão nova em segundo plano; instala ao reiniciar ou ao sair do Tako",
  "settings.general.prerelease": "Receber versões de teste",
  "settings.general.prerelease_desc": "Versões alpha e beta chegam antes, mas podem ter falhas",
  "settings.general.nightly": "Receber versões noturnas",
  "settings.general.nightly_desc": "Uma build por dia com tudo que entrou no desenvolvimento, sem revisão. Para quem quer testar",
});
Object.assign(SAMPLE_CATALOGS["en-US"], {
  "desktop.window.update_ready": "Update ready",
  "updates.finish_game": "Close the game before updating",
  "updates.apply_failed": "Couldn't start the update: {error}",
  "updates.desktop.searching": "Checking…",
  "updates.desktop.current": "You're up to date",
  "updates.desktop.check_failed": "Couldn't check for updates",
  "updates.desktop.current_version": "Current version {version}",
  "updates.desktop.current_toast": "You're up to date — {version}",
  "updates.desktop.failed_toast": "Couldn't check for updates",
  "updates.desktop.found": "Update {version} found",
  "updates.desktop.download_failed": "Couldn't update",
  "updates.desktop.progress": "{done} of {total} MB",
  "updates.desktop.remaining": "{seconds} s remaining",
  "updates.desktop.hide": "Hide",
  "updates.desktop.later": "Later",
  "updates.desktop.restart": "Restart now",
  "updates.desktop.retry": "Try again",
  "updates.desktop.downloading": "Downloading",
  "updates.desktop.notes": "What's new",
  "updates.desktop.all_notes": "View all release notes ↗",
  "updates.desktop.view": "View",
  "updates.desktop.size": "· {size} MB",
  "updates.desktop.help_downloading": "You can close this: it keeps going in the background and tells you in the top bar.",
  "updates.desktop.help_ready": '"Later": installs when you quit Tako.',
  "settings.general.updates_header": "Updates",
  "settings.general.updates": "Check for updates",
  "settings.general.updates_desc": "Checks whether there's a new version of Tako",
  "settings.general.updates_value": "Check",
  "settings.general.autoupdate": "Update automatically",
  "settings.general.autoupdate_desc": "Finds and downloads the new version in the background; installs it on restart or when you quit Tako",
  "settings.general.prerelease": "Get test versions",
  "settings.general.prerelease_desc": "Alpha and beta versions arrive earlier, but may have bugs",
  "settings.general.nightly": "Get nightly builds",
  "settings.general.nightly_desc": "One build a day with everything merged into development, unreviewed. For testers",
});

type UpdateScenario = "nova" | "em-dia" | "falha" | "sem-arquivo" | "falha-download" | "jogo-aberto";
const UPDATE_SCENARIOS: readonly UpdateScenario[] = ["nova", "em-dia", "falha", "sem-arquivo", "falha-download", "jogo-aberto"];
const updateParam = typeof location === "undefined" ? null : new URLSearchParams(location.search).get("atualizacao");
const updateScenario: UpdateScenario = UPDATE_SCENARIOS.find((s) => s === updateParam) ?? "nova";

const FAKE_UPDATE_OFFER: UpdateOffer = {
  version: "0.3.0",
  notes: [
    "Novo visual: tema TAKO com as cores da arte do jogo",
    "Downloads na barra de cima",
    "Corrigido: lista recolhida perdia o jogo escolhido",
  ],
  pageUrl: "https://github.com/lippdev/tako-launcher/releases/tag/v0.3.0",
  installable: true,
  size: Math.round(18.2 * MB),
};

let fakeUpdateGame = updateScenario === "jogo-aberto";
let fakeUpdateTimer: ReturnType<typeof setInterval> | undefined;
let fakeUpdateStarted = 0;
let fakeDownloadAttempts = 0;
let fakeUpdatePrefs: UpdatePrefs = { autoUpdate: true, prereleases: false, nightly: false };
let fakeUpdate: UpdateStatus = {
  state: "idle",
  current: "0.2.0",
  offer: null,
  progress: 0,
  downloadedBytes: 0,
  remainingSeconds: null,
  gameRunning: fakeUpdateGame,
};

function setFakeUpdate(patch: Partial<UpdateStatus>) {
  fakeUpdate = { ...fakeUpdate, ...patch, gameRunning: fakeUpdateGame };
  emitFakeEvent("updates.changed", fakeUpdate, 0);
}

function startFakeDownload() {
  fakeUpdateStarted = Date.now();
  setFakeUpdate({ state: "downloading", offer: FAKE_UPDATE_OFFER, progress: 0, downloadedBytes: 0, remainingSeconds: null });
  clearInterval(fakeUpdateTimer);
  fakeUpdateTimer = setInterval(() => {
    const progress = Math.min(1, Math.round((fakeUpdate.progress + 0.04) * 1000) / 1000);
    // Falha-download: a primeira tentativa quebra no meio, a segunda (Tentar de novo) termina.
    if (updateScenario === "falha-download" && fakeDownloadAttempts === 0 && progress >= 0.5) {
      fakeDownloadAttempts += 1;
      clearInterval(fakeUpdateTimer);
      setFakeUpdate({ state: "download_failed", remainingSeconds: null });
      return;
    }
    if (progress >= 1) {
      clearInterval(fakeUpdateTimer);
      fakeDownloadAttempts += 1;
      setFakeUpdate({ state: "ready", progress: 1, downloadedBytes: FAKE_UPDATE_OFFER.size, remainingSeconds: null });
      return;
    }
    const elapsed = (Date.now() - fakeUpdateStarted) / 1000;
    const remaining = Math.ceil((elapsed * (1 - progress)) / progress);
    setFakeUpdate({
      progress,
      downloadedBytes: Math.round(FAKE_UPDATE_OFFER.size * progress),
      remainingSeconds: remaining > 0 ? remaining : null,
    });
  }, 250);
}

function fakeUpdateCheck(): UpdateStatus {
  if (["searching", "downloading", "installing", "ready"].includes(fakeUpdate.state)) return fakeUpdate;
  setFakeUpdate({ state: "searching", offer: null, progress: 0, downloadedBytes: 0, remainingSeconds: null });
  // O "Procurando…" aparece por um instante, como no núcleo (mínimo de 600 ms).
  setTimeout(() => {
    if (updateScenario === "em-dia") setFakeUpdate({ state: "current" });
    else if (updateScenario === "falha") setFakeUpdate({ state: "check_failed" });
    else if (updateScenario === "sem-arquivo")
      setFakeUpdate({ state: "available", offer: { ...FAKE_UPDATE_OFFER, installable: false, size: 0 } });
    else startFakeDownload();
  }, 900);
  return fakeUpdate;
}

function fakeUpdateDownload(): UpdateStatus {
  if (fakeUpdate.state !== "download_failed" || fakeUpdate.offer === null)
    throw new FakeError("conflict", "nenhuma atualização para baixar");
  startFakeDownload();
  return fakeUpdate;
}

function fakeUpdateApply() {
  if (fakeUpdateGame) throw new FakeError("conflict", "encerre o jogo antes de atualizar");
  if (fakeUpdate.state !== "ready") throw new FakeError("conflict", "nenhuma atualização pronta");
  setFakeUpdate({ state: "installing" });
  // O núcleo de verdade sai aqui. No navegador não há o que fechar: volta a "pronta" para repetir o ciclo.
  console.info("[hospedeiro falso] reiniciar para atualizar");
  setTimeout(() => {
    setFakeUpdate({ state: "ready" });
  }, 2500);
}

function setFakeUpdatePrefs(payload: unknown): UpdatePrefs {
  const { autoUpdate, prereleases, nightly } = payload as UpdatePrefs;
  if (![autoUpdate, prereleases, nightly].every((v) => typeof v === "boolean")) throw new FakeError("bad_request", "preferências inválidas");
  fakeUpdatePrefs = { autoUpdate, prereleases, nightly };
  return fakeUpdatePrefs;
}

/** Abrir um jogo (`game.play`) o deixa aberto por 20 s no falso: com atualização pronta, reiniciar espera. */
const playBeforeUpdates = defaults["game.play"];
function playAndHoldTheGame(payload: unknown) {
  playBeforeUpdates?.(payload);
  fakeUpdateGame = true;
  setFakeUpdate({});
  setTimeout(() => {
    fakeUpdateGame = false;
    setFakeUpdate({});
  }, 20_000);
}

Object.assign(defaults, {
  "game.play": playAndHoldTheGame,
  "updates.status": () => fakeUpdate,
  "updates.check": fakeUpdateCheck,
  "updates.download": fakeUpdateDownload,
  "updates.apply": fakeUpdateApply,
  "updates.preferences": () => fakeUpdatePrefs,
  "updates.preferences.set": setFakeUpdatePrefs,
});

if (updateParam !== null) setTimeout(fakeUpdateCheck, 2500);

// ---- Primeira execução (WEB-16): ?primeiraVez, com conclusão persistida na sessão. ----
// Neste cenário as ROMs da leitura também entram na biblioteca: o resumo não pode somar jogos inexistentes.
function syncFirstRunRoms() {
  for (let index = games.length - 1; index >= 0; index--) {
    if (games[index]?.id.startsWith("onboarding-rom:")) games.splice(index, 1);
  }
  const count = romFolders.length * 12;
  const sample = games[0];
  if (sample) for (let index = 0; index < count; index++) games.push({ ...sample, id: `onboarding-rom:${String(index)}`, title: `ROM ${String(index + 1)}`, store: "emulators", favorite: false, status: null, lastPlayedAt: null, playTimeSeconds: 0 });
  const emulatorStore = SAMPLE_STORES.find((source) => source.store === "emulators");
  if (emulatorStore) emulatorStore.found = count;
}
Object.assign(SAMPLE_CATALOGS["pt-BR"], {
  "first_run.step": "PRIMEIROS PASSOS · {n} DE {total}",
  "first_run.welcome": "Boas-vindas ao Tako",
  "first_run.welcome_short": "Boas-vindas",
  "first_run.welcome_text": "Três passos rápidos para deixar a biblioteca pronta. Tudo dá para mudar depois em Configurações. Primeiro, o idioma:",
  "first_run.stores": "Lojas",
  "first_run.stores_text": "O Tako lê os jogos instalados destas lojas. Desligue as que você não usa.",
  "first_run.metadata": "Capas e descrições",
  "first_run.metadata_text": "Estes serviços completam capas, logos e descrições. São opcionais: sem eles, o Tako usa o que as lojas oferecem.",
  "first_run.next": "Continuar",
  "first_run.skip": "Pular por agora",
  "first_run.finish": "Concluir",
  "first_run.done_toast": "Tudo pronto. Bom jogo!",
  "first_run.count": "{n} de {total}",
  "first_run.full.emulators": "Jogos de\nemuladores.",
  "first_run.full.emulators_text": "O Tako acha os emuladores instalados e a pasta de ROMs, e abre cada jogo no emulador certo.",
  "first_run.full.emulators_note": "O Tako não baixa ROMs nem BIOS: usa só os arquivos que já estão no seu PC.",
  "first_run.full.emu_no_games": "Nenhum jogo achado ainda",
  "first_run.full.roms": "Pasta de ROMs",
  "first_run.full.roms_found": "{games} em {systems}",
  "first_run.full.systems.one": "{n} sistema",
  "first_run.full.systems.other": "{n} sistemas",
  "first_run.full.roms_none": "Nenhuma pasta escolhida",
  "first_run.full.roms_choose": "Escolher…",
  "first_run.full.add_emulator": "Adicionar emulador…",
  "first_run.full.no_emulators": "Não uso emuladores",
  "first_run.full.welcome": "Boas-vindas ao Tako.",
  "first_run.full.welcome_text": "Cinco passos rápidos para deixar a biblioteca pronta. Tudo dá para mudar depois em Configurações. Primeiro, o idioma:",
  "first_run.full.welcome_note": "O idioma também fica em Configurações › Geral.",
  "first_run.full.stores": "Vamos achar\nseus jogos.",
  "first_run.full.stores_text": "O Tako olha as pastas das lojas instaladas neste PC e monta sua biblioteca. Nada sai do seu computador.",
  "first_run.full.stores_note": "Dá para adicionar lojas e jogos à mão depois, em Configurações › Biblioteca e lojas.",
  "first_run.full.stores_search": "Procurar jogos",
  "first_run.full.stores_choose": "Escolher pastas…",
  "first_run.full.stores_unchecked": "Ainda não procurado",
  "first_run.full.stores_missing": "Não instalado",
  "first_run.full.metadata": "Capas e\ndescrições.",
  "first_run.full.metadata_text": "Capa, arte de fundo e descrição de cada jogo, buscadas na IGDB.",
  "first_run.full.metadata_note": "Capas com dúvida vão para Configurações › Capas e metadados › Revisar capas. Dá para mudar a escolha lá também.",
  "first_run.full.cover_none": "Sem capa",
  "first_run.full.cover_with": "Com capa",
  "first_run.full.igdb": "Buscar na IGDB",
  "first_run.full.igdb_desc": "Envia só o nome de cada jogo ({games}).",
  "first_run.full.igdb_key_desc": "Precisa da sua chave da IGDB (Twitch); ao escolher, abre o painel para colar.",
  "first_run.full.igdb_offline": "Sem conexão agora; as capas chegam quando a internet voltar.",
  "first_run.full.stores_only": "Usar só o que as lojas têm",
  "first_run.full.stores_only_desc": "Nada sai do PC; jogos sem capa ficam com a cor do jogo e o nome.",
  "first_run.full.mode": "Como você quer\nabrir o Tako?",
  "first_run.full.mode_text": "Escolha o jeito de abrir. Os dois mostram os mesmos jogos.",
  "first_run.full.mode_note": "F11 troca entre janela e tela cheia a qualquer hora. Com um controle conectado, \"Em tela cheia\" já vem marcado.",
  "first_run.full.mode_window": "Em janela",
  "first_run.full.mode_window_desc": "Para mouse e teclado. Fica junto das outras janelas do PC.",
  "first_run.full.mode_full": "Em tela cheia",
  "first_run.full.mode_full_desc": "Para controle e TV. Ocupa a tela toda, com tudo grande.",
  "first_run.full.mode_window_hint": "Organizar, configurar, jogar na mesa",
  "first_run.full.mode_full_hint": "Jogar no sofá, no notebook ligado na TV",
  "first_run.full.back": "Voltar",
  "first_run.full.ready": "{n} jogos\nprontos.",
  "first_run.full.ready_none": "Nenhum jogo\nainda.",
  "first_run.full.ready_text": "As capas que faltam vão chegando enquanto você navega.",
  "first_run.full.ready_none_text": "Dá para adicionar um jogo à mão ou procurar de novo pela biblioteca.",
  "first_run.full.ready_note": "F11 troca entre janela e tela cheia a qualquer hora.",
  "first_run.full.opens_in": "Abre em",
  "first_run.full.opens_window": "janela",
  "first_run.full.opens_full": "tela cheia",
  "first_run.full.change": "Trocar",
  "first_run.full.open_library": "Abrir a biblioteca",
  "first_run.full.add_game": "Adicionar jogo à mão",
  "first_run.full.search_again": "Procurar de novo",
  "first_run.full.review_covers": "Revisar capas",
  "first_run.full.skip": "Pular configuração",
  "first_run.splash.scanning": "Procurando seus jogos…",
  "first_run.splash.starting": "Iniciando o Tako…"
});
Object.assign(SAMPLE_CATALOGS["en-US"], {
  "first_run.step": "GETTING STARTED · {n} OF {total}",
  "first_run.welcome": "Welcome to Tako",
  "first_run.welcome_short": "Welcome",
  "first_run.welcome_text": "Three quick steps to get your library ready. You can change everything later in Settings. First, the language:",
  "first_run.stores": "Stores",
  "first_run.stores_text": "Tako reads the installed games from these stores. Turn off the ones you don't use.",
  "first_run.metadata": "Covers and descriptions",
  "first_run.metadata_text": "These services fill in covers, logos and descriptions. They are optional: without them, Tako uses what the stores provide.",
  "first_run.next": "Continue",
  "first_run.skip": "Skip for now",
  "first_run.finish": "Finish",
  "first_run.done_toast": "All set. Have fun!",
  "first_run.count": "{n} of {total}",
  "first_run.full.emulators": "Emulator\ngames.",
  "first_run.full.emulators_text": "Tako finds the installed emulators and your ROM folder, and opens each game in the right emulator.",
  "first_run.full.emulators_note": "Tako doesn't download ROMs or BIOS files: it only uses the files already on your PC.",
  "first_run.full.emu_no_games": "No games found yet",
  "first_run.full.roms": "ROM folder",
  "first_run.full.roms_found": "{games} in {systems}",
  "first_run.full.systems.one": "{n} system",
  "first_run.full.systems.other": "{n} systems",
  "first_run.full.roms_none": "No folder chosen",
  "first_run.full.roms_choose": "Choose…",
  "first_run.full.add_emulator": "Add emulator…",
  "first_run.full.no_emulators": "I don't use emulators",
  "first_run.full.welcome": "Welcome to Tako.",
  "first_run.full.welcome_text": "Five quick steps to get your library ready. You can change everything later in Settings. First, the language:",
  "first_run.full.welcome_note": "The language is also in Settings › General.",
  "first_run.full.stores": "Let's find\nyour games.",
  "first_run.full.stores_text": "Tako looks at the folders of the stores installed on this PC and builds your library. Nothing leaves your computer.",
  "first_run.full.stores_note": "You can add stores and games by hand later, in Settings › Library and stores.",
  "first_run.full.metadata": "Covers and\ndescriptions.",
  "first_run.full.metadata_text": "Cover, background art and description for each game, fetched from IGDB.",
  "first_run.full.metadata_note": "Covers Tako isn't sure about go to Settings › Covers and metadata › Review covers. You can change this choice there too.",
  "first_run.full.cover_none": "No cover",
  "first_run.full.cover_with": "With cover",
  "first_run.full.igdb": "Search IGDB",
  "first_run.full.igdb_desc": "Sends only the name of each game ({games}).",
  "first_run.full.igdb_key_desc": "Needs your IGDB (Twitch) key; choosing it opens the panel to paste it.",
  "first_run.full.igdb_offline": "No connection right now; covers arrive when the internet is back.",
  "first_run.full.stores_only": "Use only what the stores have",
  "first_run.full.stores_only_desc": "Nothing leaves your PC; games without a cover show their color and name.",
  "first_run.full.mode": "How do you want\nto open Tako?",
  "first_run.full.mode_text": "Pick how it opens. Both show the same games.",
  "first_run.full.mode_note": "F11 switches between window and full screen at any time. With a controller connected, \"Full screen\" comes selected.",
  "first_run.full.stores_search": "Find games",
  "first_run.full.stores_choose": "Choose folders…",
  "first_run.full.stores_unchecked": "Not searched yet",
  "first_run.full.stores_missing": "Not installed",
  "first_run.full.mode_window": "In a window",
  "first_run.full.mode_window_desc": "For mouse and keyboard. Sits with the other windows on your PC.",
  "first_run.full.mode_full": "Full screen",
  "first_run.full.mode_full_desc": "For controllers and TVs. Takes the whole screen, with everything large.",
  "first_run.full.mode_window_hint": "Organize, configure, play at the desk",
  "first_run.full.mode_full_hint": "Play on the couch or on a laptop hooked to the TV",
  "first_run.full.back": "Back",
  "first_run.full.ready": "{n} games\nready.",
  "first_run.full.ready_none": "No games\nyet.",
  "first_run.full.ready_text": "The missing covers keep arriving while you browse.",
  "first_run.full.ready_none_text": "You can add a game by hand or look again from the library.",
  "first_run.full.ready_note": "F11 switches between window and full screen at any time.",
  "first_run.full.opens_in": "Opens in",
  "first_run.full.opens_window": "a window",
  "first_run.full.opens_full": "full screen",
  "first_run.full.change": "Change",
  "first_run.full.open_library": "Open the library",
  "first_run.full.add_game": "Add a game by hand",
  "first_run.full.search_again": "Search again",
  "first_run.full.review_covers": "Review covers",
  "first_run.full.skip": "Skip setup",
  "first_run.splash.scanning": "Looking for your games…",
  "first_run.splash.starting": "Starting Tako…"
});

for (const key of ["FirstRunCompleted", "metadata.igdb"]) { KNOWN_SETTINGS.add(key); BOOL_SETTINGS.add(key); }
if (firstRunDemo) {
  syncFirstRunRoms();
  sources = { ...sources, sources: SAMPLE_STORES.map((sample, index) => sampleReport(sample, index, new Date())) };
}
if (firstRunDemo && settings.get("FirstRunCompleted") !== "true") {
  // A abertura sonda o snapshot vazio (limit=1); os passos recebem os jogos que a primeira leitura encontrou.
  uiState = { json: null };
  const beforeFirstRun = defaults["library.list"];
  Object.assign(defaults, {
    "library.list": (payload: unknown) => {
      if ((payload as LibraryListRequest).limit === 1 && settings.get("FirstRunCompleted") !== "true") return { rows: [], total: 0 };
      return beforeFirstRun?.(payload);
    },
  });
}
// ---- Controle (WEB-05b) ----
// No Tako o núcleo lê o controle pelo SDL3 e manda `input.nav`, `input.controller` e `input.notice`. No navegador, um
// controle simulado, para ver o estilo dos botões (`data-pad` na raiz) e os avisos:
//   ?controle[=xbox|playstation|nintendo]  um controle conectado desde a abertura (sem valor: Xbox);
//   ?bateria=12                              a bateria dele; com 15 % ou menos, o aviso de bateria fraca ao abrir;
//   Ctrl+Shift+F8                            desconecta e reconecta (os avisos de quem usa o controle);
//   Ctrl+Shift+F9                            troca a família dos botões;
//   window.takoFakePad.press("down")         um comando, como se viesse do controle (setas, accept, back, search…).
const padParams = typeof location === "undefined" ? new URLSearchParams() : new URLSearchParams(location.search);
const PAD_STYLES: ControllerStyle[] = ["xbox", "playstation", "nintendo"];
const PAD_NAMES: Record<ControllerStyle, string> = {
  xbox: "Xbox Wireless Controller",
  playstation: "DualSense Wireless Controller",
  nintendo: "Pro Controller",
};
const NO_PAD: ControllerInfo = { connected: false, count: 0, style: "xbox", name: "", battery: null };

function fakePadInfo(style: ControllerStyle): ControllerInfo {
  const battery = Number(padParams.get("bateria"));
  return {
    connected: true,
    count: 1,
    style,
    name: PAD_NAMES[style],
    battery: padParams.has("bateria") && Number.isFinite(battery) ? Math.round(battery) : null,
  };
}

const padStyleParam = padParams.get("controle") ?? "";
let fakePad: ControllerInfo = padParams.has("controle")
  ? fakePadInfo(PAD_STYLES.includes(padStyleParam as ControllerStyle) ? (padStyleParam as ControllerStyle) : "xbox")
  : NO_PAD;
let fakePadStyle: ControllerStyle = fakePad.style;
/** Caiu com aviso: o "conectado" só aparece depois disso (regra do núcleo, `PadNotices`). */
let fakePadLost = false;

function fakePadNotice(notice: InputNotice) {
  deliverEvent("input.notice", notice);
}

function toggleFakePad() {
  if (fakePad.connected) {
    fakePad = NO_PAD;
    fakePadLost = true;
    fakePadNotice({ kind: "disconnected", pad: null, percent: null });
  } else {
    fakePad = fakePadInfo(fakePadStyle);
    if (fakePadLost) fakePadNotice({ kind: "connected", pad: null, percent: null });
    fakePadLost = false;
  }
  deliverEvent("input.controller", fakePad);
}

function cycleFakePadStyle() {
  fakePadStyle = PAD_STYLES[(PAD_STYLES.indexOf(fakePadStyle) + 1) % PAD_STYLES.length] ?? "xbox";
  if (!fakePad.connected) return;
  fakePad = fakePadInfo(fakePadStyle);
  deliverEvent("input.controller", fakePad);
}

Object.assign(defaults, { "input.controller": () => fakePad });

// Só no navegador: os testes do falso rodam no Node com um `window` de mentira.
if (typeof window !== "undefined" && typeof window.addEventListener === "function") {
  window.addEventListener("keydown", (event) => {
    if (!installedFake || !event.ctrlKey || !event.shiftKey || event.altKey) return;
    if (event.key === "F8") {
      event.preventDefault();
      toggleFakePad();
    } else if (event.key === "F9") {
      event.preventDefault();
      cycleFakePadStyle();
    }
  });
  (window as unknown as { takoFakePad: unknown }).takoFakePad = {
    press(action: InputAction, repeat = false) {
      deliverEvent("input.nav", { action, repeat });
    },
    toggle: toggleFakePad,
    style: cycleFakePadStyle,
  };
  const battery = fakePad.battery;
  if (fakePad.connected && battery !== null && battery <= 15) {
    // Depois de a página abrir e escutar os avisos.
    setTimeout(() => {
      fakePadNotice({ kind: "battery_low", pad: fakePad.name, percent: battery });
    }, 1500);
  }
}

Object.assign(SAMPLE_CATALOGS["pt-BR"], {
  "status.pad_disconnected": "Controle desconectado",
  "status.pad_connected": "Controle conectado",
  "status.battery_low": "Bateria fraca: {pad} com {percent}%",
});
Object.assign(SAMPLE_CATALOGS["en-US"], {
  "status.pad_disconnected": "Controller disconnected",
  "status.pad_connected": "Controller connected",
  "status.battery_low": "Low battery: {pad} at {percent}%",
});

// ---- Central rápida (WEB-22): sessão longa para explorar os controles, sem alterar o SO. ----
let fakeCentralOpen = sessionParams.has("central");
let centralVolume = 0.6;
let centralMuted = false;
let centralOutput = "speakers";
const centralModes = new Map<string, string>();
if (fakeCentralOpen) {
  fakeSession = { gameId: "steam:1000", title: "God of War: Ragnarök", launchedAt: Date.now() - 42 * 60_000, state: "playing", timers: [] };
}

function readCentral(): ControlCenterState {
  return {
    open: fakeCentralOpen && fakeSession !== null,
    audio: {
      outputs: { ...SAMPLE_AUDIO, outputs: SAMPLE_AUDIO.outputs.map((o) => ({ ...o, default: o.id === centralOutput })) },
      defaultId: centralOutput, volume: centralVolume, muted: centralMuted,
    },
    displays: SAMPLE_DISPLAYS.displays.filter((d) => d.active).map((d) => ({ id: d.id, name: d.name, modes: d.modes, currentMode: centralModes.get(d.id) ?? d.modes[0] ?? null })),
    displayManaged: false,
    controller: fakePad,
    memoryMib: null,
    cpuSeconds: null,
  };
}
const resumeBeforeCentral = defaults["game.resume"];
const endBeforeCentral = defaults["game.end"];
Object.assign(defaults, {
  "control_center.state": readCentral,
  "control_center.set": (payload: unknown) => {
    if (!fakeSession) throw new FakeError("conflict", "nenhum jogo aberto");
    const action = payload as ControlCenterAction;
    switch (action.action) {
      case "volume":
        if (!Number.isFinite(action.value) || action.value < 0 || action.value > 1) throw new FakeError("bad_request", "volume inválido");
        centralVolume = action.value;
        break;
      case "mute": centralMuted = action.muted; break;
      case "audio_output":
        if (action.id === BROKEN_AUDIO_ID) throw new FakeError("internal", "saída indisponível");
        if (!SAMPLE_AUDIO.outputs.some((o) => o.id === action.id)) throw new FakeError("not_found", "saída ausente");
        centralOutput = action.id;
        break;
      case "display_mode":
        if (!SAMPLE_DISPLAYS.displays.some((d) => d.id === action.id && d.active && d.modes.includes(action.mode))) throw new FakeError("bad_request", "modo inválido");
        centralModes.set(action.id, action.mode);
        break;
    }
    return readCentral();
  },
  "game.resume": () => {
    resumeBeforeCentral?.(undefined);
    fakeCentralOpen = false;
    deliverEvent("control_center.open", false);
  },
  "game.end": (payload: unknown) => {
    fakeCentralOpen = false;
    deliverEvent("control_center.open", false);
    return endBeforeCentral?.(payload);
  },
});
if (typeof window !== "undefined" && typeof window.addEventListener === "function") {
  window.addEventListener("keydown", (event) => {
    if (event.key !== "Tab" || !event.ctrlKey || !event.shiftKey || event.altKey || event.metaKey || event.repeat || !fakeSession) return;
    event.preventDefault();
    fakeCentralOpen = !fakeCentralOpen;
    deliverEvent("control_center.open", fakeCentralOpen);
  });
}
Object.assign(SAMPLE_CATALOGS["pt-BR"], {
  "cc.back_to_game": "Voltar ao jogo",
  "cc.audio": "Áudio", "cc.screen": "Tela", "cc.end": "Encerrar jogo", "cc.end_confirm": "Encerrar o jogo",
  "cc.playing": "Jogando agora", "cc.pad_battery": "Controle com {percent}%", "cc.volume": "Volume", "cc.mute": "Sem som",
  "cc.output": "SAÍDA DE ÁUDIO", "cc.audio_unavailable": "Áudio indisponível", "cc.screen_unavailable": "Nenhuma tela encontrada",
  "settings.performance.title": "Desempenho do Tako", "settings.performance.supervisor": "Supervisor",
  "settings.performance.memory": "Memória", "settings.performance.unknown": "Indisponível",
});
Object.assign(SAMPLE_CATALOGS["en-US"], {
  "cc.back_to_game": "Back to game",
  "cc.audio": "Audio", "cc.screen": "Display", "cc.end": "Close game", "cc.end_confirm": "Close the game",
  "cc.playing": "Playing now", "cc.pad_battery": "Controller at {percent}%", "cc.volume": "Volume", "cc.mute": "Mute",
  "cc.output": "AUDIO OUTPUT", "cc.audio_unavailable": "Audio unavailable", "cc.screen_unavailable": "No displays found",
  "settings.performance.title": "Tako performance", "settings.performance.supervisor": "Supervisor",
  "settings.performance.memory": "Memory", "settings.performance.unknown": "Unavailable",
});

// ---- Seletor de imagens e Revisar capas (WEB-12d) ----
const fakeChosenArt = new Map<string, string>();

function createFakeCandidateSvg(title: string, kind: string, label: string, color: string, w = 600, h = 900): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}"><rect width="${w}" height="${h}" fill="${color}"/><circle cx="${w / 2}" cy="${h / 2 - 40}" r="${Math.min(w, h) / 4}" fill="white" fill-opacity="0.12"/><text x="${w / 2}" y="${h / 2 + 10}" fill="white" font-size="${Math.round(w / 14)}" font-weight="bold" font-family="sans-serif" text-anchor="middle">${title}</text><text x="${w / 2}" y="${h / 2 + 50}" fill="white" fill-opacity="0.7" font-size="${Math.round(w / 26)}" font-family="sans-serif" text-anchor="middle">${kind} · ${label}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function fakeArtCandidates(payload: unknown): ArtCandidatesView {
  const { gameId, kind } = payload as ArtCandidatesRequest;
  const game = games.find((g) => g.id === gameId);
  const title = game?.title ?? gameId;
  const chosen = fakeChosenArt.get(`${gameId}:${kind}`);

  const dims = kind === "cover" ? "600 × 900" : kind === "hero" ? "1920 × 1080" : kind === "logo" ? "1280 × 720" : "512 × 512";
  const [w, h] = kind === "cover" ? [600, 900] : kind === "hero" ? [1920, 1080] : kind === "logo" ? [1280, 720] : [512, 512];

  const cand0Id = "cand-0";
  const cand1Id = "cand-1";
  const cand2Id = "cand-2";
  const cand3Id = "cand-3";

  const candidates: ArtCandidateDto[] = [
    {
      id: cand0Id,
      url: createFakeCandidateSvg(title, kind, "Oficial", "darkslateblue", w, h),
      source: "Steam",
      dimensions: dims,
      style: "Oficial",
      current: chosen ? chosen === cand0Id : true,
    },
    {
      id: cand1Id,
      url: createFakeCandidateSvg(title, kind, "Alternativa", "midnightblue", w, h),
      source: "SteamGridDB",
      dimensions: dims,
      style: "Alternativa",
      current: chosen === cand1Id,
    },
    {
      id: cand2Id,
      url: createFakeCandidateSvg(title, kind, "Arte", "darkslategray", w, h),
      source: "IGDB",
      dimensions: dims,
      style: "Arte",
      current: chosen === cand2Id,
    },
    {
      id: cand3Id,
      url: createFakeCandidateSvg(title, kind, "Retro", "maroon", w, h),
      source: "SteamGridDB",
      dimensions: dims,
      style: "Retro",
      current: chosen === cand3Id,
    },
  ];

  emitFakeEvent(
    "art.candidates.ready",
    {
      gameId,
      kind,
      candidates,
    } satisfies ArtCandidatesReady,
    50,
  );

  return {
    candidates: [],
    loading: true,
  };
}

function fakeArtChoose(payload: unknown) {
  const { gameId, kind, candidateId } = payload as ArtChooseRequest;
  fakeChosenArt.set(`${gameId}:${kind}`, candidateId);
  deliverEvent("art.changed", { gameId });
  deliverEvent("library.changed");
}

function fakeArtReset(payload: unknown) {
  const { gameId, kind } = payload as ArtResetRequest;
  fakeChosenArt.delete(`${gameId}:${kind}`);
  deliverEvent("art.changed", { gameId });
  deliverEvent("library.changed");
}

let fakeReviewItems: ArtReviewItem[] = [
  {
    gameId: "steam:1000",
    title: "God of War: Ragnarök",
    store: "steam",
    currentCover: createFakeCandidateSvg("God of War: Ragnarök", "Capa", "Atual", "darkslateblue", 600, 900),
    proposedCover: createFakeCandidateSvg("God of War: Ragnarök", "Capa", "Proposta", "midnightblue", 600, 900),
    confidence: "high",
    confidencePercent: 95,
    reason: 'Achado pelo nome no IGDB como "God of War Ragnarök"',
    doubt: { source: "IGDB", id: "123", found: "God of War Ragnarök" },
    missingKinds: [],
  },
  {
    gameId: "steam:1001",
    title: "Hollow Knight",
    store: "steam",
    currentCover: createFakeCandidateSvg("Hollow Knight", "Capa", "Atual", "darkslategray", 600, 900),
    proposedCover: createFakeCandidateSvg("Hollow Knight", "Capa", "Proposta", "indigo", 600, 900),
    confidence: "medium",
    confidencePercent: 72,
    reason: 'Achado pelo nome no SteamGridDB como "Hollow Knight (PC)"',
    doubt: { source: "SteamGridDB", id: "456", found: "Hollow Knight (PC)" },
    missingKinds: [],
  },
  {
    gameId: "manual:fake1",
    title: "Chrono Trigger",
    store: "manual",
    currentCover: null,
    proposedCover: null,
    confidence: "low",
    confidencePercent: 0,
    reason: "Sem capa, fundo, logo, ícone",
    doubt: null,
    missingKinds: ["cover", "hero", "logo", "icon"],
  },
];

function fakeArtReview(): ArtReviewView {
  return {
    total: games.length,
    items: [...fakeReviewItems],
  };
}

function fakeArtReviewAccept(payload: unknown) {
  const { gameId } = payload as GameRequest;
  fakeReviewItems = fakeReviewItems.filter((item) => item.gameId !== gameId);
  deliverEvent("library.changed");
}

Object.assign(defaults, {
  "art.candidates": fakeArtCandidates,
  "art.choose": fakeArtChoose,
  "art.reset": fakeArtReset,
  "art.review": fakeArtReview,
  "art.review.accept": fakeArtReviewAccept,
});

Object.assign(SAMPLE_CATALOGS["pt-BR"], {
  "options.change_art": "Trocar imagens",
  "art_review.title": "Revisar capas",
  "art_review.desc": "As fontes casam a biblioteca sozinhas; aqui ficam só os jogos que valem conferir",
  "art_review.open": "Abrir",
  "art_review.text": "Capa, fundo, logo e ícone vêm sozinhos das fontes ligadas. Confira os jogos achados só pelo nome e os que ficaram sem alguma imagem.",
  "art_review.computing": "Conferindo a biblioteca…",
  "art_review.summary": "{count} de {total} jogos para conferir",
  "art_review.all_good": "Os {total} jogos estão com as imagens e as correspondências em ordem",
  "art_review.refresh": "Buscar de novo para a biblioteca inteira",
  "art_review.refresh_desc": "Consulta as fontes de novo para todos os jogos, em segundo plano",
  "art_review.doubt": 'Achado pelo nome no {source} como "{found}"',
  "art_review.missing": "Sem {kinds}",
  "art_review.pick_art": "Escolher imagens",
  "art_review.fix_match": "Corrigir correspondência",
  "art_review.correct": "Está certo",
  "art_review.high_title": "{n} com confiança alta",
  "art_review.high_desc": "O nome achado é praticamente o mesmo do jogo",
  "art_review.accept_all": "Aceitar as {n}",
  "art_review.to_review": "Para revisar · {n}",
  "art_review.no_cover": "Sem capa",
  "art_review.conf_high": "Confiança alta · {percent}%",
  "art_review.conf_medium": "Confiança média · {percent}%",
  "art_review.conf_low": "Confiança baixa · {percent}%",
  "art_review.accept": "Aceitar",
  "art_review.other": "Escolher outra",
  "art_review.more": "Mostrando {shown} de {n}; resolva estes para ver os outros",
  "art_review.accepted": "Capa de {game} trocada",
  "art_review.accepted_many": "{n} capas trocadas",
});

Object.assign(SAMPLE_CATALOGS["en-US"], {
  "options.change_art": "Change images",
  "art_review.title": "Review covers",
  "art_review.desc": "Sources match the whole library on their own; only the games worth checking are listed here",
  "art_review.open": "Open",
  "art_review.text": "Cover, background, logo and icon come on their own from the enabled sources. Check the games found only by name and the ones left without some image.",
  "art_review.computing": "Checking the library…",
  "art_review.summary": "{count} of {total} games to check",
  "art_review.all_good": "All {total} games have their images and matches in order",
  "art_review.refresh": "Search again for the whole library",
  "art_review.refresh_desc": "Asks the sources again for every game, in the background",
  "art_review.doubt": 'Found by name on {source} as "{found}"',
  "art_review.missing": "No {kinds}",
  "art_review.pick_art": "Choose images",
  "art_review.fix_match": "Fix match",
  "art_review.correct": "It's right",
  "art_review.high_title": "{n} with high confidence",
  "art_review.high_desc": "The name found is practically the same as the game's",
  "art_review.accept_all": "Accept all {n}",
  "art_review.to_review": "To review · {n}",
  "art_review.no_cover": "No cover",
  "art_review.conf_high": "High confidence · {percent}%",
  "art_review.conf_medium": "Medium confidence · {percent}%",
  "art_review.conf_low": "Low confidence · {percent}%",
  "art_review.accept": "Accept",
  "art_review.other": "Choose another",
  "art_review.more": "Showing {shown} of {n}; resolve these to see the rest",
  "art_review.accepted": "Cover of {game} changed",
  "art_review.accepted_many": "{n} covers changed",

});
