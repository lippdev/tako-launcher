// Copia a base visual de `design/desktop/` (só leitura) para `web/src/`, para o build não depender
// da pasta de design: tokens, fontes (Poppins e Plus Jakarta Sans), marca e ícones.
// Rode `npm run sync:design` quando o design system mudar e commite o resultado. No `npm run dev`, o Vite chama
// `syncTokens` sozinho a cada gravação de `design/desktop/tokens.css` (scripts/vite-dev.mjs).

import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const web = join(dirname(fileURLToPath(import.meta.url)), "..");
const design = join(web, "..", "design", "desktop");
const styles = join(web, "src", "styles");
const header = "// Gerado por scripts/sync-design.mjs a partir de design/desktop/. Não edite à mão.\n";

/** Grava só se mudou: arquivo igual não dispara o HMR do Vite. */
function writeIfChanged(file, text) {
  if (existsSync(file) && readFileSync(file, "utf8") === text) return false;
  writeFileSync(file, text);
  return true;
}

/**
 * Tokens (cópia fiel; o `@font-face` aponta para `fonts/` ao lado do arquivo), as fontes que eles declaram e a lista
 * de cores da página de desenvolvimento. Devolve o número de cores e os arquivos que mudaram.
 */
export function syncTokens({ copyFonts = true } = {}) {
  mkdirSync(join(styles, "fonts"), { recursive: true });
  const tokens = readFileSync(join(design, "tokens.css"), "utf8");
  const changed = [];
  const tokensFile = join(styles, "tokens.css");
  if (writeIfChanged(tokensFile, `/* Cópia de design/desktop/tokens.css (scripts/sync-design.mjs). Não edite à mão. */\n${tokens}`))
    changed.push(tokensFile);

  // Fontes: só as famílias que o tokens.css declara (Source Sans 3 fica de fora). Em dev, só as que faltam.
  for (const family of tokens.matchAll(/url\("fonts\/([^"]+)"\)/g)) {
    const target = join(styles, "fonts", family[1]);
    if (copyFonts || !existsSync(target)) copyFileSync(join(design, "fonts", family[1]), target);
  }

  // Variáveis de cor do tema base (valores `#…`), para a página de desenvolvimento listar a paleta.
  const base = tokens.match(/:root, \[data-theme="tako"\] \{([\s\S]*?)\n\}/)?.[1] ?? "";
  const colors = [...base.matchAll(/^\s*(--[\w-]+):\s*#[0-9a-fA-F]+;/gm)].map((m) => m[1]);
  const colorsFile = join(styles, "colorVars.generated.ts");
  if (
    writeIfChanged(
      colorsFile,
      `${header}\n/** Variáveis de cor do \`tokens.css\`. */\nexport const COLOR_VARS = ${JSON.stringify(colors, null, 2)} as const;\n`,
    )
  )
    changed.push(colorsFile);
  return { colors: colors.length, changed };
}

function syncAll() {
  mkdirSync(join(web, "src", "assets", "brand"), { recursive: true });
  mkdirSync(join(web, "src", "assets", "icons"), { recursive: true });

  const { colors } = syncTokens();

  // Marca.
  for (const file of readdirSync(join(design, "assets", "Logos")).filter((f) => f.endsWith(".png"))) {
    copyFileSync(join(design, "assets", "Logos", file), join(web, "src", "assets", "brand", file));
  }

  // Ícones: o miolo de cada SVG com a cor trocada por currentColor (o `<svg>` é do componente Icon).
  const iconDir = join(design, "assets", "Icones");
  const icons = readdirSync(iconDir)
    .filter((f) => f.endsWith(".svg"))
    .sort();
  const entries = icons.map((file) => {
    const svg = readFileSync(join(iconDir, file), "utf8").trim();
    const body = svg
      .replace(/^<svg[^>]*>/, "")
      .replace(/<\/svg>$/, "")
      .replaceAll("#f4f1f0", "currentColor");
    return `  ${JSON.stringify(file.replace(/\.svg$/, ""))}: ${JSON.stringify(body)},`;
  });
  writeFileSync(
    join(web, "src", "assets", "icons", "icons.generated.ts"),
    `${header}\n/** Miolo de cada SVG de 24×24 (traço e preenchimento em \`currentColor\`). */\nexport const icons = {\n${entries.join("\n")}\n} as const;\n\nexport type IconName = keyof typeof icons;\n`,
  );

  console.log(`tokens, ${icons.length} ícones, ${colors} cores`);
}

// `node scripts/sync-design.mjs` copia tudo; importado (pelo Vite), só expõe `syncTokens`.
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) syncAll();
