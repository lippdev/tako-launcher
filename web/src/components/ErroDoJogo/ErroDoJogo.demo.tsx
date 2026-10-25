import { useState } from "react";
import { ErroDoJogo, type ErroDoJogoLabels } from "./ErroDoJogo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { DemoGrupo } from "../../lib/demoGrupo";

export const title = "ErroDoJogo";
export const preview = "ErroDoJogo";

const ROTULOS: ErroDoJogoLabels = {
  close: "Fechar (Esc)",
  technicalDetails: "Detalhes técnicos",
  saveReport: "Salvar relatório",
  repair: "Reparar arquivos",
  retry: "Tentar de novo",
  titleSuffix: "fechou sozinho",
};

type Cenarios = "padrao" | "sem-codigo" | "sem-reparo" | "simples" | null;

export function Demo() {
  const [cenario, setCenario] = useState<Cenarios>(null);
  const [ultimoAviso, setUltimoAviso] = useState<string | null>(null);

  const fechar = () => {
    setCenario(null);
  };

  return (
    <div>
      <DemoGrupo titulo="Estados do diálogo de erro">
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setCenario("padrao");
            setUltimoAviso(null);
          }}
        >
          Prévio (God of War · com código e reparo)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setCenario("sem-codigo");
            setUltimoAviso(null);
          }}
        >
          Sem código de saída
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setCenario("sem-reparo");
            setUltimoAviso(null);
          }}
        >
          Sem reparo de arquivos
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setCenario("simples");
            setUltimoAviso(null);
          }}
        >
          Sem capa nem código
        </BotaoDesktop>
      </DemoGrupo>

      {ultimoAviso && (
        <p className="label-sm" style={{ marginTop: "var(--space-12)" }}>
          Ação disparada: <strong>{ultimoAviso}</strong>
        </p>
      )}

      {/* Cenário 1: Padrão fiel à prévia do design system */}
      {cenario === "padrao" && (
        <ErroDoJogo
          open={true}
          gameTitle="God of War: Ragnarök"
          subtitle="Hoje, 14:02 · 12 segundos depois de abrir"
          coverUrl="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
          exitCode="0xC0000005"
          message="Parece um problema de memória no jogo. Isso costuma vir de mods, de sobreposições (Discord, gravadores) ou de falta de memória de vídeo."
          canRepair={true}
          labels={ROTULOS}
          onRetry={() => {
            setUltimoAviso("Tentar de novo acionado");
            fechar();
          }}
          onRepair={() => {
            setUltimoAviso("Reparar arquivos acionado");
            fechar();
          }}
          onSaveReport={() => {
            setUltimoAviso("Salvar relatório acionado");
          }}
          onClose={fechar}
        />
      )}

      {/* Cenário 2: Sem código de saída (não exibe a linha do código) */}
      {cenario === "sem-codigo" && (
        <ErroDoJogo
          open={true}
          gameTitle="Hades II"
          subtitle="Ontem, 22:45 · 45 minutos jogados"
          coverUrl="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
          exitCode={null}
          message="O jogo encerrou inesperadamente sem registrar código de erro."
          canRepair={true}
          labels={ROTULOS}
          onRetry={() => {
            setUltimoAviso("Tentar de novo acionado");
            fechar();
          }}
          onRepair={() => {
            setUltimoAviso("Reparar arquivos acionado");
            fechar();
          }}
          onSaveReport={() => {
            setUltimoAviso("Salvar relatório acionado");
          }}
          onClose={fechar}
        />
      )}

      {/* Cenário 3: Sem reparo (não exibe o botão de reparar) */}
      {cenario === "sem-reparo" && (
        <ErroDoJogo
          open={true}
          gameTitle="Celeste"
          subtitle="Hoje, 10:15 · 3 minutos depois de abrir"
          coverUrl="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
          exitCode={1}
          message="O processo do jogo retornou código 1. Esta versão instalada não possui suporte a reparo automático de arquivos."
          canRepair={false}
          labels={ROTULOS}
          onRetry={() => {
            setUltimoAviso("Tentar de novo acionado");
            fechar();
          }}
          onSaveReport={() => {
            setUltimoAviso("Salvar relatório acionado");
          }}
          onClose={fechar}
        />
      )}

      {/* Cenário 4: Simples (sem capa e sem código) */}
      {cenario === "simples" && (
        <ErroDoJogo
          open={true}
          gameTitle="Jogo Desconhecido"
          subtitle="Hoje, 16:30"
          message="Ocorreu uma falha inesperada durante a execução."
          canRepair={false}
          labels={ROTULOS}
          onRetry={() => {
            setUltimoAviso("Tentar de novo acionado");
            fechar();
          }}
          onClose={fechar}
        />
      )}
    </div>
  );
}
