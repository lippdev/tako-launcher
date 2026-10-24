// Downloads na interface (WEB-14b): funções puras que transformam o que a ponte devolve (`downloads.list` e
// `downloads.summary`) no que as telas desenham. Sem importações em execução, para o teste rodar direto no Node.

import type { DownloadEntry } from "../bridge/generated/DownloadEntry";
import type { DownloadsSummary } from "../bridge/generated/DownloadsSummary";
import type { ItemDownload } from "../components/PainelDownloads/PainelDownloads";

/** O `t()` da interface, só no formato que as funções daqui precisam. */
export type Translate = (key: string, vars?: Record<string, string | number>) => string;

/** As escolhas de limite de envio em KiB/s que o núcleo aceita em `DownloadsUploadLimitKiB`; 0 = sem limite. */
export const UPLOAD_LIMITS = [0, 64, 256, 1024] as const;
export type UploadLimit = (typeof UPLOAD_LIMITS)[number];

/** O limite guardado; ausente ou fora das quatro escolhas vale "sem limite" (o padrão do núcleo). */
export function readUploadLimit(text: string | undefined): UploadLimit {
  return UPLOAD_LIMITS.find((limit) => String(limit) === text) ?? 0;
}

/** Tamanho como o núcleo mostra (B, KB, MB, GB com uma casa); em português a vírgula é a decimal. */
export function formatBytes(bytes: number, comma = true): string {
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  if (unit === 0) return `${String(Math.round(bytes))} B`;
  // Como o núcleo: KB sem casa decimal, MB e GB com uma.
  if (unit === 1) return `${String(Math.max(1, Math.floor(bytes / 1024)))} KB`;
  const text = value.toFixed(1);
  return `${comma ? text.replace(".", ",") : text} ${units[unit] ?? "GB"}`;
}

/** O download ainda está em aberto: baixando, na fila ou pausado (nem concluído nem com erro). */
export function isOpen(entry: DownloadEntry): boolean {
  return entry.state === "downloading" || entry.state === "queued" || entry.state === "paused";
}

/** A porcentagem inteira (0 a 100) de um download; `null` enquanto o tamanho é desconhecido. */
export function entryPercent(entry: DownloadEntry): number | null {
  return entry.progress === null ? null : Math.round(Math.min(1, Math.max(0, entry.progress)) * 100);
}

/** Quantos downloads a pílula da barra conta: os que baixam e os que esperam a vez. */
export function activityCount(summary: DownloadsSummary): number {
  return summary.downloading + summary.queued;
}

/** A porcentagem geral da barra: tudo o que foi baixado sobre o total dos downloads em aberto. */
export function summaryPercent(summary: DownloadsSummary): number | null {
  if (activityCount(summary) === 0 || summary.total <= 0) return null;
  return Math.round(Math.min(1, summary.done / summary.total) * 100);
}

/** Jogo → porcentagem do download em aberto vinculado a ele (a linha da lista mostra "Baixando · n%"). */
export function percentByGame(entries: readonly DownloadEntry[]): ReadonlyMap<string, number> {
  const map = new Map<string, number>();
  for (const entry of entries) {
    if (entry.gameId !== null && isOpen(entry)) map.set(entry.gameId, entryPercent(entry) ?? 0);
  }
  return map;
}

/** Tempo que falta como o app atual: minutos, depois horas ("12 min", "2 h 5 min", "11 h"). */
function formatLeft(entry: DownloadEntry): string | null {
  if (entry.etaSeconds === null || entry.speed <= 0) return null;
  const minutes = Math.max(1, Math.ceil(entry.etaSeconds / 60));
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (hours === 0) return `${String(rest)} min`;
  if (rest === 0 || hours >= 10) return `${String(hours)} h`;
  return `${String(hours)} h ${String(rest)} min`;
}

/**
 * A linha de baixo de um download, com os mesmos textos do app atual (`downloads.toml`): situação, tamanho e tempo
 * que falta. Compacta para caber no painel de 340: a porcentagem está na barra e a fonte fica de fora.
 */
export function entryLine(entry: DownloadEntry, t: Translate, comma = true): string {
  const parts: string[] = [];
  if (entry.state === "failed") {
    parts.push(t("downloads.failed", { error: entry.error ?? "" }));
  } else if (entry.total === null && entry.state === "downloading") {
    parts.push(t("downloads.metadata"));
  } else {
    if (entry.state === "paused") parts.push(t("downloads.paused"));
    if (entry.progress !== null && entry.total !== null) {
      parts.push(t("downloads.of", { done: formatBytes(entry.done, comma), total: formatBytes(entry.total, comma) }));
    }
    if (entry.state === "queued") parts.push(t("downloads.waiting"));
    const left = entry.state === "downloading" ? formatLeft(entry) : null;
    if (left) parts.push(t("downloads.left", { time: left }));
  }
  return parts.join(" · ");
}

/** O download como o PainelDownloads o desenha. */
export function toPanelItem(
  entry: DownloadEntry,
  t: Translate,
  cover: (gameId: string) => string,
  comma = true,
): ItemDownload {
  const percent = entryPercent(entry);
  return {
    id: String(entry.id),
    titulo: entry.title,
    estado:
      entry.state === "downloading"
        ? "baixando"
        : entry.state === "queued"
          ? "na_fila"
          : entry.state === "paused"
            ? "pausado"
            : entry.state === "completed"
              ? "concluido"
              : "erro",
    progresso: percent,
    subtitulo: entryLine(entry, t, comma),
    capa: entry.gameId === null ? null : cover(entry.gameId),
    erro: entry.error,
  };
}

/** O download aberto ou concluído que tem esse id da interface (o do painel é texto). */
export function findEntry(entries: readonly DownloadEntry[], id: string): DownloadEntry | undefined {
  return entries.find((entry) => String(entry.id) === id);
}
