// Regras da abertura do app (WEB-35), sem React nem ponte para o teste rodar direto no Node.
//
// Como o Slint faz (`crates/tako-ui/src/splash.rs` e `app.rs`):
// - a janela de progresso só abre com a biblioteca vazia (primeira leitura das lojas), fora da volta de um jogo;
// - o texto é "Procurando seus jogos…" enquanto lê e "Iniciando o Tako…" quando acaba; a barra mostra lojas lidas
//   sobre o total, e corre sem número até a primeira loja ser lida;
// - ela fica até a leitura acabar, no máximo 15 s (uma leitura travada não prende a abertura), e segura mais 1 s
//   (0 com "Reduzir animações") para os últimos tentáculos assentarem;
// - depois vem a entrada do app (`EntradaDoApp`), que acaba sozinha.
//
// Ao voltar de um jogo (a interface descartável reabre, D-10) a abertura não repete: o `ui.state` vive no núcleo e
// só existe depois que alguém usou a interface neste processo, então a presença dele é o sinal de "não é a
// primeira abertura".

/** Uma leitura travada não prende a abertura além disto; ela continua atrás. */
export const SCAN_LIMIT_MS = 15_000;
/** Depois da leitura pronta, a tela fica mais isto antes de abrir o app. */
export const HOLD_AFTER_SCAN_MS = 1000;

export type Abertura = "primeira-leitura" | "entrada" | "nenhuma";

export interface AberturaInput {
  /** O núcleo já tem `ui.state` guardado: a interface está reabrindo. */
  hasSavedState: boolean;
  /** Total de jogos da biblioteca agora. */
  libraryCount: number;
  /** A leitura das lojas está em andamento. */
  scanning: boolean;
}

/** Qual abertura mostrar. Reabrir nunca repete; biblioteca vazia com leitura em andamento mostra o progresso. */
export function decideAbertura({ hasSavedState, libraryCount, scanning }: AberturaInput): Abertura {
  if (hasSavedState) return "nenhuma";
  if (libraryCount === 0 && scanning) return "primeira-leitura";
  return "entrada";
}

export interface ScanProgress {
  scanning: boolean;
  completed: number;
  total: number;
}

export interface ScanView {
  /** Chave do catálogo do texto de estado. */
  textKey: "first_run.splash.scanning" | "first_run.splash.starting";
  /** 0 a 1; `null` = sem número ainda (a barra corre). */
  progress: number | null;
  /** "42%" ou vazio. */
  percent: string;
}

/** O que a tela de primeira leitura mostra: só o progresso de verdade (lojas lidas sobre o total). */
export function scanView({ scanning, completed, total }: ScanProgress): ScanView {
  if (!scanning) return { textKey: "first_run.splash.starting", progress: 1, percent: "" };
  const permille = total > 0 ? Math.min(1000, Math.max(0, Math.floor((completed * 1000) / total))) : 0;
  if (permille === 0) return { textKey: "first_run.splash.scanning", progress: null, percent: "" };
  return {
    textKey: "first_run.splash.scanning",
    progress: permille / 1000,
    percent: `${String(Math.floor(permille / 10))}%`,
  };
}

/** Quanto falta para a tela de primeira leitura fechar; `null` enquanto a leitura segue e o limite não chegou. */
export function remainingHold(opts: {
  scanning: boolean;
  elapsedMs: number;
  /** Há quanto tempo a leitura acabou (ou o limite estourou); `null` se ainda não. */
  doneForMs: number | null;
  reduceMotion: boolean;
}): number | null {
  const busy = opts.scanning && opts.elapsedMs < SCAN_LIMIT_MS;
  if (busy || opts.doneForMs === null) return null;
  const hold = opts.reduceMotion ? 0 : HOLD_AFTER_SCAN_MS;
  return Math.max(0, hold - opts.doneForMs);
}
