import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..', '..');
const localesRoot = path.join(repoRoot, 'crates', 'tako-core', 'locales');

export function parseToml(src) {
  const result = {};
  let currentSection = '';
  const lines = src.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const rawLine = lines[i];
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    if (line.startsWith('[') && line.endsWith(']')) {
      currentSection = line.slice(1, -1).trim();
      continue;
    }
    const eqIdx = line.indexOf('=');
    if (eqIdx !== -1) {
      const rawKey = line.slice(0, eqIdx).trim();
      const rawVal = line.slice(eqIdx + 1).trim();
      let val;
      if (rawVal.startsWith('"') && rawVal.endsWith('"')) {
        try {
          val = JSON.parse(rawVal);
        } catch {
          // If JSON.parse fails (e.g. unescaped character), do basic unescape
          val = rawVal.slice(1, -1).replace(/\\"/g, '"').replace(/\\\\/g, '\\');
        }
      } else {
        val = rawVal;
      }
      const fullKey = currentSection ? `${currentSection}.${rawKey}` : rawKey;
      result[fullKey] = val;
    }
  }
  return result;
}

export function loadCatalog(lang) {
  const dir = path.join(localesRoot, lang);
  const files = fs.readdirSync(dir).filter(f => f.endsWith('.toml')).sort();
  const catalog = {};
  for (const f of files) {
    const content = fs.readFileSync(path.join(dir, f), 'utf-8');
    const parsed = parseToml(content);
    for (const [k, v] of Object.entries(parsed)) {
      catalog[k] = v;
    }
  }
  return catalog;
}

export function generate() {
  const pt = loadCatalog('pt-BR');
  const en = loadCatalog('en-US');

  // Check matching
  const ptKeys = Object.keys(pt);
  const enKeys = new Set(Object.keys(en));
  const missingInEn = ptKeys.filter(k => !enKeys.has(k));
  const ptSet = new Set(ptKeys);
  const missingInPt = Object.keys(en).filter(k => !ptSet.has(k));

  if (missingInEn.length > 0 || missingInPt.length > 0) {
    console.error('Catalog mismatch!');
    if (missingInEn.length > 0) console.error('Missing in en-US:', missingInEn);
    if (missingInPt.length > 0) console.error('Missing in pt-BR:', missingInPt);
    process.exit(1);
  }

  const outPath = path.resolve(__dirname, '..', 'src', 'bridge', 'generatedCatalogs.ts');
  const fileContent = `// Gerado automaticamente por \`web/scripts/build-catalogs.mjs\`. Não editar manualmente.
export const SAMPLE_CATALOGS: Record<"pt-BR" | "en-US", Record<string, string>> = {
  "pt-BR": ${JSON.stringify(pt, null, 2)},
  "en-US": ${JSON.stringify(en, null, 2)},
};
`;

  fs.writeFileSync(outPath, fileContent, 'utf-8');
  console.log(`Generated ${outPath} with ${ptKeys.length} keys per language.`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  generate();
}
