// Biblioteca real para o hospedeiro falso (WEB-DEV-01). `tako dev-snapshot` grava `web/dev-data/snapshot.json` com
// as respostas da ponte (os tipos de `generated/`) e as imagens copiadas; o Vite serve a pasta em `/dev-data/` só em
// desenvolvimento. Sem o arquivo, o falso usa os dados de exemplo.

import type { AppInfo } from "./generated/AppInfo";
import type { Collection } from "./generated/Collection";
import type { GameDetails } from "./generated/GameDetails";
import type { LibraryGame } from "./generated/LibraryGame";
import type { ProfileCollections } from "./generated/ProfileCollections";
import type { ProfileSessions } from "./generated/ProfileSessions";
import type { ProfileSummary } from "./generated/ProfileSummary";
import type { ProfilesList } from "./generated/ProfilesList";
import type { SessionPeriod } from "./generated/SessionPeriod";
import type { SettingsValues } from "./generated/SettingsValues";
import type { Tag } from "./generated/Tag";

/** Versão do formato (`FORMAT_VERSION` em `crates/tako-app/src/dev_snapshot.rs`). */
export const DEV_SNAPSHOT_VERSION = 1;
/** Onde o Vite serve `web/dev-data/`. */
export const DEV_DATA_URL = "/dev-data/";

export type DevImageKind = "cover" | "hero" | "logo" | "icon";

/** O `snapshot.json`. Uma consulta que falhou no núcleo vem `null` e o falso usa o exemplo dela. */
export interface DevSnapshot {
  version: number;
  createdAt: string;
  appInfo: AppInfo | null;
  /** Todos os jogos, na ordem de `library.list` sem filtro (jogados por último primeiro). */
  games: LibraryGame[];
  details: Record<string, GameDetails>;
  collections: Collection[] | null;
  tags: Tag[] | null;
  settings: SettingsValues | null;
  profileSummary: ProfileSummary | null;
  profileSessions: Record<SessionPeriod, ProfileSessions | null>;
  profileCollections: ProfileCollections | null;
  profiles: ProfilesList | null;
  /** Caminho de cada imagem copiada, relativo a `web/dev-data/`. */
  images: Record<string, Partial<Record<DevImageKind, string>>>;
}

/** Confere o mínimo para não quebrar o falso com um arquivo de outra versão ou cortado. */
export function isDevSnapshot(value: unknown): value is DevSnapshot {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Partial<DevSnapshot>;
  return (
    v.version === DEV_SNAPSHOT_VERSION &&
    Array.isArray(v.games) &&
    typeof v.details === "object" &&
    v.details !== null &&
    typeof v.images === "object" &&
    v.images !== null &&
    typeof v.profileSessions === "object" &&
    v.profileSessions !== null
  );
}

/** Lê o snapshot servido pelo Vite; `null` sem arquivo, com arquivo inválido ou fora do navegador. */
export async function fetchDevSnapshot(): Promise<DevSnapshot | null> {
  if (typeof fetch === "undefined") return null;
  try {
    const response = await fetch(`${DEV_DATA_URL}snapshot.json`, { cache: "no-store" });
    if (!response.ok) return null;
    const data: unknown = await response.json();
    if (isDevSnapshot(data)) return data;
    console.warn("[hospedeiro falso] dev-data/snapshot.json de outra versão; rode `cargo run -p tako -- dev-snapshot`");
  } catch {
    // Sem snapshot (o Vite responde 404 ou o index.html): ficam os dados de exemplo.
  }
  return null;
}

/** URL da imagem copiada; muda a cada snapshot, para o navegador não mostrar a de antes. */
export function devImageUrl(snapshot: DevSnapshot, id: string, kind: DevImageKind): string | undefined {
  const path = snapshot.images[id]?.[kind];
  return path ? `${DEV_DATA_URL}${path}?v=${encodeURIComponent(snapshot.createdAt)}` : undefined;
}
