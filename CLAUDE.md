@AGENTS.md

## Design system: branch `design-system`

O design system (`design/`: prévias, tokens, fontes, ícones, Backlog e protótipo) **não** está na `main` nem na
`develop`; vive só na branch `design-system`, e `design/` está no `.gitignore`. Para usar ou editar, na raiz do
repositório: `git fetch origin design-system && git worktree add design design-system`. Commite mudanças de design
dentro de `design/` (vão para a branch `design-system`) e nunca traga a pasta de volta para as branches do app;
para levar tokens, fontes ou ícones ao app, rode `npm run sync:design` em `web/` e commite a cópia em `web/src/`.
Detalhes no AGENTS.md §3.
