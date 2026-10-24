import { useState } from "react";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { DemoGrupo } from "../../lib/demoGrupo";
import { FundoDoJogo } from "./FundoDoJogo";

export const title = "FundoDoJogo";
export const preview = "FundoDoJogo";

/** Arte larga de exemplo (SVG local), para a demo rodar sem o protocolo `tako-img://`. */
function arte(hue: number): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="620" viewBox="0 0 1920 620"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${String(hue)} 55% 52%)"/><stop offset="1" stop-color="hsl(${String((hue + 60) % 360)} 55% 16%)"/></linearGradient></defs><rect width="1920" height="620" fill="url(#g)"/><path d="M0 520 L360 330 L620 470 L980 250 L1320 440 L1620 300 L1920 460 V620 H0Z" fill="#000" fill-opacity=".35"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

const caixa = { position: "relative", width: 560, height: 280, overflow: "hidden", background: "var(--canvas)" } as const;

export function Demo() {
  const [n, setN] = useState(0);
  return (
    <div>
      <DemoGrupo titulo="Com arte · sem arte (EstadoSemArte, com a cor do jogo) · sem arte e sem cor">
        <div style={caixa}>
          <FundoDoJogo id="demo:1" temArte src={arte(210)} />
        </div>
        <div style={caixa}>
          <FundoDoJogo id="demo:2" temArte={false} corDestaque="#2f8f83" />
        </div>
        <div style={caixa}>
          <FundoDoJogo id="demo:3" temArte={false} />
        </div>
      </DemoGrupo>
      <DemoGrupo titulo="Troca de jogo: a nova arte entra em 800 ms e assenta de 1,05 para 1 em 1,8 s">
        <div style={caixa}>
          <FundoDoJogo id={`demo:troca:${String(n % 2)}`} temArte src={arte(n % 2 === 0 ? 20 : 150)} />
        </div>
        <BotaoDesktop
          variant="secondary"
          onClick={() => {
            setN((v) => v + 1);
          }}
        >
          Trocar de jogo
        </BotaoDesktop>
      </DemoGrupo>
    </div>
  );
}
