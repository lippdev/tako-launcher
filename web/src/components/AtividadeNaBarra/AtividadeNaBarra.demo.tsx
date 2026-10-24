import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { PainelDownloads, type ItemDownload } from "../PainelDownloads/PainelDownloads";
import { AtividadeNaBarra } from "./AtividadeNaBarra";

export const title = "AtividadeNaBarra";
export const preview = "AtividadeNaBarra";

const CAPA_1 = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 92" width="72" height="92">
  <rect width="72" height="92" fill="midnightblue" rx="10"/>
  <circle cx="36" cy="40" r="18" fill="cornflowerblue" opacity="0.8"/>
  <text x="36" y="78" font-family="sans-serif" font-size="10" font-weight="bold" fill="white" text-anchor="middle">WD2</text>
</svg>
`)}`;

const CAPA_2 = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 92" width="72" height="92">
  <rect width="72" height="92" fill="darkslateblue" rx="10"/>
  <polygon points="36,24 50,56 22,56" fill="coral"/>
  <text x="36" y="78" font-family="sans-serif" font-size="10" font-weight="bold" fill="white" text-anchor="middle">CP77</text>
</svg>
`)}`;

export function Demo() {
  const [aberto, setAberto] = useState(false);
  const [contagemInterativa, setContagemInterativa] = useState(2);
  const [progressoInterativo, setProgressoInterativo] = useState(56);
  const [anchor, setAnchor] = useState<HTMLButtonElement | null>(null);

  const itensInterativos: ItemDownload[] = [
    {
      id: "demo-wd2",
      titulo: "Watch Dogs 2",
      estado: "baixando",
      progresso: progressoInterativo,
      tamanho: "60.9 / 108.7 GB",
      velocidade: "12 MB/s",
      tempoRestante: "15 min",
      capa: CAPA_1,
    },
    {
      id: "demo-cp77",
      titulo: "Cyberpunk 2077",
      estado: "na_fila",
      tamanho: "83.9 GB",
      capa: CAPA_2,
    },
  ];

  return (
    <div>
      <DemoGrupo titulo="Interativo — clique para abrir o PainelDownloads como camada (Esc ou clique fora fecha)">
        <AtividadeNaBarra
          ref={setAnchor}
          contagem={contagemInterativa}
          porcentagem={progressoInterativo}
          aberto={aberto}
          onClick={() => setAberto((prev) => !prev)}
        />
        <PainelDownloads
          open={aberto}
          onClose={() => setAberto(false)}
          anchor={anchor}
          itens={itensInterativos}
          onPausar={() => {}}
          onRetomar={() => {}}
          onCancelar={() => {}}
          onAbrirJogo={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="Estados do botão na barra de cima">
        <div style={{ display: "flex", gap: "16px", alignItems: "center", flexWrap: "wrap" }}>
          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Vazio / Ocioso</p>
            <AtividadeNaBarra contagem={0} />
          </div>

          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Um baixando (56%)</p>
            <AtividadeNaBarra contagem={1} porcentagem={56} />
          </div>

          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Vários com fila (3 ativos)</p>
            <AtividadeNaBarra contagem={3} porcentagem={42} />
          </div>

          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Pausado</p>
            <AtividadeNaBarra contagem={1} porcentagem={null} />
          </div>

          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Com painel aberto</p>
            <AtividadeNaBarra contagem={2} porcentagem={68} aberto={true} />
          </div>
        </div>
      </DemoGrupo>

      <DemoGrupo titulo="Animação do contador — clique para mudar o valor e ver o pulo (tk-pop 400ms)">
        <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
          <AtividadeNaBarra
            contagem={contagemInterativa}
            porcentagem={progressoInterativo}
          />
          <button
            type="button"
            className="tb-btn"
            style={{
              height: "36px",
              padding: "0 12px",
              borderRadius: "9px",
              border: "1px solid var(--line-strong)",
              background: "var(--surface)",
              color: "var(--text)",
              cursor: "pointer",
            }}
            onClick={() => {
              setContagemInterativa((c) => c + 1);
              setProgressoInterativo((p) => Math.min(100, p + 10));
            }}
          >
            + Adicionar atividade
          </button>
          <button
            type="button"
            className="tb-btn"
            style={{
              height: "36px",
              padding: "0 12px",
              borderRadius: "9px",
              border: "1px solid var(--line-strong)",
              background: "var(--surface)",
              color: "var(--text)",
              cursor: "pointer",
            }}
            onClick={() => {
              setContagemInterativa((c) => Math.max(0, c - 1));
              setProgressoInterativo((p) => Math.max(0, p - 10));
            }}
          >
            - Remover atividade
          </button>
        </div>
      </DemoGrupo>

      <DemoGrupo titulo="Estados de interação (hover · foco · pressionado)">
        <div style={{ display: "flex", gap: "16px", alignItems: "center" }}>
          <AtividadeNaBarra contagem={2} porcentagem={56} data-force="hover" />
          <AtividadeNaBarra contagem={2} porcentagem={56} data-force="focus" />
          <AtividadeNaBarra contagem={2} porcentagem={56} data-force="pressed" />
        </div>
      </DemoGrupo>
    </div>
  );
}
