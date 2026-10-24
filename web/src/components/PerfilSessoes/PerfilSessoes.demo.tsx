import { useState } from "react";
import type { Aba } from "../Abas/Abas";
import type { OpcaoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { PerfilSessoes, type SessionGroup, type SessionsWeek, type HistorySession } from "./PerfilSessoes";

export const title = "PerfilSessoes";
export const preview = "PerfilSessoes";

// Capas de demonstração em SVG vetorial leve (permitido em .demo.tsx)
const CAPA_GOW = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 520" width="400" height="520">
  <defs>
    <linearGradient id="g1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141a29"/>
      <stop offset="100%" stop-color="#2a3b5c"/>
    </linearGradient>
  </defs>
  <rect width="400" height="520" fill="url(#g1)"/>
  <circle cx="200" cy="220" r="100" fill="#3ddc84" opacity="0.3"/>
  <text x="200" y="270" font-family="system-ui" font-size="28" font-weight="bold" fill="#ffffff" text-anchor="middle">GOW</text>
</svg>
`)}`;

const CAPA_HADES = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 520" width="400" height="520">
  <defs>
    <linearGradient id="g2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#2e141a"/>
      <stop offset="100%" stop-color="#5c202a"/>
    </linearGradient>
  </defs>
  <rect width="400" height="520" fill="url(#g2)"/>
  <circle cx="200" cy="220" r="100" fill="#ff7b7b" opacity="0.3"/>
  <text x="200" y="270" font-family="system-ui" font-size="28" font-weight="bold" fill="#ffffff" text-anchor="middle">HADES II</text>
</svg>
`)}`;

const CAPA_DD2 = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 520" width="400" height="520">
  <defs>
    <linearGradient id="g3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#222019"/>
      <stop offset="100%" stop-color="#4d4430"/>
    </linearGradient>
  </defs>
  <rect width="400" height="520" fill="url(#g3)"/>
  <circle cx="200" cy="220" r="100" fill="#ffcf4d" opacity="0.3"/>
  <text x="200" y="270" font-family="system-ui" font-size="28" font-weight="bold" fill="#ffffff" text-anchor="middle">DD2</text>
</svg>
`)}`;

const CAPA_CP = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 520" width="400" height="520">
  <defs>
    <linearGradient id="g4" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#28220f"/>
      <stop offset="100%" stop-color="#544716"/>
    </linearGradient>
  </defs>
  <rect width="400" height="520" fill="url(#g4)"/>
  <circle cx="200" cy="220" r="100" fill="#ffd000" opacity="0.4"/>
  <text x="200" y="270" font-family="system-ui" font-size="28" font-weight="bold" fill="#ffffff" text-anchor="middle">CP2077</text>
</svg>
`)}`;

const ABAS_PERIODO: readonly Aba<"semana" | "mes" | "tudo">[] = [
  { id: "semana", label: "Esta semana" },
  { id: "mes", label: "Este mês" },
  { id: "tudo", label: "Tudo" },
];

const OPCOES_JOGOS: readonly OpcaoDeSelecao<string>[] = [
  { value: "todos", label: "Todos os jogos" },
  { value: "gow", label: "God of War: Ragnarök" },
  { value: "hades2", label: "Hades II" },
  { value: "dd2", label: "Dragon's Dogma 2" },
  { value: "cp2077", label: "Cyberpunk 2077" },
];

const SEMANA_EXEMPLO: SessionsWeek = {
  label: "Tempo jogado",
  total: "19 h",
  ariaLabel: "Gráfico de horas jogadas na semana: pico no domingo com 95%",
  days: [
    { label: "S", value: 0.8 },
    { label: "T", value: 0.15 },
    { label: "Q", value: 0.35 },
    { label: "Q", value: 0.3 },
    { label: "S", value: 0.4 },
    { label: "S", value: 0.55 },
    { label: "D", value: 0.95 },
  ],
};

const GRUPOS_PADRAO: readonly SessionGroup[] = [
  {
    id: "hoje",
    title: "Hoje",
    subtitle: "22 out",
    total: "1 h 54 min",
    sessions: [
      {
        id: "s1",
        game: "God of War: Ragnarök",
        coverSrc: CAPA_GOW,
        time: "14:02 – agora",
        duration: "42 min",
        status: { kind: "playing", text: "Jogando agora" },
      },
      {
        id: "s2",
        game: "Hades II",
        coverSrc: CAPA_HADES,
        time: "09:10 – 10:22",
        duration: "1 h 12 min",
        status: { kind: "normal", text: "Fechou normalmente" },
      },
    ],
  },
  {
    id: "ontem",
    title: "Ontem",
    subtitle: "21 out",
    total: "2 h 31 min",
    sessions: [
      {
        id: "s3",
        game: "Dragon's Dogma 2",
        coverSrc: CAPA_DD2,
        time: "20:40 – 22:59",
        duration: "2 h 19 min",
        status: { kind: "normal", text: "Fechou normalmente" },
      },
      {
        id: "s4",
        game: "Cyberpunk 2077",
        coverSrc: CAPA_CP,
        time: "19:58 – 20:10",
        duration: "12 min",
        status: { kind: "error", text: "Fechou com erro", linkText: "Ver relatório" },
      },
    ],
  },
];

export function Demo() {
  const [periodo, setPeriodo] = useState<"semana" | "mes" | "tudo">("semana");
  const [jogo, setJogo] = useState("todos");
  const [relatorioAberto, setRelatorioAberto] = useState<string | null>(null);

  const handleVerRelatorio = (session: HistorySession) => {
    setRelatorioAberto(`Relatório de erro solicitado para: ${session.game} (${session.time})`);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-32)", width: "100%", maxWidth: 1040 }}>
      {/* 1. Estado completo fiel ao protótipo PerfilSessoes (com barras da semana e filtros) */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          HISTÓRICO DE SESSÕES (COM TEMPO DA SEMANA EM BARRAS)
        </h3>
        {relatorioAberto && (
          <div
            style={{
              padding: "var(--space-12) var(--space-16)",
              borderRadius: "var(--radius-control)",
              background: "var(--surface-raised)",
              border: "1px solid var(--warning)",
              color: "var(--warning)",
              fontSize: 13,
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            <span>{relatorioAberto}</span>
            <button
              type="button"
              onClick={() => {
                setRelatorioAberto(null);
              }}
              style={{
                background: "transparent",
                border: "none",
                color: "var(--text-muted)",
                cursor: "pointer",
                textDecoration: "underline",
              }}
            >
              Fechar
            </button>
          </div>
        )}
        <PerfilSessoes
          week={SEMANA_EXEMPLO}
          period={{
            label: "Período",
            tabs: ABAS_PERIODO,
            value: periodo,
            onChange: setPeriodo,
          }}
          gameFilter={{
            label: "Jogo",
            options: OPCOES_JOGOS,
            value: jogo,
            onChange: setJogo,
          }}
          summary="6 sessões · 6 h 37 min"
          groups={GRUPOS_PADRAO}
          onOpenReport={handleVerRelatorio}
        />
      </section>

      {/* 2. Sem o bloco de semana (apenas filtros e sessões) */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          APENAS LISTA DE SESSÕES (SEM O BLOCO DA SEMANA)
        </h3>
        <PerfilSessoes
          period={{
            label: "Período",
            tabs: ABAS_PERIODO,
            value: "tudo",
            onChange: () => undefined,
          }}
          gameFilter={{
            label: "Jogo",
            options: OPCOES_JOGOS,
            value: "todos",
            onChange: () => undefined,
          }}
          summary="4 sessões · 4 h 25 min"
          groups={GRUPOS_PADRAO}
          onOpenReport={handleVerRelatorio}
        />
      </section>

      {/* 3. Estado vazio (sem sessões no período) */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          ESTADO VAZIO (NENHUMA SESSÃO)
        </h3>
        <PerfilSessoes
          period={{
            label: "Período",
            tabs: ABAS_PERIODO,
            value: "mes",
            onChange: () => undefined,
          }}
          gameFilter={{
            label: "Jogo",
            options: OPCOES_JOGOS,
            value: "todos",
            onChange: () => undefined,
          }}
          summary="0 sessões · 0 h"
          groups={[]}
          emptyMessage="Nenhuma sessão encontrada para este período ou jogo selecionado."
        />
      </section>

      {/* 4. Sem dado nenhum (o bloco não renderiza) */}
      <section style={{ display: "flex", flexDirection: "column", gap: "var(--space-12)" }}>
        <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
          SEM DADOS (O BLOCO SOME)
        </h3>
        <PerfilSessoes />
        <p style={{ margin: 0, fontSize: 13, color: "var(--text-dim)" }}>
          (O componente recebeu props vazias e retornou null com sucesso)
        </p>
      </section>
    </div>
  );
}
