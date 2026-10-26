import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { CampoDeBusca } from "./CampoDeBusca";

export const title = "CampoDeBusca";
export const preview = "CampoDeBusca";

const noop = () => undefined;
const coluna = { width: 330 } as const;
/** No foco o fundo da busca da loja passa 60 da largura de layout: quem a usa deixa essa folga à direita. */
const reserva = { paddingRight: 60 } as const;

export function Demo() {
  const [lista, setLista] = useState("");
  const [loja, setLoja] = useState("");
  const [geral, setGeral] = useState("");
  const [filtro, setFiltro] = useState(true);
  const [aberto, setAberto] = useState(false);
  return (
    <div>
      <DemoGrupo titulo="Lista da biblioteca — normal (digite: filtra na hora) · hover · foco · com filtro ativo · menu aberto">
        <div style={coluna}>
          <CampoDeBusca
            variant="lista"
            label="Buscar na biblioteca"
            placeholder="Buscar"
            value={lista}
            onChange={setLista}
            action={{ label: "Ordenar, ver e filtrar", onClick: noop }}
          />
        </div>
        <div style={coluna}>
          <CampoDeBusca
            variant="lista"
            label="Buscar (hover)"
            placeholder="Buscar"
            value=""
            onChange={noop}
            data-force="hover"
            action={{ label: "Ordenar, ver e filtrar", onClick: noop }}
          />
        </div>
        <div style={coluna}>
          <CampoDeBusca
            variant="lista"
            label="Buscar (foco)"
            placeholder="Buscar"
            value="hades"
            onChange={noop}
            data-force="focus"
            action={{ label: "Ordenar, ver e filtrar", onClick: noop }}
          />
        </div>
        <div style={coluna}>
          <CampoDeBusca
            variant="lista"
            label="Buscar (filtro ativo)"
            placeholder="Buscar"
            value=""
            onChange={noop}
            action={{
              label: filtro ? "Ordenar, ver e filtrar (filtro ativo)" : "Ordenar, ver e filtrar",
              onClick: () => {
                setFiltro((v) => !v);
              },
              active: filtro,
            }}
          />
        </div>
        <div style={coluna}>
          <CampoDeBusca
            variant="lista"
            label="Buscar (menu aberto)"
            placeholder="Buscar"
            value=""
            onChange={noop}
            action={{
              label: "Ordenar, ver e filtrar",
              onClick: () => {
                setAberto((v) => !v);
              },
              expanded: aberto,
            }}
          />
        </div>
        <BotaoDesktop variant="outlined">Botão ao lado (recolher)</BotaoDesktop>
      </DemoGrupo>
      <DemoGrupo titulo="Loja — 300 × 40, pílula; no foco cresce para 360 em 350 ms (clique no campo) · foco parado">
        <div style={reserva}>
          <CampoDeBusca
            variant="loja"
            label="Buscar na loja"
            placeholder="Buscar na loja"
            hint="Ctrl K"
            keyShortcuts="Control+K"
            value={loja}
            onChange={setLoja}
          />
        </div>
        <div style={reserva}>
          <CampoDeBusca
            variant="loja"
            label="Buscar na loja (foco)"
            placeholder="Buscar na loja"
            hint="Ctrl K"
            value=""
            onChange={noop}
            data-force="focus"
          />
        </div>
      </DemoGrupo>
      <DemoGrupo titulo="Busca geral — a linha de 58 do painel (560 de largura, a 110 do topo)">
        <div style={{ width: 560, background: "var(--surface-raised)", borderRadius: "var(--radius-card)" }}>
          <CampoDeBusca
            variant="geral"
            label="Busca geral"
            placeholder="Buscar jogos, na biblioteca e na loja"
            hint="Esc"
            value={geral}
            onChange={setGeral}
          />
        </div>
      </DemoGrupo>
    </div>
  );
}
