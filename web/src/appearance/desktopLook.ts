// A cor de destaque e as opções do Liquid Glass (Configurações › Tema, UX-007), em funções puras e sem importações em
// execução, para o teste rodar direto no Node. As regras e os números são os do app Slint (`tako-ui/src/desk_theme.rs`),
// para os dois apps lerem as mesmas chaves do mesmo jeito.

/** O destaque do TAKO sem arte: a pedra do logo. */
export const STONE = "#D8C8BC";

/** As nove cores prontas, na ordem do Slint (`desk_theme::ACCENTS`); `id` é o do nome no catálogo. */
export const ACCENT_CHOICES = [
  { id: "stone", hex: STONE },
  { id: "orange", hex: "#FF8A3D" },
  { id: "gold", hex: "#FFC94D" },
  { id: "green", hex: "#3DDC84" },
  { id: "cyan", hex: "#3DD6E0" },
  { id: "blue", hex: "#4D9BFF" },
  { id: "violet", hex: "#9B7BFF" },
  { id: "pink", hex: "#FF5D9E" },
  { id: "white", hex: "#E8E8E8" },
] as const;

export type AccentId = (typeof ACCENT_CHOICES)[number]["id"];

/** O destaque dos outros temas sem escolha: o azul. */
const BLUE = "#4D9BFF";

/** Posição do deslizante de opacidade quando nada foi gravado. */
export const GLASS_OPACITY_DEFAULT = 28;

/** A cor escolhida, ou a do tema: pedra no TAKO e azul nos outros. */
export function accentOrDefault(chosen: string | null, theme: string): string {
  return chosen ?? (theme === "tako" ? STONE : BLUE);
}

/** A cor pronta que é esta (sem olhar a caixa), ou `null` para uma cor própria. */
export function presetOf(hex: string): AccentId | null {
  return ACCENT_CHOICES.find((choice) => choice.hex === hex.toUpperCase())?.id ?? null;
}

/** Opacidade das superfícies do vidro em %, de 2 a 20 (conta inteira, como o Slint). */
export function glassAlpha(position: number): number {
  return 2 + Math.floor((Math.min(100, Math.max(0, position)) * 18) / 100);
}

/** A luma que o Slint usa para escolher o texto sobre o destaque: 0,299 R + 0,587 G + 0,114 B. */
export function isLightAccent(hex: string): boolean {
  const value = Number.parseInt(hex.slice(1), 16);
  const luma = (0.299 * ((value >> 16) & 255) + 0.587 * ((value >> 8) & 255) + 0.114 * (value & 255)) / 255;
  return luma > 0.6;
}

export interface DesktopLook {
  /** O tema da interface (`tako`, `vidro`…). */
  theme: string;
  /** A cor escolhida (`#RRGGBB`), ou `null` = a do tema. Só vale fora do TAKO, onde o destaque vem da arte do jogo. */
  accent: string | null;
  /** "Efeitos de vidro": desligado, o Liquid Glass é opaco e a opacidade não conta. */
  glassEffects: boolean;
  /** Posição do deslizante de opacidade (0 a 100). */
  glassOpacity: number;
  /** "Controles em pílula". */
  glassPill: boolean;
  /** "Arte do jogo como luz ambiente": desligado, uma aurora na cor de destaque. */
  glassAmbient: boolean;
}

/** As variáveis do `tokens.css` que a escolha troca (`set`) e as que voltam ao valor do tema (`clear`). */
export interface LookVariables {
  set: Readonly<Record<string, string>>;
  clear: readonly string[];
}

const ACCENT_VARS = ["--accent", "--on-accent"] as const;
const OPACITY_VARS = ["--glass-panel", "--surface", "--group", "--tile", "--glass-control"] as const;
const PILL_VARS = ["--radius-control", "--radius-button", "--radius-tab", "--radius-search", "--radius-chip"] as const;
const AMBIENT_VARS = ["--ambient-1", "--ambient-2"] as const;

/** Branco com `percent` % de opacidade, a conta dos valores do Liquid Glass no `tokens.css` (#ffffff12 = 7%). */
function white(percent: number): string {
  return `rgb(255 255 255 / ${String(Math.round(percent * 10) / 10)}%)`;
}

/**
 * O que a escolha muda nas variáveis do tema. Só as variáveis do tema mudam, nenhuma cor nova nos componentes:
 * - o destaque escolhido vale fora do TAKO, com o texto sobre ele claro ou escuro pela luma;
 * - no Liquid Glass: a opacidade das superfícies (só com os efeitos de vidro ligados e fora do padrão), os raios dos
 *   controles em pílula e, sem a luz ambiente, a aurora na cor de destaque.
 */
export function lookVariables(look: DesktopLook): LookVariables {
  const set: Record<string, string> = {};
  if (look.theme !== "tako" && look.accent) {
    set["--accent"] = look.accent;
    set["--on-accent"] = isLightAccent(look.accent) ? "var(--on-primary)" : "var(--text)";
  }
  if (look.theme === "vidro") {
    if (look.glassEffects && look.glassOpacity !== GLASS_OPACITY_DEFAULT) {
      // Tudo relativo ao padrão (7%): painel, grupo e superfície valem 1, o bloco 5/7 e o controle 10/7.
      const base = glassAlpha(look.glassOpacity);
      const standard = glassAlpha(GLASS_OPACITY_DEFAULT);
      for (const name of ["--glass-panel", "--surface", "--group"]) set[name] = white(base);
      set["--tile"] = white((base * 5) / standard);
      set["--glass-control"] = white((base * 10) / standard);
    }
    if (look.glassPill) {
      for (const name of PILL_VARS) set[name] = "var(--radius-full)";
    }
    if (!look.glassAmbient) {
      set["--ambient-1"] = "color-mix(in srgb, var(--accent) 8%, transparent)";
      set["--ambient-2"] = "color-mix(in srgb, var(--accent) 4%, transparent)";
    }
  }
  const all = [...ACCENT_VARS, ...OPACITY_VARS, ...PILL_VARS, ...AMBIENT_VARS];
  return { set, clear: all.filter((name) => !(name in set)) };
}
