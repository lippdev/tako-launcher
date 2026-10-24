import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { Abas, type Aba } from "./Abas";

export const title = "Abas";
export const preview = "Abas";

const TOPO: readonly Aba<"loja" | "biblioteca">[] = [
  { id: "loja", label: "Loja" },
  { id: "biblioteca", label: "Biblioteca" },
];
const TOPO_ESTADOS: readonly Aba<"loja" | "biblioteca">[] = [
  { id: "loja", label: "Loja", force: "hover" },
  { id: "biblioteca", label: "Biblioteca" },
];
const TOPO_FOCO: readonly Aba<"loja" | "biblioteca">[] = [
  { id: "loja", label: "Loja", force: "focus" },
  { id: "biblioteca", label: "Biblioteca" },
];
const LISTA: readonly Aba<"todos" | "favoritos" | "instalados">[] = [
  { id: "todos", label: "Todos" },
  { id: "favoritos", label: "Favoritos" },
  { id: "instalados", label: "Instalados" },
];
const LISTA_ESTADOS: readonly Aba<"todos" | "favoritos" | "instalados">[] = [
  { id: "todos", label: "Todos" },
  { id: "favoritos", label: "Favoritos", force: "hover" },
  { id: "instalados", label: "Instalados", force: "focus" },
];
const LOJA: readonly Aba<"descobrir" | "explorar" | "desejos">[] = [
  { id: "descobrir", label: "Descobrir" },
  { id: "explorar", label: "Explorar" },
  { id: "desejos", label: "Lista de desejos", count: 2 },
];
const LOJA_ESTADOS: readonly Aba<"descobrir" | "explorar" | "desejos">[] = [
  { id: "descobrir", label: "Descobrir" },
  { id: "explorar", label: "Explorar", force: "hover" },
  { id: "desejos", label: "Lista de desejos", count: 2, force: "focus" },
];
const SEGMENTADO: readonly Aba<"populares" | "baixados" | "novos" | "ofertas">[] = [
  { id: "populares", label: "Populares" },
  { id: "baixados", label: "Mais baixados" },
  { id: "novos", label: "Novidades" },
  { id: "ofertas", label: "Ofertas" },
];
const SEGMENTADO_ESTADOS: readonly Aba<"populares" | "baixados" | "novos" | "ofertas">[] = [
  { id: "populares", label: "Populares" },
  { id: "baixados", label: "Mais baixados", force: "hover" },
  { id: "novos", label: "Novidades", force: "focus" },
  { id: "ofertas", label: "Ofertas" },
];

const barra = {
  display: "flex",
  alignItems: "stretch",
  height: "var(--topbar-height)",
  padding: "0 var(--space-16)",
  background: "var(--titlebar)",
  border: "1px solid var(--line)",
  borderRadius: "var(--radius-button)",
} as const;
const largura = { width: 374 } as const;

export function Demo() {
  const [topo, setTopo] = useState<"loja" | "biblioteca">("biblioteca");
  const [lista, setLista] = useState<"todos" | "favoritos" | "instalados">("todos");
  const [loja, setLoja] = useState<"descobrir" | "explorar" | "desejos">("descobrir");
  const [segmentado, setSegmentado] = useState<"populares" | "baixados" | "novos" | "ofertas">("populares");
  const noop = () => undefined;
  return (
    <div>
      <DemoGrupo titulo="Barra de cima — use ← → Home End (interativo) · hover e foco parados">
        <div style={barra}>
          <Abas variant="topo" label="Seção" tabs={TOPO} value={topo} onChange={setTopo} />
        </div>
        <div style={barra}>
          <Abas variant="topo" label="Seção (hover na Loja)" tabs={TOPO_ESTADOS} value="biblioteca" onChange={noop} />
        </div>
        <div style={barra}>
          <Abas variant="topo" label="Seção (foco na Loja)" tabs={TOPO_FOCO} value="biblioteca" onChange={noop} />
        </div>
      </DemoGrupo>
      <DemoGrupo titulo="Lista — Todos · Favoritos · Instalados (interativo · hover e foco parados · troque o tema para ver o trilho)">
        <div style={largura}>
          <Abas variant="lista" label="Filtro da biblioteca" tabs={LISTA} value={lista} onChange={setLista} />
        </div>
        <div style={largura}>
          <Abas variant="lista" label="Filtro (estados)" tabs={LISTA_ESTADOS} value="todos" onChange={noop} />
        </div>
      </DemoGrupo>
      <DemoGrupo titulo="Loja — Descobrir · Explorar · Lista de desejos (interativo · hover e foco parados)">
        <Abas variant="loja" label="Seção da loja" tabs={LOJA} value={loja} onChange={setLoja} />
        <Abas variant="loja" label="Seção da loja (estados)" tabs={LOJA_ESTADOS} value="descobrir" onChange={noop} />
      </DemoGrupo>
      <DemoGrupo titulo="Segmentado — a pílula desliza em 350 ms (interativo · hover e foco parados)">
        <Abas variant="segmentado" label="Ordem" tabs={SEGMENTADO} value={segmentado} onChange={setSegmentado} />
        <Abas variant="segmentado" label="Ordem (estados)" tabs={SEGMENTADO_ESTADOS} value="populares" onChange={noop} />
      </DemoGrupo>
    </div>
  );
}
