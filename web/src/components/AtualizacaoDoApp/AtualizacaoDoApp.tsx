import { Dialogo } from "../Dialogo/Dialogo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Icon } from "../Icon/Icon";
import { cx } from "../../lib/cx";
import styles from "./AtualizacaoDoApp.module.css";

export type EstadoAtualizacao = "disponivel" | "baixando" | "pronta" | "erro";

export interface AtualizacaoDoAppRotulos {
  /** Rótulo para fechar (acessibilidade / aria-label do botão ×). */
  fechar: string;
  /** Rótulo da ação de adiar a atualização (ex.: "Depois"). */
  depois: string;
  /** Rótulo da seção de novidades (ex.: "Novidades"). */
  novidades: string;
  /** Rótulo da ação de iniciar o download (ex.: "Baixar agora"). */
  baixar: string;
  /** Rótulo da ação de reiniciar imediatamente (ex.: "Reiniciar agora"). */
  reiniciar: string;
  /** Rótulo da ação de tentar novamente em caso de erro (ex.: "Tentar de novo"). */
  tentarDeNovo: string;
  /** Título no estado "disponivel" (ex.: "Atualização disponível"). */
  tituloDisponivel: string;
  /** Título no estado "baixando" (ex.: "Baixando atualização"). */
  tituloBaixando: string;
  /** Título no estado "pronta" (ex.: "Atualização pronta"). */
  tituloPronta: string;
  /** Título no estado "erro" (ex.: "Não foi possível atualizar"). */
  tituloErro: string;
  /** Texto explicativo opcional exibido durante o download. */
  ajudaBaixando?: string;
  /** Rótulo do link para as notas completas da release (ex.: "Ver todas as notas ↗"). */
  todasAsNotas?: string;
}

export interface AtualizacaoDoAppProps {
  /** Controla se o diálogo está visível. Padrão: true. */
  open?: boolean;
  /** Estado atual do fluxo de atualização. */
  estado: EstadoAtualizacao;
  /** Versão nova do aplicativo (ex.: "0.2.1" ou "TAKO 0.2.1"). */
  versao: string;
  /** Subtítulo contextual opcional (ex.: "TAKO 0.2.1 · reinicia em uns 5 s"). */
  subtitulo?: string;
  /** Lista de itens de texto com as novidades da versão. */
  novidades?: readonly string[];
  /** Progresso atual de 0 a 100 (usado no estado "baixando"). */
  progresso?: number;
  /** Texto com métricas de download (ex.: "45% · 12 MB de 27 MB"). */
  progressoTexto?: string;
  /** Mensagem descritiva de erro (usada no estado "erro"). */
  mensagemErro?: string;
  /** Letra de ajuda sob as novidades (estados "disponivel" e "pronta"): a regra do "Depois", o jogo aberto. */
  observacao?: string;
  /** Rótulos traduzidos dos textos e botões vindos dos catálogos de tradução. */
  rotulos: AtualizacaoDoAppRotulos;
  /** Callback acionado ao fechar o diálogo. */
  onFechar: () => void;
  /** Callback acionado ao clicar em "Depois". Se omitido, usa `onFechar`. */
  onDepois?: () => void;
  /** Callback acionado ao clicar em "Baixar". */
  onBaixar?: () => void;
  /** Callback acionado ao clicar em "Reiniciar agora". */
  onReiniciar?: () => void;
  /** Callback acionado ao clicar em "Tentar de novo". */
  onTentarDeNovo?: () => void;
  /** Callback do link para as notas completas; sem ele (ou sem `rotulos.todasAsNotas`) o link não aparece. */
  onVerNotas?: () => void;
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
  estado,
  versao,
  subtitulo,
  novidades,
  progresso = 0,
  progressoTexto,
  mensagemErro,
  observacao,
  rotulos,
  onFechar,
  onDepois,
  onBaixar,
  onReiniciar,
  onTentarDeNovo,
  onVerNotas,
  className,
}: AtualizacaoDoAppProps) {
  const fecharOuDepois = onDepois ?? onFechar;

  const titulo = (() => {
    switch (estado) {
      case "disponivel":
        return rotulos.tituloDisponivel;
      case "baixando":
        return rotulos.tituloBaixando;
      case "pronta":
        return rotulos.tituloPronta;
      case "erro":
        return rotulos.tituloErro;
    }
  })();

  const subtituloExibicao =
    subtitulo ??
    (estado === "baixando" && progressoTexto ? progressoTexto : versao);

  const acoes = (() => {
    switch (estado) {
      case "disponivel":
        return (
          <>
            <BotaoDesktop variant="outlined" onClick={fecharOuDepois}>
              {rotulos.depois}
            </BotaoDesktop>
            <BotaoDesktop
              variant="primary"
              size="sm"
              icon="download"
              onClick={onBaixar}
            >
              {rotulos.baixar}
            </BotaoDesktop>
          </>
        );
      case "baixando":
        return (
          <BotaoDesktop variant="outlined" onClick={fecharOuDepois}>
            {rotulos.depois}
          </BotaoDesktop>
        );
      case "pronta":
        return (
          <>
            <BotaoDesktop variant="outlined" onClick={fecharOuDepois}>
              {rotulos.depois}
            </BotaoDesktop>
            <BotaoDesktop
              variant="primary"
              size="sm"
              icon="refresh"
              onClick={onReiniciar}
            >
              {rotulos.reiniciar}
            </BotaoDesktop>
          </>
        );
      case "erro":
        return (
          <>
            <BotaoDesktop variant="outlined" onClick={onFechar}>
              {rotulos.fechar}
            </BotaoDesktop>
            <BotaoDesktop
              variant="primary"
              size="sm"
              icon="refresh"
              onClick={onTentarDeNovo}
            >
              {rotulos.tentarDeNovo}
            </BotaoDesktop>
          </>
        );
    }
  })();

  const progressoPercentual = Math.min(100, Math.max(0, progresso));
  const progressoEscala = progressoPercentual / 100;

  return (
    <Dialogo
      open={open}
      onClose={onFechar}
      size="padrao"
      title={titulo}
      closeLabel={rotulos.fechar}
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
          aria-label={rotulos.fechar}
          data-autofocus-last=""
          onClick={onFechar}
        >
          <Icon name="x" />
        </button>
      </div>

      {/* Estados "disponivel" e "pronta": exibem lista de novidades quando houver */}
      {(estado === "disponivel" || estado === "pronta") &&
        novidades &&
        novidades.length > 0 && (
          <div>
            <h4 className={styles["novidadesTitulo"]}>{rotulos.novidades}</h4>
            <ul className={styles["listaNovidades"]}>
              {novidades.map((item, index) => (
                <li key={index}>{item}</li>
              ))}
            </ul>
          </div>
        )}

      {(estado === "disponivel" || estado === "pronta") && observacao && (
        <p className={styles["ajudaTexto"]}>{observacao}</p>
      )}

      {/* Estado "baixando": barra de progresso com transform: scaleX */}
      {estado === "baixando" && (
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
          {rotulos.ajudaBaixando && (
            <p className={styles["ajudaTexto"]}>{rotulos.ajudaBaixando}</p>
          )}
        </div>
      )}

      {/* Estado "erro": mensagem descritiva de erro */}
      {estado === "erro" && mensagemErro && (
        <p className={styles["mensagemErro"]}>{mensagemErro}</p>
      )}

      {onVerNotas && rotulos.todasAsNotas && estado !== "erro" && (
        <button type="button" className={styles["linkNotas"]} onClick={onVerNotas}>
          {rotulos.todasAsNotas}
        </button>
      )}
    </Dialogo>
  );
}
