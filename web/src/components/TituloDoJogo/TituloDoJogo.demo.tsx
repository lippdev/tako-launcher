import { DemoGrupo } from "../../lib/demoGrupo";
import { TituloDoJogo } from "./TituloDoJogo";

export const title = "TituloDoJogo";
export const preview = "TituloDoJogo";

/** Logo de exemplo (SVG local): o nome em maiúsculas, alinhado à direita. */
const LOGO = `data:image/svg+xml,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="440" height="140" viewBox="0 0 440 140"><text x="436" y="96" text-anchor="end" font-family="Poppins, sans-serif" font-weight="800" font-size="46" fill="#f4f1f0">HADES II</text></svg>`,
)}`;

const fundo = { padding: 24, background: "var(--surface)", borderRadius: 16, width: 520 } as const;

export function Demo() {
  return (
    <div>
      <DemoGrupo titulo="Com logo (até 440×140) · sem logo (o nome em texto, Poppins 54/800)">
        <div style={fundo}>
          <TituloDoJogo id="demo:1" title="Hades II" hasLogo src={LOGO} />
        </div>
        <div style={fundo}>
          <TituloDoJogo id="demo:2" title="Dragon's Dogma 2" hasLogo={false} />
        </div>
      </DemoGrupo>
      <DemoGrupo titulo="Título longo e tamanho da Visão geral (320×100)">
        <div style={fundo}>
          <TituloDoJogo id="demo:3" title="Ghost of Tsushima Director's Cut" hasLogo={false} />
        </div>
        <div style={fundo}>
          <TituloDoJogo id="demo:4" title="Hades II" hasLogo src={LOGO} size="visao" />
        </div>
      </DemoGrupo>
    </div>
  );
}
