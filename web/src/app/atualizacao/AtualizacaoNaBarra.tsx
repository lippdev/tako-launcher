import { Aviso } from "../../components/Aviso/Aviso";
import { AtualizacaoDoApp } from "../../components/AtualizacaoDoApp/AtualizacaoDoApp";
import { Icon } from "../../components/Icon/Icon";
import { cx } from "../../lib/cx";
import { useT } from "../../i18n";
import { dialogView, indicator } from "./atualizacaoModel";
import {
  applyUpdate,
  closeUpdateDialog,
  dismissUpdateNotice,
  openUpdateDialog,
  retryUpdateDownload,
  useUpdates,
} from "./atualizacaoStore";
import styles from "./AtualizacaoNaBarra.module.css";

/** Abre o endereço no navegador do sistema: o hospedeiro manda janelas novas e links externos para fora do app. */
function openExternal(url: string) {
  window.open(url, "_blank", "noopener,noreferrer");
}

/**
 * A atualização do Tako na barra de cima (README da AtualizacaoDoApp): o botão que aparece com a versão nova baixando,
 * pronta, com falha ou só à espera da página, o diálogo `AtualizacaoDoApp` e o aviso dos resultados. O núcleo decide
 * tudo (procurar, baixar, conferir o SHA-256, reiniciar); aqui só se desenha o que ele diz (`updates.changed`).
 */
export function AtualizacaoNaBarra() {
  const t = useT();
  const { status, dialogOpen, notice } = useUpdates();
  const comma = !document.documentElement.lang.startsWith("en");
  const marker = indicator(status, t);
  const view = status ? dialogView(status, t, comma) : null;
  const pageUrl = status?.offer?.pageUrl;

  return (
    <>
      {marker && (
        <button
          type="button"
          className={cx(styles["button"], dialogOpen && styles["open"], status?.state === "downloading" && styles["working"])}
          aria-label={marker.label}
          title={marker.label}
          aria-haspopup="dialog"
          aria-expanded={dialogOpen}
          onClick={openUpdateDialog}
        >
          <Icon name={marker.icon} />
          {marker.attention && <span className={styles["dot"]} aria-hidden="true" />}
        </button>
      )}
      {view && status && (
        <AtualizacaoDoApp
          open={dialogOpen}
          state={view.estado}
          version={view.versao}
          subtitle={view.subtitulo}
          releaseNotes={view.novidades}
          progress={view.progresso}
          {...(view.observacao ? { note: view.observacao } : {})}
          labels={{
            close: t("common.close"),
            // Baixando, "Depois" só esconde o diálogo: o download continua (como o "Esconder" do Slint).
            later: t(view.estado === "baixando" ? "updates.desktop.hide" : "updates.desktop.later"),
            notes: t("updates.desktop.notes"),
            download: t("updates.desktop.view"),
            restart: t("updates.desktop.restart"),
            retry: t("updates.desktop.retry"),
            availableTitle: t("updates.desktop.found", { version: view.versao }),
            downloadingTitle: t("updates.desktop.downloading"),
            readyTitle: t("desktop.window.update_ready"),
            errorTitle: t("updates.desktop.download_failed"),
            downloadingHelp: t("updates.desktop.help_downloading"),
            allNotes: t("updates.desktop.all_notes"),
          }}
          onClose={closeUpdateDialog}
          // Sem arquivo conferível para baixar, o botão principal só leva à página da release.
          onDownload={() => {
            if (pageUrl) openExternal(pageUrl);
          }}
          onRestart={() => {
            void applyUpdate();
          }}
          onRetry={() => {
            void retryUpdateDownload();
          }}
          {...(pageUrl
            ? {
                onViewNotes: () => {
                  openExternal(pageUrl);
                },
              }
            : {})}
        />
      )}
      <Aviso open={notice !== null} tone={notice?.tone ?? "success"} noticeKey={notice?.key ?? 0} onClose={dismissUpdateNotice}>
        {notice?.text ?? ""}
      </Aviso>
    </>
  );
}
