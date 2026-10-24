import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { PainelDownloads, type ItemDownload } from "./PainelDownloads";

export const title = "PainelDownloads";
export const preview = "PainelDownloads";

const CAPA_WD2 = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 92" width="72" height="92">
  <rect width="72" height="92" fill="midnightblue" rx="10"/>
  <circle cx="36" cy="40" r="18" fill="cornflowerblue" opacity="0.8"/>
  <text x="36" y="78" font-family="sans-serif" font-size="10" font-weight="bold" fill="white" text-anchor="middle">WD2</text>
</svg>
`)}`;

const CAPA_CP77 = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 92" width="72" height="92">
  <rect width="72" height="92" fill="darkslateblue" rx="10"/>
  <polygon points="36,24 50,56 22,56" fill="coral"/>
  <text x="36" y="78" font-family="sans-serif" font-size="10" font-weight="bold" fill="white" text-anchor="middle">CP77</text>
</svg>
`)}`;

const CAPA_HADES = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 92" width="72" height="92">
  <rect width="72" height="92" fill="darkred" rx="10"/>
  <circle cx="36" cy="42" r="16" fill="gold" opacity="0.7"/>
  <text x="36" y="78" font-family="sans-serif" font-size="10" font-weight="bold" fill="white" text-anchor="middle">HADES</text>
</svg>
`)}`;

const CAPA_HK = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 92" width="72" height="92">
  <rect width="72" height="92" fill="teal" rx="10"/>
  <circle cx="36" cy="38" r="14" fill="white"/>
  <text x="36" y="78" font-family="sans-serif" font-size="10" font-weight="bold" fill="white" text-anchor="middle">HK</text>
</svg>
`)}`;

export function Demo() {
  const [aberto, setAberto] = useState(false);
  const [anchor, setAnchor] = useState<HTMLButtonElement | null>(null);

  // Estado interativo com callbacks
  const [itens, setItens] = useState<ItemDownload[]>([
    {
      id: "int-wd2",
      titulo: "Watch Dogs 2",
      estado: "baixando",
      progresso: 56,
      tamanho: "60.9 / 108.7 GB",
      velocidade: "12 MB/s",
      tempoRestante: "15 min",
      capa: CAPA_WD2,
    },
    {
      id: "int-cp77",
      titulo: "Cyberpunk 2077",
      estado: "na_fila",
      tamanho: "83.9 GB",
      capa: CAPA_CP77,
    },
    {
      id: "int-hk",
      titulo: "Hollow Knight",
      estado: "concluido",
      tamanho: "9.2 GB",
      capa: CAPA_HK,
    },
  ]);

  const handlePausar = (id: string) => {
    setItens((prev) =>
      prev.map((it) => (it.id === id ? { ...it, estado: "pausado" as const } : it))
    );
  };

  const handleRetomar = (id: string) => {
    setItens((prev) =>
      prev.map((it) => (it.id === id ? { ...it, estado: "baixando" as const } : it))
    );
  };

  const handleCancelar = (id: string) => {
    setItens((prev) => prev.filter((it) => it.id !== id));
  };

  const handleAbrirJogo = () => {
    // Ação simulada de abrir o jogo
  };

  return (
    <div>
      <DemoGrupo titulo="Interativo — clique para abrir o painel como camada (posicionado 6px abaixo do botão)">
        <button
          ref={setAnchor}
          type="button"
          className="tb-btn"
          style={{
            height: "36px",
            padding: "0 14px",
            borderRadius: "9px",
            border: "1px solid var(--line-strong)",
            background: "var(--surface)",
            color: "var(--text)",
            cursor: "pointer",
          }}
          onClick={() => setAberto((prev) => !prev)}
        >
          {aberto ? "Fechar PainelDownloads" : "Abrir PainelDownloads"}
        </button>

        <PainelDownloads
          open={aberto}
          onClose={() => setAberto(false)}
          anchor={anchor}
          itens={itens}
          onPausar={handlePausar}
          onRetomar={handleRetomar}
          onCancelar={handleCancelar}
          onAbrirJogo={handleAbrirJogo}
        />
        <p className="label-sm" style={{ margin: "8px 0 0" }}>
          Esc fecha a camada, Tab fica preso dentro, o foco volta para o botão que abriu.
        </p>
      </DemoGrupo>

      <DemoGrupo titulo="1. Estado Vazio (sem downloads ativos)">
        <PainelDownloads inline itens={[]} />
      </DemoGrupo>

      <DemoGrupo titulo="2. Um baixando (com tamanho, velocidade, tempo restante e barra de progresso)">
        <PainelDownloads
          inline
          itens={[
            {
              id: "demo-wd2-solo",
              titulo: "Watch Dogs 2",
              estado: "baixando",
              progresso: 56,
              tamanho: "60.9 / 108.7 GB",
              velocidade: "12 MB/s",
              tempoRestante: "15 min",
              capa: CAPA_WD2,
            },
          ]}
          onPausar={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="3. Vários com fila e tarefas em segundo plano (com e sem capa)">
        <PainelDownloads
          inline
          itens={[
            {
              id: "demo-wd2-multi",
              titulo: "Watch Dogs 2",
              estado: "baixando",
              progresso: 56,
              tamanho: "60.9 / 108.7 GB",
              velocidade: "12 MB/s",
              tempoRestante: "15 min",
              capa: CAPA_WD2,
            },
            {
              id: "demo-tarefa-sync",
              titulo: "Sincronizando nuvem",
              estado: "baixando",
              progresso: null,
              velocidade: null,
              tempoRestante: null,
              tamanho: null,
            },
            {
              id: "demo-cp77-multi",
              titulo: "Cyberpunk 2077",
              estado: "na_fila",
              tamanho: "83.9 GB",
              capa: CAPA_CP77,
            },
            {
              id: "demo-hades-multi",
              titulo: "Hades II",
              estado: "na_fila",
              tamanho: "12.4 GB",
              capa: CAPA_HADES,
            },
          ]}
          onPausar={() => {}}
          onCancelar={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="4. Pausado">
        <PainelDownloads
          inline
          itens={[
            {
              id: "demo-cp77-paused",
              titulo: "Cyberpunk 2077",
              estado: "pausado",
              progresso: 94,
              tamanho: "83.9 / 89.3 GB",
              capa: CAPA_CP77,
            },
          ]}
          onRetomar={() => {}}
          onCancelar={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="5. Erro">
        <PainelDownloads
          inline
          itens={[
            {
              id: "demo-hades-err",
              titulo: "Hades II",
              estado: "erro",
              erro: "Falha na conexão de rede",
              progresso: 32,
              tamanho: "4.1 / 12.4 GB",
              capa: CAPA_HADES,
            },
          ]}
          onRetomar={() => {}}
          onCancelar={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="6. Concluído">
        <PainelDownloads
          inline
          itens={[
            {
              id: "demo-hk-done",
              titulo: "Hollow Knight",
              estado: "concluido",
              progresso: 100,
              tamanho: "9.2 GB",
              capa: CAPA_HK,
            },
          ]}
          onAbrirJogo={() => {}}
        />
      </DemoGrupo>
    </div>
  );
}
