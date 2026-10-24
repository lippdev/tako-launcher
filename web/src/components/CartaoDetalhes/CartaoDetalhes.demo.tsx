import { DemoGrupo } from "../../lib/demoGrupo";
import { CartaoDetalhes, type DetalheDoJogo } from "./CartaoDetalhes";

export const title = "CartaoDetalhes";
export const preview = "CartaoDetalhes";

const COMPLETO: DetalheDoJogo[] = [
  { id: "developer", rotulo: "Desenvolvedora", valor: "Santa Monica Studios" },
  { id: "publisher", rotulo: "Publicadora", valor: "PlayStation Publishing LLC" },
  { id: "release", rotulo: "Lançamento", valor: "19 set, 2024" },
  { id: "genres", rotulo: "Gêneros", valor: "Ação, Aventura" },
  { id: "size", rotulo: "Tamanho no disco", valor: "23.5 GB" },
  { id: "installation", rotulo: "Situação", valor: "Instalado", ligado: true },
];

export function Demo() {
  return (
    <DemoGrupo titulo="Completo, com etiquetas (até 3 + N) · só o que o núcleo respondeu · não instalado (sem ponto)">
      <div style={{ width: 556 }}>
        <CartaoDetalhes
          aria-label="Detalhes do jogo"
          linhas={COMPLETO}
          etiquetas={["RPG", "Favorito da noite", "Co-op", "Roguelike", "Zerar em 2026"]}
          rotuloEtiquetas="Etiquetas"
        />
      </div>
      <div style={{ width: 400 }}>
        <CartaoDetalhes
          aria-label="Detalhes do jogo"
          linhas={[
            { id: "developer", rotulo: "Desenvolvedora", valor: "Capcom" },
            { id: "installation", rotulo: "Situação", valor: "Instalado", ligado: true },
          ]}
          rotuloEtiquetas="Etiquetas"
        />
      </div>
      <div style={{ width: 400 }}>
        <CartaoDetalhes
          aria-label="Detalhes do jogo"
          linhas={[{ id: "installation", rotulo: "Situação", valor: "Não instalado" }]}
          rotuloEtiquetas="Etiquetas"
        />
      </div>
    </DemoGrupo>
  );
}
