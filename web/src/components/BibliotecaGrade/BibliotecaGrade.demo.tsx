import { useEffect, useMemo, useRef, useState } from "react";
import { BibliotecaGrade, type GameRow, type GameStatus, type BibliotecaGradeLabels } from "./BibliotecaGrade";
import styles from "../ListaDeJogos/ListaDeJogos.demo.module.css";

export const title = "BibliotecaGrade";
export const preview = "BibliotecaGrade";

const LABELS: BibliotecaGradeLabels = {
  running: "Jogando",
  updateAvailable: "Atualização disponível",
  notInstalled: "Não instalado",
  favorite: "Favorito",
  downloading: (percent) => `Baixando · ${String(percent)}%`,
  status: { playing: "Jogando", completed: "Zerado", queued: "Na fila", paused: "Pausado", dropped: "Largado" },
  loading: "Lendo a biblioteca…",
  empty: "Nem com oito braços deu para achar esse.",
  play: "Jogar",
};

const NAMES = [
  "God of War: Ragnarök",
  "Dragon's Dogma 2",
  "Watch Dogs 2",
  "Elden Ring",
  "Cyberpunk 2077",
  "Hades II",
  "Baldur's Gate 3",
  "Sekiro: Shadows Die Twice",
  "The Witcher 3: Wild Hunt",
  "Stellar Blade",
  "Black Myth: Wukong",
  "Death Stranding",
  "Ghost of Tsushima Director's Cut",
];

const STATUSES: GameStatus[] = ["none", "playing", "completed", "queued", "paused", "dropped"];
const STORES = ["Steam", "Epic", "GOG"];

/** Jogos falsos determinísticos para a grade virtualizada (repetíveis). */
function fakeGames(count: number): GameRow[] {
  return Array.from({ length: count }, (_, i) => {
    const installed = i % 5 !== 3 && i % 11 !== 7;
    const downloading = !installed && i % 11 === 7;
    return {
      id: `steam:${String(1000 + i)}`,
      title: `${NAMES[i % NAMES.length] ?? "Jogo"}${i >= NAMES.length ? ` ${String(Math.floor(i / NAMES.length) + 1)}` : ""}`,
      store: STORES[i % STORES.length] ?? "Steam",
      installed,
      favorite: i % 9 === 0,
      status: STATUSES[i % STATUSES.length] ?? "none",
      running: i === 1,
      downloadPercent: downloading ? (i * 7) % 100 : null,
      updateAvailable: installed && i % 6 === 4,
      ...(i % 4 === 0 ? { hint: "Jogado hoje, 14:20" } : {}),
    };
  });
}

const HUES = [215, 25, 345, 155, 45, 275];

/** Capa em SVG para demonstração (sem cores cruas em hex ou rgb). */
function demoCover(id: string, width: number, height: number): string | null {
  const parts = id.split(":");
  const n = parts[1] ? Number(parts[1]) : 0;
  if (n % 13 === 0) return null; // Sem imagem de propósito para testar fallback
  const hue = HUES[n % HUES.length] ?? 215;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${String(width)}" height="${String(height)}" viewBox="0 0 300 400"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${String(hue)} 50% 45%)"/><stop offset="1" stop-color="hsl(${String((hue + 45) % 360)} 55% 15%)"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/><circle cx="150" cy="200" r="70" fill="hsl(${String((hue + 30) % 360)} 40% 30%)" fill-opacity="0.5"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** Observa quantas capas (`role="option"`) estão presentes no DOM atualmente. */
function useRenderedOptions() {
  const ref = useRef<HTMLDivElement>(null);
  const [count, setCount] = useState(0);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const read = () => {
      setCount(el.querySelectorAll('[role="option"]').length);
    };
    const observer = new MutationObserver(read);
    observer.observe(el, { childList: true, subtree: true });
    read();
    return () => {
      observer.disconnect();
    };
  }, []);

  return { ref, count };
}

function Marker({ children }: { children: string }) {
  return <div className={styles["marker"]}>{children}</div>;
}

export function Demo() {
  const games = useMemo(() => fakeGames(5000), []);
  const [activeId, setActiveId] = useState<string | null>(games[0]?.id ?? null);
  const [selection, setSelection] = useState<ReadonlySet<string>>(new Set());
  const [played, setPlayed] = useState<string | null>(null);
  const { ref, count } = useRenderedOptions();

  const small = useMemo(() => fakeGames(13), []);
  const [smallActive, setSmallActive] = useState<string | null>(small[1]?.id ?? null);
  const [smallSelection, setSmallSelection] = useState<ReadonlySet<string>>(
    new Set([small[2]?.id ?? "", small[4]?.id ?? ""]),
  );

  return (
    <div className={styles["stages"]}>
      <figure className={styles["figure"]}>
        <figcaption className="overline">GRADE · 5.000 JOGOS (VIRTUALIZADA)</figcaption>
        <div className={styles["column"]} ref={ref}>
          <Marker>
            {selection.size > 0
              ? `${String(selection.size)} selecionados de ${String(games.length)}`
              : "Todos · Favoritos · Instalados"}
          </Marker>
          <BibliotecaGrade
            aria-label="Biblioteca em grade"
            rows={games}
            labels={LABELS}
            activeId={activeId}
            selection={selection}
            onActivate={setActiveId}
            onSelectionChange={setSelection}
            onPlay={setPlayed}
            resolveCover={demoCover}
          />
        </div>
        <p className="label-sm">
          No DOM agora: <strong data-testid="rendered-rows">{count}</strong> capas
          {played ? ` · jogar: ${played}` : ""}. Setas navegam na grade 3×N; Enter abre; Ctrl/Shift + clique selecionam; Esc limpa.
        </p>
      </figure>

      <figure className={styles["figure"]}>
        <figcaption className="overline">ESTADOS · ESCOLHIDO, JOGANDO, BAIXANDO, SELEÇÃO</figcaption>
        <div className={styles["column"]}>
          <Marker>
            {smallSelection.size > 0
              ? `${String(smallSelection.size)} selecionados`
              : "Todos · Favoritos · Instalados"}
          </Marker>
          <BibliotecaGrade
            aria-label="Demonstração de estados"
            rows={small}
            labels={LABELS}
            activeId={smallActive}
            selection={smallSelection}
            onActivate={setSmallActive}
            onSelectionChange={setSmallSelection}
            onPlay={setPlayed}
            resolveCover={demoCover}
          />
        </div>
        <p className="label-sm">
          Estados exibidos: Dragon&apos;s Dogma 2 (jogando), Watch Dogs 2 e Cyberpunk 2077 (selecionados), Black Myth: Wukong (não instalado), Baldur&apos;s Gate 3 (atualização disponível).
        </p>
      </figure>
    </div>
  );
}
