import { useEffect, useRef, useState, type ReactNode } from "react";
import { call, on } from "../../bridge";
import { EntradaDoApp } from "../../components/EntradaDoApp/EntradaDoApp";
import { useSavedAppearance } from "../settings/useSavedAppearance";
import { SCAN_LIMIT_MS, decideAbertura, remainingHold, type Abertura as Tipo, type ScanProgress } from "./model";
import { PrimeiraLeitura } from "./PrimeiraLeitura";
import styles from "./Abertura.module.css";

const TICK_MS = 50;

const noScan: ScanProgress = { scanning: false, completed: 0, total: 0 };

function reduceMotion(): boolean {
  return document.documentElement.dataset["reduceMotion"] === "true";
}

/**
 * A abertura do app (paridade com o Slint, ver `model.ts`): enquanto o núcleo responde e a biblioteca chega só o
 * fundo; depois a tela de primeira leitura (biblioteca vazia e leitura em andamento) ou a `EntradaDoApp`, que sai com a
 * animação do design system. Ao reabrir a interface depois de um jogo (`ui.state` já guardado) o app aparece direto.
 */
export function Abertura({ children }: { children: ReactNode }) {
  // A aparência salva vem antes de decidir: a entrada lê "Reduzir animações" ao montar.
  const appearanceReady = useSavedAppearance();
  const [kind, setKind] = useState<Tipo | null>(null);
  const [scan, setScan] = useState<ScanProgress>(noScan);
  const [reading, setReading] = useState(true);
  const scanRef = useRef(scan);

  useEffect(() => {
    scanRef.current = scan;
  }, [scan]);

  // Sonda do núcleo: o app responde, a biblioteca chega e a leitura das lojas está em que pé.
  useEffect(() => {
    let cancelled = false;
    const off = on("library.scan.progress", (sources) => {
      setScan({ scanning: sources.scanning, completed: sources.completed, total: sources.total });
    });
    void (async () => {
      // Reabrindo (o núcleo já tem `ui.state`) o app aparece direto, sem esperar a biblioteca.
      const state = await call("ui.state").catch(() => ({ json: null }));
      if (cancelled) return;
      if (state.json !== null) {
        setKind("nenhuma");
        return;
      }
      const [page, sources] = await Promise.all([
        call("library.list", { limit: 1 }).catch(() => ({ total: 0 })),
        call("library.sources").catch(() => null),
      ]);
      if (cancelled) return;
      const progress = sources
        ? { scanning: sources.scanning, completed: sources.completed, total: sources.total }
        : noScan;
      setScan(progress);
      setKind(decideAbertura({ hasSavedState: false, libraryCount: page.total, scanning: progress.scanning }));
    })();
    return () => {
      cancelled = true;
      off();
    };
  }, []);

  const decided = kind !== null && appearanceReady;

  // Primeira leitura: segue o progresso e fecha quando acaba (ou aos 15 s) mais o tempo de assentar.
  const showReading = decided && kind === "primeira-leitura" && reading;
  useEffect(() => {
    if (!showReading) return;
    const started = performance.now();
    let doneAt: number | null = null;
    const timer = window.setInterval(() => {
      const now = performance.now();
      const elapsedMs = now - started;
      const { scanning } = scanRef.current;
      if (doneAt === null && (!scanning || elapsedMs >= SCAN_LIMIT_MS)) doneAt = now;
      const left = remainingHold({
        scanning,
        elapsedMs,
        doneForMs: doneAt === null ? null : now - doneAt,
        reduceMotion: reduceMotion(),
      });
      if (left === 0) setReading(false);
    }, TICK_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [showReading]);

  if (!decided) return <div className={styles["fundo"]} />;
  if (showReading) return <PrimeiraLeitura scan={scan} />;
  if (kind === "nenhuma") return <>{children}</>;
  return <EntradaDoApp fixed>{children}</EntradaDoApp>;
}
