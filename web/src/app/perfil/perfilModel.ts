// Regras da tela Perfil que não dependem de React nem da ponte: como o núcleo entrega os números e como a tela os
// escreve (duração, semana, dia). Os textos vêm do catálogo (`t`), com as mesmas chaves do app Slint
// (`tako-ui/src/shell/profile.rs`). Só tipos nas importações, para o teste rodar direto no Node.

import type { LibraryGame } from "../../bridge/generated/LibraryGame";
import type { ProfileDay } from "../../bridge/generated/ProfileDay";
import type { ProfileEntry } from "../../bridge/generated/ProfileEntry";
import type { ProfileSession } from "../../bridge/generated/ProfileSession";
import type { ProfileWeekDay } from "../../bridge/generated/ProfileWeekDay";
import type { SessionGroup, HistorySession } from "../../components/PerfilSessoes/PerfilSessoes";
import type { TFunction } from "../../i18n/core";

/** As abas da tela, na ordem. "Família" fica de fora enquanto não há limites por perfil (o Slint também não tem). */
export const PROFILE_TABS = ["overview", "sessions", "collections"] as const;
export type ProfileTab = (typeof PROFILE_TABS)[number];

/** Os períodos da guia Sessões, na ordem. */
export const PERIODS = ["week", "month", "all"] as const;

/** Os dias da semana como o catálogo chama (`profile.weekday.mon`…), de segunda a domingo. */
export const WEEKDAYS = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"] as const;

const pad = (n: number) => String(n).padStart(2, "0");

/** "42 min", "2 h", "6 h 37 min" (`profile.duration.*`). */
export function durationText(seconds: number, t: TFunction): string {
  const minutes = Math.floor(Math.max(0, seconds) / 60);
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return t("profile.duration.minutes", { m });
  if (m === 0) return t("profile.duration.hours", { h });
  return t("profile.duration.hours_minutes", { h, m: pad(m) });
}

/** O tempo de jogo total ("598 h", "42 min"); zero é "—" (`presentation.play_time.*`). */
export function playTimeText(seconds: number, t: TFunction): string {
  if (seconds <= 0) return "—";
  const minutes = Math.floor(seconds / 60);
  return minutes < 60
    ? t("presentation.play_time.minutes", { n: Math.max(1, minutes) })
    : t("presentation.play_time.hours", { n: Math.floor(minutes / 60) });
}

interface Ymd {
  y: number;
  m: number;
  d: number;
}

function parseDay(day: string): Ymd | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(day);
  return match ? { y: Number(match[1]), m: Number(match[2]), d: Number(match[3]) } : null;
}

/** Dias corridos entre duas datas `AAAA-MM-DD` (sem fuso: só o calendário). */
export function daysBetween(from: string, to: string): number {
  const a = parseDay(from);
  const b = parseDay(to);
  if (!a || !b) return 0;
  return Math.round((Date.UTC(b.y, b.m - 1, b.d) - Date.UTC(a.y, a.m - 1, a.d)) / 86_400_000);
}

/** O dia da semana de uma data `AAAA-MM-DD`, de segunda (0) a domingo (6). */
export function weekdayIndex(day: string): number {
  const p = parseDay(day);
  return p ? (new Date(Date.UTC(p.y, p.m - 1, p.d)).getUTCDay() + 6) % 7 : 0;
}

/** A data curta do idioma (`presentation.date.short`: `%d/%m` ou `%m/%d`). */
export function shortDate(day: string, t: TFunction): string {
  const p = parseDay(day);
  if (!p) return day;
  return t("presentation.date.short").replace(/%([dmY])/g, (_, f: string) =>
    f === "d" ? pad(p.d) : f === "m" ? pad(p.m) : String(p.y),
  );
}

/** O dia local de um instante ISO, `AAAA-MM-DD`. */
export function localDay(iso: string): string {
  const date = new Date(iso);
  return `${String(date.getFullYear())}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** "14:02", no relógio local. */
export function clock(iso: string): string {
  const date = new Date(iso);
  return `${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

/** "Hoje", "Ontem" ou a data curta. */
export function dayLabel(day: string, today: string, t: TFunction): string {
  const diff = daysBetween(day, today);
  if (diff === 0) return t("desktop.day.today");
  if (diff === 1) return t("desktop.day.yesterday");
  return shortDate(day, t);
}

/** O que a tela mostra no cabeçalho do grupo de um dia: o nome do dia e a data. */
export function dayHeading(day: string, today: string, t: TFunction): { title: string; subtitle: string } {
  const diff = daysBetween(day, today);
  const subtitle = shortDate(day, t);
  if (diff === 0 || diff === 1) return { title: dayLabel(day, today, t), subtitle };
  return { title: t(`profile.weekday_long.${WEEKDAYS[weekdayIndex(day)] ?? "mon"}`), subtitle };
}

export interface WeekBar {
  label: string;
  /** 0 a 1, em relação ao dia mais longo. */
  value: number;
  seconds: number;
  isToday: boolean;
  isFuture: boolean;
}

/** As barras da semana: a letra do dia (hoje é "hoje") e a altura em relação ao dia mais longo. */
export function weekBars(week: readonly ProfileWeekDay[], today: string, t: TFunction): WeekBar[] {
  const max = Math.max(0, ...week.map((d) => d.seconds));
  return week.map((d) => {
    const diff = daysBetween(d.day, today);
    const isToday = diff === 0;
    const key = WEEKDAYS[weekdayIndex(d.day)] ?? "mon";
    return {
      label: isToday ? t("profile.weekday.today") : t(`profile.weekday.${key}`),
      value: max === 0 ? 0 : d.seconds / max,
      seconds: d.seconds,
      isToday,
      isFuture: diff < 0,
    };
  });
}

/** O gráfico para leitor de tela: "Esta semana: Segunda 40 min, Terça nada, …", só até hoje. */
export function weekAriaLabel(week: readonly ProfileWeekDay[], today: string, t: TFunction): string {
  const days = week
    .filter((d) => daysBetween(d.day, today) >= 0)
    .map((d) => {
      const name = t(`profile.weekday_long.${WEEKDAYS[weekdayIndex(d.day)] ?? "mon"}`);
      return `${name} ${d.seconds === 0 ? t("profile.nothing") : durationText(d.seconds, t)}`;
    });
  return `${t("profile.this_week")}: ${days.join(", ")}`;
}

export function weekTotal(week: readonly ProfileWeekDay[]): number {
  return week.reduce((sum, d) => sum + d.seconds, 0);
}

/** O nome do perfil como se mostra: o principal sem nome é "Principal". */
export function profileName(profile: Pick<ProfileEntry, "name">, t: TFunction): string {
  return profile.name === "" ? t("profiles.main") : profile.name;
}

/** A inicial no avatar. */
export function initial(name: string): string {
  const first = [...name.trim()][0];
  return first ? first.toLocaleUpperCase() : "";
}

/** "Jogou por último ontem" (sem sessão nenhuma, nada). */
export function lastPlayedText(lastPlayedAt: string | null, today: string, t: TFunction): string | null {
  if (lastPlayedAt === null) return null;
  return t("profile.last", { when: dayLabel(localDay(lastPlayedAt), today, t).toLocaleLowerCase() });
}

/** Resumo da guia Sessões: "6 sessões · 6 h 37 min". */
export function sessionsSummary(count: number, totalSeconds: number, t: TFunction): string {
  return t("profile.summary", { n: count, time: durationText(totalSeconds, t) });
}

/** Resumo da guia Coleções: "4 coleções · 26 jogos". */
export function collectionsSummary(collections: number, games: number, t: TFunction): string {
  return `${t("profile.collections", { n: collections })} · ${t("common.games", { n: games })}`;
}

/** "6 jogos · 4 instalados"; coleção sem jogos é "Nenhum jogo ainda". */
export function collectionDetail(gameCount: number, installedCount: number, t: TFunction): string {
  if (gameCount === 0) return t("profile.no_games");
  return `${t("common.games", { n: gameCount })} · ${t("profile.installed", { n: installedCount })}`;
}

export interface RowOptions {
  today: string;
  t: TFunction;
  /** URL da capa do jogo (`tako-img://` ou a de exemplo do hospedeiro falso). */
  cover: (game: LibraryGame) => string | undefined;
  /** `true` na Visão geral: "Hoje, 14:02" e o total do jogo; `false` na guia: "14:02 – 14:44". */
  overview: boolean;
}

/** A linha de uma sessão para o componente PerfilSessoes. */
export function historySession(session: ProfileSession, index: number, options: RowOptions): HistorySession {
  const { today, t, cover, overview } = options;
  const start = session.startedAt;
  const end = new Date(new Date(start).getTime() + session.durationSeconds * 1000).toISOString();
  const time = overview
    ? `${dayLabel(localDay(start), today, t)}, ${clock(start)}`
    : `${clock(start)} – ${clock(end)}`;
  const coverSrc = cover(session.game);
  const total = session.game.playTimeSeconds;
  return {
    id: `${session.game.id}:${start}:${String(index)}`,
    game: session.game.title,
    ...(coverSrc ? { coverSrc } : {}),
    time,
    duration: session.quick ? t("profile.quick") : durationText(session.durationSeconds, t),
    // Na Visão geral a linha também diz quanto o jogo já tem no total.
    ...(overview && total > 0
      ? { status: { kind: "normal" as const, text: t("profile.in_game", { time: playTimeText(total, t) }) } }
      : {}),
  };
}

/** Os grupos por dia da guia Sessões. */
export function sessionGroups(days: readonly ProfileDay[], options: Omit<RowOptions, "overview">): SessionGroup[] {
  let index = 0;
  return days.map((day) => {
    const { title, subtitle } = dayHeading(day.day, options.today, options.t);
    return {
      id: day.day,
      title,
      subtitle,
      total: durationText(day.totalSeconds, options.t),
      sessions: day.sessions.map((s) => historySession(s, index++, { ...options, overview: false })),
    };
  });
}
