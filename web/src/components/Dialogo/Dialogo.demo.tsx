import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { CampoDeSelecao } from "../CampoDeSelecao/CampoDeSelecao";
import { Dialogo } from "./Dialogo";

export const title = "Dialogo";
export const preview = "Dialogo";

const MODO = [
  { value: "janela", label: "Janela" },
  { value: "tela-cheia", label: "Tela cheia" },
] as const;

type Qual = "padrao" | "remover" | "l" | "empilhado" | null;

export function Demo() {
  const [qual, setQual] = useState<Qual>(null);
  const [modo, setModo] = useState<(typeof MODO)[number]["value"]>("janela");
  const [confirma, setConfirma] = useState(false);
  const fechar = () => {
    setQual(null);
  };
  return (
    <div>
      <DemoGrupo titulo="Abrir (Esc fecha a de cima; Tab fica dentro; o foco volta ao botão)">
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setQual("padrao");
          }}
        >
          Diálogo (480)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setQual("remover");
          }}
        >
          Pergunta (S, 440)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setQual("l");
          }}
        >
          Grande (L, 680)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setQual("empilhado");
          }}
        >
          Menu dentro do diálogo
        </BotaoDesktop>
      </DemoGrupo>

      <Dialogo
        open={qual === "padrao"}
        onClose={fechar}
        title="Jogo pronto para jogar"
        closeLabel="Fechar"
        describedBy="demo-dialogo-texto"
        actions={
          <BotaoDesktop full icon="play" iconFilled onClick={fechar}>
            Jogar agora
          </BotaoDesktop>
        }
        fine="Você pode mudar isso depois, em Configurações."
      >
        <p id="demo-dialogo-texto" style={{ margin: 0, color: "var(--text-muted)" }}>
          Hades II terminou de baixar e está pronto. Uma ou duas frases dizem o que aconteceu e o que pode ser feito.
        </p>
      </Dialogo>

      <Dialogo
        open={qual === "remover"}
        onClose={fechar}
        size="s"
        title="Remover da biblioteca?"
        closeLabel="Fechar"
        actions={
          <>
            <BotaoDesktop variant="outlined" onClick={fechar}>
              Cancelar
            </BotaoDesktop>
            <BotaoDesktop size="sm" icon="trash" onClick={fechar}>
              Remover
            </BotaoDesktop>
          </>
        }
      >
        <p style={{ margin: 0, color: "var(--text-muted)" }}>Os arquivos do jogo continuam no disco.</p>
      </Dialogo>

      <Dialogo
        open={qual === "l"}
        onClose={fechar}
        size="l"
        title="Escolha a imagem"
        closeLabel="Fechar"
        actions={
          <BotaoDesktop size="sm" icon="check" onClick={fechar}>
            Usar esta
          </BotaoDesktop>
        }
      >
        <div
          style={{
            height: 180,
            borderRadius: "var(--radius-card)",
            border: "1px solid var(--line)",
            background: "var(--tile)",
          }}
        />
      </Dialogo>

      <Dialogo
        open={qual === "empilhado"}
        onClose={fechar}
        title="Janela do jogo"
        closeLabel="Fechar"
        actions={
          <>
            <BotaoDesktop
              variant="outlined"
              onClick={() => {
                setConfirma(true);
              }}
            >
              Pergunta por cima
            </BotaoDesktop>
            <BotaoDesktop size="sm" icon="check" onClick={fechar}>
              Pronto
            </BotaoDesktop>
          </>
        }
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <span>Modo da janela</span>
          <CampoDeSelecao label="Modo da janela" options={MODO} value={modo} onChange={setModo} />
        </div>
      </Dialogo>
      <Dialogo
        open={confirma}
        onClose={() => {
          setConfirma(false);
        }}
        size="s"
        title="Descartar mudanças?"
        closeLabel="Fechar"
        actions={
          <BotaoDesktop
            size="sm"
            icon="check"
            onClick={() => {
              setConfirma(false);
            }}
          >
            Entendi
          </BotaoDesktop>
        }
      >
        <p style={{ margin: 0, color: "var(--text-muted)" }}>Segundo diálogo por cima: o Esc fecha só este.</p>
      </Dialogo>
    </div>
  );
}
