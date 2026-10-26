// Escolha do próximo elemento por geometria. Puro: recebe retângulos e devolve a escolha, sem tocar no DOM, para
// o `node --test` rodar com retângulos falsos. Porta do `SpatialNavigator` do núcleo
// (`crates/tako-core/src/navigation/spatial.rs`) com o que a web precisa a mais: regiões, memória do último foco
// de cada região e fileiras roláveis.

export type Direction = "up" | "down" | "left" | "right";

export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Um elemento que pode receber o foco: `item` é o que o chamador quiser (no DOM, o próprio elemento). */
export interface Candidate<T> {
  item: T;
  rect: Rect;
  /** Região (`data-focus-region`); vazio fora de qualquer região. */
  region: string;
  /**
   * Fileira rolável a que pertence (`data-focus-group`). Quem está numa fileira vê as vizinhas dela inteiras, as
   * escondidas pela rolagem também; quem está fora só vê as que aparecem (`clipped = false`).
   */
  group?: unknown;
  /** Está escondido pela rolagem da própria fileira. */
  clipped?: boolean;
}

/** Fora do eixo da seta, a distância pesa mais que a do eixo: prefere o vizinho alinhado ao mais próximo torto. */
const ORTHOGONAL_WEIGHT = 2.2;
/** Folga de arredondamento (px) nos testes de "está desse lado". */
const TOLERANCE = 2;

const right = (r: Rect) => r.left + r.width;
const bottom = (r: Rect) => r.top + r.height;
const centerX = (r: Rect) => r.left + r.width / 2;
const centerY = (r: Rect) => r.top + r.height / 2;

/** Nota de um candidato na direção pedida (menor é melhor); `null` se ele não está desse lado. */
function score(from: Rect, c: Rect, direction: Direction): number | null {
  let primary: number;
  let orthogonal: number;
  switch (direction) {
    case "left":
      if (c.left + c.width > from.left + TOLERANCE && centerX(c) >= centerX(from) - TOLERANCE) return null;
      primary = from.left - right(c);
      orthogonal = Math.abs(centerY(c) - centerY(from));
      break;
    case "right":
      if (c.left < right(from) - TOLERANCE && centerX(c) <= centerX(from) + TOLERANCE) return null;
      primary = c.left - right(from);
      orthogonal = Math.abs(centerY(c) - centerY(from));
      break;
    case "up":
      if (bottom(c) > from.top + TOLERANCE && centerY(c) >= centerY(from) - TOLERANCE) return null;
      primary = from.top - bottom(c);
      orthogonal = Math.abs(centerX(c) - centerX(from));
      break;
    case "down":
      if (c.top < bottom(from) - TOLERANCE && centerY(c) <= centerY(from) + TOLERANCE) return null;
      primary = c.top - bottom(from);
      orthogonal = Math.abs(centerX(c) - centerX(from));
      break;
  }
  return Math.max(0, primary) + orthogonal * ORTHOGONAL_WEIGHT;
}

/** O vizinho mais próximo de `from` na direção, entre `candidates`; `null` na borda. */
export function findNext<T>(from: Rect, candidates: readonly Candidate<T>[], direction: Direction): Candidate<T> | null {
  let best: Candidate<T> | null = null;
  let bestScore = Infinity;
  for (const candidate of candidates) {
    const s = score(from, candidate.rect, direction);
    if (s !== null && s < bestScore) {
      bestScore = s;
      best = candidate;
    }
  }
  return best;
}

/** Os intervalos [a1, a2] e [b1, b2] se tocam (com a folga)? */
function overlaps(a1: number, a2: number, b1: number, b2: number): boolean {
  return a1 < b2 - TOLERANCE && b1 < a2 - TOLERANCE;
}

/** Caixa que envolve todos os candidatos. */
function bounds<T>(candidates: readonly Candidate<T>[]): Rect {
  let left = Infinity;
  let top = Infinity;
  let rightEdge = -Infinity;
  let bottomEdge = -Infinity;
  for (const { rect } of candidates) {
    left = Math.min(left, rect.left);
    top = Math.min(top, rect.top);
    rightEdge = Math.max(rightEdge, right(rect));
    bottomEdge = Math.max(bottomEdge, bottom(rect));
  }
  return { left, top, width: rightEdge - left, height: bottomEdge - top };
}

export interface Choice<T> {
  candidate: Candidate<T>;
  /** O foco atravessou para outra região. */
  crossed: boolean;
}

export interface ChooseOptions<T> {
  /** O elemento focado, para não escolher a si mesmo. */
  current?: T | undefined;
  /** O último foco de cada região (região → item); vale só ao atravessar, e só se ainda for candidato. */
  memory?: ReadonlyMap<string, T> | undefined;
  /**
   * A caixa de cada região na tela (o elemento `data-focus-region`, que cobre o espaço todo dela). Sem ela, vale a
   * caixa que envolve os candidatos da região, que fica menor quando eles se juntam num canto.
   */
  regions?: ReadonlyMap<string, Rect> | undefined;
}

/**
 * Para onde a seta leva a partir de `from` (o retângulo do elemento focado, que está na região `region`):
 *
 * 1. o vizinho mais próximo e alinhado na mesma região (e na mesma fileira rolável, quando há);
 * 2. senão, atravessa para a região vizinha: a região precisa ficar naquela direção e se sobrepor a `from` no
 *    outro eixo (de uma ponta da lista não se pula na diagonal para o conteúdo), e ganha o candidato mais
 *    próximo. Chegando numa região que já teve foco, volta ao elemento de onde ela saiu;
 * 3. senão, o vizinho torto da própria região, se houver.
 */
export function choose<T>(
  from: Rect,
  region: string,
  group: unknown,
  candidates: readonly Candidate<T>[],
  direction: Direction,
  { current, memory, regions }: ChooseOptions<T> = {},
): Choice<T> | null {
  // Fileira escondida pela rolagem só conta para quem está nela.
  const eligible = candidates.filter(
    (c) => c.item !== current && (c.group === undefined || c.clipped !== true || c.group === group),
  );

  const horizontal = direction === "left" || direction === "right";
  const sameRegion = eligible.filter((c) => c.region === region);
  // Alinhado: se sobrepõe a `from` no outro eixo. Sem isso o vizinho "à esquerda" podia estar duas fileiras abaixo.
  const lined = sameRegion.filter((c) =>
    horizontal
      ? overlaps(from.top, bottom(from), c.rect.top, bottom(c.rect))
      : overlaps(from.left, right(from), c.rect.left, right(c.rect)),
  );
  const inside = findNext(from, lined, direction);
  if (inside) return { candidate: inside, crossed: false };

  // Outras regiões: só as que ficam na direção e alinhadas com `from`.
  const reachable = new Map<string, Candidate<T>[]>();
  for (const c of eligible) {
    if (c.region === region) continue;
    const list = reachable.get(c.region);
    if (list) list.push(c);
    else reachable.set(c.region, [c]);
  }
  const pool: Candidate<T>[] = [];
  for (const members of reachable.values()) {
    const box = regions?.get(members[0]?.region ?? "") ?? bounds(members);
    const aligned = horizontal
      ? overlaps(from.top, bottom(from), box.top, bottom(box))
      : overlaps(from.left, right(from), box.left, right(box));
    if (aligned) pool.push(...members);
  }
  const target = findNext(from, pool, direction);
  if (!target) {
    // Última chance: o torto da própria região (uma grade irregular, a fileira de baixo mais curta).
    const askew = findNext(from, sameRegion, direction);
    return askew ? { candidate: askew, crossed: false } : null;
  }

  const remembered = memory?.get(target.region);
  const back = remembered === undefined ? undefined : pool.find((c) => c.region === target.region && c.item === remembered);
  return { candidate: back ?? target, crossed: true };
}

/** O candidato de entrada quando nada tem foco: o lembrado da região, senão o primeiro (cima para baixo, esquerda para direita). */
export function entry<T>(
  candidates: readonly Candidate<T>[],
  region: string,
  memory?: ReadonlyMap<string, T>,
): Candidate<T> | null {
  const members = candidates.filter((c) => c.region === region && c.clipped !== true);
  const remembered = memory?.get(region);
  const back = remembered === undefined ? undefined : members.find((c) => c.item === remembered);
  if (back) return back;
  let first: Candidate<T> | null = null;
  for (const c of members) {
    // Linhas: dentro de uma tolerância de altura vale a ordem da esquerda para a direita.
    if (!first || c.rect.top < first.rect.top - TOLERANCE || (Math.abs(c.rect.top - first.rect.top) <= TOLERANCE && c.rect.left < first.rect.left)) {
      first = c;
    }
  }
  return first;
}
