import type { KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import { Abas, type Aba } from "../Abas/Abas";
import { CampoDeSelecao, type OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { BarrasDaSemana, Bloco, GrupoDeBlocos } from "../GrupoDeBlocos/GrupoDeBlocos";
import { Icon } from "../Icon/Icon";
import styles from "./PerfilSessoes.module.css";

export interface StatusSessaoJogando {
  tipo: "jogando";
  /** Rótulo exibido, ex.: "Jogando agora". */
  texto: string;
}

export interface StatusSessaoNormal {
  tipo: "normal";
  /** Rótulo exibido, ex.: "Fechou normalmente". */
  texto: string;
}

export interface StatusSessaoErro {
  tipo: "erro";
  /** Rótulo exibido, ex.: "Fechou com erro". */
  texto: string;
  /** Rótulo da ação que abre o relatório, ex.: "Ver relatório". */
  linkTexto?: string;
}

export type StatusSessao = StatusSessaoJogando | StatusSessaoNormal | StatusSessaoErro;

export interface SessaoHistorico {
  id: string;
  /** Título do jogo. */
  jogo: string;
  /** URL direta ou ID da capa do jogo. */
  capaSrc?: string;
  /** ID do protocolo tako-img (ex.: steam:570). */
  capaId?: string;
  /** Texto alternativo da capa. */
  capaAlt?: string;
  /** Horário da sessão, ex.: "14:02 – agora" ou "20:40 – 22:59". */
  horario: string;
  /** Duração formatada, ex.: "42 min" ou "2 h 19 min". */
  duracao: string;
  /** Estado de encerramento da sessão ou em andamento. */
  status: StatusSessao;
  /** Callback para abrir o relatório de erro desta sessão (ErroDoJogo). */
  onVerRelatorio?: () => void;
  /** Callback ao clicar na linha da sessão. */
  onClick?: () => void;
}

export interface GrupoDeSessoes {
  id: string;
  /** Título do grupo, ex.: "Hoje", "Ontem", "20 out". */
  titulo: string;
  /** Subtítulo ou data curta, ex.: "22 out". */
  subtitulo?: string;
  /** Duração total jogada no dia, ex.: "1 h 54 min". */
  total: string;
  /** Sessões que ocorreram neste dia. */
  sessoes: readonly SessaoHistorico[];
}

export interface SemanaSessoes {
  /** Rótulo da métrica da semana, ex.: "Esta semana". */
  rotulo: string;
  /** Valor total em texto, ex.: "19 h". */
  total?: string;
  /** Nome acessível para o gráfico de barras. */
  ariaLabel: string;
  /** Altura de 0 a 1 por dia. */
  dias: readonly { label: string; value: number }[];
}

export interface PerfilSessoesProps<TPeriodo extends string = string, TJogo extends string = string> {
  /** Bloco opcional com o tempo jogado na semana em barras. Se omitido, não renderiza. */
  semana?: SemanaSessoes;
  /** Seletor de período (ex.: Esta semana, Este mês, Tudo). Se omitido, não renderiza. */
  periodo?: {
    tabs: readonly Aba<TPeriodo>[];
    value: TPeriodo;
    onChange: (value: TPeriodo) => void;
    label: string;
  };
  /** Filtro de jogo (ex.: Todos os jogos). Se omitido, não renderiza. */
  filtroJogo?: {
    options: readonly OpcaoDeSelecao<TJogo>[];
    value: TJogo;
    onChange: (value: TJogo) => void;
    label: string;
  };
  /** Resumo à direita da barra de filtros, ex.: "6 sessões · 6 h 37 min". */
  resumo?: string;
  /** Grupos de sessões separados por dia. */
  grupos?: readonly GrupoDeSessoes[];
  /** Mensagem exibida quando não houver sessões registradas. */
  mensagemVazio?: string;
  /** Callback ao clicar em "Ver relatório" numa sessão com erro. */
  onVerRelatorio?: (sessao: SessaoHistorico) => void;
  /** Callback ao clicar numa sessão. */
  onSessaoClick?: (sessao: SessaoHistorico) => void;
  className?: string;
}

/**
 * Componente PerfilSessoes (README do PerfilSessoes):
 * Exibe o histórico de sessões com filtros por período e jogo, resumo de horas e quantidade,
 * e a lista agrupada por dia. Se fornecido, exibe também as barras de tempo jogado na semana.
 * Sem dados, o bloco correspondente não é renderizado.
 */
export function PerfilSessoes<TPeriodo extends string = string, TJogo extends string = string>({
  semana,
  periodo,
  filtroJogo,
  resumo,
  grupos,
  mensagemVazio,
  onVerRelatorio,
  onSessaoClick,
  className,
}: PerfilSessoesProps<TPeriodo, TJogo>) {
  const temSemana = Boolean(semana && semana.dias.length > 0);
  const temFiltros = Boolean(periodo || filtroJogo || resumo);
  const temGrupos = Boolean(grupos && grupos.length > 0);
  const temVazio = Boolean(!temGrupos && mensagemVazio);

  // Se nenhum dado foi fornecido, o componente não renderiza
  if (!temSemana && !temFiltros && !temGrupos && !temVazio) {
    return null;
  }

  return (
    <div className={cx(styles["container"], className)}>
      {/* Bloco opcional com o tempo jogado na semana em barras */}
      {temSemana && semana && (
        <section className={styles["semanaBloco"]} aria-label={semana.ariaLabel}>
          <GrupoDeBlocos flat>
            {semana.total && (
              <Bloco label={semana.rotulo} icon="clock">
                {semana.total}
              </Bloco>
            )}
            <Bloco label={semana.rotulo}>
              <BarrasDaSemana days={[...semana.dias]} aria-label={semana.ariaLabel} />
            </Bloco>
          </GrupoDeBlocos>
        </section>
      )}

      {/* Barra de filtros (período em abas segmentadas, seletor de jogo e resumo) */}
      {temFiltros && (
        <div className={styles["barraFiltros"]} role="toolbar" aria-label="Filtros de sessões">
          {periodo && (
            <Abas
              variant="segmentado"
              label={periodo.label}
              tabs={periodo.tabs}
              value={periodo.value}
              onChange={periodo.onChange}
            />
          )}
          {filtroJogo && (
            <CampoDeSelecao
              label={filtroJogo.label}
              options={filtroJogo.options}
              value={filtroJogo.value}
              onChange={filtroJogo.onChange}
            />
          )}
          {resumo && <span className={styles["resumo"]}>{resumo}</span>}
        </div>
      )}

      {/* Grupos de sessões por dia */}
      {temGrupos &&
        grupos?.map((grupo) => (
          <section key={grupo.id} className={styles["grupo"]} aria-label={`${grupo.titulo} ${grupo.subtitulo ?? ""}`}>
            <div className={styles["grupoCabecalho"]}>
              <div className={styles["grupoTituloArea"]}>
                <strong className={styles["grupoTitulo"]}>{grupo.titulo}</strong>
                {grupo.subtitulo && <span className={styles["grupoSubtitulo"]}>{grupo.subtitulo}</span>}
              </div>
              <span className={styles["grupoTotal"]}>{grupo.total}</span>
            </div>

            <div className={styles["linhasLista"]}>
              {grupo.sessoes.map((sessao) => {
                const handleLinhaClick = () => {
                  sessao.onClick?.();
                  onSessaoClick?.(sessao);
                };

                const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
                  if (sessao.onClick || onSessaoClick) {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      handleLinhaClick();
                    }
                  }
                };

                const clicavel = Boolean(sessao.onClick || onSessaoClick);

                return (
                  <div
                    key={sessao.id}
                    className={cx(styles["linha"], clicavel && styles["linhaClicavel"])}
                    onClick={clicavel ? handleLinhaClick : undefined}
                    onKeyDown={clicavel ? handleKeyDown : undefined}
                    tabIndex={clicavel ? 0 : undefined}
                    role={clicavel ? "button" : undefined}
                    aria-label={clicavel ? `${sessao.jogo}, ${sessao.horario}, ${sessao.duracao}` : undefined}
                  >
                    {/* Capa miniatura */}
                    <div className={styles["capaMiniatura"]}>
                      {sessao.capaSrc ? (
                        <img
                          src={sessao.capaSrc}
                          alt={sessao.capaAlt ?? sessao.jogo}
                          className={styles["capaImagem"]}
                          loading="lazy"
                        />
                      ) : (
                        <span className={styles["capaPlaceholder"]}>{sessao.jogo.slice(0, 2).toUpperCase()}</span>
                      )}
                    </div>

                    {/* Miolo com informações do jogo e da sessão */}
                    <div className={styles["info"]}>
                      <span className={styles["jogoTitulo"]}>{sessao.jogo}</span>
                      <div className={styles["detalhesLinha"]}>
                        <span>{sessao.horario}</span>
                        <span className={styles["separadorPonto"]} aria-hidden="true">
                          ·
                        </span>

                        {sessao.status.tipo === "jogando" && (
                          <span className={styles["statusTextoJogando"]}>
                            <i className={styles["pontoJogando"]} aria-hidden="true" />
                            {sessao.status.texto}
                          </span>
                        )}

                        {sessao.status.tipo === "normal" && <span>{sessao.status.texto}</span>}

                        {sessao.status.tipo === "erro" && (
                          <>
                            <span className={styles["iconeAlerta"]} aria-hidden="true">
                              <Icon name="badge-alert" size="sm" />
                            </span>
                            <span>{sessao.status.texto}</span>
                            {(sessao.status.linkTexto || sessao.onVerRelatorio || onVerRelatorio) && (
                              <>
                                <span className={styles["separadorPonto"]} aria-hidden="true">
                                  ·
                                </span>
                                <button
                                  type="button"
                                  className={styles["linkRelatorio"]}
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    sessao.onVerRelatorio?.();
                                    onVerRelatorio?.(sessao);
                                  }}
                                >
                                  {sessao.status.linkTexto ?? "Ver relatório"}
                                </button>
                              </>
                            )}
                          </>
                        )}
                      </div>
                    </div>

                    {/* Duração formatada à direita */}
                    <span className={styles["duracao"]}>{sessao.duracao}</span>
                  </div>
                );
              })}
            </div>
          </section>
        ))}

      {/* Estado vazio */}
      {temVazio && <div className={styles["vazio"]}>{mensagemVazio}</div>}
    </div>
  );
}
