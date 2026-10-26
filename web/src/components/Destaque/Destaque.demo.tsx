import { useState } from "react";
import { AcoesDoJogo, type AcoesDoJogoLabels } from "../AcoesDoJogo/AcoesDoJogo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { CartaoSessoes } from "../CartaoSessoes/CartaoSessoes";
import { FundoDoJogo } from "../FundoDoJogo/FundoDoJogo";
import { PilulaDaLoja } from "../PilulaDaLoja/PilulaDaLoja";
import { TituloDoJogo } from "../TituloDoJogo/TituloDoJogo";
import { DemoGrupo } from "../../lib/demoGrupo";
import { Destaque } from "./Destaque";

export const title = "Destaque";
export const preview = "Destaque";

const ARTE = `data:image/svg+xml,${encodeURIComponent(
  `<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="620" viewBox="0 0 1920 620"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(210 55% 52%)"/><stop offset="1" stop-color="hsl(270 55% 16%)"/></linearGradient></defs><rect width="1920" height="620" fill="url(#g)"/><path d="M0 520 L360 330 L620 470 L980 250 L1320 440 L1620 300 L1920 460 V620 H0Z" fill="#000" fill-opacity=".35"/></svg>`,
)}`;

const ROTULOS: AcoesDoJogoLabels = {
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

export function Demo() {
  const [trocando, setTrocando] = useState(false);
  const [favorito, setFavorito] = useState(true);
  return (
    <DemoGrupo titulo="Dados embaixo à esquerda; situação, título, descrição e ações embaixo à direita (a troca de jogo sai e volta em escada)">
      <div style={{ width: "100%", display: "flex", flexDirection: "column", gap: 16, alignItems: "flex-start" }}>
        <BotaoDesktop
          variant="secondary"
          onClick={() => {
            setTrocando(true);
            setTimeout(() => {
              setTrocando(false);
            }, 600);
          }}
        >
          Simular a troca de jogo
        </BotaoDesktop>
        <div style={{ width: "100%" }}>
          <Destaque
            variante="visao"
            aria-label="Destaque de exemplo"
            trocando={trocando}
            fundo={<FundoDoJogo id="demo:1" temArte src={ARTE} />}
            chips={<PilulaDaLoja label="Steam" />}
            situacao={<span>INSTALADO</span>}
            title={<TituloDoJogo id="demo:1" title="Hades II" hasLogo={false} size="visao" />}
            descricao="A continuação do roguelike de ação sobre a princesa das sombras."
            dados={
              <CartaoSessoes
                aria-label="Dados"
                dados={[{ id: "t", icone: "clock", label: "Tempo de jogo", valor: "7 h" }]}
              />
            }
            acoes={
              <AcoesDoJogo
                estado="jogar"
                favorito={favorito}
                labels={ROTULOS}
                onAlternarFavorito={() => {
                  setFavorito((v) => !v);
                }}
              />
            }
          />
        </div>
      </div>
    </DemoGrupo>
  );
}
