// Atualização do app na interface (WEB-33): funções puras que transformam o que a ponte devolve (`updates.status` e o
// evento `updates.changed`) no que as telas desenham e nos avisos. O núcleo é a fonte da verdade (D-10); aqui só mora
// o que é de tela. Sem importações em execução, para o teste rodar direto no Node.

import type { UpdateStatus } from "../../bridge/generated/UpdateStatus";
import type { EstadoAtualizacao } from "../../components/AtualizacaoDoApp/AtualizacaoDoApp";

/** O `t()` da interface, só no formato que as funções daqui precisam. */
export type Translate = (key: string, vars?: Record<string, string | number>) => string;

/** Megabytes com uma casa, como o Slint (`{:.1}`); em português a vírgula é a decimal. */
export function formatMb(bytes: number, comma = true): string {
  const text = (bytes / 1048576).toFixed(1);
  return comma ? text.replace(".", ",") : text;
}

/** Em que estado o diálogo desenha cada situação do núcleo; `null` = não há o que mostrar. */
export function dialogState(status: UpdateStatus): EstadoAtualizacao | null {
  if (status.offer === null) return null;
  switch (status.state) {
    case "available":
      return "disponivel";
    case "downloading":
      return "baixando";
    case "ready":
    case "installing":
      return "pronta";
    case "download_failed":
      return "erro";
    default:
      return null;
  }
}

/** O que o diálogo precisa saber além do estado, já nos textos do idioma. */
export interface DialogView {
  estado: EstadoAtualizacao;
  versao: string;
  subtitulo: string;
  novidades: readonly string[];
  /** 0 a 100. */
  progresso: number;
  /** Letra de ajuda embaixo das novidades (a regra do "Depois" e o aviso do jogo aberto). */
  observacao: string | undefined;
  /** O botão principal de "disponível" só leva à página da release (sem arquivo conferível para baixar). */
  soPagina: boolean;
}

export function dialogView(status: UpdateStatus, t: Translate, comma = true): DialogView | null {
  const estado = dialogState(status);
  const offer = status.offer;
  if (estado === null || offer === null) return null;
  const size = offer.size;
  let subtitulo = offer.version;
  let observacao: string | undefined;
  if (estado === "baixando") {
    subtitulo = [
      offer.version,
      size > 0 ? t("updates.desktop.progress", { done: formatMb(status.downloadedBytes, comma), total: formatMb(size, comma) }) : "",
      status.remainingSeconds !== null ? t("updates.desktop.remaining", { seconds: status.remainingSeconds }) : "",
    ]
      .filter((part) => part !== "")
      .join(" · ");
  } else if (estado === "pronta") {
    if (size > 0) subtitulo = `${offer.version} ${t("updates.desktop.size", { size: formatMb(size, comma) })}`;
    // Como o Slint: reiniciar não fecha o jogo, então com um aberto o reinício espera.
    observacao = [t("updates.desktop.help_ready"), status.gameRunning ? t("updates.finish_game") : ""].filter((part) => part !== "").join(" ");
  }
  return {
    estado,
    versao: offer.version,
    subtitulo,
    novidades: offer.notes,
    progresso: Math.round(status.progress * 1000) / 10,
    observacao,
    soPagina: estado === "disponivel" && !offer.installable,
  };
}

export type NoticeTone = "success" | "error" | "info";
export interface Notice {
  tone: NoticeTone;
  text: string;
}

/** O que a tela guarda entre um evento e outro. */
export interface UiFlags {
  /** O usuário pediu a procura e ainda espera o resultado. */
  explicit: boolean;
  dialogOpen: boolean;
}

export interface Outcome extends UiFlags {
  notice: Notice | null;
}

/**
 * O que fazer quando o núcleo avisa `next`, vindo de `prev`:
 * - procura pedida pelo usuário: "em dia" e "não deu para procurar" viram aviso; versão achada abre o diálogo;
 * - procura automática ao abrir (vimos `searching`): versão achada com download abre o diálogo, como no Slint;
 * - download que falha com o diálogo fechado vira aviso; pronta só acende a barra (sem aviso, como no Slint).
 */
export function transition(prev: UpdateStatus | null, next: UpdateStatus, flags: UiFlags, t: Translate): Outcome {
  const kept: Outcome = { ...flags, notice: null };
  const sawSearch = prev?.state === "searching";
  switch (next.state) {
    case "current":
      return flags.explicit
        ? { ...kept, explicit: false, notice: { tone: "success", text: t("updates.desktop.current_toast", { version: next.current }) } }
        : kept;
    case "check_failed":
      return flags.explicit
        ? { ...kept, explicit: false, notice: { tone: "error", text: t("updates.desktop.failed_toast") } }
        : kept;
    case "available":
      return flags.explicit ? { ...kept, explicit: false, dialogOpen: true } : kept;
    case "downloading":
      return flags.explicit || sawSearch ? { ...kept, explicit: false, dialogOpen: true } : kept;
    case "download_failed":
      return !flags.dialogOpen && prev?.state === "downloading"
        ? { ...kept, notice: { tone: "error", text: t("updates.desktop.download_failed") } }
        : kept;
    default:
      return kept;
  }
}

/** O botão da barra de cima: aparece com a atualização em andamento, pronta, com falha ou só à espera da página. */
export interface Indicator {
  icon: "download" | "refresh" | "badge-alert";
  label: string;
  /** Pronta: a barra chama atenção (um ponto na cor de destaque). */
  attention: boolean;
}

export function indicator(status: UpdateStatus | null, t: Translate): Indicator | null {
  if (status === null || status.offer === null) return null;
  switch (status.state) {
    case "downloading":
      return { icon: "download", label: t("updates.desktop.downloading"), attention: false };
    case "ready":
      return { icon: "refresh", label: t("desktop.window.update_ready"), attention: true };
    case "download_failed":
      return { icon: "badge-alert", label: t("updates.desktop.download_failed"), attention: true };
    case "available":
      return { icon: "download", label: t("updates.desktop.found", { version: status.offer.version }), attention: true };
    default:
      return null;
  }
}

/** A linha "Procurar atualizações" do menu da marca: muda no lugar conforme o estado (como no Slint). */
export interface MenuLine {
  label: string;
  icon: "refresh" | "check" | "badge-alert";
  disabled: boolean;
}

export function menuLine(status: UpdateStatus | null, transient: "current" | "failed" | null, idleLabel: string, t: Translate): MenuLine {
  if (status?.state === "searching") return { label: t("updates.desktop.searching"), icon: "refresh", disabled: true };
  if (transient === "current") return { label: t("updates.desktop.current"), icon: "check", disabled: false };
  if (transient === "failed") return { label: t("updates.desktop.check_failed"), icon: "badge-alert", disabled: false };
  return { label: idleLabel, icon: "refresh", disabled: false };
}
