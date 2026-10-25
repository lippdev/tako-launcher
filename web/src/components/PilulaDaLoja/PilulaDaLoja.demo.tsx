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
        <PilulaDaLoja label="Steam" />
        <PilulaDaLoja label="GOG" />
        <PilulaDaLoja label="Epic Games" />
        <PilulaDaLoja label="Windows" icon="monitor" />
      </DemoGrupo>

      <DemoGrupo titulo="Pílula interativa (botão de ação com feedback)">
        <PilulaDaLoja
          label={copiado ? "Link copiado!" : "Compartilhar"}
          icon={copiado ? "check" : "share"}
          active={copiado}
          onClick={handleCompartilhar}
          aria-label="Compartilhar link do jogo"
        />
        <PilulaDaLoja label="Gerenciar" icon="gear" onClick={() => {}} />
      </DemoGrupo>

      <DemoGrupo titulo="Estados de interação (hover · foco · pressionado · desabilitado)">
        <PilulaDaLoja label="Normal" onClick={() => {}} />
        <PilulaDaLoja label="Hover" icon="share" onClick={() => {}} data-force="hover" />
        <PilulaDaLoja label="Foco" icon="share" onClick={() => {}} data-force="focus" />
        <PilulaDaLoja label="Pressionado" icon="share" onClick={() => {}} data-force="pressed" />
        <PilulaDaLoja label="Desabilitado" icon="share" onClick={() => {}} disabled />
      </DemoGrupo>
    </div>
  );
}
