/** Junta nomes de classe, ignorando os vazios (`styles["x"]` é `string | undefined` com `noUncheckedIndexedAccess`). */
export function cx(...parts: (string | false | undefined)[]): string {
  return parts.filter(Boolean).join(" ");
}
