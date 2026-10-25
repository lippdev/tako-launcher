import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { FileiraDeCapas, type CapaDaFileira } from "./FileiraDeCapas";

export const title = "FileiraDeCapas";
export const preview = "TelaBiblioteca";

const NOMES = [
  "Chrono Odyssey",
  "Stellaris Run",
  "Hades II",
  "Elden Ring",
  "Cyberpunk 2077",
  "Baldur's Gate 3",
  "Sekiro",
  "Death Stranding",
  "Stellar Blade",
  "Black Myth: Wukong",
];

/** Capa de exemplo em SVG local (a galeria não depende do protocolo tako-img://). */
function arte(i: number): string {
  const hue = (i * 47 + 200) % 360;
  return `data:image/svg+xml,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="600" height="900" viewBox="0 0 600 900"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${String(hue)} 55% 46%)"/><stop offset="1" stop-color="hsl(${String((hue + 60) % 360)} 55% 14%)"/></linearGradient></defs><rect width="600" height="900" fill="url(#g)"/><path d="M0 700 L150 500 L270 620 L420 420 L600 640 V900 H0Z" fill="#000" fill-opacity=".38"/></svg>`,
  )}`;
}

const CAPAS: CapaDaFileira[] = NOMES.map((title, i) => ({
  id: `demo:${String(i)}`,
  title,
  label: i % 3 === 2 ? null : `${String(7 + i * 5)} h jogadas`,
  hasArt: i !== 4,
  src: arte(i),
  accentColor: "#6b8cae",
}));

export function Demo() {
  const [ultima, setUltima] = useState("nada ainda");
  return (
    <div>
      <DemoGrupo titulo={`Cinco capas por largura, setas ‹ › que rolam a fileira. Último evento: ${ultima}`}>
        <div style={{ width: "100%" }}>
          <FileiraDeCapas
            title="Continuar jogando"
            count="10 jogos"
            covers={CAPAS}
            previousLabel="Anterior"
            nextLabel="Próximo"
            onOpen={(id) => {
              setUltima(`abrir ${id}`);
            }}
            onHighlight={(id) => {
              setUltima(`destacar ${id}`);
            }}
          />
        </div>
      </DemoGrupo>
    </div>
  );
}
