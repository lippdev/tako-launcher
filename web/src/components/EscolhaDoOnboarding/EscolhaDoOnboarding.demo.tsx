import { useState } from "react";
import {
  EscolhaDoOnboarding,
  type EscolhaDoOnboardingOption,
} from "./EscolhaDoOnboarding";

export const title = "EscolhaDoOnboarding";
export const preview = "EscolhaDoOnboarding";

export function Demo() {
  const [selectedCard, setSelectedCard] = useState("window");
  const [selectedRow, setSelectedRow] = useState("igdb");

  const cardOptions: EscolhaDoOnboardingOption[] = [
    {
      value: "window",
      title: "Em janela",
      description: "Para mouse e teclado.",
      hint: "Organizar, configurar, jogar na mesa",
      thumbnail: (
        <div
          style={{
            width: "100%",
            height: "100%",
            background: "linear-gradient(135deg, color-mix(in srgb, var(--accent) 15%, transparent), var(--canvas))",
            display: "grid",
            placeItems: "center",
            color: "var(--text-muted)",
            fontSize: "12px",
          }}
        >
          Janela do TAKO
        </div>
      ),
    },
    {
      value: "fullscreen",
      title: "Em tela cheia",
      description: "Para controle e TV.",
      hint: "Jogar no sofá",
      thumbnail: (
        <div
          style={{
            width: "100%",
            height: "100%",
            background: "linear-gradient(135deg, color-mix(in srgb, var(--text) 10%, transparent), var(--canvas))",
            display: "grid",
            placeItems: "center",
            color: "var(--text-muted)",
            fontSize: "12px",
          }}
        >
          Modo TV / Controle
        </div>
      ),
    },
  ];

  const rowOptions: EscolhaDoOnboardingOption[] = [
    {
      value: "igdb",
      title: "Buscar na IGDB",
      description: "Envia só o nome de cada jogo",
    },
    {
      value: "local",
      title: "Usar só o que as lojas têm",
      description: "Nada sai do PC",
    },
  ];

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "36px", padding: "24px" }}>
      <section>
        <h3 style={{ margin: "0 0 16px", color: "var(--text-muted)", fontSize: "14px" }}>
          Com miniatura (cartão)
        </h3>
        <EscolhaDoOnboarding
          variant="card"
          value={selectedCard}
          onChange={setSelectedCard}
          options={cardOptions}
          ariaLabel="Modo de inicialização"
        />
      </section>

      <section>
        <h3 style={{ margin: "0 0 16px", color: "var(--text-muted)", fontSize: "14px" }}>
          Em linha
        </h3>
        <EscolhaDoOnboarding
          variant="row"
          value={selectedRow}
          onChange={setSelectedRow}
          options={rowOptions}
          ariaLabel="Fonte de capas e dados"
        />
      </section>
    </div>
  );
}
