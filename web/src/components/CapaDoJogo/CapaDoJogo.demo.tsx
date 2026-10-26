import { DemoGrupo } from "../../lib/demoGrupo";
import { CapaDoJogo } from "./CapaDoJogo";

export const title = "CapaDoJogo";
export const preview = "CapaDoJogo";

// Arte de exemplo em SVG local (data URL) para a demo rodar no navegador sem depender do protocolo tako-img://
const ARTE_EXEMPLO_1 = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 900" width="600" height="900">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1f2d3d"/>
      <stop offset="50%" stop-color="#111726"/>
      <stop offset="100%" stop-color="#0a0d14"/>
    </linearGradient>
    <radialGradient id="sun" cx="65%" cy="35%" r="45%">
      <stop offset="0%" stop-color="#ff9944" stop-opacity="0.9"/>
      <stop offset="60%" stop-color="#c43b24" stop-opacity="0.4"/>
      <stop offset="100%" stop-color="#111726" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="600" height="900" fill="url(#bg)"/>
  <circle cx="390" cy="315" r="240" fill="url(#sun)"/>
  <polygon points="120,680 300,380 480,680" fill="#0d1117" opacity="0.95"/>
  <polygon points="260,680 380,480 500,680" fill="#161b22" opacity="0.8"/>
  <text x="300" y="770" font-family="system-ui, sans-serif" font-size="36" font-weight="800" fill="#f4f1f0" text-anchor="middle" letter-spacing="2">CHRONO ODYSSEY</text>
  <text x="300" y="810" font-family="system-ui, sans-serif" font-size="18" font-weight="600" fill="#958f94" text-anchor="middle">EDIÇÃO DEFINITIVA</text>
</svg>
`)}`;

const ARTE_EXEMPLO_2 = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 900" width="600" height="900">
  <defs>
    <linearGradient id="bg2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#3d1f33"/>
      <stop offset="60%" stop-color="#1e1126"/>
      <stop offset="100%" stop-color="#0a0812"/>
    </linearGradient>
    <radialGradient id="magic" cx="50%" cy="40%" r="50%">
      <stop offset="0%" stop-color="#b388ff" stop-opacity="0.8"/>
      <stop offset="70%" stop-color="#512da8" stop-opacity="0.2"/>
      <stop offset="100%" stop-color="#0a0812" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="600" height="900" fill="url(#bg2)"/>
  <circle cx="300" cy="360" r="260" fill="url(#magic)"/>
  <circle cx="300" cy="380" r="100" fill="none" stroke="#e1deff" stroke-width="6" opacity="0.4"/>
  <polygon points="300,240 330,360 450,360 350,430 390,550 300,470 210,550 250,430 150,360 270,360" fill="#f4f1f0" opacity="0.85"/>
  <text x="300" y="760" font-family="system-ui, sans-serif" font-size="34" font-weight="800" fill="#f4f1f0" text-anchor="middle">STELLARIS RUN</text>
  <text x="300" y="800" font-family="system-ui, sans-serif" font-size="18" font-weight="600" fill="#cfcacd" text-anchor="middle">EXPANSÃO II</text>
</svg>
`)}`;

export function Demo() {
  return (
    <div>
      <DemoGrupo titulo="Com arte (600×900, 2:3) — normal e hover">
        <CapaDoJogo
          title="Chrono Odyssey"
          src={ARTE_EXEMPLO_1}
          width={180}
        />
        <CapaDoJogo
          title="Stellaris Run"
          src={ARTE_EXEMPLO_2}
          width={180}
        />
        <CapaDoJogo
          title="Hover forçado (zoom 1.05 + brilho)"
          src={ARTE_EXEMPLO_1}
          width={180}
          data-force="hover"
        />
      </DemoGrupo>

      <DemoGrupo titulo="EstadoSemArte — degradê da cor de destaque do jogo e título sobreposto">
        <CapaDoJogo
          title="Elden Ring: Shadow of the Erdtree"
          accentColor="var(--warning)"
          width={180}
        />
        <CapaDoJogo
          title="Cyberpunk 2077: Phantom Liberty"
          accentColor="var(--success)"
          width={180}
        />
        <CapaDoJogo
          title="Hades II"
          accentColor="var(--error)"
          width={180}
        />
        <CapaDoJogo
          title="Dead Cells: Return to Castlevania"
          accentColor="var(--accent-exemplo)"
          width={180}
          data-force="hover"
        />
      </DemoGrupo>

      <DemoGrupo titulo="Tamanhos variados da proporção 2:3">
        <CapaDoJogo
          title="Miniatura (120px)"
          src={ARTE_EXEMPLO_1}
          width={120}
        />
        <CapaDoJogo
          title="Padrão da vitrine (200px)"
          src={ARTE_EXEMPLO_2}
          width={200}
        />
      </DemoGrupo>
    </div>
  );
}
