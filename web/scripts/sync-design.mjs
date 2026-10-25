// Copia a base visual de `design/desktop/` (só leitura) para `web/src/`, para o build não depender
// da pasta de design: tokens, fontes (Poppins e Plus Jakarta Sans), marca e ícones.
// Rode `npm run sync:design` quando o design system mudar e commite o resultado.

import { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const web = join(dirname(fileURLToPath(import.meta.url)), "..");
const design = join(web, "..", "design", "desktop");
const styles = join(web, "src", "styles");
const header = "// Gerado por scripts/sync-design.mjs a partir de design/desktop/. Não edite à mão.\n";

mkdirSync(join(styles, "fonts"), { recursive: true });
mkdirSync(join(web, "src", "assets", "brand"), { recursive: true });
mkdirSync(join(web, "src", "assets", "icons"), { recursive: true });

// Tokens: cópia fiel; o `@font-face` aponta para `fonts/` ao lado do arquivo.
const tokens = readFileSync(join(design, "tokens.css"), "utf8");
writeFileSync(join(styles, "tokens.css"), `/* Cópia de design/desktop/tokens.css (scripts/sync-design.mjs). Não edite à mão. */\n${tokens}`);

// Fontes: só as famílias que o tokens.css declara (Source Sans 3 fica de fora).
for (const family of tokens.matchAll(/url\("fonts\/([^"]+)"\)/g)) {
  copyFileSync(join(design, "fonts", family[1]), join(styles, "fonts", family[1]));
}

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

// Variáveis de cor do tema base (valores `#…`), para a página de desenvolvimento listar a paleta.
const base = tokens.match(/:root, \[data-theme="tako"\] \{([\s\S]*?)\n\}/)?.[1] ?? "";
const colors = [...base.matchAll(/^\s*(--[\w-]+):\s*#[0-9a-fA-F]+;/gm)].map((m) => m[1]);
writeFileSync(
  join(styles, "colorVars.generated.ts"),
  `${header}\n/** Variáveis de cor do \`tokens.css\`. */\nexport const COLOR_VARS = ${JSON.stringify(colors, null, 2)} as const;\n`,
);

console.log(`tokens, ${icons.length} ícones, ${colors.length} cores`);
