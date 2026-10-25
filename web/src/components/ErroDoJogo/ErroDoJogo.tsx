import { useState } from "react";
import { Dialogo } from "../Dialogo/Dialogo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Icon } from "../Icon/Icon";
import { cx } from "../../lib/cx";
import styles from "./ErroDoJogo.module.css";

export interface ErroDoJogoLabels {
  /** Rótulo do botão fechar (acessibilidade / aria-label). */
  close: string;
  /** Rótulo da linha de detalhes técnicos (ex.: "Detalhes técnicos"). */
  technicalDetails: string;
  /** Rótulo da ação de salvar relatório (ex.: "Salvar relatório"). */
  saveReport: string;
  /** Rótulo da ação de reparar arquivos (ex.: "Reparar arquivos"). */
  repair: string;
  /** Rótulo da ação de tentar de novo (ex.: "Tentar de novo"). */
  retry: string;
  /** Sufixo opcional para o título do jogo (ex.: "fechou sozinho"). */
  titleSuffix?: string;
}

export interface ErroDoJogoProps {
  /** Controla se o diálogo está visível. Padrão: true. */
  open?: boolean;
  /** Título do jogo (ou título completo do diálogo). */
  gameTitle: string;
  /** Subtítulo com momento e duração da sessão (ex.: "Hoje, 14:02 · 12 segundos depois de abrir"). */
  subtitle?: string;
  /** URL ou src da arte de capa do jogo. Se omitida, exibe um ícone de alerta. */
  coverUrl?: string;
  /** Código de saída (ex.: "0xC0000005" ou número). Sem código (`null` ou `undefined`), não mostra a linha. */
  exitCode?: string | number | null;
  /** Mensagem descritiva da provável causa do encerramento. */
  message: string;
  /** Se o jogo suporta verificação/reparo. Sem reparo (`false` ou omitido), não mostra o botão. */
  canRepair?: boolean;
  /** Rótulos de texto dos botões e cabeçalhos vindos dos catálogos de tradução. */
  labels: ErroDoJogoLabels;
  /** Callback acionado ao clicar em "Tentar de novo". */
  onRetry: () => void;
  /** Callback acionado ao clicar em "Reparar arquivos". */
  onRepair?: () => void;
  /** Callback acionado ao clicar em "Salvar relatório". Sem ele (a ponte ainda não salva o relatório) a ação não aparece. */
  onSaveReport?: () => void;
  /** Callback acionado ao fechar o diálogo. */
  onClose: () => void;
  className?: string;
}

function formataCodigo(codigo: string | number | null | undefined): string | null {
  if (codigo == null) return null;
  if (typeof codigo === "number") {
    if (codigo < 0 || codigo > 0xffff) {
      return `0x${(codigo >>> 0).toString(16).toUpperCase()}`;
    }
    return String(codigo);
  }
  return codigo.trim().length > 0 ? codigo : null;
}

/**
 * Diálogo de encerramento inesperado do jogo (SESSION-003):
 * Exibe capa com indicador de atenção, título, tempo da sessão, mensagem de diagnóstico,
 * detalhes técnicos com código de saída (quando houver) e opções de reparo, relatório e reinício.
 */
export function ErroDoJogo({
  open = true,
  gameTitle,
  subtitle,
  coverUrl,
  exitCode,
  message,
  canRepair = false,
  labels,
  onRetry,
  onRepair,
  onSaveReport,
  onClose,
  className,
}: ErroDoJogoProps) {
  const [expandido, setExpandido] = useState(false);
  const codigoTexto = formataCodigo(exitCode);
  const tituloCompleto = labels.titleSuffix
    ? `${gameTitle} ${labels.titleSuffix}`
    : gameTitle;

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      size="m"
      title={tituloCompleto}
      closeLabel={labels.close}
      className={cx(styles["dialogo"], className)}
      actions={
        <>
          {onSaveReport && (
            <button
              type="button"
              className={styles["salvarRelatorio"]}
              onClick={onSaveReport}
            >
              {labels.saveReport}
            </button>
          )}
          {canRepair && (
            <BotaoDesktop variant="outlined" onClick={onRepair}>
              {labels.repair}
            </BotaoDesktop>
          )}
          <BotaoDesktop
            variant="primary"
            size="sm"
            icon="refresh"
            onClick={onRetry}
          >
            {labels.retry}
          </BotaoDesktop>
        </>
      }
    >
      <div className={styles["cabecalho"]}>
        {coverUrl ? (
          <span className={styles["capaWrap"]}>
            <img src={coverUrl} alt="" className={styles["capa"]} />
            <i className={styles["pontoAlerta"]} aria-hidden="true" />
          </span>
        ) : (
          <span className={styles["capaWrap"]}>
            <span className={styles["capaVazia"]}>
              <Icon name="badge-alert" size="md" />
            </span>
            <i className={styles["pontoAlerta"]} aria-hidden="true" />
          </span>
        )}
        <div className={styles["titulos"]}>
          <h3 className={styles["titulo"]}>{tituloCompleto}</h3>
          {subtitle && <p className={styles["subtitulo"]}>{subtitle}</p>}
        </div>
        <button
          type="button"
          className={styles["botaoFechar"]}
          aria-label={labels.close}
          data-autofocus-last=""
          onClick={onClose}
        >
          <Icon name="x" />
        </button>
      </div>

      <p className={styles["mensagem"]}>{message}</p>

      {codigoTexto != null && (
        <>
          <button
            type="button"
            className={styles["detalhesTecnicos"]}
            onClick={() => {
              setExpandido((prev) => !prev);
            }}
            aria-expanded={expandido}
          >
            <span>{labels.technicalDetails}</span>
            <strong className={styles["codigoWrap"]}>
              <span>{codigoTexto}</span>
              <span
                className={cx(
                  styles["chevron"],
                  expandido && styles["chevronAberto"],
                )}
              >
                <Icon name="chevron-right" size="sm" />
              </span>
            </strong>
          </button>
          {expandido && (
            <div className={styles["detalhesCorpo"]}>
              <code>{codigoTexto}</code>
            </div>
          )}
        </>
      )}
    </Dialogo>
  );
}
