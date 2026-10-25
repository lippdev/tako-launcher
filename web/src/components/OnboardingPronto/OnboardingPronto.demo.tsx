import { Icon } from "../Icon/Icon";
import { TelaOnboarding } from "../TelaOnboarding/TelaOnboarding";
import {
  OnboardingPronto,
  type OnboardingSummaryItem,
} from "./OnboardingPronto";

export const title = "OnboardingPronto";
export const preview = "OnboardingPronto";

function RetroArchIcon() {
  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: "24px",
        height: "24px",
        borderRadius: "6px",
        backgroundColor: "var(--glass-control)",
        color: "var(--text)",
        fontWeight: 700,
        fontSize: "11px",
      }}
    >
      RA
    </span>
  );
}

export function Demo() {
  const storeIcon = <Icon name="store" size="sm" />;

  const summaryItems: OnboardingSummaryItem[] = [
    {
      id: "steam",
      name: "Steam",
      categoryOrType: "Loja",
      icon: storeIcon,
      countLabel: "18 jogos",
    },
    {
      id: "epic",
      name: "Epic Games",
      categoryOrType: "Loja",
      icon: storeIcon,
      countLabel: "4 jogos",
    },
    {
      id: "retroarch",
      name: "RetroArch",
      categoryOrType: "Emulador",
      icon: <RetroArchIcon />,
      countLabel: "212 jogos",
    },
  ];

  const selectedMode = {
    title: "Abre em janela",
    description: "Escolhido no passo 4",
    changeLabel: "Trocar",
    onChangeMode: () => alert("Trocar modo"),
  };

  return (
    <div style={{ width: "1600px", height: "900px", position: "relative" }}>
      <TelaOnboarding
        skipLabel="Pular configuração"
        onSkip={() => alert("Pular")}
      >
        <OnboardingPronto
          stepLabel="5 de 5"
          stepAriaLabel="Passo 5 de 5"
          title="235 jogos prontos."
          description="As capas que faltam continuam chegando em segundo plano."
          summaryItems={summaryItems}
          selectedMode={selectedMode}
          finishLabel="Abrir a biblioteca"
          onFinish={() => alert("Abrir biblioteca")}
          reviewCoversLabel="Revisar capas"
          onReviewCovers={() => alert("Revisar capas")}
        />
      </TelaOnboarding>
    </div>
  );
}
