# Paridade Rust × WPF (Fase 2 do PRD v0.2)

Estado do app Rust + Slint frente ao que o Tako WPF entrega hoje, com o recorte do PRD §3 (o que foi para o Console Mode não volta). Atualize a cada marco.

## Resumo

Tudo da §6 do PRD tem código, testes automáticos e CI verde em Windows, Linux e macOS. **Nada foi validado em hardware real ainda:** os roteiros estão em [`docs/testing/`](../testing/) e são a condição para a beta (MIG-043).

| Área (PRD §6) | Estado | Observação |
|---|---|---|
| Início, biblioteca (grade virtual, foco, navegação espacial), busca com teclado virtual, filtros, ordenação | Pronto | |
| Detalhes do jogo | Pronto | |
| Configurações em 8 categorias, pelo controle | Pronto | Atalho de teclado: 3 opções prontas, sem captura de tecla livre |
| Configurações por jogo (tela, modo, HDR, áudio, argumentos, pasta) | Pronto | |
| Editar jogo, arte por fonte, IDs por fonte | Pronto | Sem filtros e paginação do SteamGridDB no seletor; ID digitado é aplicado sem diálogo de confirmação |
| Adicionar jogo (exe, `.desktop`, AppImage, binário, `.app`, ROM) | Pronto | Título vem do nome do arquivo (não lê o nome do produto do `.exe`) |
| Central rápida reduzida (Início, voltar, áudio, tela, ponteiro, encerrar) | Pronto | Abre pelo controle (Select + Y/△) nos três SOs |
| Atalho de teclado para a Central durante o jogo | Só Windows | Linux e macOS só pelo controle (ver `tako_platform::global_hotkey`) |
| Tela cheia sem borda | Pronto | Builds de desenvolvimento abrem em janela; `TAKO_WINDOWED=1` força janela |
| Teste da tela de jogo ("Você consegue ver esta tela?", 15 s) | Pronto | |
| Lojas (Windows: Steam, Epic, GOG, Xbox, Ubisoft, EA, Battle.net, Amazon; Linux: Steam, Heroic, Lutris; macOS: Steam e Heroic) | Pronto | |
| Importar do Playnite (consentimento, prévia, coleções) | Pronto | Só Windows, como no WPF |
| Metadados (Steam Store, IGDB, ScreenScraper, SteamGridDB) e cache de imagens | Pronto | |
| Sessão: abrir, acompanhar, voltar com foco; tela e áudio por jogo; cursor escondido; cursor pelo controle | Pronto | Supervisor sem janela durante o jogo (ADR-0026) |
| Recuperação após queda (diário da sessão) | Pronto | |
| Atualização pelo GitHub Releases com SHA-256 | Pronto | Verificação automática ao abrir, se ligada |
| Portable e instalado; logs e relatório do sistema | Pronto | |
| Tema padrão + alternativo trocáveis em tempo de execução | Pronto | "Tako Alto contraste" vem junto; temas podem trocar só as cores (`tokens.toml`) |
| Migração das alphas (banco, coleções, escolhas de arte, chaves DPAPI → cofre do SO) | Pronto | |
| Uma instância só | Parcial | O segundo Tako sai sem trazer a janela do primeiro para a frente |

Ficaram de fora por decisão (PRD §3, Console Mode): controle de TV, contador e limite de FPS, Steam na bandeja, "modo console ao abrir".

## Benchmark (MIG-041)

`tako --bench` abre a interface de verdade com 1.000 jogos e navega pela grade; `tako --bench-supervisor` mede o supervisor acompanhando um jogo. O job `bench` do workflow Rust publica os dois no resumo de cada execução.

Primeira medição no CI (build release, Xvfb, renderizador por software, VM sem GPU):

| Métrica | CI | Meta (§8.1, hardware de referência) |
|---|---|---|
| Primeira tela | 3,9 s | ≤ 1,5 s (p95, SSD) |
| Resposta ao controle (p95) | 32 ms | ≤ 50 ms |
| Memória da interface em repouso | 187 MB | ≤ 150 MB |
| Memória durante o jogo (supervisor) | 10,6 MB | ≤ 40 MB |

Os números do CI são tendência, não aceite. Os que ficaram acima da meta (primeira tela e memória da interface) precisam ser medidos nas máquinas de referência antes de qualquer conclusão: no CI o renderizador por software desenha a 1080p na CPU e a VM é lenta para compilar o tema. Se a primeira tela continuar acima da meta em hardware, o primeiro suspeito é a compilação do tema `.slint` a cada abertura.

### Máquinas de referência

| Máquina | Primeira tela | Resposta (p95) | Memória (UI) | Memória (jogo) |
|---|---|---|---|---|
| _a preencher_ | | | | |

## Riscos

- **Hardware.** Tela, áudio e cursor (MIG-020 a MIG-023) passaram só nos testes com dispositivos falsos e nas checagens de compilação. Wayland, gamescope e macOS são os pontos mais incertos (ADR-0024).
- **Desempenho.** Primeira tela e memória da interface acima da meta no CI (ver acima).
- **Corte do C# (MIG-043).** Depende de uma beta Rust testada nos três SOs; o código WPF continua no repositório até lá.
