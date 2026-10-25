import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { AcoesDoJogo, type AcoesDoJogoLabels, type EstadoDoPrimario } from "./AcoesDoJogo";

export const title = "AcoesDoJogo";
export const preview = "AcoesDoJogo";

const ROTULOS_DE_EXEMPLO: AcoesDoJogoLabels = {
  jogar: "Jogar",
  abrindo: "Abrindo…",
  voltarAoJogo: "Voltar ao jogo",
  encerrar: "Encerrar jogo",
  instalar: "Instalar",
  emBreve: "Em breve",
  favoritar: "Favoritar",
  desfavoritar: "Tirar dos favoritos",
  gerenciar: "Gerenciar",
  configuracoes: "Configurações do jogo",
};

const ESTADOS: [EstadoDoPrimario, string][] = [
  ["jogar", "Jogar"],
  ["abrindo", "Abrindo…"],
  ["jogando", "Jogando (Voltar ao jogo e Encerrar)"],
  ["nao-instalado", "Não instalado"],
  ["em-breve", "Em breve"],
];

export function Demo() {
  const [favorito, setFavorito] = useState(false);
  return (
    <DemoGrupo titulo="O primário em cada estado; ★ e Gerenciar valem em todos">
      {ESTADOS.map(([estado, nome]) => (
        <div key={estado} style={{ display: "flex", flexDirection: "column", gap: 8, alignItems: "flex-end" }}>
          <small style={{ color: "var(--text-muted)" }}>{nome}</small>
          <AcoesDoJogo
            estado={estado}
            favorito={favorito}
            labels={ROTULOS_DE_EXEMPLO}
            onJogar={() => undefined}
            onVoltarAoJogo={() => undefined}
            onEncerrar={() => undefined}
            onAlternarFavorito={() => {
              setFavorito((v) => !v);
            }}
          />
        </div>
      ))}
    </DemoGrupo>
  );
}
