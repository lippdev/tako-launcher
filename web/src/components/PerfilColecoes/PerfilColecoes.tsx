import type { KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import { Abas, type Aba } from "../Abas/Abas";
import { Icon } from "../Icon/Icon";
import styles from "./PerfilColecoes.module.css";

export interface ColecaoCapa {
  /** URL direta da capa do jogo. */
  src?: string;
  /** Texto alternativo para acessibilidade. */
  alt?: string;
}

export interface ColecaoItem {
  id: string;
  /** Nome da coleção, ex.: "Roguelikes" ou "Jogados este mês". */
  nome: string;
  /** Quantidade e instalados, ex.: "6 jogos · 4 instalados". */
  detalhe?: string;
  /** Indica se é uma coleção automática por regra (exibe o selo com ponto). */
  automatica?: boolean;
  /** Até 4 capas para compor o mosaico 2×2. */
  capas?: readonly (string | ColecaoCapa)[];
  /** Callback ao clicar neste cartão de coleção. */
  onClick?: () => void;
}

export interface PerfilColecoesProps<TOrdem extends string = string> {
  /** Resumo à esquerda, ex.: "4 coleções · 26 jogos". */
  resumo?: string;
  /** Seletor segmentado de ordenação à direita, ex.: Recentes · A–Z · Mais jogos. */
  ordem?: {
    tabs: readonly Aba<TOrdem>[];
    value: TOrdem;
    onChange: (value: TOrdem) => void;
    label: string;
  };
  /** Lista de coleções exibidas na grade de 4 colunas. */
  colecoes?: readonly ColecaoItem[];
  /** Rótulo do selo para coleções automáticas, ex.: "Automática". */
  seloAutomaticaLabel?: string;
  /** Ação de "Nova coleção" no cartão tracejado. Se omitido, não renderiza. */
  novaColecao?: {
    label: string;
    onClick: () => void;
  };
  /** Ação de "Importar coleção" no cartão tracejado. Se omitido, não renderiza. */
  importarColecao?: {
    label: string;
    onClick: () => void;
  };
  /** Mensagem exibida caso não haja nenhuma coleção. */
  mensagemVazio?: string;
  /** Callback global ao clicar numa coleção. */
  onColecaoClick?: (colecao: ColecaoItem) => void;
  className?: string;
}

/**
 * Componente PerfilColecoes (README do PerfilColecoes):
 * Resumo de coleções e jogos, seletor de ordem segmentado, grade de 4 cartões com
 * mosaico 2×2 de capas e selo "Automática", e cartões tracejados de ação no fim.
 * Sem dado, o bloco correspondente não renderiza.
 */
export function PerfilColecoes<TOrdem extends string = string>({
  resumo,
  ordem,
  colecoes,
  seloAutomaticaLabel,
  novaColecao,
  importarColecao,
  mensagemVazio,
  onColecaoClick,
  className,
}: PerfilColecoesProps<TOrdem>) {
  const temBarraSuperior = Boolean(resumo || ordem);
  const temColecoes = Boolean(colecoes && colecoes.length > 0);
  const temAcoes = Boolean(novaColecao || importarColecao);
  const temVazio = Boolean(!temColecoes && mensagemVazio);

  // Sem dado nenhum, o bloco some
  if (!temBarraSuperior && !temColecoes && !temAcoes && !temVazio) {
    return null;
  }

  return (
    <div className={cx(styles["container"], className)}>
      {/* Barra superior (resumo à esquerda e ordenação à direita) */}
      {temBarraSuperior && (
        <div className={styles["barraSuperior"]} role="toolbar" aria-label="Opções de coleções">
          {resumo && <span className={styles["resumo"]}>{resumo}</span>}
          {ordem && (
            <div className={styles["ordemArea"]}>
              <Abas
                variant="segmentado"
                label={ordem.label}
                tabs={ordem.tabs}
                value={ordem.value}
                onChange={ordem.onChange}
              />
            </div>
          )}
        </div>
      )}

      {/* Grade de coleções */}
      {temColecoes && (
        <div className={styles["grade"]} role="region" aria-label={resumo ?? "Coleções de jogos"}>
          {colecoes?.map((colecao) => {
            const handleCardClick = () => {
              colecao.onClick?.();
              onColecaoClick?.(colecao);
            };

            const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleCardClick();
              }
            };

            // Preenche até 4 quadrantes para o mosaico 2×2
            const quadrantes = Array.from({ length: 4 }, (_, i) => {
              const item = colecao.capas?.[i];
              if (!item) return null;
              if (typeof item === "string") return { src: item, alt: `${colecao.nome} - item ${i + 1}` };
              return item;
            });

            return (
              <div
                key={colecao.id}
                className={styles["cartao"]}
                onClick={handleCardClick}
                onKeyDown={handleKeyDown}
                role="button"
                tabIndex={0}
                aria-label={`${colecao.nome}${colecao.detalhe ? `, ${colecao.detalhe}` : ""}${
                  colecao.automatica && seloAutomaticaLabel ? `, ${seloAutomaticaLabel}` : ""
                }`}
              >
                {/* Mosaico 2×2 */}
                <div className={styles["mosaico"]} aria-hidden="true">
                  {quadrantes.map((quadrante, idx) => (
                    <div key={idx} className={styles["mosaicoQuadrante"]}>
                      {quadrante?.src ? (
                        <img
                          src={quadrante.src}
                          alt={quadrante.alt ?? ""}
                          className={styles["mosaicoImagem"]}
                          loading="lazy"
                        />
                      ) : (
                        <span className={styles["mosaicoPlaceholder"]}>TAKO</span>
                      )}
                    </div>
                  ))}
                </div>

                {/* Rodapé com nome, contagem e selo opcional */}
                <div className={styles["cartaoRodape"]}>
                  <div className={styles["textos"]}>
                    <strong className={styles["nome"]}>{colecao.nome}</strong>
                    {colecao.detalhe && <small className={styles["detalhe"]}>{colecao.detalhe}</small>}
                  </div>
                  {colecao.automatica && seloAutomaticaLabel && (
                    <span className={styles["seloAutomatica"]}>
                      <i className={styles["seloPonto"]} aria-hidden="true" />
                      {seloAutomaticaLabel}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Cartões tracejados de ação no fim (Nova coleção e Importar coleção) */}
      {temAcoes && (
        <div className={styles["acoesGrade"]} role="group" aria-label="Ações de coleção">
          {novaColecao && (
            <button
              type="button"
              className={styles["cartaoAcao"]}
              onClick={novaColecao.onClick}
              aria-label={novaColecao.label}
            >
              <span className={styles["acaoConteudo"]}>
                <Icon name="plus" size="md" />
                <span>{novaColecao.label}</span>
              </span>
            </button>
          )}
          {importarColecao && (
            <button
              type="button"
              className={styles["cartaoAcao"]}
              onClick={importarColecao.onClick}
              aria-label={importarColecao.label}
            >
              <span className={styles["acaoConteudo"]}>
                <Icon name="download" size="md" />
                <span>{importarColecao.label}</span>
              </span>
            </button>
          )}
        </div>
      )}

      {/* Estado vazio */}
      {temVazio && <div className={styles["vazio"]}>{mensagemVazio}</div>}
    </div>
  );
}
