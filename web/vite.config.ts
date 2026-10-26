import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { designTokens, devData } from "./scripts/vite-dev.mjs";

export default defineConfig({
  // `devData` e `designTokens` só valem no `npm run dev`: biblioteca real de `dev-data/` e tokens ao vivo.
  plugins: [react(), devData(), designTokens()],
  // O build roda dentro do Tako por `tako-app://app/`: caminhos relativos, nada de `/` absoluto.
  base: "./",
  server: {
    // Porta combinada com `TAKO_WEB_DEV_URL` (docs/web/ponte.md §5).
    port: 5173,
    strictPort: true,
    open: "/#/dev",
  },
  build: {
    target: "es2023",
  },
});
