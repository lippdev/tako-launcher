# Tako — PRD e backlog

**Versão:** 0.2 (replanejamento multiplataforma)  
**Data:** 16/10/2026  
**Status:** em vigor. Substitui a versão 0.1 inteira (WPF, Windows-only, runtime absorvido do Console Mode).  
**Plataformas:** Windows, Linux e macOS, com a mesma experiência  
**Licença:** AGPL-3.0-only ([ADR-0014](adr/0014-licenca-agpl.md))

---

## 0. Como usar este documento

Este PRD descreve **o que o Tako precisa ter a partir de agora**. O que existia antes e não aparece aqui está fora do escopo.

Para quem for implementar, inclusive agentes:

1. **Ordem de verdade:** este PRD, depois os ADRs novos (a partir do ADR-0023), depois o `CONTRIBUTING.md`. Os ADRs 0011 a 0022 descrevem a implementação C#/WPF e passam a ser **referência histórica**. Cada um deve ser marcado como "Substituído" quando o equivalente em Rust existir (MIG-004).
2. **O código C# atual é a especificação executável** até o corte (Fase 4). Antes de portar um módulo, leia o código e os testes em C#. Casos de borda, formatos de arquivo das lojas e regras de metadados já estão resolvidos lá. Porte os testes junto.
3. **Uma história = uma PR**, no fluxo do `CONTRIBUTING.md` (branch a partir da `develop`, título em Conventional Commits, CI verde). Cada história tem ID e critério de aceite. Marque `[x]` neste arquivo na mesma PR que entrega a história.
4. **Não decida sozinho o que este PRD marca como decisão.** Se uma decisão se mostrar errada na prática, registre a evidência num ADR e pergunte aos mantenedores antes de mudar o rumo.

---

## 1. Visão

Um launcher de jogos para a TV e o controle, **igual no Windows, no Linux e no macOS**: abrir, escolher e jogar sem voltar ao desktop, com interface bonita, fluida e leve. Enquanto um jogo roda, o Tako quase não ocupa memória nem GPU.

**Proposta de valor:** a sua biblioteca inteira (lojas, jogos avulsos e emuladores) numa interface de console, em qualquer sistema, sem atrapalhar o desempenho do jogo.

### 1.1 Objetivos

1. Uma única base de código e uma única interface para Windows, Linux e macOS.
2. Navegação completa pelo controle, com teclado como alternativa.
3. Biblioteca unificada: lojas, executáveis manuais e emuladores de cada sistema.
4. Metadados e arte (capas, fundos, logos, arte animada) de provedores licenciados.
5. Abrir jogos, acompanhar a sessão e voltar à biblioteca com a tela e o áudio do jeito que estavam.
6. Consumo mínimo durante o jogo.
7. Temas próprios, seguros (sem execução de código) e trocáveis em tempo de execução.
8. Integração com o Console Mode, sem duplicá-lo (§3).

### 1.2 Público-alvo

- Quem joga no PC ligado à TV, no Windows ou no Linux (CachyOS, Bazzite, Nobara, SteamOS e afins).
- Donos de handhelds (Steam Deck, ROG Ally, Legion Go), com Windows ou Linux.
- Quem joga no Mac, com Steam, jogos nativos e emuladores.
- Quem usa Steam Big Picture, Playnite, ES-DE ou Pegasus e quer uma experiência única.

### 1.3 Princípios

- **Controller-first:** toda jornada principal funciona sem mouse.
- **Igual em todo lugar:** mesma interface, mesmos temas, mesmo comportamento. Diferenças por SO só onde o próprio sistema não permite (e aparecem para o usuário com explicação).
- **Leve durante o jogo:** a interface libera texturas e para de desenhar. Meta na §8.
- **Recovery-first:** nenhuma falha deixa o usuário sem tela, sem áudio ou sem caminho de volta ao desktop.
- **Offline-first:** a biblioteca e os jogos não dependem de serviços próprios na nuvem.
- **Tema não executa código:** temas são dados declarativos, validados antes de carregar.
- **O Tako não é o Console Mode:** o Tako lança e organiza jogos. Controle da TV, sobreposições e ferramentas de sessão são do Console Mode (§3).

---

## 2. Decisões

| # | Decisão | Situação |
|---|---|---|
| D-01 | **Rust** é a linguagem do Tako inteiro (app, plataforma, ferramentas). | Decidido |
| D-02 | **Slint** é o toolkit da interface: renderização na GPU, `.slint` declarativo e intérprete para carregar temas em tempo de execução. | Decidido, **sujeito ao gate do spike MIG-001**. Se falhar no gate, a alternativa é Avalonia/.NET, e a decisão volta aos mantenedores com os números. |
| D-03 | O Tako mantém **somente quatro** funções vindas do Console Mode: troca de resolução, troca de saída de áudio, esconder o cursor e cursor pelo controle. O resto sai do Tako (§3). | Decidido |
| D-04 | A integração com o Console Mode exige uma API que **ainda não existe** no Console Mode. Fica no backlog (épico CMI); o Tako não depende dela para funcionar. | Decidido |
| D-05 | **Fim do WPF e da compatibilidade com temas do Playnite 10** (`.pthm`, XAML restrito, `PlayniteCompat.P10`). Temas passam a ter formato próprio. Um conversor de temas do Playnite fica como estudo no backlog (THEME-010), sem compromisso. | Decidido |
| D-06 | **Dados dos usuários das alphas são migrados**: biblioteca, coleções, configurações, escolhas de arte e credenciais (MIG-030). Ninguém perde a biblioteca na troca. | Decidido |
| D-07 | **Releases:** `main` publica estável; `develop` publica alpha e beta quando um dev pede. Saídas: Windows (instalador e portable), Linux (deb, AppImage, tar.gz; x64 e arm64) e macOS (dmg e zip; Apple Silicon e Intel). | Decidido. O fluxo existe na branch `ci/release-pipeline` (PR pendente) e precisa ser adaptado para `cargo` (MIG-040). |
| D-08 | A UI C#/WPF continua sendo a versão publicada **até a paridade** (Fase 3). Durante a migração, ela só recebe correções críticas, sem funções novas. | Decidido |

---

## 3. Fronteira com o Console Mode

O Tako absorveu boa parte do Console Mode nas alphas. Isso será desfeito. A partir de agora:

| Fica no Tako | Sai do Tako (é do Console Mode) |
|---|---|
| Troca de resolução e taxa por jogo ("tela de jogo") | Controle de TV (ADB/Google TV, LG webOS, Samsung Tizen) |
| Troca da saída de áudio por jogo | Steam na bandeja e áudio da Steam na sessão |
| Esconder o cursor durante a sessão | Contador de FPS (PresentMon) e limite de FPS pelo driver |
| Cursor pelo controle (analógico, cliques e rolagem) | Cortina de tela, troca e fechamento de janelas, monitor de hardware (CPU/GPU/RAM) |
| | ControlFS (gerenciador de arquivos) |
| | Menu "Modo Console", assistente de TV e "abrir direto na TV / voltar ao PC" |
| | Atalho de emergência global da sessão de console |

Notas:
- A **restauração** de tela e áudio ao fim do jogo continua no Tako, porque é consequência das duas funções que ficam.
- A remoção acontece na reescrita: o app Rust simplesmente não terá essas funções. Na versão WPF elas continuam até o corte. O changelog da primeira release Rust deve listar o que saiu e apontar para o Console Mode.
- Quando o Console Mode tiver a API (épico CMI), o Tako passa a oferecer essas funções **delegando** a ele, se estiver instalado.

---

## 4. Stack

| Responsabilidade | Tecnologia | Observação |
|---|---|---|
| Linguagem | Rust (edição 2024, toolchain estável fixada em `rust-toolchain.toml`) | Workspace Cargo |
| Interface | Slint | Backend com renderização na GPU (Skia ou FemtoVG, a definir no spike). Temas `.slint` carregados pelo `slint-interpreter`. |
| Controle | SDL3 (gamecontroller) | Banco de mapeamentos da comunidade, DualSense/Edge, Switch Pro, Xbox, 8BitDo; hotplug |
| Persistência | SQLite (`rusqlite`, SQLite embutido) | Esquema atual migrado (D-06), migrações versionadas |
| HTTP | `reqwest` (rustls) | Metadados, arte e atualizações |
| Imagens | `image` + cache próprio de texturas | Miniaturas no disco, decodificação sob demanda, LRU por bytes de GPU |
| Arte animada e vídeo | Fase posterior (libmpv opcional) | Fora da paridade; ver THEME/UX |
| Segredos | `keyring` (Credential Manager, Secret Service/libsecret, Keychain) | Substitui a DPAPI; migra os segredos antigos |
| Logs | `tracing` com arquivo rotativo e redação de dados sensíveis | Mesmas regras de redação do `LogRedactor` atual |
| Serialização | `serde` (JSON, YAML, TOML) | |
| Testes | `cargo test`, `insta` para snapshots, testes de UI com o backend de testes do Slint | Rodam em Linux, Windows e macOS no CI |
| CI | GitHub Actions: matriz Windows, Linux (x64 e arm64) e macOS (arm64 e x64) | Repositório público: minutos gratuitos |
| Empacotamento | Windows: Inno Setup (instalador) e .exe portable. Linux: deb, AppImage, tar.gz. macOS: `.app` em dmg e zip | Avaliar `cargo-packager` no MIG-040 |

---

## 5. Arquitetura

### 5.1 Workspace

| Crate | Responsabilidade |
|---|---|
| `tako-core` | Modelo de jogo, biblioteca, consulta e filtros, coleções, busca, deduplicação entre lojas, metadados e arte, navegação espacial, estado da sessão, persistência. **Sem dependência de UI nem de SO.** |
| `tako-sources` | Fontes de biblioteca (uma por loja ou launcher), cada uma atrás do trait `LibrarySource`, com `cfg` por SO |
| `tako-platform` | Traits `Displays`, `AudioOutputs`, `Cursor`, `PointerInjector`, `Secrets`, `Paths`, `ProcessLauncher` e uma implementação por SO (`windows`, `linux`, `macos`) |
| `tako-themes` | Formato de tema, validação, instalação e carregamento seguro (§7) |
| `tako-ui` | Telas Slint, foco, animações e ligação com o core |
| `tako` (binário) | Inicialização, supervisor, ciclo de vida da sessão, single-instance, atualização |

Regras:
- As dependências apontam para dentro: `tako` → `tako-ui` → `tako-core`; `tako-sources` e `tako-platform` dependem de `tako-core`, nunca o contrário.
- Código específico de SO fica **só** em `tako-platform` e `tako-sources`, atrás de `cfg(target_os)`.
- Sem `unsafe` fora de `tako-platform`. Cada bloco `unsafe` tem comentário `// SAFETY:`.

### 5.2 Processos e ciclo de vida

- Um único executável, `tako`. O modo supervisor (mesmo binário) garante que tela e áudio sejam restaurados mesmo se a interface cair. A comunicação entre os dois é um socket local restrito ao usuário (named pipe no Windows, socket Unix nos demais).
- **Durante o jogo**, a interface esconde a janela, para o loop de renderização e libera texturas e caches de imagem. Ao voltar, recarrega o que estiver visível.
- Estados da sessão: `Browsing → Launching → Playing → Returning → Restoring → Browsing`, com `Error → Recovery → Desktop`. Transições idempotentes: restaurar duas vezes não pode desfazer nada a mais nem deixar estado pela metade. Um diário em disco (como o `SessionJournal` atual) permite restaurar depois de uma queda do processo.

### 5.3 Funções de plataforma por SO

As quatro funções de D-03, com a abordagem prevista. As marcadas "a validar" precisam de prova no spike de plataforma (MIG-020 a MIG-023) antes de virar promessa ao usuário.

| Função | Windows | Linux | macOS |
|---|---|---|---|
| Resolução e taxa | CCD (`SetDisplayConfig`), como o código atual | X11: XRandR. Wayland: KDE (kscreen/output-management), GNOME (Mutter `DisplayConfig` via D-Bus), wlroots (`wlr-output-management`). gamescope: resolução vem da sessão; o Tako só informa | CoreGraphics (`CGConfigureDisplayWithDisplayMode`) |
| Saída de áudio | `IPolicyConfig` (como o código atual) | PipeWire/PulseAudio (sink padrão via `libpulse` ou `pw`) | CoreAudio (dispositivo de saída padrão) |
| Esconder o cursor | Como o código atual | X11: XFixes. Wayland: só sobre a janela do Tako; sobre o jogo, **a validar** (sem API global) | `CGDisplayHideCursor` (**a validar** com o jogo em primeiro plano) |
| Cursor pelo controle | `SendInput` | `uinput`, que funciona em X11 e Wayland. Precisa de permissão em `/dev/uinput`: regra udev instalada pelo pacote | `CGEventPost`, que exige a permissão de Acessibilidade (o Tako guia o usuário) |

---

## 6. Escopo de paridade

O app Rust só substitui o WPF quando tiver, **funcionando nas três plataformas** (salvo onde indicado):

**Interface e navegação**
- Tela inicial com "Continuar jogando" e jogos recentes; coleções na tela inicial.
- Biblioteca em grade virtualizada (milhares de jogos), foco animado e navegação espacial.
- Busca com teclado virtual (sem acento), filtros (instalados, favoritos, recentes, fonte, gênero, coleção) e ordenação (nome, último jogado, tempo de jogo, nota, data de adição).
- Detalhes do jogo com capa, fundo, logo, informações e Jogar.
- Configurações completas pelo controle, em categorias, sem textos técnicos.
- Configurações por jogo: tela de jogo (resolução e taxa), saída de áudio, argumentos e pasta de trabalho.
- Edição de jogo, escolha de arte por fonte e IDs por fonte.
- Atalhos de controle, menu rápido, voltar e sair. Atalho para abrir o Tako por cima do jogo.
- Central rápida durante o jogo, **reduzida às funções que ficam** (áudio, tela, cursor, voltar ao Tako, encerrar jogo).
- Suporte a 720p até 4K, DPI e proporções diferentes; contraste da linha focada no padrão WCAG AA.
- Idiomas: português do Brasil e inglês (EUA).

**Biblioteca**
- Jogos manuais: executável (Windows), binário, `.desktop`, AppImage ou script (Linux), `.app` (macOS); com argumentos e pasta de trabalho.
- Lojas, por SO: ver épico SRC (§9). Para a paridade bastam as lojas que já existem no Windows mais Steam nos três sistemas e Heroic e Lutris no Linux.
- Duplicatas entre lojas unificadas, sem perder a identidade de cada loja.
- Tempo de jogo, última partida e favoritos.

**Metadados e arte**
- Steam Store, IGDB (chave do usuário), ScreenScraper (conta do usuário + credencial do software via segredo do CI) e SteamGridDB, com a mesma ordem de prioridade e as mesmas regras de mesclagem de hoje.
- Cache de imagens em disco com limite de tamanho.

**Sessão**
- Abrir o jogo pelo jeito certo de cada loja (URI ou cliente oficial), detectar o fim por processo ou pela estratégia da loja, voltar à biblioteca com o foco onde estava.
- Aplicar e restaurar tela e áudio por jogo; esconder e mostrar o cursor; cursor pelo controle.
- Recuperação depois de queda (diário de sessão).

**Sistema**
- Atualização pelo GitHub Releases, com verificação do SHA-256 publicado (regras do `ReleaseFeed` atual: canal estável e de teste, nunca rebaixa).
- Portable (dados ao lado do executável) e instalado (dados na pasta do usuário de cada SO).
- Logs exportáveis e relatório do sistema sem dados sensíveis.
- Tema padrão próprio e pelo menos um tema alternativo, trocáveis em tempo de execução.

---

## 7. Temas

- Um tema é uma pasta ou pacote `.takotheme` (zip) com `theme.toml` (id, nome, versão, autor, versão da API de temas) e arquivos `.slint`, imagens e fontes.
- O tema só define **apresentação**. Dados e comandos chegam por uma API de tema versionada (propriedades e callbacks expostos pelo `tako-ui`). O tema não acessa rede, disco nem processos.
- Validação antes de carregar: manifesto, versão da API, tamanho máximo do pacote, caminhos dentro do pacote (sem path traversal), tipos de arquivo permitidos e compilação do `.slint` pelo intérprete.
- Se o tema falhar ao carregar ou durante o uso, o Tako volta ao tema padrão e não entra em loop de falha (mesma ideia do `FaultBudget` atual).
- A API de temas é documentada para autores (THEME-001) e tem uma ferramenta de checagem por linha de comando (`tako theme check`).

---

## 8. Requisitos não funcionais

### 8.1 Desempenho (metas iniciais, confirmadas ou ajustadas pelo spike MIG-001)

Cenário de referência: biblioteca com 1.000 jogos, capas em cache, 1080p.

| Métrica | Meta |
|---|---|
| Abertura a frio até a tela interativa | ≤ 1,5 s no p95 (SSD) |
| Navegação | 60 fps estáveis na grade com foco animado, num iGPU de referência (Steam Deck ou equivalente) |
| Resposta ao controle | ≤ 50 ms no p95 |
| Memória da interface em repouso | ≤ 150 MB (RSS), contando o cache de texturas |
| Memória total do Tako durante o jogo | ≤ 40 MB, com a interface parada e as texturas liberadas |
| GPU durante o jogo | Zero quadros desenhados pelo Tako |
| CPU durante o jogo | Sem polling ativo; acordar só por evento ou por timer de 1 s ou mais |

O benchmark (MIG-041) roda no CI, onde os números só servem de tendência, e em máquinas de referência documentadas. Nenhum número do CI reprova a build; travar, cair ou não chegar à tela interativa reprova.

### 8.2 Compatibilidade

- **Windows** 10 22H2 e 11, x64 (ARM64 depois).
- **Linux** x64 e arm64, glibc 2.35 ou mais recente; X11 e Wayland (KDE, GNOME, wlroots); sessão gamescope.
- **macOS** 12 ou mais recente, Apple Silicon e Intel.
- Controles: Xbox, DualShock 4, DualSense e Edge (USB e Bluetooth), Switch Pro, 8BitDo; reconexão sem reiniciar.
- Resoluções 720p, 1080p, 1440p e 4K; TV e múltiplos monitores.

### 8.3 Segurança

- Temas não executam código (§7). Pacotes de tema e de dados são extraídos sem path traversal.
- Não executar arquivos baixados automaticamente. Não desativar antivírus nem anti-cheat. Não injetar nada em jogos.
- Credenciais só no cofre do SO (`keyring`), nunca em texto puro, nunca em log.
- Socket do supervisor restrito ao usuário atual.
- Atualizações só com SHA-256 conferido.
- URLs, JSON de terceiros e arquivos das lojas são dados não confiáveis: parsers com limites de tamanho e sem pânico em entrada inválida (testes com entradas quebradas).

### 8.4 Qualidade

- Testes de unidade do core e dos parsers de cada loja, com fixtures fictícias (portadas das atuais).
- Testes de interface nas telas principais (navegação por controle, foco, contraste).
- Checklist de hardware por SO (`docs/testing/`) para tela, áudio, cursor e controles.
- `cargo fmt --check`, `cargo clippy -- -D warnings` e testes nos três SOs a cada PR.
- Versionamento do banco e das configurações; migração testada a partir de um banco real das alphas.

---

## 9. Backlog

Legenda: **P0** = necessário para a paridade e o corte do WPF; **P1** = logo depois; **P2/P3** = futuro.

### Fase 0 — Decisão técnica (P0)

- [ ] **MIG-001 — Spike Slint.** Protótipo com a grade de 1.000 capas, foco animado, controle via SDL3 e um tema `.slint` carregado em tempo de execução, rodando em Windows, Linux (X11, Wayland KDE e gamescope) e macOS. **Gate:** cumpre as metas da §8.1 em pelo menos duas máquinas de referência e carrega o tema em runtime. Resultado num ADR (ADR-0023) com os números. Se não cumprir, parar e levar os números aos mantenedores (D-02). _Estado: código e números do CI prontos (ADR-0023); falta o gate nas duas máquinas de referência._
- [ ] **MIG-002 — Spike de plataforma.** Provar as quatro funções de D-03 em cada SO (§5.3), principalmente o cursor no Wayland e no macOS e a resolução no Wayland. Resultado: ADR-0024 com a matriz real de suporte. _Estado: matriz em ADR-0024; falta provar em hardware (`docs/testing/`)._
- [x] **MIG-003 — ADR da fronteira Tako × Console Mode.** Registrar a §3 e o contrato esperado da futura API (para o épico CMI). ADR-0025.
- [x] **MIG-004 — Marcar ADRs antigos.** ADRs 0011 a 0022 recebem "Substituído por" ou "Histórico (implementação C#)".

### Fase 1 — Fundação Rust (P0)

- [x] **MIG-010 — Workspace e CI.** Crates da §5.1, `rust-toolchain.toml`, CI com fmt, clippy e testes na matriz dos três SOs; padrões de código Rust no `CONTRIBUTING.md` (§5 a §7). Aceite: PR de exemplo verde nos três.
- [x] **MIG-011 — Persistência.** SQLite com o esquema atual e migrações versionadas. Aceite: abre um banco das alphas sem perder dados.
- [x] **MIG-012 — Modelo e consulta da biblioteca.** Port de `Game`, `LibraryQuery`, `LibraryCriteria`, `SearchText`, `Collections`, `StoreDuplicates`, com os testes.
- [x] **MIG-013 — Navegação espacial e grade virtual.** Port de `SpatialNavigator`, `ScrollMath`, `VirtualGrid`, `RowReveal`, com os testes.
- [x] **MIG-014 — Entrada.** SDL3, mapeamento de botões para ações, repetição ao segurar, hotplug, filtro de eco do teclado da Steam (`InputEchoFilter`) e fusão Steam + PlayStation (`GamepadFusion`).
- [x] **MIG-015 — Logs e diagnóstico.** `tracing`, arquivo rotativo, redação (port do `LogRedactor`), relatório do sistema.
- [ ] **MIG-016 — Supervisor e diário de sessão.** Socket local restrito, diário em disco, restauração após queda. Aceite: matar a interface no meio da sessão restaura tela e áudio. _Estado: supervisor e diário prontos (ADR-0026); falta o teste de queda em hardware._

### Fase 2 — Paridade (P0)

- [ ] **MIG-020 — Resolução e taxa** nos três SOs (§5.3), com aplicar, confirmar e restaurar. _Estado: código pronto; falta validar em hardware (`docs/testing/`)._
- [ ] **MIG-021 — Saída de áudio** nos três SOs. _Estado: código pronto; falta validar em hardware (`docs/testing/`)._
- [ ] **MIG-022 — Esconder o cursor** nos três SOs, com as limitações do MIG-002 explicadas na interface. _Estado: código pronto; falta validar em hardware (`docs/testing/`)._
- [ ] **MIG-023 — Cursor pelo controle** nos três SOs (analógico, cliques, rolagem, aceleração; port do `ControllerMouseMath`). Linux: regra udev no pacote. macOS: fluxo da permissão de Acessibilidade. _Estado: código pronto; falta validar em hardware (`docs/testing/`)._
- [ ] **MIG-024 — Abrir jogos e acompanhar a sessão.** Por loja e manual; fim por processo ou pela estratégia da loja; volta com foco. _Estado: código pronto; falta validar em hardware (`docs/testing/`)._
- [x] **MIG-025 — Fontes de biblioteca do Windows.** Steam, Epic, GOG, Xbox/Microsoft Store, Ubisoft, EA, Battle.net, Amazon, importação do Playnite. Port dos parsers e dos testes.
- [x] **MIG-026 — Fontes de biblioteca do Linux.** Steam (nativo, Flatpak e Snap; jogos Proton), Heroic (Epic, GOG, Amazon), Lutris, jogos manuais (`.desktop`, AppImage, binário).
- [x] **MIG-027 — Fontes de biblioteca do macOS.** Steam, GOG/Epic quando instalados, apps manuais (`.app`).
- [x] **MIG-028 — Metadados e arte.** Steam Store, IGDB, ScreenScraper, SteamGridDB, mesclagem e escolhas por fonte; cache de imagens com limite.
- [x] **MIG-029 — Interface completa.** Todas as telas da §6 no tema padrão, com controle e teclado, nos dois idiomas.
- [x] **MIG-030 — Migração de dados das alphas.** Detectar a pasta de dados do Tako C# (instalado e portable), importar banco, configurações, escolhas de arte e coleções; decifrar os segredos DPAPI e regravar no `keyring`. Aceite: um usuário da alpha abre a versão Rust e vê a mesma biblioteca, já logado nos provedores.
- [x] **MIG-031 — Temas.** Formato da §7, validação, instalação, troca em tempo de execução e volta ao padrão após falha; tema padrão e um alternativo.
- [x] **MIG-032 — Atualizações.** Port do `ReleaseFeed` e do `SemVer`, escolha do arquivo por SO e tipo de instalação, verificação do SHA-256.

### Fase 3 — Distribuição e corte (P0)

- [x] **MIG-040 — Empacotamento Rust.** Adaptar o `release.yml` e os scripts (`build/`) para `cargo`: Windows (instalador e portable), Linux (deb com regra udev, AppImage, tar.gz; x64 e arm64), macOS (`.app`, dmg e zip; arm64 e x64). Os nomes de arquivo precisam continuar compatíveis com o atualizador das alphas C# (`Tako-Setup-*.exe`, `Tako-Portable-*.exe`), para que elas encontrem a versão Rust.
- [ ] **MIG-041 — Benchmark.** Medir as métricas da §8.1 no CI e nas máquinas de referência; publicar em `docs/`. _Estado: `tako --bench` e job `bench` no CI; faltam as máquinas de referência (`docs/marco-2/paridade.md`)._
- [x] **MIG-042 — Checklists de hardware** por SO, em `docs/testing/`.
- [ ] **MIG-043 — Corte.** Remover a solução C#/WPF do repositório e do CI. Changelog da primeira release Rust com o que saiu (§3) e o que mudou (temas do Playnite). Aceite: MIG-010 a MIG-042 concluídos e uma beta Rust testada nos três SOs. _Estado: aguarda a beta Rust testada nos três SOs._

### Épico CMI — Integração com o Console Mode (BACKLOG, P1, depende do Console Mode)

- [ ] **CMI-001 — API local no Console Mode** (no repositório dele): descoberta, versão, capacidades e comandos (TV, sobreposições, sessão de console). Contrato definido no ADR-0025.
- [ ] **CMI-002 — Detecção no Tako:** saber se o Console Mode está instalado e qual versão da API ele oferece.
- [ ] **CMI-003 — Ações delegadas:** quando o Console Mode existe, a Central e as Configurações do Tako mostram as funções dele (TV, FPS, janelas), executadas por ele.
- [ ] **CMI-004 — Tako como launcher do Console Mode:** o Console Mode pode abrir o Tako como interface da sessão, no lugar do Big Picture ou do Playnite.

### Épico PLAT — Integrações de sistema (P1)

- [ ] **PLAT-001 (Linux):** sessão gamescope do Tako (entrada no gerenciador de login, como a do Steam).
- [ ] **PLAT-002 (Linux):** pacotes para Arch/CachyOS (PKGBUILD no AUR) e Flatpak.
- [ ] **PLAT-003 (Windows):** Tako como app inicial da experiência de tela cheia do Xbox, onde a Microsoft permitir (exige SCCD assinado pela Microsoft; ver `docs/spikes/MVP-007-fse.md`).
- [ ] **PLAT-004 (macOS):** assinatura Developer ID e notarização.
- [ ] **PLAT-005 (Windows):** assinatura de código (SignPath) e winget.
- [ ] **PLAT-006:** abrir com o sistema e iniciar direto em tela cheia, em cada SO.
- [ ] **PLAT-007 (Windows):** integração com o [AnyFSE](https://github.com/ashpynov/AnyFSE), projeto da comunidade que põe um launcher de terceiros como app inicial do Xbox mode (tela cheia de jogos do Windows 11), enquanto o caminho oficial (PLAT-003) depende da Microsoft. Escopo: o Tako funciona como alvo do AnyFSE (abre em tela cheia, sem janela de desktop, volta certo de um jogo), o Tako detecta o AnyFSE instalado e ensina a configuração, e um roteiro de teste em `docs/testing/windows.md`. O AnyFSE pede Modo de Desenvolvedor (ver `docs/spikes/MVP-007-fse.md`); o Tako não liga isso sozinho.

### Épico SRC — Mais fontes (P1)

- [ ] **SRC-001:** emuladores (RetroArch e standalone), bibliotecas de ROMs do usuário e sistemas, nos três SOs.
- [ ] **SRC-002 (Linux):** Bottles e itch.io.
- [ ] **SRC-003:** detectar cliente de loja ausente e oferecer abri-lo ou instalá-lo pelo caminho oficial.
- [ ] **SRC-004:** jogos não Steam adicionados à Steam (atalhos), nos três SOs.

### Épico UX (P1)

- [ ] **UX-001:** acessibilidade: tamanho de texto, redução de animações, alto contraste, sons de navegação com volume próprio.
- [ ] **UX-002:** arte animada e trailers (libmpv opcional, desligado durante o jogo).
- [ ] **UX-003:** assistente da primeira execução (idioma, lojas detectadas, provedores de metadados).
- [ ] **UX-004:** perfis locais (vários usuários na mesma máquina).

### Épico THEME (P2)

- [ ] **THEME-001:** documentação pública da API de temas.
- [ ] **THEME-002:** pré-visualização antes de aplicar.
- [ ] **THEME-003:** edição de cores, fontes e fundo pelo próprio Tako.
- [ ] **THEME-004:** instalar tema por URL de repositório, com atualização opcional.
- [ ] **THEME-005:** galeria comunitária de temas.
- [ ] **THEME-006:** criar o próprio tema pelo Tako, sem editar arquivos: partir do tema padrão (ou de um instalado), escolher cores, fonte e fundo com prévia ao vivo, dar nome, salvar como tema instalado e exportar como `.takotheme` para compartilhar. Base: os `tokens.toml` (temas que só trocam a paleta) e THEME-002/003.
- [ ] **THEME-010:** estudo de conversor de temas do Playnite 10 para o formato do Tako. Sem compromisso de fidelidade; só segue se o estudo mostrar resultado útil.

### Épico PERF (P2)

- [ ] **PERF-001:** painel de diagnóstico do próprio Tako (memória, CPU, tempo de abertura).
- [ ] **PERF-002:** presets de tela por jogo e por monitor, incluindo HDR e VRR onde o SO expõe.

### Épico EXT (P3)

- [ ] **EXT-001:** eventos de sessão (`GameStarting`, `GameStarted`, `GameExited`) e ferramentas externas por jogo (pré e pós-execução), com consentimento.
- [ ] **EXT-002:** API local de integração (a mesma base que o Console Mode usará no sentido inverso).
- [ ] **EXT-003:** plugins de metadados e de fontes, sem acesso privilegiado.

### Épico STREAM (P3)

- [ ] **STREAM-001:** integração com Sunshine/Apollo e Moonlight.
- [ ] **STREAM-002:** controle remoto pelo celular por interface web local autenticada.

### Épico CLOUD (P3)

- [ ] **CLOUD-001:** backup e restauração da biblioteca e das configurações.
- [ ] **CLOUD-002:** sincronização opcional por WebDAV ou S3 compatível.

### Épico SOURCE (P3)

- [ ] **SOURCE-001:** catálogos JSON de fontes de conteúdo autorizado, com procedência, permissões por fonte e quarentena; downloads HTTPS com pausa, retomada e checksum. Nenhuma fonte pode executar código ou instalar programas sozinha.

---

## 10. Critérios de aceite da paridade

**CA-01 — Instalação nos três SOs.** Dado um Windows, um Linux (CachyOS com KDE Wayland e uma sessão gamescope) e um macOS limpos, quando o usuário instalar pelo pacote da release, então ele abre o Tako, vê os jogos detectados das lojas instaladas e abre um jogo.

**CA-02 — Controle.** Com um controle suportado, o usuário navega pela biblioteca, busca, filtra, abre detalhes, joga e volta, sem mouse e sem teclado.

**CA-03 — Tela e áudio.** Um jogo configurado com outra resolução e outra saída de áudio abre com elas e, ao terminar (ou se o Tako cair no meio), a tela e o áudio voltam ao que eram.

**CA-04 — Cursor.** Durante a sessão, o cursor fica escondido quando configurado e o analógico move o ponteiro e clica, dentro das limitações documentadas por SO.

**CA-05 — Leve durante o jogo.** Com um jogo aberto, o Tako não desenha quadros e fica dentro da meta de memória da §8.1.

**CA-06 — Tema.** Um tema válido é instalado e aplicado em tempo de execução; um tema quebrado é rejeitado ou cai no padrão, com log e sem loop de falha.

**CA-07 — Migração.** Um usuário da alpha C# atualiza e mantém biblioteca, coleções, tempo de jogo, escolhas de arte, configurações e credenciais dos provedores.

**CA-08 — Atualização.** A versão instalada encontra a release seguinte do seu canal, baixa o arquivo certo para o SO e confere o SHA-256 antes de instalar.

---

## 11. Riscos

| Risco | Impacto | Mitigação |
|---|---|---|
| Slint não atingir fluidez ou memória na grade grande | Alto | Gate do MIG-001 antes de qualquer port; alternativa Avalonia com decisão dos mantenedores |
| Wayland sem API para esconder cursor ou trocar resolução em todos os compositores | Médio | Matriz real no MIG-002; implementação por compositor; limitações mostradas ao usuário |
| macOS exigir permissões (Acessibilidade) que assustam o usuário | Médio | Pedir só ao ligar o cursor pelo controle, com explicação |
| Usuários das alphas perderem dados na troca | Alto | MIG-030 com testes sobre bancos reais; backup automático antes de migrar |
| Reescrita longa deixar o produto parado | Alto | WPF segue publicado até a paridade (D-08); fases com entregas verificáveis |
| Remoção de funções que estavam nas alphas (TV, FPS) frustrar usuários | Médio | Changelog claro e épico CMI para recuperá-las via Console Mode |
| Fim da compatibilidade com temas do Playnite | Médio | Tema padrão forte, tema alternativo, API documentada e estudo do conversor (THEME-010) |
| Mudanças nos formatos das lojas | Médio | Parsers isolados por fonte, testes com fixtures, falha de uma fonte não derruba as outras |

---

## 12. Definition of Done

- Implementado, revisado e com testes automatizados onde aplicável (inclusive os portados do C#).
- CI verde nos três SOs.
- Funções de hardware com checklist por SO executado ou marcado como pendente na PR.
- Erros com tratamento e caminho de recuperação; nenhum `unwrap` em caminho de dado externo.
- Textos visíveis em português do Brasil e inglês.
- Changelog (`## [Unreleased]` nos dois arquivos) e documentação atualizados.
- Item marcado `[x]` neste PRD na mesma PR.

---

## 13. Referências

- Console Mode: https://github.com/lippdev/consolemode
- Slint: https://slint.dev e https://docs.slint.dev
- SDL3: https://wiki.libsdl.org/SDL3
- gamescope: https://github.com/ValveSoftware/gamescope
- Heroic Games Launcher: https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher
- Lutris: https://github.com/lutris/lutris
- Protocolo `wlr-output-management`: https://wayland.app/protocols/wlr-output-management-unstable-v1
- Mutter `DisplayConfig`: https://gitlab.gnome.org/GNOME/mutter/-/blob/main/data/dbus-interfaces/org.gnome.Mutter.DisplayConfig.xml
- Microsoft — Game launchers and handhelds: https://learn.microsoft.com/en-us/xbox/gdk/docs/gdk-dev/pc-dev/handheld/handheld-launchers
- Código e testes C# atuais (`src/`, `tests/`): especificação executável até o corte (§0)
