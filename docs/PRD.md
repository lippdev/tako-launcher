# Tako — PRD e backlog

**Versão:** 0.4 (fim da transição para a interface web e abandono do código legado)  
**Data:** 25/10/2026 (v0.3 de 24/10/2026, v0.2 de 16/10/2026)  
**Status:** em vigor. Substitui a versão 0.1 inteira (WPF, Windows-only, runtime absorvido do Console Mode). A v0.3 trocou o Slint por uma interface web (React sobre Chromium embutido) e acrescentou o instalador com onboarding (§2.1, épicos WEB e INST). A v0.4 registra o que a transição entregou (§2.2), fecha o motor (CEF), define a metodologia de versões alpha → beta → estável, pausa Linux e macOS até o fim da alpha e torna obrigatório abandonar o código legado: C# agora, Slint no corte (§2.3, Fase 5).  
**Plataformas:** Windows, Linux e macOS, com a mesma experiência. **Durante a fase alpha, só Windows** (D-15, D-20).  
**Licença:** AGPL-3.0-only ([ADR-0014](adr/0014-licenca-agpl.md))

---

## 0. Como usar este documento

Este PRD descreve **o que o Tako precisa ter a partir de agora**. O que existia antes e não aparece aqui está fora do escopo.

Para quem for implementar, inclusive agentes:

1. **Ordem de verdade:** este PRD, depois os ADRs novos (a partir do ADR-0023), depois o `CONTRIBUTING.md`. Os ADRs 0011 a 0022 descrevem a implementação C#/WPF e são **referência histórica** (MIG-004).
2. **O código C# saiu do repositório** (WEB-72, 25/10/2026; continua no histórico do git). Já tinha sido portado para Rust com os testes na Fase 1 e 2. **Até o corte do Slint (WEB-50), a `Shell` da `tako-ui` é a especificação de comportamento** das telas que ainda faltam na web (tela cheia, teclado virtual, Playnite, modo seguro, configurações restantes): estados, ações e textos vêm dela; o visual vem do `design/`.
3. **Uma história = uma tarefa**, no processo de [`docs/processo/multiagente.md`](processo/multiagente.md): cada tarefa roda numa worktree própria, com brief, verificação local (D-13) e relatório; o orquestrador revisa e junta na branch de integração (`feature/WEB-interface-web`) por squash. A PR para a `develop` sai em marcos, com aprovação do mantenedor. Marque `[x]` neste arquivo quando a história entra na integração.
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
| D-02 | ~~**Slint** é o toolkit da interface.~~ | **Substituída pela D-09** (24/10/2026). O Slint sai no corte (WEB-50, D-16). |
| D-03 | O Tako mantém **somente quatro** funções vindas do Console Mode: troca de resolução, troca de saída de áudio, esconder o cursor e cursor pelo controle. O resto sai do Tako (§3). | Decidido |
| D-04 | A integração com o Console Mode exige uma API que **ainda não existe** no Console Mode. Fica no backlog (épico CMI); o Tako não depende dela para funcionar. | Decidido |
| D-05 | **Fim do WPF e da compatibilidade com temas do Playnite 10** (`.pthm`, XAML restrito, `PlayniteCompat.P10`). Temas passam a ter formato próprio. Um conversor de temas do Playnite fica como estudo no backlog (THEME-010), sem compromisso. | Decidido |
| D-06 | **Dados dos usuários das alphas são migrados**: biblioteca, coleções, configurações, escolhas de arte e credenciais (MIG-030). Ninguém perde a biblioteca na troca. | Decidido |
| D-07 | **Releases:** `main` publica estável; `develop` publica alpha e beta pelo workflow `Release` quando um dev pede, com o próximo número do canal. Saídas: Windows (instalador), Linux (deb, AppImage, tar.gz; x64 e arm64) e macOS (dmg e zip; Apple Silicon e Intel). | Decidido. Atualizado em 25/10/2026: **sem portátil no Windows** a partir da 0.2.0-alpha.23 (com o CEF o app vira uma pasta de ~450 MB; volta se fizer falta); Linux e macOS pausados durante a alpha (D-20). |
| D-08 | ~~A UI C#/WPF continua sendo a versão publicada até a paridade.~~ | **Encerrada.** O Rust assumiu na 0.2.0 e o código C# saiu do repositório no WEB-72 (25/10/2026). |
| D-09 | **Interface web: React + TypeScript desenhados por um Chromium embutido (CEF)**, no modelo do Steam. O núcleo continua em Rust e é a fonte da verdade; o React guarda só o estado de tela. Ponte tipada pelas mensagens do CEF, sem porta de rede. Motivos na §2.1; detalhes no [ADR-0035](adr/0035-interface-web-rust-e-chromium.md). | Decidido e implementado no Windows (§2.2). Motor fechado na D-19. |
| D-10 | **Interface descartável:** durante o jogo, minimizado e na bandeja, os processos do Chromium terminam e fica só o núcleo Rust. Ao voltar, a interface reabre na tela onde estava. | **Implementado** com dois processos (`tako --web` núcleo, `tako --web-ui` interface; ADR-0035 item 4). Medido em release: sem interface, 9,4 MB e zero Chromium; jogo, bandeja e fechar para a bandeja descartam a interface (WEB-02b, WEB-20, WEB-36). |
| D-11 | **Instalador no modelo do Steam:** um instalador pequeno baixa e confere os componentes (núcleo, runtime do Chromium, pacote da interface), emenda no onboarding e depois atualiza cada componente separado. Continua existindo um pacote completo offline. | **Meta mantida** (25/10/2026), para antes da primeira estável, junto do sandbox (WEB-02c) e do onboarding. As alphas usam o instalador completo (~143 MB). |
| D-12 | O **Slint fica congelado** até o corte: só correções críticas, sem polimento nem funções novas de interface. Funções novas entram no núcleo Rust e ganham tela já na versão web. | Decidido. A partir da 0.2.0-alpha.23 o Slint só abre com `--slint` (ou se o Chromium falhar ao carregar). |
| D-13 | **Testes locais durante a migração:** fmt, clippy, testes e build rodam na máquina do desenvolvimento (agentes e orquestrador) antes de cada entrada na integração. O GitHub guarda o código, mas os workflows de teste (`CI`, `Rust`, `Packaging`, `PR title`) ficam desligados e só rodam se alguém pedir. O `Release` continua para publicar versões. | Decidido (24/10/2026). Religar com `gh workflow enable` e os gatilhos antigos quando a migração acabar. |
| D-14 | **Implementar primeiro, otimizar depois:** a interface web é construída direto na fundação definitiva, sem protótipo descartável nem linha de base do Slint antes. As medições da §8.5 acontecem sobre o app web (WEB-60; a comparação CEF × WebView2 foi cancelada pela D-19), e o que não bater a meta vira tarefa de otimização. | Decidido (24/10/2026). Substitui o gate antes do porte (MIG-W00 e MIG-W01). |
| D-15 | **Fase Windows primeiro:** a migração para a interface web mira só o Windows até a paridade com a interface Slint no Windows. Linux e macOS (hospedeiro CEF, empacotamento, testes) ficam para depois. Matriz de paridade em [`docs/plans/web-paridade-windows.md`](plans/web-paridade-windows.md). | Decidido (24/10/2026). Paridade de desktop alcançada em 25/10/2026, menos o que está na Fase 5. Estendida pela D-20. |
| D-16 | **Abandonar o código legado é obrigatório.** A transição só termina quando o Tako tiver **uma** interface (React + CEF) e **um** núcleo (Rust), sem C#, sem Slint e sem caminhos duplicados mantidos "por garantia". O C# sai agora (WEB-72). O Slint sai no corte (WEB-50), assim que a web cobrir o que só ele faz (tela cheia e as funções adiadas). Até lá ninguém investe no Slint (D-12). O histórico do git guarda tudo. | Decidido (25/10/2026), de suma importância para o mantenedor. |
| D-17 | **Metodologia de versões: alpha → beta → estável**, numeradas (`0.2.0-alpha.N`, `0.2.0-beta.N`, `0.2.0`), publicadas pelo fluxo normal (D-07) com o mesmo AppId, atualizando a instalação no lugar. **Nada de instaladores paralelos de "prévia"** para testar uma etapa: cada marco testável é a próxima alpha. A primeira com a interface web é a **0.2.0-alpha.23** (a alpha.22 ainda é Slint). | Decidido (25/10/2026). O instalador de prévia (INST-00) foi descartado. |
| D-18 | **Critério de beta:** a alpha é **sem público** (o mantenedor e testes internos); a beta é a primeira versão para **reter público**. Sai da alpha quando o app estiver funcional e com praticamente todas as funções que um launcher deve ter **validadas** (lista na Fase 5, "Saída da alpha"). | Decidido (25/10/2026) |
| D-19 | **CEF é o motor definitivo**, em todos os SOs. A comparação com o WebView2 (MIG-W01) não será feita: o CEF já está pronto e cumpre as metas (98 MB com a interface aberta contra 326 MB do Slint), e manter dois motores dobraria a ponte e os testes. | Decidido (25/10/2026). Substitui o item 5 do ADR-0035. |
| D-20 | **Linux e macOS pausados durante a alpha.** Voltam assim que a fase alpha terminar (D-18), com o hospedeiro CEF, o empacotamento e os testes de cada SO (Fase 6). Enquanto o Slint existir, os pacotes de Linux e macOS seguem saindo com ele, sem funções novas; depois do corte do Slint, não há pacote desses SOs até a Fase 6. | Decidido (25/10/2026) |

### 2.1 Por que a interface vai para a web (D-09)

A interface em Slint chegou à paridade (MIG-029) e absorveu o design system (UX-007). No uso diário, o mantenedor
viu que ela não entrega o acabamento que o Tako quer, e que esses problemas não aparecem em teste automático:

1. **Texto granulado nas animações.** Menus, dropdowns, diálogos e cards abrem com escala animada; o Slint
   redesenha o texto em tamanhos fracionários a cada quadro e os glifos "fervem". Um navegador compõe a camada
   pronta na GPU e não mexe no texto durante a animação.
2. **Qualidade do texto parado.** O renderizador do Slint em uso (FemtoVG) desenha texto visivelmente pior que o
   Chromium.
3. **Telas abaixo de 60 fps.** Seções com vidro, desfoque e várias camadas perdem quadros, e o Slint dá pouca
   ferramenta para achar a causa. O Chromium tem um compositor otimizado há mais de quinze anos e o DevTools para
   medir tela por tela.
4. **Custo de cada detalhe.** O mantenedor trabalha com React, CSS e WebView2 e já tem um app Rust + WebView2 com
   esse nível de animação. No Slint há menos componentes prontos, menos exemplos e menos gente para perguntar.
5. **É o caminho da referência.** O Steam Big Picture é React num Chromium embutido desde 2022 (é a interface do
   Steam Deck), e o cliente desktop do Steam inteiro desde 2023. Battle.net, EA app e Epic usam o mesmo motor
   (CEF).
6. **O Tako já está pronto para a troca.** O núcleo (biblioteca, lojas, metadados, sessão, plataforma, downloads)
   está em Rust, separado da interface, e a interface já termina durante o jogo (ADR-0023). Muda a camada de
   apresentação; o núcleo fica.

Por que não as alternativas: refinar no Slint resolve só parte do problema e o teto continua incerto; GPUI ainda é
instável e sem temas em tempo de execução; o WebView do sistema é o WebKitGTK no Linux, bem pior em animação, e
quebraria o "igual em todo lugar"; Electron carrega o Node.js inteiro e é o que dá a fama de app pesado.

O que se aceita em troca: o Chromium soma de 150 a 250 MB à instalação; o Tako passa a responder pelas
atualizações de segurança do Chromium; com a interface aberta, a memória base é maior que a de um toolkit nativo,
o que a interface descartável (D-10) compensa; e os temas `.slint` deixam de funcionar (§7). As metas que provam
que a troca valeu estão na §8.5.

### 2.2 O que a transição entregou (24 e 25/10/2026)

A Fase 4 foi feita em dois dias por um orquestrador e vários agentes em worktrees separadas
([`docs/processo/multiagente.md`](processo/multiagente.md)), com verificação local (D-13) e revisão cruzada entre
modelos. O quadro de tarefas está em [`docs/plans/web-quadro.md`](plans/web-quadro.md) e a matriz de paridade em
[`docs/plans/web-paridade-windows.md`](plans/web-paridade-windows.md).

**Arquitetura que existe hoje (Windows)**

| Peça | O que é | Por que assim |
|---|---|---|
| `tako-app` | Camada de serviços sem interface: boot do núcleo, biblioteca, configurações, coleções, lojas, arte, metadados, downloads, sessão, atualizações, Discord, API local, links `tako://`. | Tira da `Shell` do Slint o que é regra do app, para a web usar sem tela e com testes (WEB-01a a 01e). |
| Ponte tipada | Mensagens JSON `{v, id, name, payload}` pelas mensagens de processo do CEF; consultas e comandos com resposta ou erro tipado; eventos do núcleo (`library.changed`, `art.changed`, `game.started`, `game.ended`, `input.nav`, `updates.changed` e outros). Tipos TypeScript gerados dos tipos Rust (ts-rs) por `cargo test -p tako-app`. | Sem porta de rede entre interface e núcleo (segurança) e quebra de contrato detectada na compilação (WEB-00). |
| Dois processos | `tako --web` é o núcleo e **nunca carrega o Chromium**; `tako --web-ui` é o filho descartável com o CEF, falando com o núcleo por stdin/stdout. | O CEF não reinicia dentro do mesmo processo; sem separar, a interface "fechada" deixava 74,7 MB e 4 processos vivos. Separado: 9,4 MB e zero Chromium (WEB-02b, D-10). |
| Esquemas próprios | `tako-app://` serve o pacote da interface; `tako-img://` serve imagens do cache (capas, fundos, logos, ícones, já reduzidas ao tamanho pedido), com validação de caminho. | A interface nunca lê disco nem rede direto (WEB-00, WEB-01d). |
| `web/` | React + TypeScript + Vite; design system do `design/desktop/` quebrado em componentes; foco espacial e entrada do controle (SDL3 no núcleo) pela ponte; textos dos catálogos `locales/` via `t()`/`useT()`. | O visual vem do design do mantenedor (prévias a 1600×900 como aceite); o controle continua no núcleo para funcionar também com a interface fechada (WEB-03 a WEB-06). |
| CEF 156 | Crate `cef` 156.2.0, runtime baixado por `scripts/cef-setup.ps1`; `scripts/cef-check.ps1` e um workflow semanal avisam versão nova (WEB-41). | O Tako responde pela segurança do Chromium embutido. |

**Telas e funções com paridade no desktop Windows:** janela e barra próprias; lista da biblioteca (abas, busca,
filtros, ordenação, coleções, seleção múltipla, modos Lista, Grade e Categorias); visão geral; página do jogo;
configurações do jogo; editar e adicionar jogo; fontes e IDs; seletor de imagens e revisar capas; etiquetas;
coleções automáticas; busca geral (Ctrl+K); downloads; atividades na barra; perfil; diálogos; primeira execução
(cinco passos); configurações (Geral, Aparência, Bibliotecas, Tema, Som, Tela e áudio, Integrações, Sistema, Sobre,
aparência do desktop, emuladores e ROMs, downloads, perfis); jogar de verdade (abrir, tela e áudio por jogo,
acompanhar, restaurar, recuperação, "Abrindo…", "Fechando o jogo…", estado jogando); Central rápida por cima do
jogo com atalho global; bandeja do sistema; atualizações do app; Discord, API local e `tako://`; seletor de
arquivo e pasta do Windows; abertura do app com a primeira leitura; idiomas pt-BR e en-US em todas as telas.

**Medição em release (WEB-60, [`docs/plans/web-medicao-2026-10.md`](plans/web-medicao-2026-10.md))**

| Métrica | Web (CEF + React) | Slint | Meta (§8.5) |
|---|---|---|---|
| Memória com a interface aberta, parada | **97,6 MB** (102 a 145 MB com 318 jogos) | 325,5 MB | ≤ 250 MB: passa |
| Memória sem interface / durante o jogo | **9,4 MB**, zero Chromium | 2,3 MB | ≤ 40 MB: passa |
| Janela respondendo | **319 ms** | 4.383 ms | — |
| Abertura até a biblioteca interativa | **1,69 s** (0,96 s sem a animação de entrada) | — | ≤ 1,5 s: não passa por causa da entrada (~0,73 s); o mantenedor vai redesenhar a entrada |
| Quadros (rolagem, troca de jogo, vidro, perfil) | p95 **4,3 ms** (239 Hz) | — | passa nesta máquina; iGPU de referência não medida |
| Latência do teclado | p95 **< 16 ms** | — | ≤ 50 ms: passa |
| CPU parada | **0,98%** de um núcleo | 0,36% | ~0%: passa na prática |
| GPU parada | **0,00%** | 0,00% | passa (indício) |
| Volta do jogo (`game.ended` → biblioteca) | **314 ms** | — | ≤ 1 s: passa. O fim do jogo passou a ser detectado em < 0,5 s depois de o jogo ser visto rodando (WEB-60c); **remedir** |

**Decisões tomadas no caminho:** construir direto na fundação, sem protótipo (D-14); Windows primeiro (D-15);
testes só locais (D-13); sandbox do Chromium pelo modelo bootstrap oficial do CEF, junto do instalador (WEB-02c);
CEF definitivo (D-19); versões numeradas em vez de instalador de prévia (D-17); abandonar C# e Slint (D-16).

**Adiado pelo mantenedor (24/10):** importação do Playnite (WEB-31), teclado virtual (WEB-23), painel de desempenho
e modo seguro (WEB-37), configurações restantes (WEB-13c2: capas e metadados, controles, Sunshine completo,
backup). **Esperando design:** tela cheia / Big Picture (WEB-15).

### 2.3 Estado da transição: o que falta

**A transição não está completa.** O desktop Windows tem paridade, mas o Slint ainda é o único que faz a tela
cheia e as funções adiadas, e o código legado ainda está no repositório. O objetivo (D-16) é terminar com uma
interface só. Em ordem:

1. **Alpha 23** (em andamento, 25/10/2026): ícone do Tako na janela (WEB-70), ícones `.ico` da Steam na lista
   (WEB-71), remoção do C# (WEB-72) e pacote oficial com a web como padrão (INST-05). Fase 4, "Fechamento".
2. **Paridade restante e corte do Slint** (Fase 5): tela cheia web, as funções adiadas, validações em hardware e
   remoção da `tako-ui`. Ao fim, sai a beta (D-18).
3. **Instalador por componentes, sandbox e onboarding** (épico INST, antes da estável).
4. **Linux e macOS na web** (Fase 6, depois da alpha; D-20).

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
| Interface | React + TypeScript (Vite), desenhados por Chromium embutido (CEF 156, crate `cef` 156.2.0) | D-09 e D-19 (CEF em todos os SOs). Tipos da ponte gerados dos tipos Rust com ts-rs. Padrão no Windows a partir da 0.2.0-alpha.23; Slint só com `--slint` até o corte (D-16). |
| Instalador | Alphas: Inno Setup completo (núcleo + runtime do CEF + `web/dist`). Meta: bootstrapper próprio em Rust (`tako-installer`) | D-11: baixa e confere os componentes com SHA-256; pacote offline continua; portátil no Windows suspenso (D-07) |
| Controle | SDL3 (gamecontroller) | Banco de mapeamentos da comunidade, DualSense/Edge, Switch Pro, Xbox, 8BitDo; hotplug |
| Persistência | SQLite (`rusqlite`, SQLite embutido) | Esquema atual migrado (D-06), migrações versionadas |
| HTTP | `reqwest` (rustls) | Metadados, arte e atualizações |
| Imagens | `image` + cache próprio de texturas | Miniaturas no disco, decodificação sob demanda, LRU por bytes de GPU |
| Arte animada e vídeo | Fase posterior (libmpv opcional) | Fora da paridade; ver THEME/UX |
| Segredos | `keyring` (Credential Manager, Secret Service/libsecret, Keychain) | Substitui a DPAPI; migra os segredos antigos |
| Logs | `tracing` com arquivo rotativo e redação de dados sensíveis | Mesmas regras de redação do `LogRedactor` atual |
| Serialização | `serde` (JSON, YAML, TOML) | |
| Testes | `cargo test`, `insta` para snapshots; no `web/`, `tsc`, ESLint e `node --test` | Durante a migração, só locais (D-13). Os testes de UI do backend do Slint saem com o corte |
| CI | GitHub Actions: matriz Windows, Linux (x64 e arm64) e macOS (arm64 e x64) | Workflows de teste desligados durante a migração (D-13); `Release` e o aviso do CEF (`cef-check`) ativos |
| Empacotamento | Windows: Inno Setup (instalador; sem portátil desde a alpha.23). Linux: deb, AppImage, tar.gz, Flatpak, AUR. macOS: `.app` em dmg e zip | Linux e macOS pausados durante a alpha (D-20) |

---

## 5. Arquitetura

### 5.1 Workspace

| Crate | Responsabilidade |
|---|---|
| `tako-core` | Modelo de jogo, biblioteca, consulta e filtros, coleções, busca, deduplicação entre lojas, metadados e arte, navegação espacial, estado da sessão, persistência. **Sem dependência de UI nem de SO.** |
| `tako-sources` | Fontes de biblioteca (uma por loja ou launcher), cada uma atrás do trait `LibrarySource`, com `cfg` por SO |
| `tako-platform` | Traits `Displays`, `AudioOutputs`, `Cursor`, `PointerInjector`, `Secrets`, `Paths`, `ProcessLauncher` e uma implementação por SO (`windows`, `linux`, `macos`) |
| `tako-themes` | Formato de tema, validação, instalação e carregamento seguro (§7) |
| `tako-ui` | Telas Slint, foco, animações e ligação com o core. **Legado: sai no corte (WEB-50, D-16)**; até lá, só correções críticas (D-12) e serve de especificação de comportamento (§0). |
| `tako-app` | Camada de serviços sem interface: comandos, consultas e eventos do app (abrir jogo, editar, configurações, downloads, temas…), extraídos da `Shell` da `tako-ui`. Usada pela ponte web e testável sem tela. Gera os tipos TypeScript da ponte. |
| `tako-web` | Hospedeiro do Chromium: processo do núcleo (`--web`) e processo descartável da interface (`--web-ui`), janela com barra própria, ponte tipada React ↔ `tako-app`, esquemas próprios (`tako-app://` para o pacote da interface, `tako-img://` para imagens do cache), controles, bandeja, Central, seletor de arquivos |
| `web/` (fora do workspace Cargo) | Interface em React + TypeScript (Vite): design system, telas do desktop (e do Big Picture no WEB-15), foco e navegação por controle |
| `tako-installer` (futuro) | Instalador por componentes e onboarding de instalação (D-11, INST-01) |
| `tako` (binário) | Inicialização, supervisor, ciclo de vida da sessão, single-instance, atualização |

Regras:
- As dependências apontam para dentro: `tako` → `tako-web` → `tako-app` → `tako-core` (e `tako-ui` → `tako-core` até o corte); `tako-sources` e `tako-platform` dependem de `tako-core`, nunca o contrário.
- O React não guarda dado de domínio: tudo que precisa sobreviver a fechar a interface vive no Rust (D-10).
- Código específico de SO fica **só** em `tako-platform` e `tako-sources`, atrás de `cfg(target_os)`.
- Sem `unsafe` fora de `tako-platform`. Cada bloco `unsafe` tem comentário `// SAFETY:`.

### 5.2 Processos e ciclo de vida

- Um único executável, `tako`. O modo supervisor (mesmo binário) garante que tela e áudio sejam restaurados mesmo se a interface cair. A comunicação entre os dois é um socket local restrito ao usuário (named pipe no Windows, socket Unix nos demais).
- **Durante o jogo**, a interface esconde a janela, para o loop de renderização e libera texturas e caches de imagem. Ao voltar, recarrega o que estiver visível.
- **Com a interface web (D-10):** o núcleo Rust (`tako --web`, padrão no Windows desde a alpha.23) é o processo que fica; ele nunca carrega o Chromium. A interface é o processo filho `tako --web-ui`, com os processos do CEF (principal, GPU, renderização), ligado ao núcleo por pipes de stdin/stdout. Ela termina durante o jogo e na bandeja e reabre na tela onde o usuário estava; o estado que importa (tela, foco, filtros) é salvo no núcleo.
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

O app Rust só substitui o WPF quando tiver, **funcionando nas três plataformas** (salvo onde indicado). _Na interface web, durante a alpha, este escopo vale **só para o Windows** (D-15, D-20); a matriz por função está em [`docs/plans/web-paridade-windows.md`](plans/web-paridade-windows.md). Linux e macOS voltam a contar na Fase 6._

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

**Com a interface web (D-09, WEB-40):** o formato acima vale até o corte. Depois dele:
- As paletas (`tokens.toml`, THEME-003 e THEME-006) continuam valendo e viram variáveis CSS; quem só trocou cores não perde nada.
- Um tema passa a ser `theme.toml` + CSS + imagens, fontes e sons. **Sem JavaScript**: o pacote do tema roda sob uma CSP que bloqueia scripts e rede, e o CSS é validado antes de carregar.
- Layouts alternativos (como o Fita) viram **layouts oficiais** em React, que o tema escolhe e estiliza.
- Temas `.slint` de terceiros deixam de funcionar; o changelog da release do corte avisa.

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

### 8.5 Metas da migração para a interface web (D-09)

A troca só vale se provar ganho, e não só no visual. Toda meta é medida **duas vezes, com o mesmo método e nas
mesmas máquinas de referência**: primeiro a linha de base do Slint (MIG-W00), depois a interface web (WEB-60,
com o app completo). Os resultados ficam em
[`docs/plans/web-medicao-2026-10.md`](plans/web-medicao-2026-10.md); resumo na §2.2. **Ainda não medido:** iGPU
de referência, latência do controle, volta do jogo depois da detecção rápida do fim (WEB-60c), contagem exata de
quadros parados.

Memória = soma de **todos** os processos do Tako (núcleo, supervisor e os do Chromium), em memória privada
(Private Working Set no Windows, PSS no Linux, footprint no macOS). Cenário: biblioteca de 1.000 jogos, capas em
cache, 1080p.

| Dimensão | Métrica | Meta |
|---|---|---|
| **Design** | Texto durante animações (abrir menu, dropdown, diálogo, hover de card, troca de página) | Sem tremor nem mudança de nitidez, na avaliação do mantenedor em 1080p e 4K |
| | Fidelidade ao design system (`DESIGN.md`, tokens) | Checklist visual por tela aprovado pelo mantenedor |
| **Desempenho** | Quadros em **todas** as telas, inclusive as que hoje caem (vidro, desfoque, camadas) | p95 ≥ 58 fps num iGPU de referência (Steam Deck ou equivalente); nenhuma tela abaixo da linha de base |
| | Resposta ao controle e ao teclado | ≤ 50 ms no p95 (igual à §8.1) |
| | Abertura a frio até a tela interativa | ≤ 1,5 s no p95 (SSD), com o splash cobrindo a carga do Chromium |
| | Volta do jogo até a biblioteca (interface reabrindo, D-10) | ≤ 1 s no p95, no jogo e na posição de onde o usuário saiu |
| **Memória** | Interface aberta, em repouso | ≤ 250 MB. Relatar também a diferença para a linha de base do Slint; se a web passar dela, a diferença precisa caber nesta meta |
| | Minimizado ou na bandeja, depois do tempo de descarte | ≤ 40 MB (só o núcleo, sem processos do Chromium) |
| | Durante o jogo | ≤ 40 MB e **zero** processos do Chromium (igual à §8.1) |
| **CPU e GPU** | Interface aberta, parada, sem animação na tela | CPU ~0% (sem trabalho contínuo) e nenhum quadro desenhado |
| | Durante o jogo | Igual à §8.1: zero quadros, sem polling |
| **Funcionalidade** | Escopo da §6 | 100% da paridade no Windows antes do corte (WEB-50); Linux e macOS na Fase 6 (D-20) |
| | Ganhos que a web habilita | Trailers e arte animada (UX-002) e leitor de tela nas telas principais, entregues ou com caminho provado |
| **Conforto** | Tarefas principais pelo controle (achar um jogo, jogar, voltar, mudar uma configuração, trocar tema) | Mesmo número de passos ou menos que no Slint e sem nenhuma tarefa mais lenta, medido num roteiro em `docs/testing/` |
| | Instalação até o primeiro jogo aberto (D-11) | Instalador + onboarding sem nenhuma pergunta técnica; tempo registrado no roteiro de teste |
| **Tamanho** | Instalador pequeno (bootstrapper) | ≤ 10 MB |
| | Instalação completa | Registrada a cada release; sem meta rígida, porque o Chromium domina |

**Quando medir (D-14):** depois que o app web estiver de pé, não antes. A linha de base do Slint é opcional: se
ainda houver um build Slint à mão, mede-se com o mesmo método para ter o "antes e depois". Meta não cumprida vira
tarefa de otimização; se alguma for impossível de cumprir com o Chromium, os números voltam ao mantenedor (§0,
item 4).

---

## 9. Backlog

Legenda: **P0** = necessário para terminar a transição (interface web, corte do legado, beta); **P1** = logo depois; **P2/P3** = futuro.

**Ordem atual (25/10/2026):** Fase 4, "Fechamento" (alpha.23) → Fase 5 (paridade restante, validações, corte do
Slint, beta) → épico INST (antes da estável) → Fase 6 (Linux e macOS). Os épicos P1 a P3 abaixo só ganham tela
nova na web; nenhum recebe trabalho no Slint (D-12).

### Prioridade MoSCoW da rodada de 19/10

Ordem do que falta nesta rodada (decisão do mantenedor de usar MoSCoW). Fora daqui ficam os itens que dependem de hardware (MIG-*), o CMI-001 (no repositório do Console Mode) e os já fechados.

| | Itens | Por quê |
|---|---|---|
| **Must** | CLOUD-001, DL-001, EXT-002, SESSION-005, SRC-002, THEME-003 | já estão em PR ou em andamento; protegem dados do usuário (backup) e fecham a paridade no Linux e a base de integração (API local, que o CMI usa) |
| **Should** | THEME-007, THEME-008 (Fita), STREAM-001, CMI-002, THEME-006 | valor visível com pouco risco; CMI-002 é só detecção sobre a API local; THEME-006 reaproveita o THEME-003 |
| **Could** | CLOUD-002, DL-002, EXT-003, STREAM-002, THEME-005, UX-002, SOURCE-001, CMI-003, CMI-004, INPUT-002, SESSION-001, ART-002 | maiores ou dependentes de outro item, de ADR aceita (PR #135) ou do Console Mode |
| **Won't (agora)** | DATA-001, DEALS-001, ART-003, VOICE-001, THEME-011 | precisam de serviço externo, fonte de preços, servidor ou decisão ainda não tomada; os outros temas oficiais ficaram para depois do Fita |

A partir de 24/10, a Fase 4 (interface web) e o épico INST passam à frente. Os itens acima que ainda pedem tela
nova entregam a parte do núcleo agora e ganham tela já na versão web (D-12). THEME-011 depende do WEB-40.

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

- [x] **MIG-040 — Empacotamento Rust.** Adaptar o `release.yml` e os scripts (`build/`) para `cargo`: Windows (instalador e portable), Linux (deb com regra udev, AppImage, tar.gz; x64 e arm64), macOS (`.app`, dmg e zip; arm64 e x64). Os nomes de arquivo precisam continuar compatíveis com o atualizador das alphas C# (`Tako-Setup-*.exe`, `Tako-Portable-*.exe`), para que elas encontrem a versão Rust. _A partir da alpha.23 o Windows publica só o `Tako-Setup-*.exe` (INST-05, D-07)._
- [ ] **MIG-041 — Benchmark.** Medir as métricas da §8.1 no CI e nas máquinas de referência; publicar em `docs/`. _Estado: `tako --bench` e job `bench` no CI; faltam as máquinas de referência (`docs/marco-2/paridade.md`)._
- [x] **MIG-042 — Checklists de hardware** por SO, em `docs/testing/`.
- [x] **MIG-043 — Corte do C#.** Remover a solução C#/WPF do repositório e do CI. _Estado: antecipado pela D-16 e feito no WEB-72 (25/10/2026): saem `src/`, `tests/`, `tools/` C#, a solução, os `Directory.*.props`, o `global.json`, o pipeline WPF e as instruções de .NET._

### Fase 4 — Interface web no Windows (P0, D-09 a D-15)

Plano detalhado em [`docs/plans/2026-10-24-interface-web.md`](plans/2026-10-24-interface-web.md); quadro de
tarefas em [`docs/plans/web-quadro.md`](plans/web-quadro.md). Ordem (D-14): fundar, portar, medir e otimizar,
cortar. Processo de trabalho: [`docs/processo/multiagente.md`](processo/multiagente.md).

**Medição**
- [x] **MIG-W00 — Linha de base do Slint.** Memória, CPU, GPU e abertura do Slint medidas com o mesmo método no WEB-60 (§2.2). Quadros e latência do Slint não medidos (opcional, não bloqueia).
- [x] ~~**MIG-W01 — Comparação de motores no Windows.**~~ Cancelada pela D-19: CEF definitivo.
- [x] **WEB-60 — Medição.** Métricas da §8.5 no release (§2.2 e [`docs/plans/web-medicao-2026-10.md`](plans/web-medicao-2026-10.md)). Ajustes do WEB-60c: fim do jogo detectado em menos de meio segundo depois de o jogo ser visto rodando; primeiro quadro mais leve no Perfil. Pendências de medição na Fase 5 (VAL-02).

**Fundação**
- [x] **WEB-00 — Contrato da ponte:** consultas, comandos e eventos, formato, erros, versão, esquemas.
- [x] **WEB-01 — `tako-app`** (fatias 01a a 01e: boot, configurações e coleções, lojas, arte, downloads).
- [x] **WEB-02 — `tako-web` no Windows:** hospedeiro CEF com barra própria (02a) e interface em processo próprio (02b). Linux e macOS na Fase 6. Sandbox no WEB-02c (épico INST).
- [x] **WEB-03 — Projeto `web/`:** Vite + React + TypeScript, ESLint, build no pacote, hot reload do Vite dentro do Tako e DevTools no debug.
- [x] **WEB-04 — Design system na web:** componentes de `design/desktop/` (lotes 04a a 04n). _A validação visual do mantenedor segue como polimento contínuo (VAL-04)._
- [x] **WEB-05 — Foco e controle:** navegação espacial no React (05a) e SDL3 no núcleo pela ponte (05b). _Falta teste com controle real (VAL-01)._
- [x] **WEB-06 — Textos:** catálogos `locales/` servidos à interface, troca de idioma em tempo de execução, todas as telas migradas (06a, 06b).

**Porte das telas (desktop)**
- [x] **WEB-10 — Barra, janela e visão geral.**
- [x] **WEB-11 — Biblioteca** (lista, Grade, Categorias, busca, filtros, ordenação, coleções automáticas).
- [x] **WEB-12 — Página do jogo** (configurações do jogo, edição, adicionar jogo, fontes e IDs, arte, etiquetas, seletor de arquivo).
- [ ] **WEB-13 — Configurações.** Feitas 13a, 13b e 13c1. _Falta o WEB-13c2 (Fase 5)._
- [x] **WEB-14 — Camadas** (busca geral, downloads, atividades, perfil, diálogos).
- [x] **WEB-16 — Primeira execução** (cinco passos). _A entrada automática em tela cheia depende do WEB-15._
- [x] **WEB-20 a WEB-22 — Sessão:** jogar de verdade, "Abrindo…"/"Fechando o jogo…", estado jogando, Central rápida com atalho global.
- [x] **WEB-32 a WEB-36 — Núcleo na web:** metadados, atualizações, Discord, API local, `tako://`, abertura do app, bandeja.
- [x] **WEB-41 — Atualização do Chromium:** `scripts/cef-check.ps1`, workflow semanal que só avisa, procedimento em `docs/web/atualizar-cef.md`; CEF atualizado para 156.0.14 pelo próprio procedimento (WEB-41b).

**Fechamento: 0.2.0-alpha.23** (primeira alpha com a interface web, D-17)
- [x] **WEB-70 — Ícone do Tako na janela.** A janela do CEF mostrava o ícone do Chromium na barra de tarefas, no Alt+Tab e na barra de título, porque o hospedeiro não definia ícone. Aceite: ícone do Tako nos três lugares.
- [ ] **WEB-71 — Ícones `.ico` da Steam na lista.** Jogos como The Witcher 3, CONTROL Resonant e ARC Raiders ficavam sem ícone: o `.ico` da Steam traz uma imagem de 512×512 declarada como 256×256 e o decodificador recusava o arquivo inteiro. Aceite: leitura tolerante (a melhor entrada que decodificar), com teste do caso e de arquivo quebrado sem pânico.
- [x] **WEB-72 — Remover o C#** (MIG-043, D-16).
- [ ] **INST-05 — Pacote oficial com a web como padrão.** `Tako.exe` sem flag abre a interface web (inclusive pela inicialização com o Windows, `tako://` e instância única); `--slint` abre o legado; se o CEF falhar ao carregar, cai no Slint com log. `Package-Tako.ps1` e `tako.iss` levam o exe com a feature `web`, o runtime do CEF e o `web/dist`, com o mesmo AppId (atualiza a alpha.22 no lugar e mantém `%LOCALAPPDATA%\Tako`). Sem portátil; o atualizador de uma portátil antiga avisa e abre a página da release. O workflow `Release` monta o `web/` e o CEF. O instalador de prévia (INST-00, `Package-Web.ps1`, `tako-web.iss`) sai.
- [ ] **REL-23 — Publicar a 0.2.0-alpha.23.** PR da integração para a `develop` com aprovação do mantenedor; workflow `Release`, canal alpha; teste da atualização por cima da alpha.22 instalada (dados mantidos, abre na web, ícone certo). Changelog com a troca de interface, o fim do portátil e o `--slint`.

### Fase 5 — Paridade restante e corte do Slint (P0, D-16 e D-18)

Termina a transição: a web passa a fazer tudo o que só o Slint faz, o Slint sai e o app chega à beta. O
comportamento de cada item vem da `Shell` da `tako-ui` (§0); o visual, do `design/`.

**Paridade restante**
- [ ] **WEB-15 — Tela cheia (Big Picture).** Tema padrão da tela cheia, troca desktop ↔ tela cheia (F11, controle conectado, iniciar em tela cheia), "Abrindo…" em tela cheia, prévia de tema, isolar o controle do jogo com HidHide (INPUT-005) e entrada automática da primeira execução. _Espera o design system da tela cheia do mantenedor._
- [ ] **WEB-40 — Temas na web** (§7): paletas `tokens.toml` como variáveis CSS, CSS validado sem JavaScript (CSP), layouts oficiais (Fita), prévia, edição e criação de tema. Depende do WEB-15 para os layouts da tela cheia.
- [ ] **WEB-23 — Teclado virtual** para busca e campos pelo controle.
- [ ] **WEB-31 — Importação do Playnite.**
- [ ] **WEB-37 — Desempenho do Tako (PERF-001) e modo seguro / recuperação de tema.**
- [ ] **WEB-13c2 — Configurações restantes:** capas e metadados (chaves no cofre), controles (atalhos, mapeamento, HidHide), Sunshine com prévia e confirmação, backup e restauração.
- [ ] **WEB-14e — Downloads no núcleo:** gerenciar fontes e abrir a pasta.
- [ ] **WEB-38 — Lacunas menores da web:** "Voltar a jogar" na visão geral; reabrir com o perfil trocado; estado da API local e token nas Configurações; chave da bandeja nas Configurações; link `tako://` com a interface aberta traz a janela para frente em vez de fechar e reabrir (`window.raise`, `link.opened`); comandos antes e depois do jogo editáveis com consentimento.

**Validação (pelo mantenedor ou em hardware)**
- [ ] **VAL-01 — Jogo real com controle** pela interface web: abrir, Central por cima do jogo, voltar, restaurar tela e áudio; controle real (navegação, bateria, reconexão).
- [ ] **VAL-02 — Medições pendentes:** volta do jogo depois do WEB-60c, latência do controle, iGPU de referência (Steam Deck ou equivalente), primeiro quadro do Perfil no CEF real.
- [ ] **VAL-03 — Atualizador contra release real** (alpha.23 → alpha seguinte).
- [ ] **VAL-04 — Validação visual** de todas as telas contra as prévias do `design/desktop/` (polimento contínuo; não precisa ser perfeito para a beta, só sem erro visível).
- [ ] **VAL-05 — Checklists de hardware do Windows** (MIG-016, MIG-020 a MIG-024, INPUT-005) executados com a interface web.
- [ ] **VAL-06 — Discord Rich Presence** com o ID do aplicativo do mantenedor (`DISCORD_CLIENT_ID`).

**Corte**
- [ ] **WEB-50 — Corte do Slint.** Remover `crates/tako-ui`, os `.slint`, as dependências e a feature do Slint, o `--slint`, a queda para o Slint quando o CEF falha (vira mensagem de erro com caminho de reparo), o modo dev de hot reload do Slint e os testes de UI do Slint. Mover para a `tako-platform` o código de SO que a `tako-app` herdou do boot do Slint (DPAPI das alphas, `Vault`). Changelog avisa que os temas `.slint` deixam de funcionar. Aceite: paridade restante e validações acima concluídas; `git grep -i slint` só em docs históricos.
- [ ] **WEB-51 — Limpeza pós-corte:** docs operacionais (`README`, `CONTRIBUTING`, `docs/testing/`) sem Slint; ADRs do Slint marcados como históricos; religar os workflows de teste (D-13) para o Windows.

**Saída da alpha (D-18):** beta quando a Fase 5 estiver concluída, ou seja, o app funcional, com praticamente
todas as funções de um launcher validadas e uma interface só. A partir da beta o Tako passa a ter público.

**Otimizações (sem bloquear a beta, D-14):** abertura até a biblioteca ≤ 1,5 s com a entrada nova do mantenedor;
reabertura lenta da interface (WIP na branch `t3/investigate-slow-reopen`); CPU parada da fila de metadados (WIP
em `t3/metadata-queue-idle-cpu`).

### Fase 6 — Linux e macOS na interface web (P1, depois da alpha, D-20)

- [ ] **WEB-80 — Hospedeiro CEF no Linux:** X11, Wayland (KDE, GNOME, wlroots) e gamescope; x64 e arm64.
- [ ] **WEB-81 — Hospedeiro CEF no macOS:** `.app` com os processos auxiliares do CEF, assinatura e notarização (PLAT-004).
- [ ] **WEB-82 — Empacotamento:** deb, AppImage, tar.gz, Flatpak e AUR com o runtime do CEF; dmg e zip no macOS.
- [ ] **WEB-83 — Paridade e checklists** de cada SO (§6, `docs/testing/`), com as funções de plataforma da §5.3.

### Épico INST — Instalador e onboarding (P0 antes da primeira estável, D-11)

Desenho de telas e fluxo fica para depois, com o mantenedor; aqui ficam os requisitos.

- [x] ~~**INST-00 — Instalador de prévia da web.**~~ Descartado pela D-17; substituído pelo INST-05.
- [ ] **INST-01 — Instalador por componentes.** Instalador pequeno (≤ 10 MB) que baixa o núcleo, o runtime do Chromium e o pacote da interface do GitHub Releases, confere o SHA-256 de cada um, instala e abre o onboarding. Retoma downloads interrompidos e explica o erro sem termo técnico.
- [ ] **INST-02 — Atualização por componentes.** O atualizador (MIG-032) passa a atualizar cada componente separado: a interface (pequena e frequente) sem baixar o Chromium de novo. Mesmas regras de hoje: canal estável e de teste, SHA-256 conferido, nunca rebaixa.
- [ ] **INST-03 — Onboarding.** Fluxo contínuo do instalador à primeira tela: idioma, lojas detectadas, provedores de metadados, controle e preferências de abertura (a primeira execução do WEB-16 entra aqui), com desenho a definir.
- [ ] **INST-04 — Pacotes completos.** Continua existindo o instalador offline completo no Windows (o portátil fica suspenso, D-07). No Linux e no macOS (Fase 6), os pacotes levam tudo e o gerenciador de pacotes faz o papel do instalador; Flatpak não se autoatualiza.
- [ ] **WEB-02c — Sandbox do Chromium no Windows** pelo modelo bootstrap oficial do CEF (o executável da interface vira uma DLL carregada por `bootstrap.exe`; o crate `cef` não expõe o `sandbox_info` no Windows). Feito junto do INST-01, porque muda o empacotamento. Diagnóstico em `docs/web/hospedeiro.md`.

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
- [x] **UX-006:** atividades em segundo plano na barra de cima (AtividadeNaBarra).
- [x] **UX-007:** rebranding do desktop pelo novo design system (tako-design 188a48a).
  - [x] Tokens e temas
  - [x] Marca
  - [x] Barra de cima e janela
  - [x] Lista da biblioteca
  - [x] Visão geral
  - [x] Página do jogo (sai a DestaqueFases)
  - [x] Configurações
  - [x] Camadas
  - [x] Loja (fora: sem área de loja no app)
  - [x] Perfil e onboarding (se existirem)
  - [x] Movimento (sai a BarraLateralUnida e o vidro flutuante)

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

**CA-09 — Interface web descartável.** Com um jogo aberto, pela bandeja ou fechando para a bandeja, não sobra nenhum processo do Chromium e o Tako fica dentro da meta de memória; ao voltar, a interface reabre na tela de onde o usuário saiu.

**CA-10 — Uma interface só.** Depois do corte (WEB-50), o repositório não tem C#, Slint nem caminho alternativo de interface; o pacote não leva nada deles; `Tako.exe` abre sempre a interface web e, se o Chromium não carregar, mostra o erro com um caminho de reparo.

---

## 11. Riscos

| Risco | Impacto | Mitigação |
|---|---|---|
| Transição ficar pela metade, com duas interfaces mantidas para sempre | Alto | D-16: o corte do Slint é P0 e condição da beta (D-18); nada novo entra no Slint (D-12); a queda para o Slint quando o CEF falha também sai no corte |
| Usuários de Linux e macOS sem versão nova durante a alpha | Médio | A alpha não tem público (D-18); pacotes com Slint seguem até o corte; Fase 6 logo depois da alpha (D-20) |
| Funções que só o Slint tem (tela cheia, teclado virtual, Playnite, modo seguro) se perderem no corte | Alto | O corte só acontece com a paridade restante da Fase 5 concluída e validada; a `Shell` da `tako-ui` é a especificação até lá |
| Alpha.23 falhar ao atualizar a alpha.22 ou o CEF não carregar em alguma máquina | Alto | Mesmo AppId e teste de atualização por cima (REL-23); queda para o Slint com log enquanto ele existir |
| Wayland sem API para esconder cursor ou trocar resolução em todos os compositores | Médio | Matriz real no MIG-002; implementação por compositor; limitações mostradas ao usuário |
| macOS exigir permissões (Acessibilidade) que assustam o usuário | Médio | Pedir só ao ligar o cursor pelo controle, com explicação |
| Usuários das alphas perderem dados na troca | Alto | MIG-030 com testes sobre bancos reais; backup automático antes de migrar |
| Remoção de funções que estavam nas alphas (TV, FPS) frustrar usuários | Médio | Changelog claro e épico CMI para recuperá-las via Console Mode |
| Fim da compatibilidade com temas do Playnite | Médio | Tema padrão forte, tema alternativo, API documentada e estudo do conversor (THEME-010) |
| Mudanças nos formatos das lojas | Médio | Parsers isolados por fonte, testes com fixtures, falha de uma fonte não derruba as outras |
| Interface web não cumprir as metas de memória ou quadros | Médio | Medido no WEB-60: memória e quadros cumprem nesta máquina; falta o iGPU de referência (VAL-02); interface descartável (D-10) |
| Chromium embutido desatualizado (falha de segurança) | Alto | WEB-41: runtime atualizado em poucos dias a cada estável do CEF; atualização por componentes (INST-02) |
| Tamanho da instalação crescer com o Chromium | Médio | Instalador completo de ~143 MB nas alphas; instalador e atualização por componentes antes da estável (D-11, INST-01 e INST-02) |
| CEF instável no Wayland, gamescope, Linux arm64 ou macOS | Alto | Bindings já provados no Windows (crate `cef` 156); cada SO tem tarefa própria na Fase 6, com a lista do que não funcionar antes de publicar |
| Chromium sem sandbox no Windows | Médio | WEB-02c pelo modelo bootstrap do CEF antes da estável; a interface só carrega o pacote próprio (`tako-app://`), sem navegação externa |
| Temas `.slint` de terceiros pararem de funcionar | Baixo | Paletas mantidas, Fita como layout oficial, aviso no changelog (§7) |

---

## 12. Definition of Done

- Implementado, revisado e com testes automatizados onde aplicável (inclusive os portados do C#).
- CI verde nos três SOs. Durante a migração (D-13): fmt, clippy, testes e build verdes **localmente**, com a saída no relatório da tarefa.
- Funções de hardware com checklist por SO executado ou marcado como pendente na PR.
- Erros com tratamento e caminho de recuperação; nenhum `unwrap` em caminho de dado externo.
- Textos visíveis em português do Brasil e inglês.
- Changelog (`## [Unreleased]` nos dois arquivos) e documentação atualizados.
- Item marcado `[x]` neste PRD na mesma PR.

---

## 13. Referências

- Console Mode: https://github.com/lippdev/consolemode
- Slint: https://slint.dev e https://docs.slint.dev (até o corte, WEB-50)
- Chromium Embedded Framework: https://bitbucket.org/chromiumembedded/cef e bindings Rust: https://github.com/tauri-apps/cef-rs
- SDL3: https://wiki.libsdl.org/SDL3
- gamescope: https://github.com/ValveSoftware/gamescope
- Heroic Games Launcher: https://github.com/Heroic-Games-Launcher/HeroicGamesLauncher
- Lutris: https://github.com/lutris/lutris
- Protocolo `wlr-output-management`: https://wayland.app/protocols/wlr-output-management-unstable-v1
- Mutter `DisplayConfig`: https://gitlab.gnome.org/GNOME/mutter/-/blob/main/data/dbus-interfaces/org.gnome.Mutter.DisplayConfig.xml
- Microsoft — Game launchers and handhelds: https://learn.microsoft.com/en-us/xbox/gdk/docs/gdk-dev/pc-dev/handheld/handheld-launchers
- Código C# (removido no WEB-72): histórico do git antes de 25/10/2026
- Transição para a web: [plano](plans/2026-10-24-interface-web.md), [quadro](plans/web-quadro.md), [paridade no Windows](plans/web-paridade-windows.md), [medição](plans/web-medicao-2026-10.md), [ADR-0035](adr/0035-interface-web-rust-e-chromium.md)
