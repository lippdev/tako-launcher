import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { applyAppearance, defaultAppearance } from "./appearance/appearance";
import "./styles/tokens.css";
import "./styles/base.css";

async function start() {
  // Sem o Tako por trás (`window.takoHost` ausente) o navegador usa o hospedeiro falso, só em desenvolvimento.
  if (import.meta.env.DEV && !window.takoHost) {
    const { installFakeHost } = await import("./bridge/fake");
    installFakeHost();
  }
  applyAppearance(defaultAppearance);
  const root = document.getElementById("root");
  if (!root) throw new Error("#root não existe em index.html");
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void start();
