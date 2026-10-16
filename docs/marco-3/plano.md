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

- **LIB-001** Status de jogo; **LIB-004** jogos quebrados na biblioteca toda.
- **SESSION-002** Histórico de sessões.
- **INPUT-003** Bateria dos controles; **INPUT-004** reconexão assistida.
- **EXT-004** Discord Rich Presence.
- **UX-003** Assistente da primeira execução (idioma, lojas, provedores de metadados).
- **SRC-003** Cliente de loja ausente: oferecer abrir ou instalar, além do aviso.
- Atalho de teclado da Central durante o jogo no Linux (X11) e no macOS.
- **SRC-004** Jogos não Steam adicionados à Steam.
- **PLAT-007** Integração com o AnyFSE (Tako como app inicial do Xbox mode pelo projeto da comunidade). O WPF publicava um kit de experimento para o mesmo fim (`Tako-FseKit`), que o Rust não tem.
- **PLAT-002** Pacotes AUR e Flatpak.
- Testes de interface das telas principais (§8.4).
- **PLAT-008** Canal Nightly (build diária da `develop`): acelera os testes de cada PR.

## COULD

- **LIB-002** Tags; **LIB-003** coleções automáticas; **LIB-005** armazenamento.
- **SESSION-003** Aviso de crash; **SESSION-004** variáveis e comandos por jogo; **SESSION-005** Proton/Wine (Linux); **SESSION-006** Reparar jogo.
- **EXT-005** Links `tako://`.
- **THEME-006** Criar o próprio tema pelo Tako (pedido do mantenedor).
- **THEME-007** Música de fundo e sons por tema; **THEME-008** cinco temas oficiais bem diferentes.
- **ART-001** Capas certas para a biblioteca inteira, como o Plex; **ART-002** um clique no site do SteamGridDB (como o SGDBoop); **ART-003** galeria de capas online.
- **BRAND-001** Animação do Tako (polvo); **BRAND-002** animação de entrada.
- **DEALS-001** Página de promoções com links de afiliado.
- **INPUT-001** Remapear os controles do Tako; **INPUT-002** remapear para os jogos, como a Entrada Steam (estudo).
- **SESSION-001** Retomada rápida, como o Quick Resume (estudo).
- **VOICE-001** Busca por voz; **AI-001** assistente de IA para a biblioteca (estudo).
- **DATA-001** Dados de melhoria, só com opt-in.
- **DL-001** Downloads por torrent de fontes `.json` do usuário, vinculados ao jogo da biblioteca; **DL-002** Real-Debrid. O Tako só fornece a ferramenta.
- **UX-001** Tamanho de texto e sons de navegação.
- **SRC-001** Emuladores e ROMs, incluindo importar dos principais emuladores; **SRC-002** Bottles e itch.io.
- **UX-002** Arte animada, **UX-004** Perfis locais.
- **THEME-001 a 005**, **PERF-001/002**.

- Catálogo de ideias em `docs/backlog/ideias.md` (IDEA-*): COULD até o mantenedor priorizar.

## WON'T (agora)

- **PLAT-004/005** Assinatura no macOS e no Windows, e winget: precisam das contas e certificados do mantenedor.
- **PLAT-003** Experiência em tela cheia do Xbox: precisa de SCCD assinado pela Microsoft.
- **PLAT-001** Sessão gamescope: precisa de validação em hardware.
- **CMI-*** Integração com o Console Mode: depende da API dele.
- **EXT, STREAM, CLOUD, SOURCE**: P3.
