import { useState } from "react";
import { DemoGrupo } from "../../lib/demoGrupo";
import {
  ConfiguracoesDoJogo,
  type ChaveConfiguracaoJogo,
  type ValorConfiguracaoJogo,
  type ConfiguracoesDoJogoRotulos,
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

const ROTULOS_PADRAO: ConfiguracoesDoJogoRotulos = {
  titulo: "Configurações deste jogo",
  fechar: "Fechar (Esc)",
  grupoTela: "Tela",
  grupoSom: "Som",
  grupoAbrirOJogo: "Abrir o jogo",
  monitor: "Usar a tela de jogo",
  resolucaoETaxa: "Resolução e taxa",
  hdr: "HDR",
  vrr: "VRR",
  saidaDeAudio: "Saída de áudio",
  argumentos: "Argumentos extras",
  pastaDeTrabalho: "Pasta de trabalho",
  variaveisDeAmbiente: "Variáveis de ambiente",
  comandosAntesEDepois: "Comandos antes e depois",
  usarTudoDoTako: "Usar tudo do TAKO",
  padraoPrefixo: "Padrão: ",
  nenhum: "Nenhum",
  nenhuma: "Nenhuma",
  ligado: "Ligado",
  desligado: "desligado",
  singularMudanca: "mudança",
  pluralMudancas: "mudanças",
  nenhumaMudanca: "nenhuma mudança",
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
    { value: "automatico", label: "Automático" },
  ],
  vrr: [
    { value: "desligado", label: "Desligado" },
    { value: "ligado", label: "Ligado" },
  ],
  saidaDeAudio: [
    { value: "nao_mudar", label: "Não mudar" },
    { value: "tv", label: "TV (HDMI)" },
    { value: "fone", label: "Fones de ouvido" },
    { value: "alto_falantes", label: "Alto-falantes" },
  ],
};

const VALORES_PADRAO_GOW: ValoresConfiguracoesDoJogo = {
  monitor: false,
  resolucaoETaxa: "do_jogo",
  hdr: "desligado",
  saidaDeAudio: "nao_mudar",
  argumentos: "",
  variaveisDeAmbiente: "",
  comandosAntesEDepois: "",
};

export function Demo() {
  const [painelAberto, setPainelAberto] = useState<"mudado" | "padrao" | "sem_dado" | null>(null);

  // Cenário 1: Valores com mudanças (Fiel ao preview.html)
  const [valoresMudados, setValoresMudados] = useState<ValoresConfiguracoesDoJogo>({
    monitor: true,
    resolucaoETaxa: "do_jogo",
    hdr: "ligado",
    saidaDeAudio: "nao_mudar",
    argumentos: "",
    variaveisDeAmbiente: "",
    comandosAntesEDepois: "",
  });

  const descricoesMudados: DescricoesConfiguracoesDoJogo = {
    monitor: "Padrão: Tela 1",
    resolucaoETaxa: "2560 × 1440 · 144 Hz",
    hdr: "Padrão: desligado",
    saidaDeAudio: "Segue o TAKO",
    argumentos: "Nenhum",
    variaveisDeAmbiente: "Nenhuma",
    comandosAntesEDepois: "Nenhum",
  };

  const padroesMudados: Partial<ValoresConfiguracoesDoJogo> = {
    ...VALORES_PADRAO_GOW,
  };

  const handleMudarValoresMudados = (chave: ChaveConfiguracaoJogo, valor: ValorConfiguracaoJogo) => {
    setValoresMudados((prev) => ({ ...prev, [chave]: valor }));
  };

  const handleResetarMudados = () => {
    setValoresMudados({ ...VALORES_PADRAO_GOW });
  };

  // Cenário 2: Todos os valores no padrão do TAKO (0 mudanças)
  const [valoresPadrao, setValoresPadrao] = useState<ValoresConfiguracoesDoJogo>({
    monitor: false,
    resolucaoETaxa: "do_jogo",
    hdr: "desligado",
    saidaDeAudio: "nao_mudar",
    argumentos: "",
    pastaDeTrabalho: "",
    variaveisDeAmbiente: "",
    comandosAntesEDepois: "",
  });

  const descricoesPadrao: DescricoesConfiguracoesDoJogo = {
    resolucaoETaxa: "1920 × 1080 · 60 Hz",
    saidaDeAudio: "Segue o TAKO",
    argumentos: "Nenhum",
    pastaDeTrabalho: "Padrão da instalação",
    variaveisDeAmbiente: "Nenhuma",
    comandosAntesEDepois: "Nenhum",
  };

  // Cenário 3: Linhas sem dado escondidas
  const [valoresSemDado, setValoresSemDado] = useState<ValoresConfiguracoesDoJogo>({
    resolucaoETaxa: "1080p",
    saidaDeAudio: "fone",
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-24)" }}>
      <DemoGrupo titulo="Interativo — abrir em camada lateral (PainelLateral wide de 480px)">
        <div style={{ display: "flex", flexWrap: "wrap", gap: "10px" }}>
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
            Abrir: God of War (2 mudanças destacadas)
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
            Abrir: Hades II (valores padrão, 0 mudanças)
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
            Abrir: Celeste (linhas sem dado escondidas)
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
            tituloDoJogo="God of War: Ragnarök"
            capaUrl={CAPA_GOW}
            valores={valoresMudados}
            padroes={padroesMudados}
            opcoes={OPCOES_EXEMPLO}
            descricoes={descricoesMudados}
            rotulos={ROTULOS_PADRAO}
            onChange={handleMudarValoresMudados}
            onResetarAoPadrao={handleResetarMudados}
            onCliqueLinha={(chave) => {
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
            tituloDoJogo="Hades II"
            valores={valoresPadrao}
            padroes={valoresPadrao}
            opcoes={OPCOES_EXEMPLO}
            descricoes={descricoesPadrao}
            rotulos={ROTULOS_PADRAO}
            onChange={(chave: ChaveConfiguracaoJogo, v: ValorConfiguracaoJogo) =>
              setValoresPadrao((p) => ({ ...p, [chave]: v }))
            }
            onResetarAoPadrao={() => {}}
          />
        )}

        {painelAberto === "sem_dado" && (
          <ConfiguracoesDoJogo
            open={true}
            onClose={() => setPainelAberto(null)}
            tituloDoJogo="Celeste"
            valores={valoresSemDado}
            opcoes={OPCOES_EXEMPLO}
            rotulos={ROTULOS_PADRAO}
            onChange={(chave: ChaveConfiguracaoJogo, v: ValorConfiguracaoJogo) =>
              setValoresSemDado((p) => ({ ...p, [chave]: v }))
            }
          />
        )}
      </DemoGrupo>

      <DemoGrupo titulo="1. Cenário Fiel ao preview.html — God of War: Ragnarök (2 mudanças destacadas com 'Padrão: …')">
        <ConfiguracoesDoJogo
          inline
          tituloDoJogo="God of War: Ragnarök"
          capaUrl={CAPA_GOW}
          valores={valoresMudados}
          padroes={padroesMudados}
          opcoes={OPCOES_EXEMPLO}
          descricoes={descricoesMudados}
          rotulos={ROTULOS_PADRAO}
          onChange={handleMudarValoresMudados}
          onResetarAoPadrao={handleResetarMudados}
          onCliqueLinha={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="2. Cenário Valores Padrão — Hades II (0 mudanças, nenhuma linha destacada)">
        <ConfiguracoesDoJogo
          inline
          tituloDoJogo="Hades II"
          valores={valoresPadrao}
          padroes={valoresPadrao}
          opcoes={OPCOES_EXEMPLO}
          descricoes={descricoesPadrao}
          rotulos={ROTULOS_PADRAO}
          onChange={(chave: ChaveConfiguracaoJogo, v: ValorConfiguracaoJogo) =>
            setValoresPadrao((p) => ({ ...p, [chave]: v }))
          }
          onResetarAoPadrao={() => {}}
        />
      </DemoGrupo>

      <DemoGrupo titulo="3. Cenário Linhas Sem Dado Escondidas — Celeste (apenas resolução e saída de áudio)">
        <ConfiguracoesDoJogo
          inline
          tituloDoJogo="Celeste"
          valores={valoresSemDado}
          opcoes={OPCOES_EXEMPLO}
          rotulos={ROTULOS_PADRAO}
          onChange={(chave: ChaveConfiguracaoJogo, v: ValorConfiguracaoJogo) =>
            setValoresSemDado((p) => ({ ...p, [chave]: v }))
          }
        />
      </DemoGrupo>
    </div>
  );
}
