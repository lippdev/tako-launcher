import { useState } from "react";
import { TelaOnboarding } from "../TelaOnboarding/TelaOnboarding";
import {
  OnboardingModo,
  type OnboardingModoOption,
} from "./OnboardingModo";

export const title = "OnboardingModo";
export const preview = "OnboardingModo";

function WindowThumbnail() {
  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        borderRadius: "8px",
        overflow: "hidden",
        backgroundColor: "var(--surface)",
        border: "1px solid var(--line)",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div
        style={{
          height: "18px",
          backgroundColor: "var(--surface-raised)",
          borderBottom: "1px solid var(--line)",
          display: "flex",
          alignItems: "center",
          padding: "0 6px",
          gap: "4px",
        }}
      >
        <span
          style={{
            width: "6px",
            height: "6px",
            borderRadius: "50%",
            backgroundColor: "var(--line-strong)",
          }}
        />
        <span
          style={{
            width: "6px",
            height: "6px",
            borderRadius: "50%",
            backgroundColor: "var(--line-strong)",
          }}
        />
        <span
          style={{
            width: "6px",
            height: "6px",
            borderRadius: "50%",
            backgroundColor: "var(--line-strong)",
          }}
        />
      </div>
      <div style={{ flex: 1, display: "flex" }}>
        <div
          style={{
            width: "28%",
            backgroundColor: "var(--surface)",
            borderRight: "1px solid var(--line)",
            padding: "6px 4px",
            display: "flex",
            flexDirection: "column",
            gap: "4px",
          }}
        >
          <div
            style={{
              height: "4px",
              borderRadius: "2px",
              backgroundColor: "var(--line-strong)",
            }}
          />
          <div
            style={{
              height: "4px",
              width: "70%",
              borderRadius: "2px",
              backgroundColor: "var(--line)",
            }}
          />
        </div>
        <div
          style={{
            flex: 1,
            padding: "8px",
            display: "grid",
            gridTemplateColumns: "repeat(3, 1fr)",
            gap: "6px",
          }}
        >
          <div
            style={{
              borderRadius: "4px",
              backgroundColor: "var(--tile)",
            }}
          />
          <div
            style={{
              borderRadius: "4px",
              backgroundColor: "var(--tile)",
            }}
          />
          <div
            style={{
              borderRadius: "4px",
              backgroundColor: "var(--tile)",
            }}
          />
        </div>
      </div>
    </div>
  );
}

function FullscreenThumbnail() {
  return (
    <div
      style={{
        width: "100%",
        height: "100%",
        borderRadius: "8px",
        overflow: "hidden",
        backgroundColor: "var(--canvas)",
        border: "1px solid var(--line)",
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        alignItems: "center",
        gap: "8px",
        padding: "12px",
        boxSizing: "border-box",
      }}
    >
      <div
        style={{
          display: "flex",
          gap: "8px",
          width: "100%",
          height: "60px",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <div
          style={{
            width: "44px",
            height: "58px",
            borderRadius: "6px",
            backgroundColor: "var(--tile)",
            border: "1px solid var(--accent)",
          }}
        />
        <div
          style={{
            width: "36px",
            height: "48px",
            borderRadius: "5px",
            backgroundColor: "var(--surface)",
            opacity: 0.6,
          }}
        />
        <div
          style={{
            width: "36px",
            height: "48px",
            borderRadius: "5px",
            backgroundColor: "var(--surface)",
            opacity: 0.6,
          }}
        />
      </div>
    </div>
  );
}

export function Demo() {
  const [selectedMode, setSelectedMode] = useState("window");

  const options: OnboardingModoOption[] = [
    {
      value: "window",
      title: "Em janela",
      description: "Para mouse e teclado.",
      hint: "Organizar, configurar, jogar na mesa",
      thumbnail: <WindowThumbnail />,
    },
    {
      value: "fullscreen",
      title: "Em tela cheia",
      description: "Para controle e TV.",
      hint: "Jogar no sofá",
      thumbnail: <FullscreenThumbnail />,
    },
  ];

  return (
    <div style={{ width: "1600px", height: "900px", position: "relative" }}>
      <TelaOnboarding
        skipLabel="Pular configuração"
        onSkip={() => alert("Pular")}
      >
        <OnboardingModo
          stepLabel="4 de 5"
          stepAriaLabel="Passo 4 de 5"
          title="Como você quer abrir o TAKO?"
          description="Os dois modos mostram os mesmos jogos."
          optionsAriaLabel="Modo de inicialização"
          options={options}
          value={selectedMode}
          onChange={setSelectedMode}
          continueLabel="Continuar"
          onContinue={() => alert("Continuar")}
          backLabel="Voltar"
          onBack={() => alert("Voltar")}
          footerNote="Troque a qualquer hora com F11."
        />
      </TelaOnboarding>
    </div>
  );
}
