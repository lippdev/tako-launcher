import { useState } from "react";
import { AssistenteInicial, type AssistenteInicialStep } from "./AssistenteInicial";
import { GrupoDeConfiguracao, LinhaDeConfiguracao } from "../LinhaDeConfiguracao/LinhaDeConfiguracao";
import { Chave } from "../Chave/Chave";
import { EscolhaDoOnboarding } from "../EscolhaDoOnboarding/EscolhaDoOnboarding";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { DemoGrupo } from "../../lib/demoGrupo";

export const title = "AssistenteInicial";
export const preview = "AssistenteInicial";

const PASSOS: readonly AssistenteInicialStep[] = [
  { id: "boas-vindas", label: "Boas-vindas" },
  { id: "lojas", label: "Lojas" },
  { id: "capas", label: "Capas e descrições" },
];

export function Demo() {
  const [passoAtivo, setPassoAtivo] = useState<number>(2);
  const [modalAberto, setModalAberto] = useState(false);
  const [ultimoAviso, setUltimoAviso] = useState<string | null>(null);

  // Estado do passo 1 (Modo de início)
  const [modoInicio, setModoInicio] = useState("desktop");

  // Estado do passo 2 (Origens de jogos instalados)
  const [steamAtivo, setSteamAtivo] = useState(true);
  const [epicAtivo, setEpicAtivo] = useState(true);
  const [gogAtivo, setGogAtivo] = useState(false);

  // Estado do passo 3 (Capas e descrições automáticas)
  const [baixarCapas, setBaixarCapas] = useState(true);
  const [buscarMetadados, setBuscarMetadados] = useState(true);
  const [economizarRede, setEconomizarRede] = useState(false);

  const obterConteudoDoPasso = (passo: number) => {
    if (passo === 1) {
      return (
        <EscolhaDoOnboarding
          variant="card"
          value={modoInicio}
          onChange={setModoInicio}
          ariaLabel="Modo de inicialização"
          options={[
            {
              value: "desktop",
              title: "Modo Desktop",
              description: "Interface clássica para teclado e mouse, com barra lateral e painel de detalhes.",
            },
            {
              value: "fullscreen",
              title: "Tela cheia",
              description: "Visual otimizado para TV e navegação completa pelo direcional do controle.",
            },
          ]}
        />
      );
    }

    if (passo === 2) {
      return (
        <GrupoDeConfiguracao>
          <LinhaDeConfiguracao
            title="Steam"
            description="18 jogos encontrados"
            control={(ids) => (
              <Chave
                checked={steamAtivo}
                onChange={setSteamAtivo}
                aria-labelledby={ids.labelId}
                aria-describedby={ids.descriptionId}
                aria-label="Ligar Steam"
              />
            )}
          />
          <LinhaDeConfiguracao
            title="Epic Games"
            description="4 jogos encontrados"
            control={(ids) => (
              <Chave
                checked={epicAtivo}
                onChange={setEpicAtivo}
                aria-labelledby={ids.labelId}
                aria-describedby={ids.descriptionId}
                aria-label="Ligar Epic Games"
              />
            )}
          />
          <LinhaDeConfiguracao
            title="GOG Galaxy"
            description="1 jogo encontrado"
            control={(ids) => (
              <Chave
                checked={gogAtivo}
                onChange={setGogAtivo}
                aria-labelledby={ids.labelId}
                aria-describedby={ids.descriptionId}
                aria-label="Ligar GOG Galaxy"
              />
            )}
          />
        </GrupoDeConfiguracao>
      );
    }

    return (
      <GrupoDeConfiguracao>
        <LinhaDeConfiguracao
          title="Baixar capas automaticamente"
          description="Busca capas verticais e fundos em alta resolução da comunidade"
          control={(ids) => (
            <Chave
              checked={baixarCapas}
              onChange={setBaixarCapas}
              aria-labelledby={ids.labelId}
              aria-describedby={ids.descriptionId}
              aria-label="Baixar capas"
            />
          )}
        />
        <LinhaDeConfiguracao
          title="Buscar dados e notas"
          description="Tempo de conclusão, notas de crítica e desenvolvedores"
          control={(ids) => (
            <Chave
              checked={buscarMetadados}
              onChange={setBuscarMetadados}
              aria-labelledby={ids.labelId}
              aria-describedby={ids.descriptionId}
              aria-label="Buscar dados"
            />
          )}
        />
        <LinhaDeConfiguracao
          title="Economizar dados de rede"
          description="Sincronizar imagens apenas quando conectado a rede não tarifada"
          control={(ids) => (
            <Chave
              checked={economizarRede}
              onChange={setEconomizarRede}
              aria-labelledby={ids.labelId}
              aria-describedby={ids.descriptionId}
              aria-label="Economizar rede"
            />
          )}
        />
      </GrupoDeConfiguracao>
    );
  };

  const titulos = [
    {
      kicker: "Primeiros passos · 1 de 3",
      title: "Boas-vindas ao TAKO",
      desc: "Configure o básico para organizar e abrir seus jogos em um só lugar.",
      nextLabel: "Continuar",
    },
    {
      kicker: "Primeiros passos · 2 de 3",
      title: "Lojas",
      desc: "O TAKO lê os jogos instalados destas lojas. Desligue as que você não usa.",
      nextLabel: "Continuar",
    },
    {
      kicker: "Primeiros passos · 3 de 3",
      title: "Capas e descrições",
      desc: "Baixar capas em alta resolução e dados dos jogos automaticamente.",
      nextLabel: "Concluir",
    },
  ];

  const infoAtual = titulos[passoAtivo - 1] ?? titulos[0]!;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-24)" }}>
      <DemoGrupo titulo="Navegação entre os passos do assistente inicial">
        <BotaoDesktop
          variant={passoAtivo === 1 ? "primary" : "outlined"}
          size="sm"
          onClick={() => {
            setPassoAtivo(1);
          }}
        >
          Passo 1: Boas-vindas
        </BotaoDesktop>
        <BotaoDesktop
          variant={passoAtivo === 2 ? "primary" : "outlined"}
          size="sm"
          onClick={() => {
            setPassoAtivo(2);
          }}
        >
          Passo 2: Lojas (Padrão da prévia)
        </BotaoDesktop>
        <BotaoDesktop
          variant={passoAtivo === 3 ? "primary" : "outlined"}
          size="sm"
          onClick={() => {
            setPassoAtivo(3);
          }}
        >
          Passo 3: Capas e descrições
        </BotaoDesktop>
        <BotaoDesktop
          variant="chip"
          size="sm"
          icon="panel"
          onClick={() => {
            setModalAberto(true);
            setUltimoAviso(null);
          }}
        >
          Abrir como diálogo modal (com véu)
        </BotaoDesktop>
      </DemoGrupo>

      {ultimoAviso && (
        <p className="label-sm" style={{ margin: 0, color: "var(--text-muted)" }}>
          Ação disparada: <strong>{ultimoAviso}</strong>
        </p>
      )}

      {/* Visualização inline para a galeria */}
      <AssistenteInicial
        inline
        steps={PASSOS}
        currentStep={passoAtivo}
        stepsAriaLabel="Passos do assistente inicial"
        kicker={infoAtual.kicker}
        title={infoAtual.title}
        description={infoAtual.desc}
        skipLabel="Pular por agora"
        onSkip={() => {
          setUltimoAviso("Ação: Pular por agora");
        }}
        backLabel={passoAtivo > 1 ? "Voltar" : undefined}
        onBack={() => {
          setPassoAtivo((p) => Math.max(1, p - 1));
          setUltimoAviso("Ação: Voltar");
        }}
        nextLabel={infoAtual.nextLabel}
        nextIcon={passoAtivo === 3 ? "check" : "chevron-right"}
        onNext={() => {
          if (passoAtivo < 3) {
            setPassoAtivo((p) => p + 1);
            setUltimoAviso(`Avançou para o passo ${passoAtivo + 1}`);
          } else {
            setUltimoAviso("Assistente concluído com sucesso");
          }
        }}
        closeLabel="Fechar"
      >
        {obterConteudoDoPasso(passoAtivo)}
      </AssistenteInicial>

      {/* Diálogo modal para testar o comportamento em tela cheia com véu */}
      <AssistenteInicial
        open={modalAberto}
        onClose={() => {
          setModalAberto(false);
        }}
        steps={PASSOS}
        currentStep={passoAtivo}
        stepsAriaLabel="Passos do assistente inicial"
        kicker={infoAtual.kicker}
        title={infoAtual.title}
        description={infoAtual.desc}
        skipLabel="Pular por agora"
        onSkip={() => {
          setModalAberto(false);
          setUltimoAviso("Fechou ao pular");
        }}
        backLabel={passoAtivo > 1 ? "Voltar" : undefined}
        onBack={() => {
          setPassoAtivo((p) => Math.max(1, p - 1));
        }}
        nextLabel={infoAtual.nextLabel}
        nextIcon={passoAtivo === 3 ? "check" : "chevron-right"}
        onNext={() => {
          if (passoAtivo < 3) {
            setPassoAtivo((p) => p + 1);
          } else {
            setModalAberto(false);
            setUltimoAviso("Assistente concluído no modal");
          }
        }}
        closeLabel="Fechar"
      >
        {obterConteudoDoPasso(passoAtivo)}
      </AssistenteInicial>
    </div>
  );
}
