// Cliente da ponte com o núcleo Rust (docs/web/ponte.md). Os tipos de `generated/` saem do Rust
// (`cargo test -p tako-app`); não edite à mão.

import type { BridgeError } from "./generated/BridgeError";
import type { Call } from "./generated/Call";
import type { Event } from "./generated/Event";
import type { Reply } from "./generated/Reply";

export type { BridgeError, Call, Event, Reply };

/** Versão do contrato que esta interface fala; tem que bater com `BRIDGE_VERSION` do Rust. */
export const BRIDGE_VERSION = 1;

export type CallName = Call["name"];
export type CallPayload<N extends CallName> =
  Extract<Call, { name: N }> extends { payload: infer P } ? P : undefined;
export type ReplyData<N extends CallName> =
  Extract<Reply, { name: N }> extends { data: infer D } ? D : undefined;
export type EventName = Event["name"];
export type EventPayload<N extends EventName> =
  Extract<Event, { name: N }> extends { payload: infer P } ? P : undefined;

/** O que o hospedeiro (CEF, WebView2 ou o falso do navegador) injeta antes da página carregar. */
export interface TakoHost {
  post(message: string): void;
}

declare global {
  interface Window {
    takoHost?: TakoHost;
    /** Chamado pelo hospedeiro com cada resposta ou evento (texto JSON). */
    __takoDeliver?: (message: string) => void;
  }
}

/** Erro de chamada com o código do núcleo; a tela escolhe o texto pelo `code`. */
export class CallError extends Error {
  constructor(readonly error: BridgeError) {
    super(`${error.code}: ${error.message}`);
  }
}

type Pending = { resolve: (data: unknown) => void; reject: (e: unknown) => void };
type Wire =
  | { v: number; id: number; ok: true; reply: Reply }
  | { v: number; id: number; ok: false; error: BridgeError }
  | { v: number; event: Event };

const pending = new Map<number, Pending>();
const listeners = new Map<string, Set<(payload: unknown) => void>>();
let nextId = 1;

function host(): TakoHost {
  const h = window.takoHost;
  if (!h) throw new Error("ponte: nenhum hospedeiro (window.takoHost) nesta página");
  return h;
}

function deliver(message: string): void {
  let wire: Wire;
  try {
    wire = JSON.parse(message) as Wire;
  } catch {
    console.error("ponte: mensagem que não é JSON", message);
    return;
  }
  if ("event" in wire) {
    const payload = "payload" in wire.event ? wire.event.payload : undefined;
    listeners.get(wire.event.name)?.forEach((fn) => fn(payload));
    return;
  }
  const p = pending.get(wire.id);
  if (!p) return;
  pending.delete(wire.id);
  if (wire.ok) p.resolve("data" in wire.reply ? wire.reply.data : undefined);
  else p.reject(new CallError(wire.error));
}

window.__takoDeliver = deliver;

/** Chama o núcleo. A resposta tem o tipo da `Reply` com o mesmo nome. */
export function call<N extends CallName>(
  name: N,
  ...args: CallPayload<N> extends undefined ? [] : [payload: CallPayload<N>]
): Promise<ReplyData<N>> {
  const id = nextId;
  nextId = nextId >= 0xffffffff ? 1 : nextId + 1;
  const message: Record<string, unknown> = { v: BRIDGE_VERSION, id, name };
  if (args.length) message.payload = args[0];
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve: resolve as (d: unknown) => void, reject });
    try {
      host().post(JSON.stringify(message));
    } catch (e) {
      pending.delete(id);
      reject(e instanceof Error ? e : new Error(String(e)));
    }
  });
}

/** Escuta um evento do núcleo; devolve a função que para de escutar. */
export function on<N extends EventName>(
  name: N,
  handler: (payload: EventPayload<N>) => void,
): () => void {
  const set = listeners.get(name) ?? new Set();
  listeners.set(name, set);
  const fn = handler as (payload: unknown) => void;
  set.add(fn);
  return () => set.delete(fn);
}
