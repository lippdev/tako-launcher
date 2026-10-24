import { useState } from "react";
import {
  BibliotecaCategorias,
  type GameCategoryGroup,
  type CategoryGame,
} from "./BibliotecaCategorias";

export const title = "BibliotecaCategorias";
export const preview = "BibliotecaCategorias";

const HUES: Record<string, number> = {
  gow: 205,
  dd2: 30,
  hades2: 345,
  cp: 45,
  elden: 195,
  bg3: 220,
  wukong: 25,
  ds: 180,
  hades: 350,
  starfield: 215,
  wd2: 145,
  rdr2: 15,
};

/** Capa de exemplo em SVG determinística sem cores cruas em hex ou rgb. */
function demoCover(id: string, width: number, height: number): string {
  const hue = HUES[id] ?? 210;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${String(width)}" height="${String(height)}" viewBox="0 0 300 400"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${String(hue)} 55% 45%)"/><stop offset="1" stop-color="hsl(${String((hue + 50) % 360)} 55% 15%)"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/><circle cx="150" cy="200" r="75" fill="hsl(${String((hue + 35) % 360)} 40% 30%)" fill-opacity="0.5"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

function game(id: string, gameTitle: string, hint?: string): CategoryGame {
  return {
    id,
    title: gameTitle,
    hint: hint ?? null,
    coverUrl: demoCover(id, 300, 400),
  };
}

const DEMO_GROUPS: readonly GameCategoryGroup[] = [
  {
    id: "jogando",
    title: "Jogando",
    games: [
      game("gow", "God of War: Ragnarök", "Jogado hoje, 16:17"),
      game("dd2", "Dragon's Dogma 2", "Jogado hoje, 04:57"),
      game("hades2", "Hades II", "Jogado em 14 out"),
    ],
  },
  {
    id: "na-fila",
    title: "Na fila",
    games: [
      game("cp", "Cyberpunk 2077", "Jogado em 17 out"),
      game("elden", "Elden Ring", "Jogado em 20 out"),
      game("bg3", "Baldur's Gate 3", "Atualização disponível"),
      game("wukong", "Black Myth: Wukong", "Instalado"),
      game("ds", "Death Stranding", "62 GB"),
    ],
  },
  {
    id: "zerados",
    title: "Zerados",
    games: [
      game("hades", "Hades", "100% conquistas"),
      game("starfield", "Starfield", "Zerado em 12 set"),
      game("wd2", "Watch Dogs 2", "Zerado em 21 out"),
      game("rdr2", "Red Dead Redemption 2", "Zerado em 28 ago"),
    ],
  },
];

export function Demo() {
  const [status, setStatus] = useState("Nenhum jogo selecionado");

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-24)", width: "100%" }}>
      <p className="label-sm" data-testid="categoria-status">
        Interação: <strong>{status}</strong>. Cada grupo possui título, contagem e setas ‹ › para navegação horizontal (7 capas por largura).
      </p>

      <BibliotecaCategorias
        groups={DEMO_GROUPS}
        labels={{
          previous: "Anterior",
          next: "Próximo",
          gamesCount: (n) => `${String(n)} ${n === 1 ? "jogo" : "jogos"}`,
          empty: "Nenhuma categoria para exibir.",
          region: "Biblioteca por categorias",
        }}
        onOpen={(id) => {
          setStatus(`Abrir jogo ${id}`);
        }}
        onHighlight={(id) => {
          setStatus(`Destacar jogo ${id}`);
        }}
      />
    </div>
  );
}
