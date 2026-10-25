import { useState } from "react";
import { TelaOnboarding } from "../TelaOnboarding/TelaOnboarding";
import {
  OnboardingCapas,
  type OnboardingCapasOption,
} from "./OnboardingCapas";

export const title = "OnboardingCapas";
export const preview = "OnboardingCapas";

const SAMPLE_COVER = `data:image/svg+xml,${encodeURIComponent(`
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 400" width="300" height="400">
  <defs>
    <linearGradient id="coverGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="navy"/>
      <stop offset="100%" stop-color="darkslateblue"/>
    </linearGradient>
  </defs>
  <rect width="300" height="400" fill="url(#coverGrad)"/>
  <circle cx="150" cy="170" r="65" fill="mediumpurple" opacity="0.5"/>
  <text x="150" y="280" font-family="sans-serif" font-size="20" font-weight="bold" fill="white" text-anchor="middle">GOD OF WAR</text>
</svg>
`)}`;

export function Demo() {
  const [selectedSource, setSelectedSource] = useState("igdb");

  const options: OnboardingCapasOption[] = [
    {
      value: "igdb",
      title: "Buscar na IGDB",
      description: "Envia só o nome de cada jogo · uns 40 MB de imagens",
    },
    {
      value: "local",
      title: "Usar só o que as lojas têm",
      description: "Nada sai do PC · 9 jogos ficam sem capa",
    },
  ];

  return (
    <div style={{ width: "1600px", height: "900px", position: "relative" }}>
      <TelaOnboarding
        skipLabel="Pular configuração"
        onSkip={() => alert("Pular")}
      >
        <OnboardingCapas
          stepLabel="3 de 5"
          stepAriaLabel="Passo 3 de 5"
          title="Capas e descrições."
          description="A IGDB traz capa, arte de fundo, descrição e gênero de cada jogo."
          gameTitle="God of War Ragnarök"
          gameCoverUrl={SAMPLE_COVER}
          comparisonNote="Sem a IGDB fica só o nome sobre a cor do jogo."
          optionsAriaLabel="Fonte de capas e dados"
          options={options}
          value={selectedSource}
          onChange={setSelectedSource}
          continueLabel="Continuar"
          onContinue={() => alert("Continuar")}
          backLabel="Voltar"
          onBack={() => alert("Voltar")}
          footerNote="Capas com dúvida vão para Configurações › Capas e dados."
        />
      </TelaOnboarding>
    </div>
  );
}
