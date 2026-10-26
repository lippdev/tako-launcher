// Textos e idiomas da interface (docs/web/ponte.md §8). O catálogo vem do núcleo (`i18n.catalog`); aqui ficam só a
// busca da chave, o preenchimento dos parâmetros e o estado do idioma atual. Sem importações em execução, para o
// teste rodar direto no Node; a ligação com a ponte e o React está em `index.ts`.

/** Chave completa → texto, como o núcleo entrega: `{nome}` nos parâmetros, `chave.one` e `chave.other` nos plurais. */
export type Messages = Readonly<Record<string, string>>;
export type Vars = Readonly<Record<string, string | number>>;

const PLACEHOLDER = /\{([^{}]+)\}/g;

function fill(text: string, vars: Vars | undefined): string {
  if (!vars) return text;
  return text.replace(PLACEHOLDER, (whole, name: string) => {
    const value = Object.hasOwn(vars, name) ? vars[name] : undefined;
    return value === undefined ? whole : String(value);
  });
}

/**
 * O texto da chave, com `{nome}` trocado pelo valor em `vars`. Se a chave não existe mas `vars.n` é um número, vale
 * o plural do núcleo: `chave.one` quando `n == 1`, senão `chave.other` (o `{n}` também é preenchido).
 * Sem texto, devolve a própria chave e chama `onMissing(chave)`; nunca lança.
 */
export function translate(
  messages: Messages,
  key: string,
  vars?: Vars,
  onMissing?: (key: string) => void,
): string {
  let text = Object.hasOwn(messages, key) ? messages[key] : undefined;
  if (text === undefined && typeof vars?.["n"] === "number") {
    const pluralKey = `${key}.${vars["n"] === 1 ? "one" : "other"}`;
    text = Object.hasOwn(messages, pluralKey) ? messages[pluralKey] : undefined;
  }
  if (text === undefined) {
    onMissing?.(key);
    return key;
  }
  return fill(text, vars);
}

export type TFunction = (key: string, vars?: Vars) => string;

export interface I18nSnapshot {
  /** Código do idioma (`pt-BR`), ou `null` enquanto o primeiro catálogo não chegou. */
  readonly lang: string | null;
  readonly messages: Messages;
  /** Muda a cada catálogo novo; o `t` de um snapshot antigo continua valendo para quem já o tem. */
  readonly t: TFunction;
}

export interface CatalogData {
  lang: string;
  messages: Messages;
}

export interface I18nOptions {
  /** Pede o catálogo do idioma atual ao núcleo (`i18n.catalog`). */
  fetchCatalog: () => Promise<CatalogData>;
  /** Registra o aviso de chave sem texto; só em desenvolvimento. Cada chave avisa uma vez. */
  warn?: ((message: string) => void) | undefined;
}

/**
 * O estado do idioma: o catálogo atual, quem escuta e o carregamento. `reload()` vale na abertura e a cada
 * `i18n.changed`; se duas respostas chegam fora de ordem, vale a do pedido mais novo. Enquanto o primeiro catálogo não
 * chega, `t` devolve a chave sem avisar (não é texto faltando, é carregamento).
 */
export function createI18n({ fetchCatalog, warn }: I18nOptions) {
  const listeners = new Set<() => void>();
  const warned = new Set<string>();
  let requested = 0;
  let snapshot: I18nSnapshot = build(null, {});

  function build(lang: string | null, messages: Messages): I18nSnapshot {
    const onMissing =
      lang === null
        ? undefined
        : (key: string) => {
            if (warned.has(`${lang}:${key}`)) return;
            warned.add(`${lang}:${key}`);
            warn?.(`i18n: a chave "${key}" não tem texto em ${lang}`);
          };
    return { lang, messages, t: (key, vars) => translate(messages, key, vars, onMissing) };
  }

  async function reload(): Promise<void> {
    const mine = ++requested;
    try {
      const data = await fetchCatalog();
      if (mine !== requested) return;
      snapshot = build(data.lang, data.messages);
      listeners.forEach((fn) => {
        fn();
      });
    } catch (error) {
      if (mine === requested) warn?.(`i18n: não foi possível ler o catálogo (${String(error)})`);
    }
  }

  return {
    reload,
    getSnapshot: () => snapshot,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
