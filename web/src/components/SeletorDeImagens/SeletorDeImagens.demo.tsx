import { useState } from "react";
import {
  SeletorDeImagens,
  type OpcaoDeImagem,
  type TipoDeImagem,
} from "./SeletorDeImagens";
import { BotaoDesktop } from "../BotaoDesktop/BotaoDesktop";
import { DemoGrupo } from "../../lib/demoGrupo";

export const title = "SeletorDeImagens";
export const preview = "SeletorDeImagens";

function criaSvgArte(tipo: string, num: number, cor: string, w = 600, h = 900): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${w} ${h}" width="${w}" height="${h}"><rect width="${w}" height="${h}" fill="${cor}"/><circle cx="${w / 2}" cy="${h / 2 - 40}" r="${Math.min(w, h) / 4}" fill="white" fill-opacity="0.12"/><text x="${w / 2}" y="${h / 2 + 10}" fill="white" font-size="${Math.round(w / 12)}" font-weight="bold" font-family="sans-serif" text-anchor="middle">Hades II</text><text x="${w / 2}" y="${h / 2 + 50}" fill="white" fill-opacity="0.7" font-size="${Math.round(w / 24)}" font-family="sans-serif" text-anchor="middle">${tipo} #${num}</text></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

// Miniatura para o cabeçalho
const MINIATURA_JOGO = criaSvgArte("Capa", 1, "darkslateblue", 44, 44);

// Opções de Capa (600 × 900)
const OPCOES_CAPA: readonly OpcaoDeImagem[] = [
  { id: "c1", url: criaSvgArte("Capa Oficial", 1, "darkslateblue", 600, 900), fonte: "Steam", dimensoes: "600 × 900", estilo: "Oficial" },
  { id: "c2", url: criaSvgArte("Capa Alternativa", 2, "midnightblue", 600, 900), fonte: "SteamGridDB", dimensoes: "600 × 900", estilo: "Alternativa", atual: true },
  { id: "c3", url: criaSvgArte("Capa Estilizada", 3, "darkslategray", 600, 900), fonte: "SteamGridDB", dimensoes: "600 × 900", estilo: "Minimalista" },
  { id: "c4", url: criaSvgArte("Capa Retro", 4, "indigo", 600, 900), fonte: "SteamGridDB", dimensoes: "600 × 900", estilo: "Retro" },
  { id: "c5", url: criaSvgArte("Capa Arte Conceito", 5, "maroon", 600, 900), fonte: "SteamGridDB", dimensoes: "600 × 900", estilo: "Arte" },
  { id: "c6", url: criaSvgArte("Capa Edição Deluxe", 6, "teal", 600, 900), fonte: "Steam", dimensoes: "600 × 900", estilo: "Oficial" },
  { id: "c7", url: criaSvgArte("Capa Sem Texto", 7, "purple", 600, 900), fonte: "SteamGridDB", dimensoes: "600 × 900", estilo: "Sem texto" },
  { id: "c8", url: criaSvgArte("Capa Cartaz", 8, "darkolivegreen", 600, 900), fonte: "SteamGridDB", dimensoes: "600 × 900", estilo: "Cartaz" },
];

// Opções de Fundo (1920 × 1080)
const OPCOES_FUNDO: readonly OpcaoDeImagem[] = [
  { id: "f1", url: criaSvgArte("Fundo Submundo", 1, "midnightblue", 1920, 1080), fonte: "Steam", dimensoes: "1920 × 1080", estilo: "Oficial", atual: true },
  { id: "f2", url: criaSvgArte("Fundo Tártaro", 2, "darkslategray", 1920, 1080), fonte: "SteamGridDB", dimensoes: "1920 × 1080", estilo: "Captura" },
  { id: "f3", url: criaSvgArte("Fundo Olimpo", 3, "darkslateblue", 1920, 1080), fonte: "SteamGridDB", dimensoes: "1920 × 1080", estilo: "Arte" },
];

// Opções de Logo (contain, 1280 × 720)
const OPCOES_LOGO: readonly OpcaoDeImagem[] = [
  { id: "l1", url: criaSvgArte("Logo Principal", 1, "transparent", 1280, 720), fonte: "Steam", dimensoes: "1280 × 720", estilo: "Oficial", atual: true },
  { id: "l2", url: criaSvgArte("Logo Branco", 2, "transparent", 1280, 720), fonte: "SteamGridDB", dimensoes: "1280 × 720", estilo: "Monocromático" },
];

// Opções de Ícone (512 × 512)
const OPCOES_ICONE: readonly OpcaoDeImagem[] = [
  { id: "i1", url: criaSvgArte("Ícone Padrão", 1, "darkslateblue", 512, 512), fonte: "Steam", dimensoes: "512 × 512", atual: true },
  { id: "i2", url: criaSvgArte("Ícone Alternativo", 2, "indigo", 512, 512), fonte: "SteamGridDB", dimensoes: "512 × 512" },
  { id: "i3", url: criaSvgArte("Ícone Pixel", 3, "darkslategray", 512, 512), fonte: "SteamGridDB", dimensoes: "512 × 512" },
];

// Opções com falha simulada (imagem quebrada)
const OPCOES_COM_FALHA: readonly OpcaoDeImagem[] = [
  { id: "quebrada-1", url: "tako-invalida://imagem-inexistente.png", fonte: "SteamGridDB", dimensoes: "600 × 900" },
  { id: "valida-1", url: criaSvgArte("Capa Funcional", 1, "teal", 600, 900), fonte: "Steam", dimensoes: "600 × 900", atual: true },
  { id: "quebrada-2", url: "https://tako.local/falha-404.jpg", fonte: "Arquivo local", dimensoes: "600 × 900" },
];

type ModoDemo = "padrao" | "carregando" | "vazio" | "quebrada" | null;

export function Demo() {
  const [modoAberto, setModoAberto] = useState<ModoDemo>(null);
  const [tipoAtivo, setTipoAtivo] = useState<TipoDeImagem>("capa");
  const [ultimoAviso, setUltimoAviso] = useState<string | null>(null);

  const fechar = () => {
    setModoAberto(null);
  };

  const handleEscolher = (opcao: OpcaoDeImagem, tipo: TipoDeImagem) => {
    setUltimoAviso(`Imagem escolhida para ${tipo}: ${opcao.id} (${opcao.fonte} · ${opcao.dimensoes ?? "sem tamanho"})`);
    fechar();
  };

  return (
    <div>
      <DemoGrupo titulo="Estados do SeletorDeImagens — Padrão da prévia (1180px) · Cada tipo · Atual marcada · Carregando · Vazio · Imagem quebrada">
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setModoAberto("padrao");
            setTipoAtivo("capa");
            setUltimoAviso(null);
          }}
        >
          Diálogo completo (Hades II · Capa / Fundo / Logo / Ícone)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setModoAberto("carregando");
            setUltimoAviso(null);
          }}
        >
          Estado Carregando (com esqueletos pulsando)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setModoAberto("vazio");
            setUltimoAviso(null);
          }}
        >
          Estado Vazio (nenhuma opção encontrada)
        </BotaoDesktop>
        <BotaoDesktop
          variant="outlined"
          onClick={() => {
            setModoAberto("quebrada");
            setTipoAtivo("capa");
            setUltimoAviso(null);
          }}
        >
          Imagem quebrada (estado Sem Imagem)
        </BotaoDesktop>
      </DemoGrupo>

      {ultimoAviso && (
        <p className="label-sm" style={{ marginTop: "var(--space-12)" }}>
          Ação disparada: <strong>{ultimoAviso}</strong>
        </p>
      )}

      {/* Cenário 1: Diálogo completo fiel à prévia do design system */}
      {modoAberto === "padrao" && (
        <SeletorDeImagens
          open={true}
          tituloDoJogo="Hades II"
          capaMiniaturaUrl={MINIATURA_JOGO}
          tipoAtivo={tipoAtivo}
          onTrocarTipo={setTipoAtivo}
          opcoesPorTipo={{
            capa: OPCOES_CAPA,
            fundo: OPCOES_FUNDO,
            logo: OPCOES_LOGO,
            icone: OPCOES_ICONE,
          }}
          onEscolher={handleEscolher}
          onVoltarAoOriginal={() => {
            setUltimoAviso("Voltar à original acionado");
          }}
          onEscolherDoComputador={() => {
            setUltimoAviso("Escolher do computador acionado");
          }}
          onFechar={fechar}
        />
      )}

      {/* Cenário 2: Estado Carregando */}
      {modoAberto === "carregando" && (
        <SeletorDeImagens
          open={true}
          tituloDoJogo="Hades II"
          capaMiniaturaUrl={MINIATURA_JOGO}
          tipoAtivo="capa"
          carregandoPorTipo={{ capa: true }}
          onEscolher={handleEscolher}
          onFechar={fechar}
        />
      )}

      {/* Cenário 3: Estado Vazio */}
      {modoAberto === "vazio" && (
        <SeletorDeImagens
          open={true}
          tituloDoJogo="Hades II"
          capaMiniaturaUrl={MINIATURA_JOGO}
          tipoAtivo="capa"
          vazioPorTipo={{ capa: true }}
          opcoesPorTipo={{ capa: [] }}
          onEscolher={handleEscolher}
          onFechar={fechar}
        />
      )}

      {/* Cenário 4: Imagem quebrada que exibe fallback sem arte */}
      {modoAberto === "quebrada" && (
        <SeletorDeImagens
          open={true}
          tituloDoJogo="Hades II"
          capaMiniaturaUrl={MINIATURA_JOGO}
          tipoAtivo="capa"
          opcoesPorTipo={{ capa: OPCOES_COM_FALHA }}
          onEscolher={handleEscolher}
          onFechar={fechar}
        />
      )}
    </div>
  );
}
