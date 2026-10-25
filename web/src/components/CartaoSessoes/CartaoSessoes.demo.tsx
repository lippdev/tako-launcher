import { DemoGrupo } from "../../lib/demoGrupo";
import { CartaoSessoes } from "./CartaoSessoes";

export const title = "CartaoSessoes";
export const preview = "CartaoSessoes";

const sobre = { padding: 24, background: "linear-gradient(135deg, var(--accent-exemplo), var(--canvas))", borderRadius: 16 } as const;

export function Demo() {
  return (
    <div>
      <DemoGrupo titulo="A barra de dados sobre a arte: Última sessão · Tempo de jogo · Metacritic (verde, amarelo, vermelho)">
        <div style={sobre}>
          <CartaoSessoes
            aria-label="Dados do jogo"
            dados={[
              { id: "last", icone: "calendar", label: "Última sessão", valor: "23/10, 16:17" },
              { id: "time", icone: "clock", label: "Tempo de jogo", valor: "7 h" },
              {
                id: "mc",
                icone: "badge",
                label: "Metacritic",
                valor: "Aclamação universal",
                nota: { valor: 94, faixa: "good" },
              },
            ]}
          />
        </div>
        <div style={sobre}>
          <CartaoSessoes
            aria-label="Dados do jogo"
            dados={[
              { id: "mc", icone: "badge", label: "Metacritic", valor: "Críticas mistas", nota: { valor: 62, faixa: "mixed" } },
            ]}
          />
        </div>
        <div style={sobre}>
          <CartaoSessoes
            aria-label="Dados do jogo"
            dados={[
              {
                id: "mc",
                icone: "badge",
                label: "Metacritic",
                valor: "Críticas desfavoráveis",
                nota: { valor: 38, faixa: "bad" },
              },
            ]}
          />
        </div>
      </DemoGrupo>
      <DemoGrupo titulo="Lista das sessões recentes">
        <div style={{ width: 556 }}>
          <CartaoSessoes
            variante="lista"
            title="Sessões recentes"
            sessoes={[
              { inicio: "23/10, 16:17", duracao: "42 min" },
              { inicio: "21/10, 21:03", duracao: "7 h 20 min" },
              { inicio: "19/10, 19:40", duracao: "1 h 30 min" },
            ]}
          />
        </div>
      </DemoGrupo>
    </div>
  );
}
