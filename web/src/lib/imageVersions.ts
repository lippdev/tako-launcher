// Só estado de apresentação: cada publicação invalida as URLs daquele jogo, inclusive após desmontar a tela.
const versions = new Map<string, number>();
const listeners = new Set<() => void>();
let revision = 0;

export function imageVersion(id: string): number {
  return versions.get(id) ?? 0;
}

export function artChanged(id: string): void {
  versions.set(id, imageVersion(id) + 1);
  revision += 1;
  listeners.forEach((listener) => listener());
}

export const artRevision = () => revision;
export function subscribeArt(listener: () => void): () => void {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}

export function versionedImageUrl(src: string): string {
  if (!src.startsWith("tako-img://")) return src;
  const url = new URL(src);
  const version = imageVersion(url.searchParams.get("id") ?? "");
  // Conserva encodeURIComponent: o esquema do núcleo decodifica %20, sem converter '+' em espaço.
  return version > 0 ? `${src.replace(/&v=[^&]*/g, "")}&v=${String(version)}` : src;
}

/** Dimensões já em pixels físicos; a versão é comum aos quatro tipos de arte. */
export function imageUrl(kind: "cover" | "hero" | "logo" | "icon", id: string, width: number, height: number): string {
  return versionedImageUrl(`tako-img://${kind}?id=${encodeURIComponent(id)}&w=${String(Math.round(width))}&h=${String(Math.round(height))}`);
}
