import { useState } from "react";
import {
  AtualizacaoDoApp,
  type AtualizacaoDoAppRotulos,
  type EstadoAtualizacao,
} from "./AtualizacaoDoApp";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { DemoGrupo } from "../../lib/demoGrupo";

export const title = "AtualizacaoDoApp";
export const preview = "AtualizacaoDoApp";

const ROTULOS: AtualizacaoDoAppRotulos = {
  fechar: "Fechar (Esc)",
  depois: "Depois",
  novidades: "Novidades",
  baixar: "Baixar atualização",
  reiniciar: "Reiniciar agora",
  tentarDeNovo: "Tentar de novo",
  tituloDisponivel: "Atualização disponível",
  tituloBaixando: "Baixando atualização",
  tituloPronta: "Atualização pronta",
  tituloErro: "Não foi possível atualizar",
  ajudaBaixando: "Pode fechar: o download continua em segundo plano e avisa ao terminar.",
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
          estado="pronta"
          versao="TAKO 0.2.1"
          subtitulo="TAKO 0.2.1 · reinicia em uns 5 s"
          novidades={NOVIDADES_EXEMPLO}
          rotulos={ROTULOS}
          onDepois={() => {
            setUltimoAviso("Depois acionado");
            fechar();
          }}
          onReiniciar={() => {
            setUltimoAviso("Reiniciar agora acionado");
            fechar();
          }}
          onFechar={fechar}
        />
      )}

      {/* Estado 2: Baixando (barra de progresso com scaleX) */}
      {estadoAtivo === "baixando" && (
        <AtualizacaoDoApp
          open={true}
          estado="baixando"
          versao="TAKO 0.2.1"
          progresso={progresso}
          progressoTexto={`${progresso}% · ${Math.round((progresso * 27) / 100)} MB de 27 MB`}
          rotulos={ROTULOS}
          onDepois={() => {
            setUltimoAviso("Depois acionado durante download");
            fechar();
          }}
          onFechar={fechar}
        />
      )}

      {/* Estado 3: Disponível (versão e novidades) */}
      {estadoAtivo === "disponivel" && (
        <AtualizacaoDoApp
          open={true}
          estado="disponivel"
          versao="TAKO 0.2.1"
          novidades={NOVIDADES_EXEMPLO}
          rotulos={ROTULOS}
          onBaixar={() => {
            setUltimoAviso("Baixar atualização acionado");
            setEstadoAtivo("baixando");
          }}
          onDepois={() => {
            setUltimoAviso("Depois acionado");
            fechar();
          }}
          onFechar={fechar}
        />
      )}

      {/* Estado 4: Erro */}
      {estadoAtivo === "erro" && (
        <AtualizacaoDoApp
          open={true}
          estado="erro"
          versao="TAKO 0.2.1"
          mensagemErro="Não foi possível conectar ao servidor de atualizações. Verifique sua conexão com a internet e tente novamente."
          rotulos={ROTULOS}
          onTentarDeNovo={() => {
            setUltimoAviso("Tentar de novo acionado");
            setEstadoAtivo("baixando");
          }}
          onFechar={fechar}
        />
      )}
    </div>
  );
}
