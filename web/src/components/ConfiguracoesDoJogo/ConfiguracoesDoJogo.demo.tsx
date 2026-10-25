import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import {
  ConfiguracoesDoJogo,
  type ChaveConfiguracaoJogo,
  type ValorConfiguracaoJogo,
  type ConfiguracoesDoJogoLabels,
  type OpcoesConfiguracoesDoJogo,
  type DescricoesConfiguracoesDoJogo,
  type ValoresConfiguracoesDoJogo,
} from "./ConfiguracoesDoJogo";

export const title = "ConfiguracoesDoJogo";
export const preview = "ConfiguracoesDoJogo";

const CAPA_GOW = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 72 92" width="72" height="92">
  <rect width="72" height="92" fill="midnightblue" rx="12"/>
  <circle cx="36" cy="40" r="20" fill="darkslateblue"/>
  <path d="M22 64 C28 50, 44 50, 50 64 Z" fill="coral"/>
  <text x="36" y="80" font-family="sans-serif" font-size="9" font-weight="bold" fill="white" text-anchor="middle">GOW</text>
</svg>
`)}`;

const ROTULOS_PADRAO: ConfiguracoesDoJogoLabels = {
  title: "Configurações deste jogo",
  close: "Fechar (Esc)",
  screenGroup: "Tela",
  soundGroup: "Som",
  launchGroup: "Abrir o jogo",
  monitor: "Usar a tela de jogo",
  resolutionAndRate: "Resolução e taxa",
  hdr: "HDR",
  vrr: "VRR",
  audioOutput: "Saída de áudio",
  arguments: "Argumentos extras",
  workingDirectory: "Pasta de trabalho",
  environmentVariables: "Variáveis de ambiente",
  beforeAndAfterCommands: "Comandos antes e depois",
  useAllFromTako: "Usar tudo do TAKO",
  defaultPrefix: "Padrão: ",
  none: "Nenhum",
  noneFem: "Nenhuma",
  on: "Ligado",
  off: "desligado",
  singularChange: "mudança",
  pluralChanges: "mudanças",
  noChanges: "nenhuma mudança",
};

const OPCOES_EXEMPLO: OpcoesConfiguracoesDoJogo = {
  resolucaoETaxa: [
    { value: "do_jogo", label: "Do jogo" },
    { value: "1080p", label: "1920 × 1080 · 60 Hz" },
    { value: "1440p", label: "2560 × 1440 · 144 Hz" },
    { value: "4k", label: "3840 × 2160 · 120 Hz" },
  ],
  hdr: [
    { value: "desligado", label: "desligado" },
    { value: "ligado", label: "Ligado" },
  ],
  vrr: [
    { value: "do_jogo", label: "Do jogo" },
    { value: "ligado", label: "Ligado" },
    { value: "desligado", label: "desligado" },
  ],
  saidaDeAudio: [
    { value: "padrao", label: "Padrão do sistema" },
    { value: "fone", label: "Headphone (Realtek Audio)" },
    { value: "hdmi", label: "LG TV (NVIDIA High Definition Audio)" },
  ],
};

const VALORES_PADRAO_GOW: ValoresConfiguracoesDoJogo = {
  monitor: false,
  resolucaoETaxa: "do_jogo",
  hdr: "desligado",
  vrr: "do_jogo",
  saidaDeAudio: "padrao",
  argumentos: "",
  pastaDeTrabalho: "",
  variaveisDeAmbiente: "",
  comandosAntesEDepois: "",
};

export function Demo() {
  const [painelAberto, setPainelAberto] = useState<"mudado" | "padrao" | "sem_dado" | null>(null);

  // Cenário 1: 2 mudanças (fiel ao preview.html)
  const [valoresMudados, setValoresMudados] = useState<ValoresConfiguracoesDoJogo>({
    ...VALORES_PADRAO_GOW,
    monitor: true,
    resolucaoETaxa: "1440p",
    hdr: "ligado",
  });

  const handleMudarValoresMudados = (chave: ChaveConfiguracaoJogo, valor: ValorConfiguracaoJogo) => {
    setValoresMudados((prev) => ({ ...prev, [chave]: valor }));
  };

  const descricoesMudados: DescricoesConfiguracoesDoJogo = {
    monitor: "Apenas monitores conectados",
    vrr: "Sem suporte neste monitor",
    pastaDeTrabalho: "Pasta do executável",
    comandosAntesEDepois: "Script antes de abrir",
  };

  const padroesMudados: Partial<ValoresConfiguracoesDoJogo> = {
    ...VALORES_PADRAO_GOW,
  };

  const handleResetarMudados = () => {
    setValoresMudados({ ...VALORES_PADRAO_GOW });
  };

  // Cenário 2: tudo no padrão
  const [valoresPadrao, setValoresPadrao] = useState<ValoresConfiguracoesDoJogo>({
    monitor: false,
    resolucaoETaxa: "do_jogo",
    hdr: "desligado",
    vrr: "do_jogo",
    saidaDeAudio: "padrao",
    argumentos: "",
    pastaDeTrabalho: "",
    variaveisDeAmbiente: "",
    comandosAntesEDepois: "",
  });

  const descricoesPadrao: DescricoesConfiguracoesDoJogo = {
    vrr: "Sem suporte neste monitor",
  };

  // Cenário 3: campos sem dado ocultos
  const [valoresSemDado, setValoresSemDado] = useState<ValoresConfiguracoesDoJogo>({
    resolucaoETaxa: "1080p",
    saidaDeAudio: "fone",
  });

  return (
    <div>
      <DemoGrupo titulo="Interativo — abrir como painel lateral (gaveta à direita)">
        <div style={{ display: "flex", gap: "12px", flexWrap: "wrap" }}>
          <button
            type="button"
            className="tb-btn"
            style={{
              height: "36px",
              padding: "0 14px",
              borderRadius: "9px",
              border: "1px solid var(--line-strong)",
              background: "var(--surface)",
              color: "var(--text)",
              cursor: "pointer",
            }}
            onClick={() => setPainelAberto("mudado")}
          >
            Abrir: God of War (2 mudanças)
          </button>
          <button
            type="button"
            className="tb-btn"
            style={{
              height: "36px",
              padding: "0 14px",
              borderRadius: "9px",
              border: "1px solid var(--line-strong)",
              background: "var(--surface)",
              color: "var(--text)",
              cursor: "pointer",
            }}
            onClick={() => setPainelAberto("padrao")}
          >
            Abrir: Hades II (padrão)
          </button>
          <button
            type="button"
            className="tb-btn"
            style={{
              height: "36px",
              padding: "0 14px",
              borderRadius: "9px",
              border: "1px solid var(--line-strong)",
              background: "var(--surface)",
              color: "var(--text)",
              cursor: "pointer",
            }}
            onClick={() => setPainelAberto("sem_dado")}
          >
            Abrir: Celeste (sem dados opcionais)
          </button>
        </div>
        <p className="label-sm" style={{ margin: "8px 0 0" }}>
          Esc fecha a camada, Tab fica preso dentro, o foco volta para o botão que abriu.
        </p>

        {/* Camada ativa */}
        {painelAberto === "mudado" && (
          <ConfiguracoesDoJogo
            open={true}
            onClose={() => setPainelAberto(null)}
            gameTitle="God of War: Ragnarök"
            coverUrl={CAPA_GOW}
            values={valoresMudados}
            defaults={padroesMudados}
            options={OPCOES_EXEMPLO}
            descriptions={descricoesMudados}
            labels={ROTULOS_PADRAO}
            onChange={handleMudarValoresMudados}
            onResetToDefault={handleResetarMudados}
            onLineClick={(chave) => {
              if (chave === "argumentos" || chave === "variaveisDeAmbiente" || chave === "comandosAntesEDepois") {
                // Simulação de navegação para detalhes
              }
            }}
          />
        )}

        {painelAberto === "padrao" && (
          <ConfiguracoesDoJogo
            open={true}
            onClose={() => setPainelAberto(null)}
            gameTitle="Hades II"
            values={valoresPadrao}
            defaults={valoresPadrao}
            options={OPCOES_EXEMPLO}
            descriptions={descricoesPadrao}
            labels={ROTULOS_PADRAO}
            onChange={(chave: ChaveConfiguracaoJogo, v: ValorConfiguracaoJogo) =>
              setValoresPadrao((p) => ({ ...p, [chave]: v }))
            }
            onResetToDefault={() => {}}
          />
        )}

        {painelAberto === "sem_dado" && (
          <ConfiguracoesDoJogo
            open={true}
            onClose={() => setPainelAberto(null)}
            gameTitle="Celeste"
            values={valoresSemDado}
            options={OPCOES_EXEMPLO}
            labels={ROTULOS_PADRAO}
            onChange={(chave: ChaveConfiguracaoJogo, v: ValorConfiguracaoJogo) =>
              setValoresSemDado((p) => ({ ...p, [chave]: v }))
            }
          />
        )}
      </DemoGrupo>

      <DemoGrupo titulo="1. Cenário Fiel ao preview.html — God of War: Ragnarök (2 mudanças destacadas com 'Padrão: …')">
        <ConfiguracoesDoJogo
          inline
          gameTitle="God of War: Ragnarök"
          coverUrl={CAPA_GOW}
          values={valoresMudados}
          defaults={padroesMudados}
          options={OPCOES_EXEMPLO}
          descriptions={descricoesMudados}
          labels={ROTULOS_PADRAO}
          onChange={handleMudarValoresMudados}
          onResetToDefault={handleResetarMudados}
          onLineClick={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="2. Cenário Valores Padrão — Hades II (0 mudanças, nenhuma linha destacada)">
        <ConfiguracoesDoJogo
          inline
          gameTitle="Hades II"
          values={valoresPadrao}
          defaults={valoresPadrao}
          options={OPCOES_EXEMPLO}
          descriptions={descricoesPadrao}
          labels={ROTULOS_PADRAO}
          onChange={(chave: ChaveConfiguracaoJogo, v: ValorConfiguracaoJogo) =>
            setValoresPadrao((p) => ({ ...p, [chave]: v }))
          }
          onResetToDefault={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="3. Cenário Linhas Sem Dado Escondidas — Celeste (apenas resolução e saída de áudio)">
        <ConfiguracoesDoJogo
          inline
          gameTitle="Celeste"
          values={valoresSemDado}
          options={OPCOES_EXEMPLO}
          labels={ROTULOS_PADRAO}
          onChange={(chave: ChaveConfiguracaoJogo, v: ValorConfiguracaoJogo) =>
            setValoresSemDado((p) => ({ ...p, [chave]: v }))
          }
        />
      </DemoGrupo>
    </div>
  );
}
