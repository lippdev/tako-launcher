/** Escala da interface (`zoom` do `<html>`, "Tamanho da interface"): `left`/`top` em `fixed` valem no espaço com zoom. */
export function rootZoom(): number {
  const zoom = Number.parseFloat(getComputedStyle(document.documentElement).zoom);
  return Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
}

export interface Anchor {
  /** Retângulo do botão que abriu (viewport) ou, se for um ponto, `left === right` e `top === bottom`. */
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface Placement {
  left: number;
  top: number;
  /** Para que lado o menu abriu: define a origem da animação. */
  side: "bottom" | "top";
}

interface PlaceOptions {
  /** `end` alinha a borda direita do menu à do botão (padrão do MenuDeContexto); `start`, a esquerda. */
  align: "start" | "end";
  /** Distância entre o botão e o menu. */
  gap: number;
  /** Folga mínima até a borda da janela. */
  margin: number;
}

/**
 * Posição de um menu de `size` junto a `anchor`: abaixo, e acima se não couber; sempre dentro da janela.
 * Tudo em pixels da viewport; quem aplica divide por `rootZoom()`.
 */
export function placeMenu(
  anchor: Anchor,
  size: { width: number; height: number },
  viewport: { width: number; height: number },
  { align, gap, margin }: PlaceOptions,
): Placement {
  const below = anchor.bottom + gap;
  const fitsBelow = below + size.height + margin <= viewport.height;
  const above = anchor.top - gap - size.height;
  const fitsAbove = above >= margin;
  const side = fitsBelow || !fitsAbove ? "bottom" : "top";
  const top = Math.max(margin, Math.min(side === "bottom" ? below : above, viewport.height - size.height - margin));

  const wanted = align === "end" ? anchor.right - size.width : anchor.left;
  const left = Math.max(margin, Math.min(wanted, viewport.width - size.width - margin));
  return { left, top, side };
}
