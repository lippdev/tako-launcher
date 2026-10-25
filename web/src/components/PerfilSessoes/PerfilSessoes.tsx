import type { KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import { Abas, type Aba } from "../Abas/Abas";
import { CampoDeSelecao, type OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { BarrasDaSemana, Bloco, GrupoDeBlocos } from "../GrupoDeBlocos/GrupoDeBlocos";
import { Icon } from "../Icon/Icon";
import styles from "./PerfilSessoes.module.css";

export interface SessionStatusPlaying {
  kind: "playing";
  /** Rótulo exibido, ex.: "Jogando agora". */
  text: string;
}

export interface SessionStatusNormal {
  kind: "normal";
  /** Rótulo exibido, ex.: "Fechou normalmente" ou "Abriu e fechou". */
  text: string;
}

export interface SessionStatusError {
  kind: "error";
  /** Rótulo exibido, ex.: "Fechou com erro". */
  text: string;
  /** Rótulo da ação que abre o relatório, ex.: "Ver relatório"; sem ele não há ação. */
  linkText?: string;
}

export type SessionStatus = SessionStatusPlaying | SessionStatusNormal | SessionStatusError;

export interface HistorySession {
  id: string;
  /** Título do jogo. */
  game: string;
  /** URL da capa do jogo; sem ela, as iniciais. */
  coverSrc?: string;
  /** Texto alternativo da capa. */
  coverAlt?: string;
  /** Horário da sessão, ex.: "14:02 – agora" ou "20:40 – 22:59". */
  time: string;
  /** Duração formatada, ex.: "42 min" ou "2 h 19 min". */
  duration: string;
  /** Como a sessão terminou; sem dado (o núcleo não registra o fim), a linha mostra só o horário. */
  status?: SessionStatus;
  /** Callback para abrir o relatório de erro desta sessão (ErroDoJogo). */
  onOpenReport?: () => void;
  /** Callback ao clicar na linha da sessão. */
  onClick?: () => void;
}

export interface SessionGroup {
  id: string;
  /** Título do grupo, ex.: "Hoje", "Ontem", "20 out". Vazio, junto com o total, é um grupo sem cabeçalho. */
  title: string;
  /** Subtítulo ou data curta, ex.: "22 out". */
  subtitle?: string;
  /** Duração total jogada no dia, ex.: "1 h 54 min". */
  total: string;
  /** Sessões que ocorreram neste dia. */
  sessions: readonly HistorySession[];
}

export interface SessionsWeek {
  /** Rótulo da métrica da semana, ex.: "Esta semana". */
  label: string;
  /** Valor total em texto, ex.: "19 h". */
  total?: string;
  /** Nome acessível para o gráfico de barras. */
  ariaLabel: string;
  /** Altura de 0 a 1 por dia. */
  days: readonly { label: string; value: number }[];
}

export interface PerfilSessoesProps<TPeriod extends string = string, TGame extends string = string> {
  /** Bloco opcional com o tempo jogado na semana em barras. Se omitido, não renderiza. */
  week?: SessionsWeek;
  /** Seletor de período (ex.: Esta semana, Este mês, Tudo). Se omitido, não renderiza. */
  period?: {
    tabs: readonly Aba<TPeriod>[];
    value: TPeriod;
    onChange: (value: TPeriod) => void;
    label: string;
  };
  /** Filtro de jogo (ex.: Todos os jogos). Se omitido, não renderiza. */
  gameFilter?: {
    options: readonly OpcaoDeSelecao<TGame>[];
    value: TGame;
    onChange: (value: TGame) => void;
    label: string;
  };
  /** Resumo à direita da barra de filtros, ex.: "6 sessões · 6 h 37 min". */
  summary?: string;
  /** Nome da barra de filtros para leitores de tela. */
  toolbarLabel?: string;
  /** Grupos de sessões separados por dia. */
  groups?: readonly SessionGroup[];
  /** Mensagem exibida quando não houver sessões registradas. */
  emptyMessage?: string;
  /** Callback ao clicar em "Ver relatório" numa sessão com erro. */
  onOpenReport?: (session: HistorySession) => void;
  /** Callback ao clicar numa sessão. */
  onSessionClick?: (session: HistorySession) => void;
  className?: string;
}

/**
 * Componente PerfilSessoes (README do PerfilSessoes):
 * Exibe o histórico de sessões com filtros por período e jogo, resumo de horas e quantidade,
 * e a lista agrupada por dia. Se fornecido, exibe também as barras de tempo jogado na semana.
 * Sem dados, o bloco correspondente não é renderizado.
 */
export function PerfilSessoes<TPeriod extends string = string, TGame extends string = string>({
  week,
  period,
  gameFilter,
  summary,
  toolbarLabel,
  groups,
  emptyMessage,
  onOpenReport,
  onSessionClick,
  className,
}: PerfilSessoesProps<TPeriod, TGame>) {
  const hasWeek = Boolean(week && week.days.length > 0);
  const hasFilters = Boolean(period || gameFilter || summary);
  const hasGroups = Boolean(groups && groups.length > 0);
  const hasEmpty = Boolean(!hasGroups && emptyMessage);

  // Se nenhum dado foi fornecido, o componente não renderiza
  if (!hasWeek && !hasFilters && !hasGroups && !hasEmpty) {
    return null;
  }

  return (
    <div className={cx(styles["container"], className)}>
      {/* Bloco opcional com o tempo jogado na semana em barras */}
      {hasWeek && week && (        <section className={styles["semanaBloco"]} aria-label={week.ariaLabel}>
          <GrupoDeBlocos flat>
            {week.total && (
              <Bloco label={week.label} icon="clock">
                {week.total}
              </Bloco>
            )}
            <Bloco label={week.label}>
              <BarrasDaSemana days={[...week.days]} aria-label={week.ariaLabel} />
            </Bloco>
          </GrupoDeBlocos>
        </section>
      )}

      {/* Barra de filtros (período em abas segmentadas, seletor de jogo e resumo) */}
      {hasFilters && (
        <div className={styles["barraFiltros"]} role="toolbar" {...(toolbarLabel ? { "aria-label": toolbarLabel } : {})}>
          {period && (
            <Abas
              variant="segmentado"
              label={period.label}
              tabs={period.tabs}
              value={period.value}
              onChange={period.onChange}
            />
          )}
          {gameFilter && (
            <CampoDeSelecao
              label={gameFilter.label}
              options={gameFilter.options}
              value={gameFilter.value}
              onChange={gameFilter.onChange}
            />
          )}
          {summary && <span className={styles["resumo"]}>{summary}</span>}
        </div>
      )}

      {/* Grupos de sessões por dia */}
      {hasGroups &&
        groups?.map((group) => (
          <section key={group.id} className={styles["grupo"]} aria-label={`${group.title} ${group.subtitle ?? ""}`}>
            {/* Sem título nem total (a lista da Visão geral), o grupo é só as linhas. */}
            {(group.title || group.total) && (
              <div className={styles["grupoCabecalho"]}>
                <div className={styles["grupoTituloArea"]}>
                  <strong className={styles["grupoTitulo"]}>{group.title}</strong>
                  {group.subtitle && <span className={styles["grupoSubtitulo"]}>{group.subtitle}</span>}
                </div>
                <span className={styles["grupoTotal"]}>{group.total}</span>
              </div>
            )}

            <div className={styles["linhasLista"]}>
              {group.sessions.map((session) => {
                const handleRowClick = () => {
                  session.onClick?.();
                  onSessionClick?.(session);
                };

                const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
                  if (session.onClick || onSessionClick) {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      handleRowClick();
                    }
                  }
                };

                const clickable = Boolean(session.onClick || onSessionClick);
                const status = session.status;

                return (
                  <div
                    key={session.id}
                    className={cx(styles["linha"], clickable && styles["linhaClicavel"])}
                    onClick={clickable ? handleRowClick : undefined}
                    onKeyDown={clickable ? handleKeyDown : undefined}
                    tabIndex={clickable ? 0 : undefined}
                    role={clickable ? "button" : undefined}
                    aria-label={clickable ? `${session.game}, ${session.time}, ${session.duration}` : undefined}
                  >
                    {/* Capa miniatura */}
                    <div className={styles["capaMiniatura"]}>
                      {session.coverSrc ? (
                        <img
                          src={session.coverSrc}
                          alt={session.coverAlt ?? session.game}
                          className={styles["capaImagem"]}
                          width={40}
                          height={52}
                          decoding="async"
                          loading="lazy"
                        />
                      ) : (
                        <span className={styles["capaPlaceholder"]}>{session.game.slice(0, 2).toUpperCase()}</span>
                      )}
                    </div>

                    {/* Miolo com informações do jogo e da sessão */}
                    <div className={styles["info"]}>
                      <span className={styles["jogoTitulo"]}>{session.game}</span>
                      <div className={styles["detalhesLinha"]}>
                        <span>{session.time}</span>
                        {status && (
                          <span className={styles["separadorPonto"]} aria-hidden="true">
                            ·
                          </span>
                        )}

                        {status?.kind === "playing" && (
                          <span className={styles["statusTextoJogando"]}>
                            <i className={styles["pontoJogando"]} aria-hidden="true" />
                            {status.text}
                          </span>
                        )}

                        {status?.kind === "normal" && <span>{status.text}</span>}

                        {status?.kind === "error" && (
                          <>
                            <span className={styles["iconeAlerta"]} aria-hidden="true">
                              <Icon name="badge-alert" size="sm" />
                            </span>
                            <span>{status.text}</span>
                            {status.linkText && (session.onOpenReport || onOpenReport) && (
                              <>
                                <span className={styles["separadorPonto"]} aria-hidden="true">
                                  ·
                                </span>
                                <button
                                  type="button"
                                  className={styles["linkRelatorio"]}
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    session.onOpenReport?.();
                                    onOpenReport?.(session);
                                  }}
                                >
                                  {status.linkText}
                                </button>
                              </>
                            )}
                          </>
                        )}
                      </div>
                    </div>

                    {/* Duração formatada à direita */}
                    <span className={styles["duracao"]}>{session.duration}</span>
                  </div>
                );
              })}
            </div>
          </section>
        ))}

      {/* Estado vazio */}
      {hasEmpty && <div className={styles["vazio"]}>{emptyMessage}</div>}
    </div>
  );
}
