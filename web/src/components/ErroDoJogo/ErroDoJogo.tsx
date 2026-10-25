import { useState } from "react";
import { Dialogo } from "../Dialogo/Dialogo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Icon } from "../Icon/Icon";
import { cx } from "../../lib/cx";
import styles from "./ErroDoJogo.module.css";

export interface ErroDoJogoRotulos {
  /** Rótulo do botão fechar (acessibilidade / aria-label). */
  fechar: string;
  /** Rótulo da linha de detalhes técnicos (ex.: "Detalhes técnicos"). */
  detalhesTecnicos: string;
  /** Rótulo da ação de salvar relatório (ex.: "Salvar relatório"). */
  salvarRelatorio: string;
  /** Rótulo da ação de reparar arquivos (ex.: "Reparar arquivos"). */
  reparar: string;
  /** Rótulo da ação de tentar de novo (ex.: "Tentar de novo"). */
  tentarDeNovo: string;
  /** Sufixo opcional para o título do jogo (ex.: "fechou sozinho"). */
  sufixoTitulo?: string;
}

export interface ErroDoJogoProps {
  /** Controla se o diálogo está visível. Padrão: true. */
  open?: boolean;
  /** Título do jogo (ou título completo do diálogo). */
  tituloDoJogo: string;
  /** Subtítulo com momento e duração da sessão (ex.: "Hoje, 14:02 · 12 segundos depois de abrir"). */
  subtitulo?: string;
  /** URL ou src da arte de capa do jogo. Se omitida, exibe um ícone de alerta. */
  capaUrl?: string;
  /** Código de saída (ex.: "0xC0000005" ou número). Sem código (`null` ou `undefined`), não mostra a linha. */
  codigoDeSaida?: string | number | null;
  /** Mensagem descritiva da provável causa do encerramento. */
  mensagem: string;
  /** Se o jogo suporta verificação/reparo. Sem reparo (`false` ou omitido), não mostra o botão. */
  podeReparar?: boolean;
  /** Rótulos de texto dos botões e cabeçalhos vindos dos catálogos de tradução. */
  rotulos: ErroDoJogoRotulos;
  /** Callback acionado ao clicar em "Tentar de novo". */
  onTentarDeNovo: () => void;
  /** Callback acionado ao clicar em "Reparar arquivos". */
  onReparar?: () => void;
  /** Callback acionado ao clicar em "Salvar relatório". Sem ele (a ponte ainda não salva o relatório) a ação não aparece. */
  onSalvarRelatorio?: () => void;
  /** Callback acionado ao fechar o diálogo. */
  onFechar: () => void;
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
  tituloDoJogo,
  subtitulo,
  capaUrl,
  codigoDeSaida,
  mensagem,
  podeReparar = false,
  rotulos,
  onTentarDeNovo,
  onReparar,
  onSalvarRelatorio,
  onFechar,
  className,
}: ErroDoJogoProps) {
  const [expandido, setExpandido] = useState(false);
  const codigoTexto = formataCodigo(codigoDeSaida);
  const tituloCompleto = rotulos.sufixoTitulo
    ? `${tituloDoJogo} ${rotulos.sufixoTitulo}`
    : tituloDoJogo;

  return (
    <Dialogo
      open={open}
      onClose={onFechar}
      size="m"
      title={tituloCompleto}
      closeLabel={rotulos.fechar}
      className={cx(styles["dialogo"], className)}
      actions={
        <>
          {onSalvarRelatorio && (
            <button
              type="button"
              className={styles["salvarRelatorio"]}
              onClick={onSalvarRelatorio}
            >
              {rotulos.salvarRelatorio}
            </button>
          )}
          {podeReparar && (
            <BotaoDesktop variant="outlined" onClick={onReparar}>
              {rotulos.reparar}
            </BotaoDesktop>
          )}
          <BotaoDesktop
            variant="primary"
            size="sm"
            icon="refresh"
            onClick={onTentarDeNovo}
          >
            {rotulos.tentarDeNovo}
          </BotaoDesktop>
        </>
      }
    >
      <div className={styles["cabecalho"]}>
        {capaUrl ? (
          <span className={styles["capaWrap"]}>
            <img src={capaUrl} alt="" className={styles["capa"]} />
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
          {subtitulo && <p className={styles["subtitulo"]}>{subtitulo}</p>}
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

      <p className={styles["mensagem"]}>{mensagem}</p>

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
            <span>{rotulos.detalhesTecnicos}</span>
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
