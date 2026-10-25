import { imageUrl as coreImageUrl } from "../../lib/imageVersions.ts";
// Regras da página do jogo que não dependem de tela: formatos, mapeamento de situação e quais blocos existem.
// Sem importações em execução (só tipos), para o teste rodar direto no Node.

import type { GameDetails } from "../../bridge/generated/GameDetails";
import type { GameStatus } from "../../bridge/generated/GameStatus";
import type { GameStatus as StatusChip } from "../../components/StatusDoJogo/StatusDoJogo";

const toChip: Record<GameStatus, StatusChip> = {
  playing: "playing",
  beaten: "completed",
  wishlist: "queued",
  paused: "paused",
  abandoned: "dropped",
};

/** A situação do núcleo na que o chip do design system conhece (`none` = sem situação). */
export function statusToChip(status: GameStatus | null): StatusChip {
  return status ? toChip[status] : "none";
}

/** O caminho de volta: o que o chip escolheu, na situação que o núcleo guarda. */
export function chipToStatus(chip: StatusChip): GameStatus | null {
  const found = (Object.keys(toChip) as GameStatus[]).find((status) => toChip[status] === chip);
  return found ?? null;
}

/** Duração em português: "menos de 1 min", "42 min", "7 h 20 min", "40 h". */
export function formatDuration(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  if (minutes < 1) return "menos de 1 min";
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (hours === 0) return `${String(rest)} min`;
  if (hours < 10 && rest > 0) return `${String(hours)} h ${String(rest)} min`;
  return `${String(hours)} h`;
}

/** Tempo de jogo; zero é "nunca jogado", ou seja, sem dado (o bloco some). */
export function formatPlayTime(seconds: number): string | null {
  return seconds > 0 ? formatDuration(seconds) : null;
}

function parts(date: Date, options: Intl.DateTimeFormatOptions): Record<string, string> {
  const found: Record<string, string> = {};
  for (const part of new Intl.DateTimeFormat("pt-BR", options).formatToParts(date)) found[part.type] = part.value;
  return found;
}

/** "23/10, 16:17" (com o ano se não for o de `now`). `null` = nunca jogado. */
export function formatLastPlayed(iso: string | null, now: Date = new Date(), timeZone?: string): string | null {
  if (!iso) return null;
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return null;
  const zone = timeZone ? { timeZone } : {};
  const p = parts(date, {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
    ...zone,
  });
  const sameYear = p["year"] === parts(now, { year: "numeric", ...zone })["year"];
  const year = sameYear ? "" : `/${p["year"] ?? ""}`;
  return `${p["day"] ?? ""}/${p["month"] ?? ""}${year}, ${p["hour"] ?? ""}:${p["minute"] ?? ""}`;
}

/** "19 set, 2024" a partir de `2024-09-19`. Texto que não é data fica como veio. */
export function formatReleaseDate(iso: string | null): string | null {
  if (!iso) return null;
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(iso);
  if (!match) return iso;
  const date = new Date(Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3])));
  if (Number.isNaN(date.getTime())) return iso;
  const p = parts(date, { day: "numeric", month: "short", year: "numeric", timeZone: "UTC" });
  return `${p["day"] ?? ""} ${(p["month"] ?? "").replace(".", "")}, ${p["year"] ?? ""}`;
}

export type MetacriticTone = "good" | "mixed" | "bad";
export type MetacriticVerdict = "universal" | "favorable" | "mixed" | "unfavorable";

/** Faixas do Metacritic: verde 75+, amarelo 50–74, vermelho até 49 (README do CartaoSessoes). */
export function metacriticTone(score: number): MetacriticTone {
  return score >= 75 ? "good" : score >= 50 ? "mixed" : "bad";
}

/** O veredito da faixa (90+ aclamação universal, 75–89 favorável, 50–74 mista, abaixo disso desfavorável). */
export function metacriticVerdict(score: number): MetacriticVerdict {
  return score >= 90 ? "universal" : score >= 75 ? "favorable" : score >= 50 ? "mixed" : "unfavorable";
}

/** Nota válida de 0 a 100; qualquer outra coisa conta como sem dado. */
export function validScore(score: number | null): number | null {
  return score !== null && Number.isFinite(score) && score >= 0 && score <= 100 ? Math.round(score) : null;
}

export type ImageKind = "hero" | "logo" | "cover";

/** `tako-img://` do núcleo (docs/web/ponte.md §4); largura e altura em pixels físicos. */
export function imageUrl(kind: ImageKind, id: string, width: number, height: number): string {
  return coreImageUrl(kind, id, width, height);
}

/** Texto em parágrafos: linhas em branco separam, e espaços nas pontas não contam. */
export function paragraphs(text: string | null): string[] {
  return (text ?? "")
    .split(/\n\s*\n/)
    .map((p) => p.trim())
    .filter(Boolean);
}

/** Lista que veio vazia ou só de brancos é sem dado. */
export function cleanList(list: readonly string[] | null): string[] {
  return (list ?? []).map((item) => item.trim()).filter(Boolean);
}

/** O que a página mostra, já formatado; o que o núcleo deixou `null` ou vazio não vira bloco. */
export interface GameBlocks {
  description: string[];
  developer: string | null;
  publisher: string | null;
  release: string | null;
  genres: string[];
  tags: string[];
  size: string | null;
  lastPlayed: string | null;
  playTime: string | null;
  metacritic: number | null;
  sessions: { startedAt: string; duration: string }[];
}

function text(value: string | null): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
}

export function gameBlocks(details: GameDetails, now: Date = new Date(), timeZone?: string): GameBlocks {
  const { game } = details;
  return {
    description: paragraphs(details.description),
    developer: text(details.developer),
    publisher: text(details.publisher),
    release: formatReleaseDate(text(details.releaseDate)),
    genres: cleanList(details.genres),
    tags: cleanList(details.tags),
    size: text(details.sizeLabel),
    lastPlayed: formatLastPlayed(game.lastPlayedAt, now, timeZone),
    playTime: formatPlayTime(game.playTimeSeconds),
    metacritic: validScore(details.metacriticScore),
    sessions: (details.recentSessions ?? []).flatMap((session) => {
      const started = formatLastPlayed(session.startedAt, now, timeZone);
      return started ? [{ startedAt: started, duration: formatDuration(session.durationSeconds) }] : [];
    }),
  };
}

/** O que o primário faz: joga, não pode jogar ainda (lançamento futuro) ou não está instalado. */
export type PrimaryState = "play" | "upcoming" | "not-installed";

export function primaryState(details: GameDetails): PrimaryState {
  if (details.canPlay) return "play";
  if (details.upcoming) return "upcoming";
  return details.game.installed ? "play" : "not-installed";
}
