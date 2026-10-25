import { imageUrl } from "../lib/imageVersions";
import { useArtRevision } from "../lib/useImageVersion";
import { useCallback, useState } from "react";
import { call } from "../bridge";
import { AtividadeNaBarra } from "../components/AtividadeNaBarra/AtividadeNaBarra";
import { Aviso } from "../components/Aviso/Aviso";
import { BotaoDesktop } from "../components/BotaoDesktop/BotaoDesktop";
import { Dialogo } from "../components/Dialogo/Dialogo";
import { PainelDownloads } from "../components/PainelDownloads/PainelDownloads";
import { useT } from "../i18n";
import { activityCount, findEntry, summaryPercent, toPanelItem } from "./downloadsModel";
import { refreshDownloads, useDownloads } from "./downloadsStore";
import { downloadsStrings as s } from "./downloadsStrings";
import { strings } from "./strings";
import styles from "./DownloadsNaBarra.module.css";

interface DownloadsNaBarraProps {
  /** Abre a página do jogo na biblioteca. */
  onOpenGame: (gameId: string) => void;
  /** Abre Configurações › Downloads (o painel de downloads desligados leva até lá). */
  onOpenSettings: () => void;
}

/** A capa do jogo vinculado, no tamanho da miniatura do painel (36×46), em pixels físicos. */
function coverUrl(gameId: string): string {
  const ratio = window.devicePixelRatio || 1;
  const w = Math.round(36 * ratio);
  const h = Math.round(46 * ratio);
  return imageUrl("cover", gameId, w, h);
}

/**
 * O botão Downloads da barra de cima (AtividadeNaBarra) e o painel que ele abre (PainelDownloads). Pausar, retomar,
 * cancelar (com a pergunta sobre os arquivos) e abrir o jogo passam pela ponte; `downloads.changed` atualiza tudo.
 * Desligados, a interface não chama o motor: o painel diz que estão desligados e leva às Configurações.
 */
export function DownloadsNaBarra({ onOpenGame, onOpenSettings }: DownloadsNaBarraProps) {
  useArtRevision();
  const t = useT();
  const { enabled, entries, summary } = useDownloads();
  const [open, setOpen] = useState(false);
  const [anchor, setAnchor] = useState<HTMLButtonElement | null>(null);
  const [cancelId, setCancelId] = useState<number | null>(null);
  const [failures, setFailures] = useState(0);
  const [noticeClosedAt, setNoticeClosedAt] = useState(0);

  const comma = !document.documentElement.lang.startsWith("en");
  const items = enabled ? entries.map((entry) => toPanelItem(entry, t, coverUrl, comma)) : [];
  const count = enabled ? activityCount(summary) : 0;
  const percent = enabled ? summaryPercent(summary) : null;

  const run = useCallback((action: Promise<unknown>) => {
    void action
      .catch((error: unknown) => {
        console.error("downloads: ação falhou", error);
        setFailures((n) => n + 1);
      })
      .finally(() => {
        void refreshDownloads();
      });
  }, []);

  const cancelEntry = cancelId === null ? undefined : entries.find((entry) => entry.id === cancelId);
  const closeCancel = () => {
    setCancelId(null);
  };
  const cancel = (deleteFiles: boolean) => {
    if (cancelId !== null) run(call("downloads.cancel", { downloadId: cancelId, deleteFiles }));
    closeCancel();
  };

  return (
    <>
      <AtividadeNaBarra
        ref={setAnchor}
        count={count}
        progress={percent}
        open={open}
        title={strings.topbar.downloads}
        aria-label={count > 0 ? t("downloads.n_active", { n: count }) : strings.topbar.downloads}
        onClick={() => {
          setOpen((value) => !value);
        }}
      />
      <PainelDownloads
        open={open}
        onClose={() => {
          setOpen(false);
        }}
        anchor={anchor}
        items={items}
        title={t("downloads.title")}
        activeLabel={() => t("downloads.n_active", { n: summary.downloading })}
        emptyLabel={enabled ? t("downloads.empty") : s.off}
        sectionLabels={{
          downloading: t("downloads.group_active"),
          queued: t("downloads.group_queued"),
          completed: t("downloads.group_done"),
        }}
        stateLabels={{
          baixando: t("downloads.group_active"),
          pausado: t("downloads.paused"),
          na_fila: t("downloads.group_queued"),
          concluido: t("downloads.group_done"),
          erro: s.error,
        }}
        actionLabels={{
          pause: t("downloads.pause"),
          resume: t("downloads.resume"),
          cancel: t("downloads.cancel"),
          openGame: s.openGame,
        }}
        onPause={(id) => {
          const entry = findEntry(entries, id);
          if (entry) run(call("downloads.pause", { downloadId: entry.id }));
        }}
        onResume={(id) => {
          const entry = findEntry(entries, id);
          if (entry) run(call("downloads.resume", { downloadId: entry.id }));
        }}
        onCancel={(id) => {
          const entry = findEntry(entries, id);
          if (entry) setCancelId(entry.id);
        }}
        onOpenGame={(id) => {
          const gameId = findEntry(entries, id)?.gameId;
          if (!gameId) return;
          setOpen(false);
          onOpenGame(gameId);
        }}
      >
        {enabled === false && (
          <div className={styles["off"]}>
            <p className={styles["offText"]}>{t("downloads.enable_desc")}</p>
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                setOpen(false);
                onOpenSettings();
              }}
            >
              {s.openSettings}
            </BotaoDesktop>
          </div>
        )}
      </PainelDownloads>

      <Dialogo
        open={cancelEntry !== undefined}
        onClose={closeCancel}
        size="l"
        closeLabel={t("common.close")}
        title={
          cancelEntry?.gameId
            ? t("downloads.cancel_title_game", { title: cancelEntry.title })
            : t("downloads.cancel_title")
        }
        actions={
          <>
            <BotaoDesktop variant="secondary" size="sm" onClick={closeCancel}>
              {t("downloads.cancel_keep")}
            </BotaoDesktop>
            <BotaoDesktop
              variant="secondary"
              size="sm"
              onClick={() => {
                cancel(false);
              }}
            >
              {t("downloads.cancel_keep_files")}
            </BotaoDesktop>
            <BotaoDesktop
              variant="primary"
              size="sm"
              onClick={() => {
                cancel(true);
              }}
            >
              {t("downloads.cancel_delete")}
            </BotaoDesktop>
          </>
        }
      >
        <p className={styles["dialogText"]}>{t("downloads.cancel_text")}</p>
      </Dialogo>

      <Aviso
        open={failures > noticeClosedAt}
        onClose={() => {
          setNoticeClosedAt(failures);
        }}
        noticeKey={failures}
        tone="error"
      >
        {s.actionError}
      </Aviso>
    </>
  );
}
