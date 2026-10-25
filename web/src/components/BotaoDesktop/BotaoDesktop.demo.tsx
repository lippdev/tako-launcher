import { useEffect, useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { BotaoDesktop } from "./BotaoDesktop";

export const title = "BotaoDesktop";
export const preview = "BotaoDesktop";

/** Estados parados (`data-force`) para comparar com a prévia sem precisar do mouse. */
const FORCED = [
  ["Normal", {}],
  ["Hover", { "data-force": "hover" }],
  ["Foco", { "data-force": "focus" }],
  ["Pressionado", { "data-force": "pressed" }],
  ["Desabilitado", { disabled: true }],
] as const;

function Downloading() {
  const [progress, setProgress] = useState(0.15);
  useEffect(() => {
    const id = setInterval(() => {
      setProgress((p) => (p >= 1 ? 0.05 : p + 0.05));
    }, 600);
    return () => {
      clearInterval(id);
    };
  }, []);
  return (
    <BotaoDesktop icon="download" tone="state" progress={progress}>
      Baixando {Math.round(progress * 100)}%
    </BotaoDesktop>
  );
}

export function Demo() {
  const [favorite, setFavorite] = useState(true);
  const [wish, setWish] = useState(false);
  return (
    <div>
      <DemoGrupo titulo="Primário — um por região (normal · hover · foco · pressionado · desabilitado)">
        {FORCED.map(([name, extra]) => (
          <BotaoDesktop key={name} icon="play" iconFilled {...extra}>
            Jogar
          </BotaoDesktop>
        ))}
      </DemoGrupo>
      <DemoGrupo titulo="Primário — pequeno, de estado (Atualizar), baixando, largura toda">
        <BotaoDesktop size="sm" icon="play" iconFilled>
          Jogar
        </BotaoDesktop>
        <BotaoDesktop icon="refresh" tone="state">
          Atualizar
        </BotaoDesktop>
        <Downloading />
        <BotaoDesktop icon="download">Comprar · R$ 59,99</BotaoDesktop>
        <div style={{ width: 280 }}>
          <BotaoDesktop full icon="download">
            Instalar
          </BotaoDesktop>
        </div>
      </DemoGrupo>
      <DemoGrupo titulo="Secundário (fantasma) — normal · hover · foco · pressionado · desabilitado">
        {FORCED.map(([name, extra]) => (
          <BotaoDesktop key={name} variant="secondary" icon="gear" {...extra}>
            Gerenciar
          </BotaoDesktop>
        ))}
      </DemoGrupo>
      <DemoGrupo titulo="Secundário — ligado, pequeno, Pôr no carrinho">
        <BotaoDesktop variant="secondary" icon="check" on>
          Instalado
        </BotaoDesktop>
        <BotaoDesktop variant="secondary" size="sm" icon="gear">
          Gerenciar
        </BotaoDesktop>
        <BotaoDesktop variant="secondary" icon="cart">
          Pôr no carrinho
        </BotaoDesktop>
      </DemoGrupo>
      <DemoGrupo titulo="Quadrado só de ícone — ★ ♥ ✕ (clique alterna); hover · foco · pressionado · desabilitado">
        <BotaoDesktop
          variant="square"
          icon="star"
          aria-label="Favorito"
          tone="warning"
          on={favorite}
          onClick={() => {
            setFavorite((v) => !v);
          }}
        />
        <BotaoDesktop
          variant="square"
          icon="heart"
          aria-label="Lista de desejos"
          tone="wish"
          on={wish}
          onClick={() => {
            setWish((v) => !v);
          }}
        />
        <BotaoDesktop variant="square" icon="x" aria-label="Remover" />
        <BotaoDesktop variant="square" icon="star" aria-label="Favorito (hover)" data-force="hover" />
        <BotaoDesktop variant="square" icon="star" aria-label="Favorito (foco)" data-force="focus" />
        <BotaoDesktop variant="square" icon="star" aria-label="Favorito (pressionado)" data-force="pressed" />
        <BotaoDesktop variant="square" icon="star" aria-label="Favorito (desabilitado)" disabled />
      </DemoGrupo>
      <DemoGrupo titulo="Contornado — Trocar · Procurar · Copiar; hover · foco · desabilitado · ligado">
        <BotaoDesktop variant="outlined">Trocar</BotaoDesktop>
        <BotaoDesktop variant="outlined" icon="search">
          Procurar
        </BotaoDesktop>
        <BotaoDesktop variant="outlined" icon="share">
          Copiar
        </BotaoDesktop>
        <BotaoDesktop variant="outlined" data-force="hover">
          Hover
        </BotaoDesktop>
        <BotaoDesktop variant="outlined" data-force="focus">
          Foco
        </BotaoDesktop>
        <BotaoDesktop variant="outlined" disabled>
          Desabilitado
        </BotaoDesktop>
        <BotaoDesktop variant="outlined" icon="check" on>
          Ligado
        </BotaoDesktop>
      </DemoGrupo>
      <DemoGrupo titulo="Chip — situação, Windows, Compartilhar; hover · foco · desabilitado">
        <BotaoDesktop variant="chip" dot="success" trailingIcon="chevron-down">
          Jogando
        </BotaoDesktop>
        <BotaoDesktop variant="chip" icon="monitor">
          Windows
        </BotaoDesktop>
        <BotaoDesktop variant="chip" icon="share">
          Compartilhar
        </BotaoDesktop>
        <BotaoDesktop variant="chip" icon="share" data-force="hover">
          Hover
        </BotaoDesktop>
        <BotaoDesktop variant="chip" icon="share" data-force="focus">
          Foco
        </BotaoDesktop>
        <BotaoDesktop variant="chip" icon="share" disabled>
          Desabilitado
        </BotaoDesktop>
      </DemoGrupo>
    </div>
  );
}
