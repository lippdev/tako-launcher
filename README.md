# Tako — design system

Esta branch guarda só o design system do Tako (prévias, tokens, fontes, ícones e o protótipo). O código do app
fica na `develop` e na `main`, que não trazem mais a pasta `design/`.

- `desktop/`: design system da interface (README, `tokens.css`, `components/<Nome>/`, `assets/`, `fonts/`).
- `prototype/`: protótipo navegável por controle.

## Uso junto do código

Faça o checkout desta branch como uma worktree em `design/`, na raiz do repositório (a pasta está no
`.gitignore` da `develop`), e os caminhos `design/desktop/…` voltam a funcionar como antes:

```sh
git fetch origin design-system
git worktree add design design-system
```

Com ela no lugar, `npm run sync:design` (em `web/`) copia tokens, fontes, marca e ícones para `web/src/`, e o
`npm run dev` acompanha `design/desktop/tokens.css` ao vivo. Mudanças no design system são commitadas aqui,
dentro de `design/`; a cópia gerada em `web/src/` é commitada na branch do app.
