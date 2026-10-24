// Textos dos downloads que não existem nos catálogos de `locales/` (WEB-14b). Os que existem (aviso legal, painel,
// limite de envio…) vêm de `t("downloads.*")`; aqui ficam só os da interface nova, até migrarem para o catálogo.

export const downloadsStrings = {
  /** Painel e barra com os downloads desligados. */
  off: "Downloads desligados",
  openSettings: "Abrir Configurações › Downloads",
  openGame: "Abrir o jogo",
  error: "Erro",
  actionError: "Não foi possível atualizar o download.",
  /** Configurações › Downloads. */
  settings: {
    sourcesEmpty: "Nenhuma fonte adicionada",
    sourcesLoading: "Lendo as fontes…",
    sourcesError: "Não foi possível ler as fontes.",
    sourceDropped: (n: number) => `${String(n)} ${n === 1 ? "item descartado" : "itens descartados"}`,
  },
} as const;
