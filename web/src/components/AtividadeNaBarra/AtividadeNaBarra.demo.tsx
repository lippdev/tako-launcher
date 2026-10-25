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
      title: "Watch Dogs 2",
      state: "baixando",
      progress: progressoInterativo,
      size: "60.9 / 108.7 GB",
      speed: "12 MB/s",
      remainingTime: "15 min",
      cover: CAPA_1,
    },
    {
      id: "demo-cp77",
      title: "Cyberpunk 2077",
      state: "na_fila",
      size: "83.9 GB",
      cover: CAPA_2,
    },
  ];

  return (
    <div>
      <DemoGrupo titulo="Interativo — clique para abrir o PainelDownloads como camada (Esc ou clique fora fecha)">
        <AtividadeNaBarra
          ref={setAnchor}
          count={contagemInterativa}
          progress={progressoInterativo}
          open={aberto}
          onClick={() => setAberto((prev) => !prev)}
        />
        <PainelDownloads
          open={aberto}
          onClose={() => setAberto(false)}
          anchor={anchor}
          items={itensInterativos}
          onPause={() => {}}
          onResume={() => {}}
          onCancel={() => {}}
          onOpenGame={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="Estados do botão na barra de cima">
        <div style={{ display: "flex", gap: "16px", alignItems: "center", flexWrap: "wrap" }}>
          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Vazio / Ocioso</p>
            <AtividadeNaBarra count={0} />
          </div>

          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Um baixando (56%)</p>
            <AtividadeNaBarra count={1} progress={56} />
          </div>

          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Vários com fila (3 ativos)</p>
            <AtividadeNaBarra count={3} progress={42} />
          </div>

          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Pausado</p>
            <AtividadeNaBarra count={1} progress={null} />
          </div>

          <div>
            <p className="label-sm" style={{ margin: "0 0 6px" }}>Com painel aberto</p>
            <AtividadeNaBarra count={2} progress={68} open={true} />
          </div>
        </div>
      </DemoGrupo>

      <DemoGrupo titulo="Animação do contador — clique para mudar o valor e ver o pulo (tk-pop 400ms)">
        <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
          <AtividadeNaBarra
            count={contagemInterativa}
            progress={progressoInterativo}
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
          <AtividadeNaBarra count={2} progress={56} data-force="hover" />
          <AtividadeNaBarra count={2} progress={56} data-force="focus" />
          <AtividadeNaBarra count={2} progress={56} data-force="pressed" />
        </div>
      </DemoGrupo>
    </div>
  );
}
