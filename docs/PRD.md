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

### Prioridade MoSCoW da rodada de 19/10

Ordem do que falta nesta rodada (decisão do mantenedor de usar MoSCoW). Fora daqui ficam os itens que dependem de hardware (MIG-*), o CMI-001 (no repositório do Console Mode) e os já fechados.

| | Itens | Por quê |
|---|---|---|
| **Must** | CLOUD-001, DL-001, EXT-002, SESSION-005, SRC-002, THEME-003 | já estão em PR ou em andamento; protegem dados do usuário (backup) e fecham a paridade no Linux e a base de integração (API local, que o CMI usa) |
| **Should** | THEME-007, THEME-008 (Fita), STREAM-001, CMI-002, THEME-006 | valor visível com pouco risco; CMI-002 é só detecção sobre a API local; THEME-006 reaproveita o THEME-003 |
| **Could** | CLOUD-002, DL-002, EXT-003, STREAM-002, THEME-005, UX-002, SOURCE-001, CMI-003, CMI-004, INPUT-002, SESSION-001, ART-002 | maiores ou dependentes de outro item, de ADR aceita (PR #135) ou do Console Mode |
| **Won't (agora)** | DATA-001, DEALS-001, ART-003, VOICE-001, THEME-011 | precisam de serviço externo, fonte de preços, servidor ou decisão ainda não tomada; os outros temas oficiais ficaram para depois do Fita |

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

Ideias ainda não priorizadas ficam em [`docs/backlog/ideias.md`](backlog/ideias.md); entram aqui quando forem para o plano.

### Épico CMI — Integração com o Console Mode (BACKLOG, P1, depende do Console Mode)

- [ ] **CMI-001 — API local no Console Mode** (no repositório dele): descoberta, versão, capacidades e comandos (TV, sobreposições, sessão de console). Contrato definido no ADR-0025.
- [x] **CMI-002 — Detecção no Tako:** saber se o Console Mode está instalado e qual versão da API ele oferece.
- [ ] **CMI-003 — Ações delegadas:** quando o Console Mode existe, a Central e as Configurações do Tako mostram as funções dele (TV, FPS, janelas), executadas por ele.
- [ ] **CMI-004 — Tako como launcher do Console Mode:** o Console Mode pode abrir o Tako como interface da sessão, no lugar do Big Picture ou do Playnite.

### Épico PLAT — Integrações de sistema (P1)

- [ ] **PLAT-001 (Linux):** sessão gamescope do Tako (entrada no gerenciador de login, como a do Steam).
- [x] **PLAT-002 (Linux):** pacotes para Arch/CachyOS (PKGBUILD no AUR) e Flatpak. _Estado: PKGBUILDs `tako-launcher-bin` e `tako-launcher` e Flatpak `io.github.lippdev.Tako` no repositório, validados no CI; publicar no AUR e no Flathub depende da conta do mantenedor e do ID do Flathub._
- [ ] **PLAT-003 (Windows):** Tako como app inicial da experiência de tela cheia do Xbox, onde a Microsoft permitir (exige SCCD assinado pela Microsoft; ver `docs/spikes/MVP-007-fse.md`).
- [ ] **PLAT-004 (macOS):** assinatura Developer ID e notarização.
- [ ] **PLAT-005 (Windows):** assinatura de código (SignPath) e winget.
- [x] **PLAT-006:** abrir com o sistema e iniciar direto em tela cheia, em cada SO.
- [x] **PLAT-008:** canal Nightly: build automática da `develop` todo dia em que houver mudança, publicada como pré-release `nightly` (sem changelog manual, notas pelos commits) e escolhida no atualizador como "Versões noturnas", separada de alpha e beta.
- [x] **PLAT-007 (Windows):** integração com o [AnyFSE](https://github.com/ashpynov/AnyFSE), projeto da comunidade que põe um launcher de terceiros como app inicial do Xbox mode (tela cheia de jogos do Windows 11), enquanto o caminho oficial (PLAT-003) depende da Microsoft. Escopo: o Tako funciona como alvo do AnyFSE (abre em tela cheia, sem janela de desktop, volta certo de um jogo), o Tako detecta o AnyFSE instalado e ensina a configuração, e um roteiro de teste em `docs/testing/windows.md`. O AnyFSE pede Modo de Desenvolvedor (ver `docs/spikes/MVP-007-fse.md`); o Tako não liga isso sozinho.

### Épico SRC — Mais fontes (P1)

- [x] **SRC-001:** emuladores (RetroArch e standalone), bibliotecas de ROMs do usuário e sistemas, nos três SOs. Inclui importar direto dos principais emuladores e frontends os jogos que eles já conhecem (playlists do RetroArch, listas do ES-DE, bibliotecas do Dolphin, PCSX2, DuckStation, PPSSPP, Ryujinx e afins), com o comando certo para abrir cada jogo no emulador.
- [ ] **SRC-002 (Linux):** Bottles e itch.io. _Estado: Bottles e itch.io lidos no Linux; falta validar em máquina real (docs/testing/linux.md)._
- [x] **SRC-003:** detectar cliente de loja ausente e oferecer abri-lo ou instalá-lo pelo caminho oficial.
- [x] **SRC-005:** jogos que o usuário possui mas não instalou, e instalar pelo Tako: listar a biblioteca de cada loja (Steam, Epic, GOG e outras, pela conta do usuário ou pelo cliente da loja) com o selo "Não instalado", e o botão Instalar abre a instalação no cliente da loja (por exemplo `steam://install/<appid>`). Quando a instalação termina, o jogo vira instalado sem precisar reiniciar. Pedido do mantenedor no primeiro teste. _Estado: Steam, Epic, GOG, Xbox e Amazon (Windows); progresso com porcentagem só na Steam. Ubisoft fica para um estudo e a EA não entra (ADR-0029)._
- [x] **SRC-004:** jogos não Steam adicionados à Steam (atalhos), nos três SOs.

### Épico UX (P1)

- [x] **UX-001:** acessibilidade: tamanho de texto, redução de animações, alto contraste, sons de navegação com volume próprio.
- [ ] **UX-002:** arte animada e trailers (libmpv opcional, desligado durante o jogo).
- [x] **UX-003:** assistente da primeira execução (idioma, lojas detectadas, provedores de metadados; oferece o driver opcional do INPUT-005).
- [ ] **UX-004:** perfis locais (vários usuários na mesma máquina).
- [x] **UX-005:** ícone do jogo na lista, como na Steam; a capa fica só na Grade. O ícone vem da própria loja (Steam, Epic, GOG, Xbox) ou do executável no Windows, com a inicial do título quando não houver.

### Épico THEME (P2)

- [x] **THEME-001:** documentação pública da API de temas.
- [x] **THEME-002:** pré-visualização antes de aplicar.
- [x] **THEME-003:** edição de cores, fontes e fundo pelo próprio Tako.
- [x] **THEME-004:** instalar tema por URL de repositório, com atualização opcional.
- [ ] **THEME-005:** galeria comunitária de temas.
- [x] **THEME-006:** criar o próprio tema pelo Tako, sem editar arquivos: partir do tema padrão (ou de um instalado), escolher cores, fonte e fundo com prévia ao vivo, dar nome, salvar como tema instalado e exportar como `.takotheme` para compartilhar. Base: os `tokens.toml` (temas que só trocam a paleta) e THEME-002/003.
- [x] **THEME-007:** música de fundo e sons de navegação por tema (o tema traz os arquivos; volume próprio e opção de desligar; nunca toca durante o jogo).
- [x] **THEME-008:** primeiro tema oficial alternativo, o **Fita** (carrossel horizontal com arte em tela cheia), escolhido entre as propostas de `docs/themes/THEME-008-propostas.md`.
- [ ] **THEME-011 (futuro):** os outros quatro temas oficiais escolhidos em `docs/themes/THEME-008-propostas.md`: Deck Handheld, Lista & Cinema, Console Retrô CRT e Estante Clássica. Fica para depois do Fita, reaproveitando o que ele ensinar (contraste sobre a arte, texto em 1,5x).
- [x] **THEME-010:** estudo de conversor de temas do Playnite 10 para o formato do Tako. Sem compromisso de fidelidade; só segue se o estudo mostrar resultado útil. _Estado: estudado (`docs/spikes/AI-001-THEME-010-estudo.md`): só as cores convertem, então não segue._

### Épico LIB — Organização da biblioteca (P2)

- [x] **LIB-001:** status de cada jogo (jogando, zerado, pausado, abandonado, quero jogar), mudado pelos detalhes ou pelo menu do card, com filtro no painel.
- [x] **LIB-002:** tags próprias do usuário: criar, aplicar a vários jogos de uma vez e filtrar.
- [x] **LIB-003:** coleções automáticas: salvar uma combinação de filtros (gênero, fonte, tempo jogado, status, tag) como coleção que se atualiza sozinha.
- [x] **LIB-004:** jogos quebrados: verificar em segundo plano executáveis e pastas que sumiram, listar esses jogos e oferecer corrigir o caminho ou remover. Hoje só os detalhes avisam, um jogo por vez.
- [x] **LIB-005:** armazenamento: quanto cada jogo ocupa, total por disco, e ordenar por tamanho.

### Épico ART — Capas e arte (P2)

- [x] **ART-001:** casar a biblioteca inteira com as capas certas sozinho, como o Plex faz com filmes: reconhece o nome do jogo, escolhe capa, fundo, logo e ícone, e o usuário só corrige o que ficou errado. Refs: Steam ROM Manager, BoilR, SteamGridDB Decky, Steam Art Manager.
- [ ] **ART-002:** aplicar uma imagem do site do SteamGridDB num jogo com um clique (protocolo próprio, como o SGDBoop). _Estado: fora da rodada de 17/10; a API do botão do SteamGridDB usa um token de aplicativo, não a chave do usuário. Código no PR #94 (fechado)._
- [ ] **ART-003:** biblioteca de capas online: o usuário escolhe capas personalizadas de uma galeria e envia as suas, além das do SteamGridDB.

### Épico BRAND — Identidade (P2)

- [x] **BRAND-001:** animação do Tako (o polvo), usada no app (espera, estados vazios, Bem-vindo de volta).
- [x] **BRAND-002:** animação de entrada ao abrir o app, curta, pulável, desligada com "Reduzir animações" e sem atrasar a primeira tela (§8.1).

### Épico PERF (P2)

- [x] **PERF-001:** painel de diagnóstico do próprio Tako (memória, CPU, tempo de abertura).
- [x] **PERF-002:** presets de tela por jogo e por monitor, incluindo HDR e VRR onde o SO expõe. _Estado: VRR por jogo no KDE (kscreen-doctor) e no wlroots (wlr-randr); no Windows e no macOS segue sem API pública._

### Épico INPUT — Controles (P2)

- [x] **INPUT-001:** personalizar os controles do próprio Tako: remapear cada ação (aceitar, voltar, menu, seções, atalho da Central) por controle, com perfis.
- [x] **INPUT-003:** bateria dos controles sem fio na barra superior e na Central, com aviso de bateria fraca.
- [x] **INPUT-004:** reconexão assistida: quando o controle cai, o Tako avisa, não perde o foco e volta a navegar assim que um controle reconecta (ou outro assume).
- [ ] **INPUT-002:** remapear controles para os jogos, como a Entrada Steam: perfis por jogo, botões, analógicos, gatilhos e mouse/teclado. Precisa de um controle virtual no SO (uinput no Linux; no Windows o ViGEmBus foi descontinuado, então é preciso escolher outro driver). Começa por um estudo. _Estado: estudo em `docs/spikes/SESSION-001-INPUT-002.md`; a implementação depende de ADR e dos experimentos listados._
- [x] **INPUT-005:** isolar o controle do jogo enquanto a Central está aberta (Windows). Jogos que leem por XInput (ex.: Resident Evil 2) continuam recebendo o controle sem estar em foco. Driver HidHide, **opcional**: o Tako esconde o controle do jogo ao abrir a Central e devolve ao voltar. A instalação é oferecida no assistente da primeira execução (UX-003) e em Configurações; sem o driver, tudo segue como hoje. Visto no teste manual de 2026-10-16. _Estado: falta confirmar em hardware se o jogo já aberto deixa de ver o controle (`docs/testing/windows.md` §17)._

### Épico SESSION — Sessões de jogo (P2)

- [ ] **SESSION-001:** retomada rápida, como o Quick Resume do Xbox Series: suspender o jogo aberto (sem fechar) ao abrir outro e voltar exatamente onde estava. No PC fica na memória, não sobrevive a reiniciar o computador. Não funciona com jogos online nem com anti-cheat, que precisam ficar de fora. Começa por um estudo. _Estado: estudo em `docs/spikes/SESSION-001-INPUT-002.md`; a implementação depende de ADR e dos experimentos listados._
- [x] **SESSION-002:** histórico de sessões: data, hora e duração de cada vez que o jogo rodou (hoje só o total e a última vez), nos detalhes do jogo.
- [x] **SESSION-003:** jogo que fechou sozinho em poucos segundos ou com código de erro gera um aviso com o que se sabe (código, log do Tako, sugestão), em vez de voltar em silêncio. _Estado: o código de saída só existe quando o Tako abre o executável direto; jogos abertos pela loja ficam com o aviso por tempo._
- [x] **SESSION-004:** por jogo, variáveis de ambiente e comandos antes e depois de jogar (argumentos e pasta de trabalho já existem), com consentimento. Base do EXT-001.
- [ ] **SESSION-005 (Linux):** escolher Proton ou Wine (versão e prefixo) para jogos de Windows adicionados à mão. _Estado: Proton e Wine escolhidos por jogo manual, com prefixo validado; falta validar em máquina Linux real (docs/testing/linux.md)._
- [x] **SESSION-006:** "Reparar jogo": abrir a verificação de arquivos da loja dona do jogo (Steam, Epic, GOG, etc.) quando ela oferece. _Estado: só a Steam oferece a verificação de fora do app (`steam://validate`)._

### Épico VOICE e AI — Busca por voz e IA (P3)

- [ ] **VOICE-001:** procurar jogos falando ("abrir Hades", "jogos de corrida"), com reconhecimento de fala local (sem mandar áudio para fora) e nos dois idiomas. _Estado: fora da rodada de 17/10; precisa de ADR (motor, modelo, dependências) e design. Levantamento na issue #102._
- [x] **AI-001:** estudo de um assistente de IA para organizar e gerenciar a biblioteca (coleções, limpeza, recomendações, configurações por jogo). Downloads pelo assistente seguem a mesma regra do épico DL. _Estado: estudo em `docs/spikes/AI-001-THEME-010-estudo.md`; três decisões ficam com o mantenedor._

### Épico DATA — Dados de melhoria (P3)

- [ ] **DATA-001:** enviar dados de uso e falhas para os desenvolvedores, **desligado por padrão** e com opt-in explícito: anônimo, sem lista de jogos nem caminhos, com tela mostrando o que é enviado e opção de apagar. Precisa seguir LGPD e GDPR e escolher o serviço que recebe os dados.

### Épico DL — Downloads (P3)

Decisão do mantenedor (2026-10-16): o Tako fornece só a ferramenta. As fontes são do usuário e ficam a critério dele; o Tako não traz, não recomenda e não lista fontes, nem na interface nem na documentação.

- [ ] **DL-001:** baixar jogos por torrent a partir de fontes `.json` que o próprio usuário adiciona, em formato de fonte .json. Quando um item da fonte é um jogo que já existe numa biblioteca (Steam, Epic, adicionado à mão, etc.), o download se vincula a esse jogo: o card e os detalhes mostram "Baixando Batman: Arkham Knight" com progresso, em vez de criar um jogo novo. Ao terminar, o jogo fica instalado e jogável pelo Tako. _Estado: downloads por torrent de fontes do usuário, desligados por padrão (ADR-0028); falta validar com rede e fontes reais._
- [ ] **DL-002:** depois do DL-001, usar serviços de download acelerado como o Real-Debrid, com a conta do próprio usuário.

### Épico DEALS — Promoções (P3)

- [ ] **DEALS-001:** página de promoções: jogos em oferta nas lojas (Green Man Gaming, Nuuvem, Instant Gaming e outras), com preço, desconto e link de afiliado, em layout parecido com uma loja. O Tako não vende nada: o link abre a loja no navegador. Precisa dizer claramente que os links são de afiliado e escolher a fonte dos preços (API da loja, programa de afiliados ou agregador).

### Épico EXT (P3)

- [x] **EXT-001:** eventos de sessão (`GameStarting`, `GameStarted`, `GameExited`) e ferramentas externas por jogo (pré e pós-execução), com consentimento.
- [x] **EXT-002:** API local de integração (a mesma base que o Console Mode usará no sentido inverso).
- [x] **EXT-004:** Discord Rich Presence: mostrar o jogo aberto no Discord, desligado por padrão.
- [x] **EXT-005:** links `tako://` para abrir um jogo ou uma tela do Tako de fora (atalhos, Stream Deck, ART-002). _Estado: Windows e Linux entregues (0.2.0-alpha.5); falta o macOS, onde o toolkit ainda não repassa os eventos `OpenURLs`._
- [ ] **EXT-003:** plugins de metadados e de fontes, sem acesso privilegiado.

### Épico STREAM (P3)

- [ ] **STREAM-001:** integração com Sunshine/Apollo e Moonlight. _Estado: exportação para o Sunshine/Apollo com prévia e backup do apps.json (ADR-0034); falta validar com Sunshine e Moonlight reais (docs/testing/stream-001-sunshine.md)._
- [ ] **STREAM-002:** controle remoto pelo celular por interface web local autenticada.

### Épico CLOUD (P3)

- [x] **CLOUD-001:** backup e restauração da biblioteca e das configurações.
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
