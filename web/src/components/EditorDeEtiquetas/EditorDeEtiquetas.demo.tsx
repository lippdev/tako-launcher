import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { EditorDeEtiquetas, type EditorDeEtiquetasRotulos, type SugestaoDeEtiqueta } from "./EditorDeEtiquetas";

export const title = "EditorDeEtiquetas";
export const preview = "EtiquetasDoJogo";

const ROTULOS: EditorDeEtiquetasRotulos = {
  lista: "Etiquetas do jogo",
  adicionar: "Etiqueta",
  campo: "Nova etiqueta",
  titulo: "Etiquetas",
  tirar: (nome) => `Tirar ${nome}`,
  criar: (nome) => `Criar "${nome}"`,
  jogos: (n) => `${String(n)} ${n === 1 ? "jogo" : "jogos"}`,
};

const EXISTENTES: SugestaoDeEtiqueta[] = [
  { nome: "Co-op local", jogos: 3 },
  { nome: "Com mods", jogos: 5 },
  { nome: "Sofá", jogos: 2 },
  { nome: "Zerado", jogos: 9 },
];

/** O editor guarda as etiquetas em memória, como a página faz pela ponte. */
function Exemplo({ inicial, tamanho, alinhamento }: { inicial: string[]; tamanho?: "md" | "sm"; alinhamento?: "inicio" | "fim" }) {
  const [etiquetas, setEtiquetas] = useState(inicial);
  const [todas, setTodas] = useState(EXISTENTES);
  return (
    <EditorDeEtiquetas
      etiquetas={etiquetas}
      sugestoes={todas}
      rotulos={ROTULOS}
      {...(tamanho ? { tamanho } : {})}
      {...(alinhamento ? { alinhamento } : {})}
      onAdicionar={(nome) => {
        const existente = todas.find((e) => e.nome.toLocaleLowerCase("pt-BR") === nome.toLocaleLowerCase("pt-BR"));
        const final = existente?.nome ?? nome;
        setEtiquetas((atuais) => (atuais.includes(final) ? atuais : [...atuais, final]));
        setTodas((atuais) =>
          existente
            ? atuais.map((e) => (e === existente ? { ...e, jogos: e.jogos + 1 } : e))
            : [...atuais, { nome: final, jogos: 1 }],
        );
      }}
      onRemover={(nome) => {
        setEtiquetas((atuais) => atuais.filter((e) => e !== nome));
        setTodas((atuais) => atuais.map((e) => (e.nome === nome ? { ...e, jogos: e.jogos - 1 } : e)).filter((e) => e.jogos > 0));
      }}
    />
  );
}

export function Demo() {
  return (
    <div>
      <DemoGrupo titulo="Chips com ✕ e o campo tracejado — clique em “+ Etiqueta”, digite e use ↑ ↓ Enter">
        <div style={{ width: 520 }}>
          <Exemplo inicial={["Zerado", "Co-op local", "Com mods"]} />
        </div>
      </DemoGrupo>

      <DemoGrupo titulo="Sem etiquetas — só o campo tracejado">
        <Exemplo inicial={[]} />
      </DemoGrupo>

      <DemoGrupo titulo="Em Detalhes (chips de 26)">
        <div style={{ width: 360 }}>
          <Exemplo inicial={["Zerado", "Sofá"]} tamanho="sm" alinhamento="fim" />
        </div>
      </DemoGrupo>
    </div>
  );
}
