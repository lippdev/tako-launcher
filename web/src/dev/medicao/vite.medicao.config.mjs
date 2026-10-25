// Servidor e construção de medição (WEB-60b). Não faz parte do produto: o `vite.config.ts` e o `npm run build`
// não sabem que ele existe.
//
// Faz duas coisas: injeta `medicao.ts` no `index.html` (sem tocar em `index.html`, `main.tsx` nem `src/app/`) e
// responde ao coletor que a página usa para entregar os números. Serve para dois usos:
//   npx vite --config src/dev/medicao/vite.medicao.config.mjs                    servidor de desenvolvimento
//   npx vite build --config src/dev/medicao/vite.medicao.config.mjs --mode medicao   pacote de produção com a medição
//     (depois, `vite preview` com a mesma configuração serve esse pacote)
//
// Variáveis de ambiente (todas definidas pelo `scripts/measure-web.ps1`):
//   TAKO_MEDICAO_PORTA   porta do servidor (padrão 5199; a 5173 é do orquestrador)
//   TAKO_MEDICAO_SAIDA   arquivo onde o coletor grava uma linha JSON por evento da página
//   TAKO_MEDICAO_JOGOS   arquivo de texto com um caminho por linha: os jogos de teste que o cenário `semear` adiciona
//   TAKO_MEDICAO_JOGO    caminho do jogo de teste que fecha sozinho (cenário `retorno`)
//   TAKO_MEDICAO_DIST    pasta de saída da construção (padrão target/medicao-dist, fora do `web/dist`)

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const raiz = path.resolve(import.meta.dirname, "../../..");
const porta = Number(process.env.TAKO_MEDICAO_PORTA ?? "5199");
const saida = process.env.TAKO_MEDICAO_SAIDA ?? "";
const visitas = new Map();

function responder(res, corpo) {
  res.setHeader("content-type", "application/json");
  res.setHeader("cache-control", "no-store");
  res.end(JSON.stringify(corpo));
}

function linhas(arquivo) {
  if (!arquivo || !fs.existsSync(arquivo)) return [];
  return fs
    .readFileSync(arquivo, "utf8")
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);
}

function coletor(req, res, next) {
  const url = new URL(req.url ?? "/", "http://localhost");
  if (!url.pathname.startsWith("/__medicao")) return next();
  if (req.method === "POST" && url.pathname === "/__medicao") {
    const partes = [];
    req.on("data", (p) => partes.push(p));
    req.on("end", () => {
      if (saida) fs.appendFileSync(saida, `${Buffer.concat(partes).toString("utf8")}\n`);
      res.statusCode = 204;
      res.end();
    });
    return undefined;
  }
  if (url.pathname === "/__medicao/jogos") return responder(res, linhas(process.env.TAKO_MEDICAO_JOGOS));
  if (url.pathname === "/__medicao/jogo-de-teste") {
    return responder(res, process.env.TAKO_MEDICAO_JOGO ? [process.env.TAKO_MEDICAO_JOGO] : []);
  }
  if (url.pathname === "/__medicao/visita") {
    // Uma contagem por cenário e rodada: cada rodada começa na primeira visita.
    const chave = `${url.searchParams.get("c") ?? ""}|${url.searchParams.get("r") ?? ""}`;
    const n = (visitas.get(chave) ?? 0) + 1;
    visitas.set(chave, n);
    return responder(res, { n });
  }
  res.statusCode = 404;
  res.end();
  return undefined;
}

/** Injeta a medição antes de qualquer script do app, para ela instalar o contador de quadros primeiro. */
function medicao() {
  return {
    name: "tako-medicao",
    // `pre`: a construção só empacota os scripts que já estão no HTML quando o Vite o processa.
    transformIndexHtml: {
      order: "pre",
      handler() {
        return [{ tag: "script", attrs: { type: "module", src: "/src/dev/medicao/medicao.ts" }, injectTo: "head-prepend" }];
      },
    },
    configureServer(servidor) {
      servidor.middlewares.use(coletor);
    },
    configurePreviewServer(servidor) {
      servidor.middlewares.use(coletor);
    },
  };
}

export default defineConfig({
  root: raiz,
  plugins: [react(), medicao()],
  // Como o `vite.config.ts` do app: caminhos relativos, o pacote roda dentro do Tako.
  base: "./",
  server: { host: "127.0.0.1", port: porta, strictPort: true, hmr: false },
  preview: { host: "127.0.0.1", port: porta, strictPort: true },
  build: {
    target: "es2023",
    outDir: process.env.TAKO_MEDICAO_DIST ?? path.resolve(raiz, "../target/medicao-dist"),
    emptyOutDir: true,
  },
});
