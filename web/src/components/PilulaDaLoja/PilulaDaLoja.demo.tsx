import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { PilulaDaLoja } from "./PilulaDaLoja";

export const title = "PilulaDaLoja";
export const preview = "PilulaDaLoja";

export function Demo() {
  const [copiado, setCopiado] = useState(false);

  const handleCompartilhar = () => {
    setCopiado(true);
    setTimeout(() => {
      setCopiado(false);
    }, 2000);
  };

  return (
    <div>
      <DemoGrupo titulo="Pílulas de sistemas e plataformas (estáticas) — só texto ou com ícone genérico">
        <PilulaDaLoja rotulo="Steam" />
        <PilulaDaLoja rotulo="GOG" />
        <PilulaDaLoja rotulo="Epic Games" />
        <PilulaDaLoja rotulo="Windows" icone="monitor" />
      </DemoGrupo>

      <DemoGrupo titulo="Pílula interativa (botão de ação com feedback)">
        <PilulaDaLoja
          rotulo={copiado ? "Link copiado!" : "Compartilhar"}
          icone={copiado ? "check" : "share"}
          ativo={copiado}
          onClick={handleCompartilhar}
          aria-label="Compartilhar link do jogo"
        />
        <PilulaDaLoja rotulo="Gerenciar" icone="gear" onClick={() => {}} />
      </DemoGrupo>

      <DemoGrupo titulo="Estados de interação (hover · foco · pressionado · desabilitado)">
        <PilulaDaLoja rotulo="Normal" onClick={() => {}} />
        <PilulaDaLoja rotulo="Hover" icone="share" onClick={() => {}} data-force="hover" />
        <PilulaDaLoja rotulo="Foco" icone="share" onClick={() => {}} data-force="focus" />
        <PilulaDaLoja rotulo="Pressionado" icone="share" onClick={() => {}} data-force="pressed" />
        <PilulaDaLoja rotulo="Desabilitado" icone="share" onClick={() => {}} desabilitado />
      </DemoGrupo>
    </div>
  );
}
