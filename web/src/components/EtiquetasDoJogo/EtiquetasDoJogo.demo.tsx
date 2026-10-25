import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { EtiquetasDoJogo } from "./EtiquetasDoJogo";

export const title = "EtiquetasDoJogo";
export const preview = "EtiquetasDoJogo";

const EXEMPLO_TAGS = [
  "Zerado",
  "Co-op",
  "Com mods",
  "História rica",
  "Trilha sonora épica",
  "Mundo aberto",
];

export function Demo() {
  const [selecionada, setSelecionada] = useState<string | null>(null);

  return (
    <div>
      <DemoGrupo titulo="Exibição normal — todas as etiquetas">
        <EtiquetasDoJogo tags={EXEMPLO_TAGS} />
      </DemoGrupo>

      <DemoGrupo titulo="Com limite de espaço (até 3 chips + '+3')">
        <EtiquetasDoJogo tags={EXEMPLO_TAGS} limit={3} />
      </DemoGrupo>

      <DemoGrupo titulo="Em Detalhes (tamanho compacto 26px com limite e '+N')">
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          <span style={{ fontSize: 13, color: "var(--text-muted)" }}>Etiquetas:</span>
          <EtiquetasDoJogo tags={EXEMPLO_TAGS} limit={1} size="sm" />
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 16, marginTop: 12 }}>
          <span style={{ fontSize: 13, color: "var(--text-muted)" }}>Etiquetas:</span>
          <EtiquetasDoJogo tags={EXEMPLO_TAGS} limit={3} size="sm" />
        </div>
      </DemoGrupo>

      <DemoGrupo titulo="Etiquetas clicáveis (filtro na biblioteca)">
        <EtiquetasDoJogo
          tags={EXEMPLO_TAGS}
          onTagClick={(t) => setSelecionada(t)}
        />
        {selecionada && (
          <span style={{ fontSize: 12.5, color: "var(--accent)" }}>
            Filtro ativo: {selecionada}
          </span>
        )}
      </DemoGrupo>
    </div>
  );
}
