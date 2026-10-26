// Tipos de `vite-dev.mjs` para o `vite.config.ts` (o projeto não traz os tipos do Node).
import type { Plugin } from "vite";

/** Serve `web/dev-data/` em `/dev-data/` no `npm run dev` e avisa se há biblioteca real (WEB-DEV-01). */
export function devData(): Plugin;
/** Copia `design/desktop/tokens.css` para `src/styles/tokens.css` ao salvar, no `npm run dev`. */
export function designTokens(): Plugin;
