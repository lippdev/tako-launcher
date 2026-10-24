import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { Aviso, AvisoCartao, type AvisoTone } from "./Aviso";

export const title = "Aviso";
export const preview = "Aviso";

interface Notice {
  key: number;
  tone: AvisoTone;
  text: string;
  undo?: boolean;
}

const noop = () => undefined;

export function Demo() {
  const [notice, setNotice] = useState<Notice | null>(null);
  const [open, setOpen] = useState(false);
  const [counter, setCounter] = useState(0);
  const show = (tone: AvisoTone, text: string, undo = false) => {
    const next: Notice = { key: counter + 1, tone, text };
    if (undo) next.undo = true;
    setCounter(next.key);
    setNotice(next);
    setOpen(true);
  };
  return (
    <div>
      <DemoGrupo titulo="Disparar — embaixo, no centro, 26 acima da borda; some em 2,3 s (pausa com o mouse ou o foco em cima)">
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            show("success", "Coleção criada");
          }}
        >
          Sucesso
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            show("error", "Não foi possível abrir o jogo");
          }}
        >
          Erro
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            show("success", "Jogo removido da coleção", true);
          }}
        >
          Com desfazer
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            show("tako", "Tentáculos no controle · abrindo Hades II");
          }}
        >
          Notícia do TAKO (polvo)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            show("info", "Atualização disponível");
          }}
        >
          Informação
        </BotaoDesktop>
      </DemoGrupo>
      <DemoGrupo titulo="Os cartões, parados">
        <AvisoCartao tone="success">Coleção criada</AvisoCartao>
        <AvisoCartao tone="error">Não foi possível abrir o jogo</AvisoCartao>
        <AvisoCartao tone="success" action={{ label: "Desfazer", onAction: noop }}>
          Jogo removido da coleção
        </AvisoCartao>
        <AvisoCartao tone="tako">Tentáculos no controle · abrindo Hades II</AvisoCartao>
        <AvisoCartao tone="info">Atualização disponível</AvisoCartao>
      </DemoGrupo>
      <Aviso
        open={open}
        noticeKey={notice?.key ?? 0}
        tone={notice?.tone ?? "success"}
        onClose={() => {
          setOpen(false);
        }}
        {...(notice?.undo
          ? {
              action: {
                label: "Desfazer",
                onAction: () => {
                  setOpen(false);
                },
              },
            }
          : {})}
      >
        {notice?.text}
      </Aviso>
    </div>
  );
}
