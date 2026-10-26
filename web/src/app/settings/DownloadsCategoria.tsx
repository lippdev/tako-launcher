import { useEffect, useState } from "react";
import { call, on } from "../../bridge";
import type { DownloadSources } from "../../bridge/generated/DownloadSources";
import { BotaoDesktop } from "../../components/BotaoDesktop/BotaoDesktop";
import { Dialogo } from "../../components/Dialogo/Dialogo";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../../components/LinhaDeConfiguracao/LinhaDeConfiguracao";
import { useT } from "../../i18n";
import { UPLOAD_LIMITS, formatBytes, readUploadLimit } from "../downloadsModel";
import { refreshDownloads } from "../downloadsStore";
import { LinhaChave, LinhaSelecao, type CategoryProps } from "./Linhas";
import { DEFAULTS, KEYS, boolText, readBool, readLanguage } from "./model";
import styles from "./TelaConfiguracoes.module.css";

/** "24 de out. de 2026" no idioma da interface; data ilegível não mostra nada. */
function formatDate(iso: string, language: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  return new Intl.DateTimeFormat(language, { day: "numeric", month: "short", year: "numeric" }).format(date);
}

/** As fontes do usuário, só leitura: com os downloads ligados a lista vem do núcleo e se refaz em `downloads.changed`. */
function useSources(enabled: boolean) {
  const [sources, setSources] = useState<DownloadSources | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    const load = () => {
      call("downloads.sources", { limit: 0 })
        .then((next) => {
          if (cancelled) return;
          setSources(next);
          setFailed(false);
        })
        .catch((error: unknown) => {
          console.error("downloads.sources falhou", error);
          if (!cancelled) setFailed(true);
        });
    };
    load();
    const stop = on("downloads.changed", load);
    return () => {
      cancelled = true;
      stop();
    };
  }, [enabled]);
  return { sources: enabled ? sources : null, failed: enabled && failed };
}

/**
 * Configurações › Downloads (ADR-0028): ligar (com o aviso legal na primeira vez), o limite de envio e as fontes do
 * usuário. Desligados, só a chave e o aviso de privacidade aparecem; o núcleo nem é consultado.
 */
export function DownloadsCategoria({ values, save }: CategoryProps) {
  const t = useT();
  const language = readLanguage(values);
  // Como o núcleo: ligados só com a chave e o aviso legal aceito.
  const accepted = readBool(values, KEYS.downloadsAccepted, DEFAULTS.downloadsAccepted);
  const enabled = readBool(values, KEYS.downloadsEnabled, DEFAULTS.downloadsEnabled) && accepted;
  const [legalOpen, setLegalOpen] = useState(false);
  const { sources, failed } = useSources(enabled);
  const comma = language === "pt-BR";

  /** Grava e avisa a barra e a lista, que dependem de a função estar ligada. */
  async function saveAndRefresh(key: string, value: string): Promise<boolean> {
    const saved = await save(key, value);
    void refreshDownloads();
    return saved;
  }

  async function toggle(on: boolean) {
    if (!on) {
      await saveAndRefresh(KEYS.downloadsEnabled, boolText(false));
      return;
    }
    // Sem o aviso legal aceito antes, ligar abre o aviso; só "Entendi e aceito" grava as duas chaves.
    if (accepted) {
      await saveAndRefresh(KEYS.downloadsEnabled, boolText(true));
    } else {
      setLegalOpen(true);
    }
  }

  async function accept() {
    setLegalOpen(false);
    if (await save(KEYS.downloadsAccepted, boolText(true))) await saveAndRefresh(KEYS.downloadsEnabled, boolText(true));
  }

  const limit = readUploadLimit(values[KEYS.downloadsUpload]);
  const limitOptions = UPLOAD_LIMITS.map((kib) => ({
    value: String(kib),
    label: kib === 0 ? t("downloads.upload_none") : `${formatBytes(kib * 1024, comma)}/s`,
  }));

  return (
    <>
      <GrupoDeConfiguracao>
        <LinhaChave
          title={t("downloads.enable")}
          description={t("downloads.enable_desc")}
          checked={enabled}
          onChange={(on) => void toggle(on)}
        />
        <LinhaDeConfiguracao title={t("downloads.privacy")} description={t("downloads.privacy_desc")} />
      </GrupoDeConfiguracao>

      {enabled && (
        <>
          <GrupoDeConfiguracao>
            {/* TODO(WEB-14b): "Pasta dos downloads" (abrir a pasta) espera uma chamada da ponte; hoje o núcleo não a expõe. */}
            <LinhaSelecao
              title={t("downloads.upload")}
              description={t("downloads.upload_desc")}
              options={limitOptions}
              value={String(limit)}
              onChange={(kib) => void saveAndRefresh(KEYS.downloadsUpload, kib)}
            />
          </GrupoDeConfiguracao>

          <GrupoDeConfiguracao title={t("downloads.sources")}>
            {/* TODO(WEB-14b): adicionar, atualizar e remover fontes ainda não existem na ponte; a lista é só leitura. */}
            {failed && <LinhaDeConfiguracao title={t("downloads.sources_error")} />}
            {!failed && sources === null && <LinhaDeConfiguracao title={t("downloads.sources_loading")} />}
            {sources?.sources.length === 0 && !sources.loading && (
              <LinhaDeConfiguracao title={t("downloads.sources_empty")} description={t("downloads.sources_desc", { count: 0 })} />
            )}
            {sources?.sources.map((source) => (
              <LinhaDeConfiguracao
                key={source.id}
                title={source.name}
                description={[
                  t("downloads.source_desc", {
                    count: source.itemCount,
                    date: formatDate(source.updatedAt, language),
                  }),
                  source.host,
                  source.dropped > 0 ? t("downloads.source_dropped", { n: source.dropped }) : "",
                ]
                  .filter(Boolean)
                  .join(" · ")}
              />
            ))}
            {sources?.loading && <LinhaDeConfiguracao title={t("downloads.source_loading")} />}
          </GrupoDeConfiguracao>

          <GrupoDeConfiguracao>
            <LinhaDeConfiguracao title={t("downloads.tool")} description={t("downloads.tool_desc")} />
          </GrupoDeConfiguracao>
        </>
      )}

      <Dialogo
        open={legalOpen}
        onClose={() => {
          setLegalOpen(false);
        }}
        size="m"
        closeLabel={t("common.close")}
        title={t("downloads.legal_title")}
        actions={
          <>
            <BotaoDesktop
              variant="secondary"
              size="sm"
              onClick={() => {
                setLegalOpen(false);
              }}
            >
              {t("common.cancel")}
            </BotaoDesktop>
            <BotaoDesktop variant="primary" size="sm" onClick={() => void accept()}>
              {t("downloads.legal_accept")}
            </BotaoDesktop>
          </>
        }
      >
        <p className={styles["legal"]}>{t("downloads.legal_text")}</p>
      </Dialogo>
    </>
  );
}
