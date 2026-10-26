import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { StatusDoJogo, type GameStatus } from "./StatusDoJogo";

export const title = "StatusDoJogo";
export const preview = "StatusDoJogo";

export function Demo() {
  const [status, setStatus] = useState<GameStatus>("playing");

  return (
    <div>
      <DemoGrupo titulo="Interativo — clique para abrir o menu e mudar a situação">
        <StatusDoJogo status={status} onStatusChange={setStatus} />
      </DemoGrupo>

      <DemoGrupo titulo="Todas as situações (somente leitura)">
        <StatusDoJogo status="none" />
        <StatusDoJogo status="playing" />
        <StatusDoJogo status="completed" />
        <StatusDoJogo status="queued" />
        <StatusDoJogo status="paused" />
        <StatusDoJogo status="dropped" />
      </DemoGrupo>

      <DemoGrupo titulo="Estados de interação do botão (hover · foco · pressionado)">
        <StatusDoJogo status="playing" onStatusChange={() => {}} />
        <StatusDoJogo status="playing" onStatusChange={() => {}} data-force="hover" />
        <StatusDoJogo status="playing" onStatusChange={() => {}} data-force="focus" />
        <StatusDoJogo status="playing" onStatusChange={() => {}} data-force="pressed" />
      </DemoGrupo>
    </div>
  );
}
