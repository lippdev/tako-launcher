// Plugins do Vite só para o `npm run dev` (WEB-DEV-01). Em JavaScript, com os tipos em `vite-dev.d.mts`, porque usam
// as APIs do Node e o projeto não traz os tipos dele.

import { createReadStream, existsSync, readFileSync, statSync } from "node:fs";
import { dirname, extname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { syncTokens } from "./sync-design.mjs";

const web = join(dirname(fileURLToPath(import.meta.url)), "..");

/** Tipos dos arquivos de `dev-data/`: o JSON e as imagens que `tako dev-snapshot` copia. */
const DEV_DATA_TYPES = {
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".gif": "image/gif",
  ".webp": "image/webp",
  ".bmp": "image/bmp",
  ".ico": "image/x-icon",
};

/**
 * Serve `web/dev-data/` em `/dev-data/` para o hospedeiro falso usar a biblioteca real, diz no terminal se há
 * snapshot e recarrega a página quando ele é gravado de novo. Fora do `publicDir` de propósito: o build nunca leva a
 * biblioteca de ninguém.
 * @returns {import("vite").Plugin}
 */
export function devData() {
  const dir = join(web, "dev-data");
  const snapshot = join(dir, "snapshot.json");
  const describe = () => {
    if (!existsSync(snapshot))
      return "dados de exemplo (para a biblioteca real: cargo run -p tako -- dev-snapshot)";
    try {
      const data = JSON.parse(readFileSync(snapshot, "utf8"));
      const count = Array.isArray(data.games) ? data.games.length : 0;
      return `biblioteca real de web/dev-data/ (${count} jogos, ${data.createdAt ?? "?"})`;
    } catch {
      return "web/dev-data/snapshot.json ilegível; usando os dados de exemplo";
    }
  };
  return {
    name: "tako-dev-data",
    apply: "serve",
    configureServer(server) {
      const log = (text) => server.config.logger.info(`  \x1b[36m➜\x1b[0m  Hospedeiro falso: ${text}`);
      server.httpServer?.once("listening", () => log(describe()));
      server.watcher.add(snapshot);
      const reload = (file) => {
        if (resolve(file) !== snapshot) return;
        log(describe());
        server.ws.send({ type: "full-reload" });
      };
      server.watcher.on("add", reload);
      server.watcher.on("change", reload);
      server.watcher.on("unlink", reload);
      server.middlewares.use("/dev-data", (req, res, next) => {
        let path;
        try {
          path = decodeURIComponent((req.url ?? "/").split("?")[0]);
        } catch {
          path = "";
        }
        const file = resolve(dir, `.${path}`);
        const type = DEV_DATA_TYPES[extname(file).toLowerCase()];
        // Nada fora de `dev-data/` e nada além de JSON e imagens.
        if (!type || !file.startsWith(dir + sep) || relative(dir, file).startsWith("..") || !existsSync(file)) {
          res.statusCode = 404;
          res.end();
          return;
        }
        if (!statSync(file).isFile()) {
          next();
          return;
        }
        res.setHeader("Content-Type", type);
        res.setHeader("Cache-Control", "no-cache");
        createReadStream(file).pipe(res);
      });
    },
  };
}

/**
 * Cores e medidas ao vivo. A fonte da verdade é `design/desktop/tokens.css`; ao salvá-lo, roda
 * `scripts/sync-design.mjs`, que regrava `src/styles/tokens.css`, e o HMR do CSS atualiza a página.
 * @returns {import("vite").Plugin}
 */
export function designTokens() {
  const source = resolve(web, "..", "design", "desktop", "tokens.css");
  return {
    name: "tako-design-tokens",
    apply: "serve",
    configureServer(server) {
      if (!existsSync(source)) return;
      const logger = server.config.logger;
      server.watcher.add(source);
      server.watcher.on("change", (file) => {
        if (resolve(file) !== source) return;
        // Só os arquivos que saem dos tokens (e só se mudaram): marca e ícones ficam para o `npm run sync:design`.
        try {
          const { changed } = syncTokens({ copyFonts: false });
          if (changed.length > 0)
            logger.info("[tokens] design/desktop/tokens.css → src/styles/tokens.css", { timestamp: true });
        } catch (error) {
          logger.error(`[tokens] cópia falhou: ${error instanceof Error ? error.message : String(error)}`);
        }
      });
    },
  };
}
