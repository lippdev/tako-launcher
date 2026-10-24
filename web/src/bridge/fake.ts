// Hospedeiro falso para rodar a interface no navegador (Vite) sem o Tako: responde com dados de exemplo.
// Só para desenvolvimento; o build de produção não instala isto.

import type { Call, Event, Reply } from "./index";
import type { BridgeError } from "./generated/BridgeError";

type Handler = (payload: unknown) => Reply["data"] | Promise<Reply["data"]>;
export type FakeHandlers = Partial<Record<Call["name"], Handler>>;

const defaults: FakeHandlers = {
  "app.info": () => ({ version: "dev", bridgeVersion: 1 }),
};

/** Instala o hospedeiro falso se não houver um de verdade. Devolve uma função para emitir eventos. */
export function installFakeHost(handlers: FakeHandlers = {}, latencyMs = 30) {
  const all = { ...defaults, ...handlers };
  const send = (wire: object) =>
    setTimeout(() => window.__takoDeliver?.(JSON.stringify(wire)), latencyMs);

  if (!window.takoHost) {
    window.takoHost = {
      post(message) {
        const req = JSON.parse(message) as { v: number; id: number; name: Call["name"]; payload?: unknown };
        const handler = all[req.name];
        if (!handler) {
          const error: BridgeError = { code: "unknown_call", message: req.name };
          send({ v: 1, id: req.id, ok: false, error });
          return;
        }
        Promise.resolve(handler(req.payload)).then(
          (data) => send({ v: 1, id: req.id, ok: true, reply: { name: req.name, data } }),
          (e: unknown) =>
            send({ v: 1, id: req.id, ok: false, error: { code: "internal", message: String(e) } }),
        );
      },
    };
  }

  return (event: Event) => send({ v: 1, event });
}
