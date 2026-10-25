import { Dialogo } from "../Dialogo/Dialogo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Icon } from "../Icon/Icon";
import { cx } from "../../lib/cx";
import styles from "./AtualizacaoDoApp.module.css";

export type EstadoAtualizacao = "disponivel" | "baixando" | "pronta" | "erro";

export interface AtualizacaoDoAppLabels {
  /** Rótulo para fechar (acessibilidade / aria-label do botão ×). */
  close: string;
  /** Rótulo da ação de adiar a atualização (ex.: "Depois"). */
  later: string;
  /** Rótulo da seção de novidades (ex.: "Novidades"). */
  notes: string;
  /** Rótulo da ação de iniciar o download (ex.: "Baixar agora"). */
  download: string;
  /** Rótulo da ação de reiniciar imediatamente (ex.: "Reiniciar agora"). */
  restart: string;
  /** Rótulo da ação de tentar novamente em caso de erro (ex.: "Tentar de novo"). */
  retry: string;
  /** Título no estado "disponivel" (ex.: "Atualização disponível"). */
  availableTitle: string;
  /** Título no estado "baixando" (ex.: "Baixando atualização"). */
  downloadingTitle: string;
  /** Título no estado "pronta" (ex.: "Atualização pronta"). */
  readyTitle: string;
  /** Título no estado "erro" (ex.: "Não foi possível atualizar"). */
  errorTitle: string;
  /** Texto explicativo opcional exibido durante o download. */
  downloadingHelp?: string;
  /** Rótulo do link para as notas completas da release (ex.: "Ver todas as notas ↗"). */
  allNotes?: string;
}

export interface AtualizacaoDoAppProps {
  /** Controla se o diálogo está visível. Padrão: true. */
  open?: boolean;
  /** Estado atual do fluxo de atualização. */
  state: EstadoAtualizacao;
  /** Versão nova do aplicativo (ex.: "0.2.1" ou "TAKO 0.2.1"). */
  version: string;
  /** Subtítulo contextual opcional (ex.: "TAKO 0.2.1 · reinicia em uns 5 s"). */
  subtitle?: string;
  /** Lista de itens de texto com as novidades da versão. */
  releaseNotes?: readonly string[];
  /** Progresso atual de 0 a 100 (usado no estado "baixando"). */
  progress?: number;
  /** Texto com métricas de download (ex.: "45% · 12 MB de 27 MB"). */
  progressText?: string;
  /** Mensagem descritiva de erro (usada no estado "erro"). */
  errorMessage?: string;
  /** Letra de ajuda sob as novidades (estados "disponivel" e "pronta"): a regra do "Depois", o jogo aberto. */
  note?: string;
  /** Rótulos traduzidos dos textos e botões vindos dos catálogos de tradução. */
  labels: AtualizacaoDoAppLabels;
  /** Callback acionado ao fechar o diálogo. */
  onClose: () => void;
  /** Callback acionado ao clicar em "Depois". Se omitido, usa `onClose`. */
  onLater?: () => void;
  /** Callback acionado ao clicar em "Baixar". */
  onDownload?: () => void;
  /** Callback acionado ao clicar em "Reiniciar agora". */
  onRestart?: () => void;
  /** Callback acionado ao clicar em "Tentar de novo". */
  onRetry?: () => void;
  /** Callback do link para as notas completas; sem ele (ou sem `labels.allNotes`) o link não aparece. */
  onViewNotes?: () => void;
  className?: string;
}

/**
 * Diálogo de atualização do aplicativo Tako (README da AtualizacaoDoApp):
 * Suporta os quatro estados:
 * - "disponivel": exibe versão e novidades em texto, com botão para baixar ou adiar.
 * - "baixando": exibe barra de progresso com `scaleX` de 0 a 100 e texto contextual.
 * - "pronta": diálogo padrão da prévia com as novidades e ação "Reiniciar agora".
 * - "erro": exibe mensagem de falha e botão "Tentar de novo".
 */
export function AtualizacaoDoApp({
  open = true,
  state,
  version,
  subtitle,
  releaseNotes,
  progress = 0,
  progressText,
  errorMessage,
  note,
  labels,
  onClose,
  onLater,
  onDownload,
  onRestart,
  onRetry,
  onViewNotes,
  className,
}: AtualizacaoDoAppProps) {
  const fecharOuDepois = onLater ?? onClose;

  const titulo = (() => {
    switch (state) {
      case "disponivel":
        return labels.availableTitle;
      case "baixando":
        return labels.downloadingTitle;
      case "pronta":
        return labels.readyTitle;
      case "erro":
        return labels.errorTitle;
    }
  })();

  const subtituloExibicao =
    subtitle ??
    (state === "baixando" && progressText ? progressText : version);

  const acoes = (() => {
    switch (state) {
      case "disponivel":
        return (
          <>
            <BotaoDesktop variant="outlined" onClick={fecharOuDepois}>
              {labels.later}
            </BotaoDesktop>
            <BotaoDesktop
              variant="primary"
              size="sm"
              icon="download"
              onClick={onDownload}
            >
              {labels.download}
            </BotaoDesktop>
          </>
        );
      case "baixando":
        return (
          <BotaoDesktop variant="outlined" onClick={fecharOuDepois}>
            {labels.later}
          </BotaoDesktop>
        );
      case "pronta":
        return (
          <>
            <BotaoDesktop variant="outlined" onClick={fecharOuDepois}>
              {labels.later}
            </BotaoDesktop>
            <BotaoDesktop
              variant="primary"
              size="sm"
              icon="refresh"
              onClick={onRestart}
            >
              {labels.restart}
            </BotaoDesktop>
          </>
        );
      case "erro":
        return (
          <>
            <BotaoDesktop variant="outlined" onClick={onClose}>
              {labels.close}
            </BotaoDesktop>
            <BotaoDesktop
              variant="primary"
              size="sm"
              icon="refresh"
              onClick={onRetry}
            >
              {labels.retry}
            </BotaoDesktop>
          </>
        );
    }
  })();

  const progressoPercentual = Math.min(100, Math.max(0, progress));
  const progressoEscala = progressoPercentual / 100;

  return (
    <Dialogo
      open={open}
      onClose={onClose}
      size="padrao"
      title={titulo}
      closeLabel={labels.close}
      className={cx(styles["dialogo"], className)}
      actions={acoes}
    >
      <div className={styles["cabecalho"]}>
        <div className={styles["titulos"]}>
          <h3 className={styles["titulo"]}>{titulo}</h3>
          {subtituloExibicao && (
            <p className={styles["subtitulo"]}>{subtituloExibicao}</p>
          )}
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

      {/* Estados "disponivel" e "pronta": exibem lista de novidades quando houver */}
      {(state === "disponivel" || state === "pronta") &&
        releaseNotes &&
        releaseNotes.length > 0 && (
          <div>
            <h4 className={styles["novidadesTitulo"]}>{labels.notes}</h4>
            <ul className={styles["listaNovidades"]}>
              {releaseNotes.map((item, index) => (
                <li key={index}>{item}</li>
              ))}
            </ul>
          </div>
        )}

      {(state === "disponivel" || state === "pronta") && note && (
        <p className={styles["ajudaTexto"]}>{note}</p>
      )}

      {/* Estado "baixando": barra de progresso com transform: scaleX */}
      {state === "baixando" && (
        <div>
          <div
            className={styles["trilhoProgresso"]}
            role="progressbar"
            aria-valuenow={Math.round(progressoPercentual)}
            aria-valuemin={0}
            aria-valuemax={100}
          >
            <span
              className={styles["barraProgresso"]}
              style={{ transform: `scaleX(${progressoEscala})` }}
            />
          </div>
          {labels.downloadingHelp && (
            <p className={styles["ajudaTexto"]}>{labels.downloadingHelp}</p>
          )}
        </div>
      )}

      {/* Estado "erro": mensagem descritiva de erro */}
      {state === "erro" && errorMessage && (
        <p className={styles["mensagemErro"]}>{errorMessage}</p>
      )}

      {onViewNotes && labels.allNotes && state !== "erro" && (
        <button type="button" className={styles["linkNotas"]} onClick={onViewNotes}>
          {labels.allNotes}
        </button>
      )}
    </Dialogo>
  );
}
