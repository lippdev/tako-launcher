import { useEffect, useRef, useState } from "react";
import { call, on } from "../../bridge";
import type { LibrarySourceReport } from "../../bridge/generated/LibrarySourceReport";
import type { LibrarySources } from "../../bridge/generated/LibrarySources";
import { Aviso } from "../../components/Aviso/Aviso";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { settingsStrings } from "../strings";
import { LinhaChave, type CategoryProps } from "./Linhas";
import { KEYS, isStoreDisabled, readLanguage, withStoreEnabled } from "./model";
import styles from "./TelaConfiguracoes.module.css";

const s = settingsStrings.library;

/** "24 out., 16:17" no idioma da interface; data ilegível não mostra nada. */
function formatWhen(iso: string, language: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return new Intl.DateTimeFormat(language, { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" }).format(date);
}

/** O que a linha de uma loja diz: quantos jogos achou, o aviso do núcleo e quando leu por último. */
function describeSource(report: LibrarySourceReport, disabled: boolean, language: string): string {
  if (disabled) return s.off;
  // Ligada depois da última leitura: o relatório é de quando estava desligada.
  if (!report.enabled) return s.notRead;
  const parts: string[] = [];
  if (!report.clientInstalled && report.found === 0 && !report.error) parts.push(s.noClient);
  else parts.push(s.found(report.found));
  if (report.error) parts.push(report.error);
  const when = report.lastScannedAt ? formatWhen(report.lastScannedAt, language) : "";
  if (when) parts.push(s.readAt(when));
  return parts.join(" · ");
}

function progressText(sources: LibrarySources, finished: boolean): string | null {
  if (sources.scanning) {
    return sources.measuringSizes ? s.progress.measuring : s.progress.scanning(sources.completed, sources.total);
  }
  if (sources.error) return sources.error;
  return finished ? s.progress.done : null;
}

/** Configurações › Biblioteca e lojas: liga e desliga as lojas (`DisabledStores`) e lê de novo, com o andamento. */
export function BibliotecaCategoria({ values, save }: CategoryProps) {
  const language = readLanguage(values);
  const [sources, setSources] = useState<LibrarySources | null>(null);
  const [finished, setFinished] = useState(false);
  const [starting, setStarting] = useState(false);
  const [problem, setProblem] = useState<{ key: number; text: string } | null>(null);
  const wasScanning = useRef(false);

  function fail(text: string) {
    setProblem((previous) => ({ key: (previous?.key ?? 0) + 1, text }));
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
        if (!cancelled) fail(s.sourcesError);
      });
    return () => {
      cancelled = true;
      stop();
    };
  }, []);

  function scan() {
    setStarting(true);
    setFinished(false);
    call("library.scan")
      .catch((error: unknown) => {
        console.error("library.scan falhou", error);
        fail(s.scanError);
      })
      .finally(() => {
        setStarting(false);
      });
  }

  const disabledStores = values[KEYS.disabledStores];
  const scanning = starting || (sources?.scanning ?? false);
  const progress = sources ? progressText(sources, finished) : null;

  return (
    <>
      <GrupoDeConfiguracao title={s.stores}>
        {sources && sources.sources.length > 0 ? (
          sources.sources.map((report) => {
            const disabled = isStoreDisabled(disabledStores, report.store);
            return (
              <LinhaChave
                key={report.store}
                title={report.name}
                description={describeSource(report, disabled, language)}
                checked={!disabled}
                onChange={(enabled) => void save(KEYS.disabledStores, withStoreEnabled(disabledStores, report.store, enabled))}
              />
            );
          })
        ) : (
          <LinhaDeConfiguracao title={sources ? s.empty : ""} />
        )}
      </GrupoDeConfiguracao>
      <div>
        <GrupoDeConfiguracao title={s.reading}>
          <LinhaDeConfiguracao
            title={s.scan.title}
            description={s.scan.description}
            control={
              <BotaoDesktop variant="outlined" icon="refresh" disabled={scanning} onClick={scan}>
                {s.scan.action}
              </BotaoDesktop>
            }
          />
        </GrupoDeConfiguracao>
        {/* O andamento fala na região ao vivo: a leitura segue em segundo plano e o leitor de tela avisa. */}
        <p role="status" className={styles["progress"]}>
          {progress}
        </p>
      </div>
      <Aviso open={problem !== null} onClose={() => { setProblem(null); }} noticeKey={problem?.key ?? 0} tone="error">
        {problem?.text}
      </Aviso>
    </>
  );
}
