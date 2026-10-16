# Marco 3 — Depois da primeira alpha Rust

A v0.2.0-alpha.1 fechou a paridade em código (Fase 2 do PRD). Daqui em diante o trabalho segue em branches e PRs para `develop`, com CI nos três sistemas, e cada PR é testada na alpha seguinte.

## Critérios

| Classe | Entra quando |
|---|---|
| **MUST** | O app WPF fazia e o Rust ainda não faz (regressão de paridade), **ou** o fluxo principal (abrir → achar o jogo → jogar → voltar) quebra ou parece quebrado para um usuário real num sistema suportado. Precisa dar para fazer e conferir sem contas, certificados ou hardware que só o mantenedor tem. |
| **SHOULD** | Melhora bem o fluxo principal, mas hoje existe um contorno. |
| **COULD** | Funcionalidade nova, sem impacto no fluxo principal. |
| **WON'T (agora)** | Depende de contas ou certificados, de hardware específico, de outro repositório ou de validação que ainda não existe. |

## MUST

| Item | Por quê |
|---|---|
| **M-01** Abrir o Tako de novo traz a janela aberta para a frente (durante o jogo, abre a Central) | Paridade: no WPF o segundo Tako ativava o primeiro. Hoje ele só sai e parece que não abriu |
| **M-02** Confirmação antes de aplicar um ID digitado em Fontes e IDs | Paridade: um ID errado troca capas e dados do jogo sem aviso |
| **M-03** Ao adicionar um `.exe`, o nome vem do nome do produto gravado no arquivo | Paridade (Windows): hoje vem do nome do arquivo ("game_x64_shipping") |
| **M-04** Seletor de imagens: filtros e paginação do SteamGridDB | Paridade: sem eles só aparecem as primeiras imagens |
| **M-05** Atalho de teclado da Central: capturar qualquer tecla | Paridade: hoje são 3 opções prontas |

Conferido sem precisar de mudança: escala de 720p a 4K (o layout acompanha a altura da janela).

## SHOULD

- **UX-003** Assistente da primeira execução (idioma, lojas, provedores de metadados).
- **SRC-003** Cliente de loja ausente: oferecer abrir ou instalar, além do aviso.
- Atalho de teclado da Central durante o jogo no Linux (X11) e no macOS.
- **SRC-004** Jogos não Steam adicionados à Steam.
- **PLAT-007** Integração com o AnyFSE (Tako como app inicial do Xbox mode pelo projeto da comunidade). O WPF publicava um kit de experimento para o mesmo fim (`Tako-FseKit`), que o Rust não tem.
- **PLAT-002** Pacotes AUR e Flatpak.
- Testes de interface das telas principais (§8.4).

## COULD

- **THEME-006** Criar o próprio tema pelo Tako (pedido do mantenedor).
- **UX-001** Tamanho de texto e sons de navegação.
- **SRC-001** Emuladores e ROMs, **SRC-002** Bottles e itch.io.
- **UX-002** Arte animada, **UX-004** Perfis locais.
- **THEME-001 a 005**, **PERF-001/002**.

## WON'T (agora)

- **PLAT-004/005** Assinatura no macOS e no Windows, e winget: precisam das contas e certificados do mantenedor.
- **PLAT-003** Experiência em tela cheia do Xbox: precisa de SCCD assinado pela Microsoft.
- **PLAT-001** Sessão gamescope: precisa de validação em hardware.
- **CMI-*** Integração com o Console Mode: depende da API dele.
- **EXT, STREAM, CLOUD, SOURCE**: P3.
