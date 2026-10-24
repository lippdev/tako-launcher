import { useEffect, useMemo, useRef, useState } from "react";
import { cx } from "../../lib/cx";
import { ListaDeJogos, type GameRow, type GameStatus, type ListaDeJogosLabels } from "./ListaDeJogos";
import styles from "./ListaDeJogos.demo.module.css";

export const title = "ListaDeJogos";
export const preview = "ListaDeJogos";

const LABELS: ListaDeJogosLabels = {
  running: "Jogando",
  updateAvailable: "Atualização disponível",
  notInstalled: "Não instalado",
  favorite: "Favorito",
  downloading: (percent) => `Baixando · ${percent}%`,
  status: { playing: "Jogando", completed: "Zerado", queued: "Na fila", paused: "Pausado", dropped: "Largado" },
  loading: "Lendo a biblioteca…",
  empty: "Nem com oito braços deu para achar esse.",
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

/** Jogos falsos e sempre iguais (nada de aleatório: a prévia tem de ser repetível). */
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
      ...(i % 4 === 0 ? { hint: "Jogado ontem, 12:38" } : {}),
    };
  });
}

const HUES = [210, 20, 340, 150, 45, 275];

/** Ícone de exemplo (SVG num data URI, só da demo): um degradê com a inicial. Alguns jogos ficam sem imagem de propósito. */
function demoIcon(id: string, px: number): string | null {
  const n = Number(id.split(":")[1]);
  if (n % 7 === 0) return null;
  const hue = HUES[n % HUES.length] ?? 210;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${String(px)}" height="${String(px)}"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${String(hue)} 45% 45%)"/><stop offset="1" stop-color="hsl(${String((hue + 40) % 360)} 50% 18%)"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/** Quantas linhas (`role="option"`) a lista tem no DOM agora. */
function useRenderedRows() {
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

// Abas e busca da lista são de outra tarefa (Abas, CampoDeBusca): aqui só um marcador simples.
function Marker({ children }: { children: string }) {
  return <div className={styles["marker"]}>{children}</div>;
}

export function Demo() {
  const games = useMemo(() => fakeGames(5000), []);
  const [activeId, setActiveId] = useState<string | null>(games[0]?.id ?? null);
  const [selection, setSelection] = useState<ReadonlySet<string>>(new Set());
  const [played, setPlayed] = useState<string | null>(null);
  const { ref, count } = useRenderedRows();
  const small = useMemo(() => fakeGames(13), []);
  const [smallActive, setSmallActive] = useState<string | null>(small[3]?.id ?? null);
  const [smallSelection, setSmallSelection] = useState<ReadonlySet<string>>(new Set());

  return (
    <div className={styles["stages"]}>
      <figure className={styles["figure"]}>
        <figcaption className="overline">LISTA · 5.000 JOGOS (VIRTUALIZADA)</figcaption>
        <div className={styles["column"]} ref={ref}>
          <Marker>
            {selection.size > 0 ? `${String(selection.size)} selecionados de ${String(games.length)}` : "Todos · Favoritos · Instalados"}
          </Marker>
          <ListaDeJogos
            aria-label="Biblioteca"
            rows={games}
            labels={LABELS}
            activeId={activeId}
            selection={selection}
            onActivate={setActiveId}
            onSelectionChange={setSelection}
            onPlay={setPlayed}
            resolveIcon={demoIcon}
          />
        </div>
        <p className="label-sm">
          No DOM agora: <strong data-testid="rendered-rows">{count}</strong> linhas
          {played ? ` · jogar: ${played}` : ""}. Ctrl ou Shift + clique marcam; Esc limpa; Ctrl + A marca tudo; setas, Home e End andam.
        </p>
      </figure>

      <figure className={styles["figure"]}>
        <figcaption className="overline">LISTA · 13 JOGOS</figcaption>
        <div className={styles["column"]}>
          <Marker>
            {smallSelection.size > 0 ? `${String(smallSelection.size)} selecionados de ${String(small.length)}` : "Todos · Favoritos · Instalados"}
          </Marker>
          <ListaDeJogos
            aria-label="Biblioteca (13 jogos)"
            rows={small}
            labels={LABELS}
            activeId={smallActive}
            selection={smallSelection}
            onActivate={setSmallActive}
            onSelectionChange={setSmallSelection}
            resolveIcon={demoIcon}
          />
        </div>
      </figure>

      <figure className={styles["figure"]}>
        <figcaption className="overline">RECOLHIDA</figcaption>
        <div className={cx(styles["column"], styles["narrow"])}>
          <ListaDeJogos
            aria-label="Biblioteca recolhida"
            collapsed
            rows={small}
            labels={LABELS}
            activeId={smallActive}
            selection={new Set()}
            onActivate={setSmallActive}
            onSelectionChange={() => undefined}
            resolveIcon={demoIcon}
          />
        </div>
      </figure>

      <figure className={styles["figure"]}>
        <figcaption className="overline">VAZIA</figcaption>
        <div className={styles["column"]}>
          <Marker>Todos · Favoritos · Instalados</Marker>
          <ListaDeJogos
            aria-label="Busca sem resultado"
            rows={[]}
            labels={LABELS}
            activeId={null}
            selection={new Set()}
            onActivate={() => undefined}
            onSelectionChange={() => undefined}
          />
        </div>
      </figure>

      <figure className={styles["figure"]}>
        <figcaption className="overline">CARREGANDO</figcaption>
        <div className={styles["column"]}>
          <Marker>Lendo a biblioteca…</Marker>
          <ListaDeJogos
            aria-label="Biblioteca"
            loading
            rows={[]}
            labels={LABELS}
            activeId={null}
            selection={new Set()}
            onActivate={() => undefined}
            onSelectionChange={() => undefined}
          />
        </div>
      </figure>
    </div>
  );
}

