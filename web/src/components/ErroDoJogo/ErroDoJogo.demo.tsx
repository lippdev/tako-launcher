import { useState } from "react";
import { ErroDoJogo, type ErroDoJogoRotulos } from "./ErroDoJogo";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { DemoGrupo } from "../../lib/demoGrupo";

export const title = "ErroDoJogo";
export const preview = "ErroDoJogo";

const ROTULOS: ErroDoJogoRotulos = {
  fechar: "Fechar (Esc)",
  detalhesTecnicos: "Detalhes técnicos",
  salvarRelatorio: "Salvar relatório",
  reparar: "Reparar arquivos",
  tentarDeNovo: "Tentar de novo",
  sufixoTitulo: "fechou sozinho",
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
          tituloDoJogo="God of War: Ragnarök"
          subtitulo="Hoje, 14:02 · 12 segundos depois de abrir"
          capaUrl="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
          codigoDeSaida="0xC0000005"
          mensagem="Parece um problema de memória no jogo. Isso costuma vir de mods, de sobreposições (Discord, gravadores) ou de falta de memória de vídeo."
          podeReparar={true}
          rotulos={ROTULOS}
          onTentarDeNovo={() => {
            setUltimoAviso("Tentar de novo acionado");
            fechar();
          }}
          onReparar={() => {
            setUltimoAviso("Reparar arquivos acionado");
            fechar();
          }}
          onSalvarRelatorio={() => {
            setUltimoAviso("Salvar relatório acionado");
          }}
          onFechar={fechar}
        />
      )}

      {/* Cenário 2: Sem código de saída (não exibe a linha do código) */}
      {cenario === "sem-codigo" && (
        <ErroDoJogo
          open={true}
          tituloDoJogo="Hades II"
          subtitulo="Ontem, 22:45 · 45 minutos jogados"
          capaUrl="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
          codigoDeSaida={null}
          mensagem="O jogo encerrou inesperadamente sem registrar código de erro."
          podeReparar={true}
          rotulos={ROTULOS}
          onTentarDeNovo={() => {
            setUltimoAviso("Tentar de novo acionado");
            fechar();
          }}
          onReparar={() => {
            setUltimoAviso("Reparar arquivos acionado");
            fechar();
          }}
          onSalvarRelatorio={() => {
            setUltimoAviso("Salvar relatório acionado");
          }}
          onFechar={fechar}
        />
      )}

      {/* Cenário 3: Sem reparo (não exibe o botão de reparar) */}
      {cenario === "sem-reparo" && (
        <ErroDoJogo
          open={true}
          tituloDoJogo="Celeste"
          subtitulo="Hoje, 10:15 · 3 minutos depois de abrir"
          capaUrl="data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
          codigoDeSaida={1}
          mensagem="O processo do jogo retornou código 1. Esta versão instalada não possui suporte a reparo automático de arquivos."
          podeReparar={false}
          rotulos={ROTULOS}
          onTentarDeNovo={() => {
            setUltimoAviso("Tentar de novo acionado");
            fechar();
          }}
          onSalvarRelatorio={() => {
            setUltimoAviso("Salvar relatório acionado");
          }}
          onFechar={fechar}
        />
      )}

      {/* Cenário 4: Simples (sem capa e sem código) */}
      {cenario === "simples" && (
        <ErroDoJogo
          open={true}
          tituloDoJogo="Jogo Desconhecido"
          subtitulo="Hoje, 16:30"
          mensagem="Ocorreu uma falha inesperada durante a execução."
          podeReparar={false}
          rotulos={ROTULOS}
          onTentarDeNovo={() => {
            setUltimoAviso("Tentar de novo acionado");
            fechar();
          }}
          onFechar={fechar}
        />
      )}
    </div>
  );
}
