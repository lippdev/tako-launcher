import { imageUrl } from "../../lib/imageVersions";
import { useMemo } from "react";
import mark from "../../assets/brand/tako-mark.png";
import { FileiraDeCapas, type CapaDaFileira } from "../FileiraDeCapas/FileiraDeCapas";
import { cx } from "../../lib/cx";
import styles from "./BibliotecaCategorias.module.css";

/** Um jogo de uma categoria. */
export interface CategoryGame {
  id: string;
  title: string;
  /** Rótulo pequeno acima do nome ("7 h jogadas"). Sem dado, não aparece. */
  hint?: string | null | undefined;
  accentColor?: string | null | undefined;
  /** Tem capa. Sem ela, vale o estado sem arte da CapaDoJogo. Padrão: `true`. */
  hasArt?: boolean | undefined;
  /** URL da capa quando não vem do núcleo (galeria e hospedeiro falso); sem ela, usa `resolveCover`. */
  coverUrl?: string | null | undefined;
  /** O jogo aberto na página dele. */
  active?: boolean | undefined;
  /** Marcado na seleção múltipla. */
  selected?: boolean | undefined;
}

export interface GameCategoryGroup {
  id: string;
  title: string;
  countLabel?: string | null | undefined;
  games: readonly CategoryGame[];
  className?: string | undefined;
}

export interface BibliotecaCategoriasLabels {
  previous: string;
  next: string;
  gamesCount: (count: number) => string;
  empty: string;
  region: string;
}

export interface BibliotecaCategoriasProps {
  groups: readonly GameCategoryGroup[];
  labels: BibliotecaCategoriasLabels;
  loading?: boolean | undefined;
  onOpen: (id: string) => void;
  onHighlight?: ((id: string) => void) | undefined;
  /** A rolagem de uma fileira chegou perto do fim: hora de pedir mais capas dela (recebe o id do grupo). */
  onNearEnd?: ((groupId: string) => void) | undefined;
  resolveCover?: ((id: string, width: number, height: number) => string | null) | undefined;
  className?: string | undefined;
}

function defaultResolveCover(id: string, width: number, height: number): string {
  return imageUrl("cover", id, width, height);
}

/**
 * Componente BibliotecaCategorias: exibe os jogos da biblioteca organizados em fileiras por categoria.
 * Reúne cada grupo numa FileiraDeCapas com título, contagem e controles de navegação ‹ ›.
 * Sobrescreve as colunas da fileira para 7 capas por largura conforme as especificações do design system.
 */
export function BibliotecaCategorias({
  groups,
  labels,
  loading = false,
  onOpen,
  onHighlight,
  onNearEnd,
  resolveCover = defaultResolveCover,
  className,
}: BibliotecaCategoriasProps) {
  const countFormatter = labels.gamesCount;

  // Transforma cada grupo em dados prontos para a FileiraDeCapas
  const processedGroups = useMemo(() => {
    return groups.map((group) => {
      const capas: CapaDaFileira[] = group.games.map((game) => ({
        id: game.id,
        title: game.title || game.id,
        label: game.hint ?? null,
        accentColor: game.accentColor ?? null,
        hasArt: game.hasArt !== false,
        src: game.coverUrl ?? resolveCover(game.id, 233, 311) ?? undefined,
        active: game.active === true,
        selected: game.selected === true,
      }));

      const contagem = group.countLabel ?? countFormatter(group.games.length);

      return {
        ...group,
        capas,
        contagem,
      };
    });
  }, [groups, countFormatter, resolveCover]);

  const ariaLabel = labels.region;
  const previousLabel = labels.previous;
  const nextLabel = labels.next;
  const emptyLabel = labels.empty;

  if (!loading && groups.length === 0) {
    return (
      <div className={cx(styles["categorias"], className)} role="region" aria-label={ariaLabel}>
        <div className={styles["emptyState"]}>
          <img src={mark} alt="" className={styles["emptyIcon"]} />
          <p>{emptyLabel}</p>
        </div>
      </div>
    );
  }

  return (
    <div className={cx(styles["categorias"], className)} role="region" aria-label={ariaLabel}>
      {processedGroups.map((group) => (
        <FileiraDeCapas
          key={group.id}
          title={group.title}
          count={group.contagem}
          covers={group.capas}
          previousLabel={previousLabel}
          nextLabel={nextLabel}
          onOpen={onOpen}
          onHighlight={onHighlight}
          onNearEnd={onNearEnd ? () => { onNearEnd(group.id); } : undefined}
          className={cx(styles["fileiraCategorias"], group.className)}
        />
      ))}
    </div>
  );
}
