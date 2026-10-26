import { BarrasDaSemana, Bloco, BlocoDeJogo, GrupoDeBlocos } from "./GrupoDeBlocos";

export const title = "GrupoDeBlocos";
export const preview = "GrupoDeBlocos";

const WEEK = [
  { label: "S", value: 0.8 },
  { label: "T", value: 0.15 },
  { label: "Q", value: 0.35 },
  { label: "Q", value: 0.3 },
  { label: "S", value: 0.4 },
  { label: "S", value: 0.55 },
  { label: "D", value: 0.95 },
];

export function Demo() {
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-24)" }}>
      <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
        SUA SEMANA
      </h3>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: "var(--space-16)" }}>
        <GrupoDeBlocos aria-label="Tempo">
          <Bloco label="Tempo de jogo" icon="clock">
            19 h
          </Bloco>
          <Bloco label="Esta semana">
            <BarrasDaSemana days={WEEK} aria-label="Esta semana: domingo foi o dia mais longo" />
          </Bloco>
        </GrupoDeBlocos>
        <GrupoDeBlocos aria-label="Jogos">
          <Bloco label="Jogos jogados" icon="gamepad">
            3
          </Bloco>
          <Bloco label="Conquistas" icon="badge-alert">
            27
          </Bloco>
        </GrupoDeBlocos>
        <GrupoDeBlocos aria-label="Jogo da semana">
          <BlocoDeJogo
            cover={<div style={{ width: "100%", height: "100%", background: "linear-gradient(160deg, var(--accent), var(--accent-deep))" }} />}
            title="Dragon's Dogma 2"
            text="8 horas nesta semana!"
          />
        </GrupoDeBlocos>
        <GrupoDeBlocos aria-label="Sessão">
          <Bloco label="Última sessão" icon="clock">
            3 h
          </Bloco>
          <Bloco label="Capturas" icon="camera">
            12
          </Bloco>
        </GrupoDeBlocos>
      </div>
      <h3 className="overline" style={{ margin: 0, color: "var(--text-muted)" }}>
        VARIANTES: BLOCOS FIXOS DE 132 E GRUPO OPACO
      </h3>
      <div style={{ display: "flex", gap: "var(--space-16)", alignItems: "flex-start" }}>
        <GrupoDeBlocos>
          <Bloco fixed label="Tempo médio" icon="clock">
            80 h
          </Bloco>
          <Bloco fixed label="Nota" icon="badge">
            93%
          </Bloco>
          <Bloco fixed label="Jogadores" icon="users">
            51K
          </Bloco>
        </GrupoDeBlocos>
        <GrupoDeBlocos flat>
          <Bloco fixed label="Tempo médio" icon="clock">
            80 h
          </Bloco>
          <Bloco fixed label="Nota" icon="badge">
            93%
          </Bloco>
        </GrupoDeBlocos>
      </div>
    </div>
  );
}
