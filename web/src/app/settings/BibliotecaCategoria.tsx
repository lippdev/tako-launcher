import { useEffect, useRef, useState } from "react";
import { call, on } from "../../bridge";
import type { LibrarySourceReport } from "../../bridge/generated/LibrarySourceReport";
import type { LibrarySources } from "../../bridge/generated/LibrarySources";
import { Aviso } from "../../components/Aviso/Aviso";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT, type TFunction } from "../../i18n";
import { ColecoesAutomaticas } from "./ColecoesAutomaticas";
import { EmuladoresERoms } from "./EmuladoresERoms";
import { LinhaChave, type CategoryProps } from "./Linhas";
import type { Notify } from "./notice";
import { KEYS, isStoreDisabled, readLanguage, withStoreEnabled } from "./model";
import styles from "./TelaConfiguracoes.module.css";

/** "24 out., 16:17" no idioma da interface; data ilegível não mostra nada. */
function formatWhen(iso: string, language: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return new Intl.DateTimeFormat(language, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" }).format(date);
}

/** O que a linha de uma loja diz: quantos jogos achou, o aviso do núcleo e quando leu por último. */
function describeSource(report: LibrarySourceReport, disabled: boolean, language: string, t: TFunction): string {
  if (disabled) return t("settings.library_web.off");
  // Ligada depois da última leitura: o relatório é de quando estava desligada.
  if (!report.enabled) return t("settings.library_web.not_read");
  const parts: string[] = [];
  if (!report.clientInstalled && report.found === 0 && !report.error) parts.push(t("settings.library_web.no_client"));
  else parts.push(t("common.games", { n: report.found }));
  if (report.error) parts.push(report.error);
  const when = report.lastScannedAt ? formatWhen(report.lastScannedAt, language) : "";
  if (when) parts.push(t("settings.library_web.read_at", { when }));
  return parts.join(" · ");
}

interface NoticeState {
  key: number;
  text: string;
  tone: "success" | "error";
}

/** Um aviso novo leva uma chave nova, para o mesmo texto aparecer de novo. */
function nextNotice(previous: NoticeState | null, tone: NoticeState["tone"], text: string): NoticeState {
  return { key: (previous?.key ?? 0) + 1, text, tone };
}

function progressText(sources: LibrarySources, finished: boolean, t: TFunction): string | null {
  if (sources.scanning) {
    return sources.measuringSizes
      ? t("settings.library_web.progress_measuring")
      : t("settings.library_web.progress_scanning", { done: sources.completed, total: sources.total });
  }
  if (sources.error) return sources.error;
  return finished ? t("settings.library_web.progress_done") : null;
}

/**
 * Configurações › Biblioteca e lojas: liga e desliga as lojas (`DisabledStores`) e lê de novo, com o andamento; os
 * emuladores e as pastas de ROMs; e as coleções automáticas. Cada parte avisa o resultado no mesmo aviso de baixo.
 */
export function BibliotecaCategoria({ values, save }: CategoryProps) {
  const t = useT();
  const language = readLanguage(values);
  const [sources, setSources] = useState<LibrarySources | null>(null);
  const [finished, setFinished] = useState(false);
  const [starting, setStarting] = useState(false);
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const wasScanning = useRef(false);

  const notify: Notify = (tone, text) => {
    setNotice((previous) => nextNotice(previous, tone, text));
  };

  function fail(text: string) {
    setNotice((previous) => nextNotice(previous, "error", text));
  }

  function receive(next: LibrarySources) {
    if (wasScanning.current && !next.scanning) setFinished(true);
    wasScanning.current = next.scanning;
    setSources(next);
  }

  useEffect(() => {
    let cancelled = false;
    // O evento chega durante a leitura; a consulta cobre o estado de quando a categoria abriu.
    const stop = on("library.scan.progress", (next) => {
      if (!cancelled) receive(next);
    });
    call("library.sources")
      .then((next) => {
        if (!cancelled) receive(next);
      })
      .catch((error: unknown) => {
        console.error("library.sources falhou", error);
        if (!cancelled) fail(t("settings.library_web.sources_error"));
      });
    return () => {
      cancelled = true;
      stop();
    };
  }, [t]);

  function scan() {
    setStarting(true);
    setFinished(false);
    call("library.scan")
      .catch((error: unknown) => {
        console.error("library.scan falhou", error);
        fail(t("settings.library_web.scan_error"));
      })
      .finally(() => {
        setStarting(false);
      });
  }

  const disabledStores = values[KEYS.disabledStores];
  const scanning = starting || (sources?.scanning ?? false);
  const progress = sources ? progressText(sources, finished, t) : null;

  return (
    <>
      <GrupoDeConfiguracao title={t("settings.library_web.stores")}>
        {sources && sources.sources.length > 0 ? (
          sources.sources.map((report) => {
            const disabled = isStoreDisabled(disabledStores, report.store);
            return (
              <LinhaChave
                key={report.store}
                title={report.name}
                description={describeSource(report, disabled, language, t)}
                checked={!disabled}
                onChange={(enabled) => void save(KEYS.disabledStores, withStoreEnabled(disabledStores, report.store, enabled))}
              />
            );
          })
        ) : (
          <LinhaDeConfiguracao title={sources ? t("settings.library_web.empty") : ""} />
        )}
      </GrupoDeConfiguracao>
      <div>
        <GrupoDeConfiguracao title={t("settings.library_web.reading")}>
          <LinhaDeConfiguracao
            title={t("settings.library.rescan")}
            description={t("settings.library.rescan_desc")}
            control={
              <BotaoDesktop variant="outlined" icon="refresh" disabled={scanning} onClick={scan}>
                {t("settings.library.rescan")}
              </BotaoDesktop>
            }
          />
        </GrupoDeConfiguracao>
        {/* O andamento fala na região ao vivo: a leitura segue em segundo plano e o leitor de tela avisa. */}
        <p role="status" className={styles["progress"]}>
          {progress}
        </p>
      </div>
      <EmuladoresERoms notify={notify} onScan={scan} scanning={scanning} />
      <ColecoesAutomaticas notify={notify} />
      <Aviso
        open={notice !== null}
        onClose={() => {
          setNotice(null);
        }}
        noticeKey={notice?.key ?? 0}
        tone={notice?.tone ?? "error"}
      >
        {notice?.text}
      </Aviso>
    </>
  );
}
