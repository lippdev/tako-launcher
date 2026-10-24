// O que o campo do EditorDeEtiquetas oferece enquanto a pessoa digita (a mesma regra do editor do app Slint,
// `tako-ui/src/shell/tags_editor.rs`). Só dados e funções puras, para o teste rodar direto no Node.

export interface SugestaoDeEtiqueta {
  nome: string;
  /** Quantos jogos têm a etiqueta. */
  jogos: number;
}

/** Tamanho máximo do nome de uma etiqueta (`tako_core::library::tags::MAX_NAME_LENGTH`). */
export const MAX_NOME = 40;

/** Sem caixa, sem acento e sem espaços repetidos: "Sofá  Co-op" e "sofa co-op" são o mesmo nome. */
export function normalizar(texto: string): string {
  return texto
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .replace(/\s+/g, " ")
    .trim()
    .toLowerCase();
}

/** O nome como o núcleo guarda: sem espaços nas pontas nem repetidos no meio. */
export function limparNome(texto: string): string {
  return texto.replace(/\s+/g, " ").trim();
}

export interface Sugestoes {
  /** As etiquetas que existem, ainda não aplicadas, e que combinam com o digitado. */
  existentes: SugestaoDeEtiqueta[];
  /** O nome que seria criado: válido e que ainda não existe. */
  criar: string | null;
}

/** As etiquetas que combinam com o digitado e, no fim, a que seria criada. Campo vazio lista o que falta. */
export function sugerir(
  todas: readonly SugestaoDeEtiqueta[],
  aplicadas: readonly string[],
  digitado: string,
): Sugestoes {
  const busca = normalizar(digitado);
  const jaAplicada = new Set(aplicadas.map(normalizar));
  const existentes = todas.filter((e) => {
    const nome = normalizar(e.nome);
    return !jaAplicada.has(nome) && nome.includes(busca);
  });
  const limpo = limparNome(digitado);
  const valido = limpo !== "" && [...limpo].length <= MAX_NOME;
  const existe = todas.some((e) => normalizar(e.nome) === normalizar(limpo));
  return { existentes, criar: valido && !existe ? limpo : null };
}
