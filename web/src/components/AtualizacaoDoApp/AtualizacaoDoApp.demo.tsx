import { useState } from "react";
import {
  AtualizacaoDoApp,
  type AtualizacaoDoAppLabels,
  type EstadoAtualizacao,
} from "./AtualizacaoDoApp";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { DemoGrupo } from "../../lib/demoGrupo";

export const title = "AtualizacaoDoApp";
export const preview = "AtualizacaoDoApp";

const ROTULOS: AtualizacaoDoAppLabels = {
  close: "Fechar (Esc)",
  later: "Depois",
  notes: "Novidades",
  download: "Baixar atualização",
  restart: "Reiniciar agora",
  retry: "Tentar de novo",
  availableTitle: "Atualização disponível",
  downloadingTitle: "Baixando atualização",
  readyTitle: "Atualização pronta",
  errorTitle: "Não foi possível atualizar",
  downloadingHelp: "Pode fechar: o download continua em segundo plano e avisa ao terminar.",
};

const NOVIDADES_EXEMPLO: readonly string[] = [
  "Novo visual: tema TAKO com as cores da arte do jogo",
  "Downloads na barra de cima",
  "Corrigido: lista recolhida perdia o jogo escolhido",
];

export function Demo() {
  const [estadoAtivo, setEstadoAtivo] = useState<EstadoAtualizacao | null>(null);
  const [progresso, setProgresso] = useState(45);
  const [ultimoAviso, setUltimoAviso] = useState<string | null>(null);

  const fechar = () => {
    setEstadoAtivo(null);
  };

  return (
    <div>
      <DemoGrupo titulo="Estados do fluxo de atualização">
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setEstadoAtivo("pronta");
            setUltimoAviso(null);
          }}
        >
          Pronta (padrão da prévia)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setEstadoAtivo("baixando");
            setUltimoAviso(null);
          }}
        >
          Baixando (progresso: {progresso}%)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setEstadoAtivo("disponivel");
            setUltimoAviso(null);
          }}
        >
          Disponível (novidades em texto)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setEstadoAtivo("erro");
            setUltimoAviso(null);
          }}
        >
          Erro na atualização
        </BotaoDesktop>
      </DemoGrupo>

      <DemoGrupo titulo="Ajuste do progresso para teste do estado 'baixando'">
        <BotaoDesktop
          variant="chip"
          onClick={() => {
            setProgresso(15);
          }}
        >
          15%
        </BotaoDesktop>
        <BotaoDesktop
          variant="chip"
          onClick={() => {
            setProgresso(45);
          }}
        >
          45%
        </BotaoDesktop>
        <BotaoDesktop
          variant="chip"
          onClick={() => {
            setProgresso(85);
          }}
        >
          85%
        </BotaoDesktop>
        <BotaoDesktop
          variant="chip"
          onClick={() => {
            setProgresso(100);
          }}
        >
          100%
        </BotaoDesktop>
      </DemoGrupo>

      {ultimoAviso && (
        <p className="label-sm" style={{ marginTop: "var(--space-12)" }}>
          Ação disparada: <strong>{ultimoAviso}</strong>
        </p>
      )}

      {/* Estado 1: Pronta (diálogo fiel à prévia do design system) */}
      {estadoAtivo === "pronta" && (
        <AtualizacaoDoApp
          open={true}
          state="pronta"
          version="TAKO 0.2.1"
          subtitle="TAKO 0.2.1 · reinicia em uns 5 s"
          releaseNotes={NOVIDADES_EXEMPLO}
          labels={ROTULOS}
          onLater={() => {
            setUltimoAviso("Depois acionado");
            fechar();
          }}
          onRestart={() => {
            setUltimoAviso("Reiniciar agora acionado");
            fechar();
          }}
          onClose={fechar}
        />
      )}

      {/* Estado 2: Baixando (barra de progresso com scaleX) */}
      {estadoAtivo === "baixando" && (
        <AtualizacaoDoApp
          open={true}
          state="baixando"
          version="TAKO 0.2.1"
          progress={progresso}
          progressText={`${progresso}% · ${Math.round((progresso * 27) / 100)} MB de 27 MB`}
          labels={ROTULOS}
          onLater={() => {
            setUltimoAviso("Depois acionado durante download");
            fechar();
          }}
          onClose={fechar}
        />
      )}

      {/* Estado 3: Disponível (versão e novidades) */}
      {estadoAtivo === "disponivel" && (
        <AtualizacaoDoApp
          open={true}
          state="disponivel"
          version="TAKO 0.2.1"
          releaseNotes={NOVIDADES_EXEMPLO}
          labels={ROTULOS}
          onDownload={() => {
            setUltimoAviso("Baixar atualização acionado");
            setEstadoAtivo("baixando");
          }}
          onLater={() => {
            setUltimoAviso("Depois acionado");
            fechar();
          }}
          onClose={fechar}
        />
      )}

      {/* Estado 4: Erro */}
      {estadoAtivo === "erro" && (
        <AtualizacaoDoApp
          open={true}
          state="erro"
          version="TAKO 0.2.1"
          errorMessage="Não foi possível conectar ao servidor de atualizações. Verifique sua conexão com a internet e tente novamente."
          labels={ROTULOS}
          onRetry={() => {
            setUltimoAviso("Tentar de novo acionado");
            setEstadoAtivo("baixando");
          }}
          onClose={fechar}
        />
      )}
    </div>
  );
}
