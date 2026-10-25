import { useState } from "react";
import {
  PassosDoOnboarding,
  PassosDoOnboardingMovimento,
  PassosDoOnboardingStaggerItem,
} from "./PassosDoOnboarding";

export const title = "PassosDoOnboarding";
export const preview = "PassosDoOnboarding";

export function Demo() {
  const [activeStep, setActiveStep] = useState(1);
  const total = 5;

  const nextStep = () => setActiveStep((s) => (s < total ? s + 1 : 1));
  const prevStep = () => setActiveStep((s) => (s > 1 ? s - 1 : total));

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "32px", padding: "24px" }}>
      <section>
        <h3 style={{ margin: "0 0 16px", color: "var(--text-muted)", fontSize: "14px" }}>
          Estados estáticos (1 a 5)
        </h3>
        <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
          <PassosDoOnboarding
            currentStep={1}
            totalSteps={5}
            label="1 de 5"
            ariaLabel="Passo 1 de 5"
          />
          <PassosDoOnboarding
            currentStep={2}
            totalSteps={5}
            label="2 de 5"
            ariaLabel="Passo 2 de 5"
          />
          <PassosDoOnboarding
            currentStep={3}
            totalSteps={5}
            label="3 de 5"
            ariaLabel="Passo 3 de 5"
          />
          <PassosDoOnboarding
            currentStep={4}
            totalSteps={5}
            label="4 de 5"
            ariaLabel="Passo 4 de 5"
          />
          <PassosDoOnboarding
            currentStep={5}
            totalSteps={5}
            label="5 de 5"
            ariaLabel="Passo 5 de 5"
          />
        </div>
      </section>

      <section>
        <h3 style={{ margin: "0 0 16px", color: "var(--text-muted)", fontSize: "14px" }}>
          Movimento de troca de passo (PassosDoOnboardingMovimento)
        </h3>
        <div style={{ display: "flex", gap: "12px", marginBottom: "20px" }}>
          <button
            type="button"
            onClick={prevStep}
            style={{
              padding: "6px 14px",
              borderRadius: "8px",
              border: "1px solid var(--line-strong)",
              background: "var(--surface)",
              color: "var(--text)",
              cursor: "pointer",
            }}
          >
            Passo anterior
          </button>
          <button
            type="button"
            onClick={nextStep}
            style={{
              padding: "6px 14px",
              borderRadius: "8px",
              border: "1px solid var(--line-strong)",
              background: "var(--surface)",
              color: "var(--text)",
              cursor: "pointer",
            }}
          >
            Próximo passo
          </button>
        </div>

        <PassosDoOnboardingMovimento stepKey={activeStep}>
          <PassosDoOnboardingStaggerItem index={0}>
            <PassosDoOnboarding
              currentStep={activeStep}
              totalSteps={total}
              label={`${activeStep} de ${total}`}
              ariaLabel={`Passo ${activeStep} de ${total}`}
            />
          </PassosDoOnboardingStaggerItem>

          <PassosDoOnboardingStaggerItem index={1}>
            <h2
              style={{
                fontSize: "32px",
                fontWeight: 800,
                letterSpacing: "-0.025em",
                margin: "16px 0 8px",
                color: "var(--text)",
              }}
            >
              Título do passo {activeStep}
            </h2>
          </PassosDoOnboardingStaggerItem>

          <PassosDoOnboardingStaggerItem index={2}>
            <p
              style={{
                fontSize: "15px",
                lineHeight: 1.55,
                color: "var(--text-muted)",
                margin: "0 0 16px",
                maxWidth: "520px",
              }}
            >
              Conteúdo com transição escalonada suave de 55ms entre elementos, utilizando apenas opacidade e translação vertical.
            </p>
          </PassosDoOnboardingStaggerItem>
        </PassosDoOnboardingMovimento>
      </section>
    </div>
  );
}
