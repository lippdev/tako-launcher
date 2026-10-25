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
          estado={view.estado}
          versao={view.versao}
          subtitulo={view.subtitulo}
          novidades={view.novidades}
          progresso={view.progresso}
          {...(view.observacao ? { observacao: view.observacao } : {})}
          rotulos={{
            fechar: t("common.close"),
            // Baixando, "Depois" só esconde o diálogo: o download continua (como o "Esconder" do Slint).
            depois: t(view.estado === "baixando" ? "updates.desktop.hide" : "updates.desktop.later"),
            novidades: t("updates.desktop.notes"),
            baixar: t("updates.desktop.view"),
            reiniciar: t("updates.desktop.restart"),
            tentarDeNovo: t("updates.desktop.retry"),
            tituloDisponivel: t("updates.desktop.found", { version: view.versao }),
            tituloBaixando: t("updates.desktop.downloading"),
            tituloPronta: t("desktop.window.update_ready"),
            tituloErro: t("updates.desktop.download_failed"),
            ajudaBaixando: t("updates.desktop.help_downloading"),
            todasAsNotas: t("updates.desktop.all_notes"),
          }}
          onFechar={closeUpdateDialog}
          // Sem arquivo conferível para baixar, o botão principal só leva à página da release.
          onBaixar={() => {
            if (pageUrl) openExternal(pageUrl);
          }}
          onReiniciar={() => {
            void applyUpdate();
          }}
          onTentarDeNovo={() => {
            void retryUpdateDownload();
          }}
          {...(pageUrl
            ? {
                onVerNotas: () => {
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
