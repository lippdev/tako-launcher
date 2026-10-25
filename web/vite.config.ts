import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
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
