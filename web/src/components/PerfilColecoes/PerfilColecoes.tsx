import type { KeyboardEvent } from "react";
import { cx } from "../../lib/cx";
import { Abas, type Aba } from "../Abas/Abas";
import { Icon } from "../Icon/Icon";
import styles from "./PerfilColecoes.module.css";

export interface CollectionCover {
  /** URL direta da capa do jogo. */
  src?: string;
  /** Texto alternativo para acessibilidade. */
  alt?: string;
}

export interface CollectionItem {
  id: string;
  /** Nome da coleção, ex.: "Roguelikes" ou "Jogados este mês". */
  name: string;
  /** Quantidade e instalados, ex.: "6 jogos · 4 instalados". */
  detail?: string;
  /** Indica se é uma coleção automática por regra (exibe o selo com ponto). */
  automatic?: boolean;
  /** Até 4 capas para compor o mosaico 2×2. */
  covers?: readonly (string | CollectionCover)[];
  /** Callback ao clicar neste cartão de coleção. */
  onClick?: () => void;
}

export interface PerfilColecoesProps<TOrder extends string = string> {
  /** Resumo à esquerda, ex.: "4 coleções · 26 jogos". */
  summary?: string;
  /** Nome da barra de opções para leitores de tela. */
  toolbarLabel?: string;
  /** Seletor segmentado de ordenação à direita, ex.: Recentes · A–Z · Mais jogos. */
  order?: {
    tabs: readonly Aba<TOrder>[];
    value: TOrder;
    onChange: (value: TOrder) => void;
    label: string;
  };
  /** Lista de coleções exibidas na grade de 4 colunas. */
  collections?: readonly CollectionItem[];
  /** Nome da grade para leitores de tela; sem ele, vale o resumo. */
  gridLabel?: string;
  /** Rótulo do selo para coleções automáticas, ex.: "Automática". */
  autoBadgeLabel?: string;
  /** Nome do grupo de ações para leitores de tela. */
  actionsLabel?: string;
  /** Ação de "Nova coleção" no cartão tracejado. Se omitido, não renderiza. */
  newCollection?: {
    label: string;
    onClick: () => void;
  };
  /** Ação de "Importar coleção" no cartão tracejado. Se omitido, não renderiza. */
  importCollection?: {
    label: string;
    onClick: () => void;
  };
  /** Mensagem exibida caso não haja nenhuma coleção. */
  emptyMessage?: string;
  /** Callback global ao clicar numa coleção. */
  onCollectionClick?: (collection: CollectionItem) => void;
  className?: string;
}

/**
 * Componente PerfilColecoes (README do PerfilColecoes):
 * Resumo de coleções e jogos, seletor de ordem segmentado, grade de 4 cartões com
 * mosaico 2×2 de capas e selo "Automática", e cartões tracejados de ação no fim.
 * Sem dado, o bloco correspondente não renderiza.
 */
export function PerfilColecoes<TOrder extends string = string>({
  summary,
  toolbarLabel,
  order,
  collections,
  gridLabel,
  autoBadgeLabel,
  actionsLabel,
  newCollection,
  importCollection,
  emptyMessage,
  onCollectionClick,
  className,
}: PerfilColecoesProps<TOrder>) {
  const hasTopBar = Boolean(summary || order);
  const hasCollections = Boolean(collections && collections.length > 0);
  const hasActions = Boolean(newCollection || importCollection);
  const hasEmpty = Boolean(!hasCollections && emptyMessage);

  // Sem dado nenhum, o bloco some
  if (!hasTopBar && !hasCollections && !hasActions && !hasEmpty) {
    return null;
  }

  const gridName = gridLabel ?? summary;

  return (
    <div className={cx(styles["container"], className)}>
      {/* Barra superior (resumo à esquerda e ordenação à direita) */}
      {hasTopBar && (
        <div
          className={styles["barraSuperior"]}
          role="toolbar"
          {...(toolbarLabel ? { "aria-label": toolbarLabel } : {})}
        >
          {summary && <span className={styles["resumo"]}>{summary}</span>}
          {order && (
            <div className={styles["ordemArea"]}>
              <Abas
                variant="segmentado"
                label={order.label}
                tabs={order.tabs}
                value={order.value}
                onChange={order.onChange}
              />
            </div>
          )}
        </div>
      )}

      {/* Grade de coleções */}
      {hasCollections && (
        <div className={styles["grade"]} role="region" {...(gridName ? { "aria-label": gridName } : {})}>
          {collections?.map((collection) => {
            const handleCardClick = () => {
              collection.onClick?.();
              onCollectionClick?.(collection);
            };

            const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                handleCardClick();
              }
            };

            // Preenche até 4 quadrantes para o mosaico 2×2
            const quadrants = Array.from({ length: 4 }, (_, i): CollectionCover | null => {
              const item = collection.covers?.[i];
              if (!item) return null;
              return typeof item === "string" ? { src: item } : item;
            });

            return (
              <div
                key={collection.id}
                className={styles["cartao"]}
                onClick={handleCardClick}
                onKeyDown={handleKeyDown}
                role="button"
                tabIndex={0}
                aria-label={`${collection.name}${collection.detail ? `, ${collection.detail}` : ""}${
                  collection.automatic && autoBadgeLabel ? `, ${autoBadgeLabel}` : ""
                }`}
              >
                {/* Mosaico 2×2 */}
                <div className={styles["mosaico"]} aria-hidden="true">
                  {quadrants.map((quadrant, idx) => (
                    <div key={idx} className={styles["mosaicoQuadrante"]}>
                      {quadrant?.src ? (
                        <img
                          src={quadrant.src}
                          alt={quadrant.alt ?? ""}
                          className={styles["mosaicoImagem"]}
                          loading="lazy"
                          decoding="async"
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
                    <strong className={styles["nome"]}>{collection.name}</strong>
                    {collection.detail && <small className={styles["detalhe"]}>{collection.detail}</small>}
                  </div>
                  {collection.automatic && autoBadgeLabel && (
                    <span className={styles["seloAutomatica"]}>
                      <i className={styles["seloPonto"]} aria-hidden="true" />
                      {autoBadgeLabel}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Cartões tracejados de ação no fim (Nova coleção e Importar coleção) */}
      {hasActions && (
        <div className={styles["acoesGrade"]} role="group" {...(actionsLabel ? { "aria-label": actionsLabel } : {})}>
          {newCollection && (
            <button
              type="button"
              className={styles["cartaoAcao"]}
              onClick={newCollection.onClick}
              aria-label={newCollection.label}
            >
              <span className={styles["acaoConteudo"]}>
                <Icon name="plus" size="md" />
                <span>{newCollection.label}</span>
              </span>
            </button>
          )}
          {importCollection && (
            <button
              type="button"
              className={styles["cartaoAcao"]}
              onClick={importCollection.onClick}
              aria-label={importCollection.label}
            >
              <span className={styles["acaoConteudo"]}>
                <Icon name="download" size="md" />
                <span>{importCollection.label}</span>
              </span>
            </button>
          )}
        </div>
      )}

      {/* Estado vazio */}
      {hasEmpty && <div className={styles["vazio"]}>{emptyMessage}</div>}
    </div>
  );
}
