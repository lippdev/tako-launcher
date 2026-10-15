# Console Launcher — PRD & Product Backlog

**Versão:** 0.1 — Draft  
**Data:** 10/10/2026  
**Status:** Planejamento e validação técnica  
**Produto de origem:** Console Mode  
**Plataforma inicial:** Windows 11 x64  
**Estratégia:** Open source, arquitetura modular, gamepad-first

---

## 1. Visão do produto

Criar um launcher de jogos para Windows que ofereça uma experiência de console completa, controlável por joystick, com personalização visual por temas da comunidade, gerenciamento integrado de jogos e utilização das otimizações disponíveis no Windows Full Screen Experience (FSE).

O produto pretende eliminar a necessidade frequente de voltar ao desktop para tarefas relacionadas aos jogos.

**Proposta de valor:** jogar, gerenciar, configurar e organizar o PC pelo controle, preservando os recursos e a flexibilidade do Windows.

### 1.1 Objetivos

1. Construir um launcher independente do Console Mode, reutilizando seu runtime.
2. Integrar-se ao FSE pelos caminhos oficialmente suportados.
3. Executar normalmente em Fullscreen quando o FSE não estiver disponível.
4. Importar e renderizar temas Fullscreen compatíveis do Playnite 10.
5. Permitir navegação completa pelo controle.
6. Iniciar e encerrar jogos locais com segurança.
7. Manter o consumo de recursos baixo durante os jogos.
8. Preparar uma arquitetura extensível para downloads, lojas, navegador, arquivos, mods, upscaling e streaming.

### 1.2 Público-alvo

- Jogadores que utilizam o PC conectado a televisores.
- Usuários que desejam substituir a experiência de desktop por uma experiência de console.
- Proprietários de handhelds Windows.
- Usuários do Steam Big Picture, Playnite e Console Mode.
- Jogadores de títulos independentes, jogos standalone, emuladores e jogos modificados.
- Usuários avançados que desejam controlar gráficos, desempenho e ferramentas externas em uma única interface.

### 1.3 Princípios do produto

- **Controller-first:** todas as jornadas principais devem funcionar sem mouse.
- **Lean while gaming:** componentes desnecessários não devem permanecer consumindo recursos.
- **Modular:** cada integração deve poder ser instalada, removida ou desativada.
- **Recovery-first:** nenhuma falha pode deixar o usuário sem acesso ao desktop.
- **Compatibility-first:** oferecer caminhos alternativos quando APIs ou hardwares não forem compatíveis.
- **Offline-first:** a biblioteca local e os jogos não devem depender de serviços próprios na nuvem.
- **User-controlled:** ferramentas que modificam jogos ou sistema exigem consentimento e possibilidade de reversão.

---

## 2. Definição do MVP

O MVP deve validar três hipóteses:

**H1:** Um launcher independente consegue funcionar corretamente como experiência Fullscreen e, em ambientes compatíveis, utilizar o FSE do Windows.

**H2:** É possível renderizar temas Fullscreen do Playnite sem exigir que o Playnite esteja instalado ou em execução, por meio de uma camada de compatibilidade WPF.

**H3:** Um launcher que compartilha o runtime do Console Mode consegue iniciar jogos e recuperar a sessão sem comprometer a experiência existente.

### 2.1 Funcionalidades obrigatórias

| Funcionalidade | MVP |
|---|---|
| Launcher Fullscreen próprio | Obrigatório |
| Navegação com controle | Obrigatório |
| Biblioteca de jogos locais | Obrigatório |
| Importação manual de executáveis | Obrigatório |
| Tela de detalhes e botão Jogar | Obrigatório |
| Importação `.pthm` do Playnite 10 | Obrigatório |
| Tema nativo de recuperação | Obrigatório |
| Integração com Console Runtime | Obrigatório |
| Detecção de FSE | Obrigatório |
| Inicialização via FSE onde oficialmente suportado | Obrigatório validar |
| Fullscreen alternativo | Obrigatório |
| Recuperação de desktop e sessão | Obrigatório |
| Importação de lojas | Pós-MVP |
| Fontes JSON e downloads | Pós-MVP |
| Navegador e explorador avançado | Pós-MVP |
| OptiScaler, framegen e DLSS | Pós-MVP |

### 2.2 Fluxo principal do MVP

1. Usuário instala e abre o launcher.
2. Aplicativo verifica compatibilidade do ambiente com o FSE.
3. Usuário entra em uma interface Fullscreen.
4. Launcher apresenta biblioteca de jogos locais.
5. Usuário importa um tema Fullscreen do Playnite.
6. Interface aplica o tema sem exigir o Playnite.
7. Usuário navega com o controle e seleciona um jogo.
8. Runtime configura a sessão e inicia o executável.
9. Interface do launcher libera os recursos dispensáveis.
10. Quando o jogo termina, o launcher retorna e restaura a sessão conforme a configuração do usuário.

### 2.3 Fora do escopo inicial

O MVP não pretende substituir completamente Playnite, Steam ou Xbox; implementar uma loja; fornecer geração de quadros própria; substituir drivers gráficos; contornar DRM ou sistemas anti-cheat; nem oferecer compatibilidade universal com todos os temas da comunidade.

---

## 3. Stack tecnológica

| Responsabilidade | Tecnologia proposta | Justificativa |
|---|---|---|
| Linguagem principal | C# | Compatibilidade com o Console Mode e ecossistema WPF |
| Framework | .NET 10 LTS | Base atual com suporte prolongado |
| Interface principal | WPF / XAML | Reutilização de conceitos e recursos dos temas Playnite |
| Arquitetura de UI | MVVM | Separação entre apresentação e lógica |
| Runtime de sessões | C# / Win32 | Reaproveitamento do Console Mode |
| Código nativo opcional | C++20 | Casos em que APIs ou benchmarks justifiquem código nativo |
| Banco de dados | SQLite | Persistência local sem servidor |
| Acesso a dados | Microsoft.Data.Sqlite | Implementação leve e previsível |
| Comunicação entre processos | Named Pipes + JSON-RPC versionado | IPC local com contratos explícitos |
| Input | Serviços existentes + GameInput/XInput | Reutilização e evolução do suporte a controles |
| Gerenciamento de processos | APIs Windows / Process | Execução e acompanhamento de jogos |
| Temas | WPF ResourceDictionary + adaptadores | Compatibilidade gradual com Playnite |
| Metadados | Providers desacoplados | Suporte futuro a múltiplas origens |
| Downloads | HTTP client com retomada | Base para downloads futuros |
| Navegador futuro | WebView2 em processo sob demanda | Navegação com isolamento de ciclo de vida |
| Logs | Microsoft.Extensions.Logging | Observabilidade estruturada |
| Testes | xUnit + testes de UI no Windows | Cobertura automatizada e manual |
| CI/CD | GitHub Actions com Windows runner | Compilação e publicação |
| Distribuição | Instalador + versão portable | Alinhamento com Console Mode |

**Decisão:** não reescrever todo o Console Mode em C ou C++. Migrar componentes para código nativo somente com necessidade comprovada.

O Console Mode utiliza atualmente .NET 8, cujo suporte termina em novembro de 2026. A migração do runtime compartilhado para .NET 10 deve ter planejamento e testes próprios.

---

## 4. Arquitetura de software

### 4.1 Processos principais

**Launcher.Host**

Responsável pela inicialização, supervisão da UI, integração com o modo de execução escolhido e recuperação de falhas.

**Launcher.UI.Wpf**

Responsável pelas telas Fullscreen, foco, navegação, animações e composição visual dos temas.

**Launcher.Core**

Responsável pela biblioteca local, catálogo, metadados, perfis de jogos, comandos de execução e persistência.

**Launcher.Themes**

Responsável por descobrir, validar, instalar, aplicar e restaurar temas.

**Launcher.PlayniteCompat.P10**

Responsável por adaptar temas Playnite 10 ao contrato visual do launcher, implementando os componentes e bindings necessários.

**Console.Runtime**

Responsável pelo gerenciamento de sessões, controle de monitores, áudio, entrada de controle, estado do jogo, overlays e restauração de ambiente.

### 4.2 Reutilização de código

A estratégia será extrair serviços reutilizáveis do Console Mode para bibliotecas compartilhadas, evitando duplicação.

A interface WinUI 3 existente permanecerá independente da interface WPF do novo launcher.

A extração deve respeitar a licença AGPL-3.0-only do Console Mode. Integrações de código, distribuição e definição da licença do novo projeto passarão por revisão específica antes da publicação.

### 4.3 Contratos arquiteturais

- Nenhum tema deve acessar diretamente serviços privilegiados do runtime.
- A UI não deve ser responsável pela restauração de monitores.
- O runtime deve continuar operacional após falha da UI.
- Comunicação entre processos deve ser autenticada/restrita ao usuário local e possuir versionamento.
- Serviços opcionais só devem iniciar quando necessários.
- Todos os estados de uma sessão devem possuir caminhos de erro e recuperação.
- A biblioteca deve poder funcionar sem conexão à internet.
- O gerenciamento de jogos deve ser independente do mecanismo visual de temas.

### 4.4 Estados de sessão

`Idle → EnteringFullscreen → Browsing → LaunchingGame → Playing → Returning → Restoring → Browsing`

Estados excepcionais:

`Error → Recovery → DesktopFallback`

Transições devem ser idempotentes para evitar restaurações duplicadas, processos órfãos e inconsistências nas configurações de monitores.

---

# 5. Backlog P0 — MVP

Todas as histórias abaixo têm prioridade **P0** e devem ser consideradas candidatas ao marco de validação do produto.

## Épico MVP-A — Fundação técnica

- [ ] **MVP-001 — Criar estrutura do projeto.** Definir projetos Host, Core, UI, Themes e Runtime. Aceite: solução compila em CI Windows com dependências documentadas.
- [ ] **MVP-002 — Extrair contratos do Console Runtime.** Reutilizar interfaces de sessão, monitores e áudio. Aceite: integração funciona sem regressão nos fluxos do Console Mode.
- [ ] **MVP-003 — Implementar comunicação IPC.** Named Pipes com protocolo versionado. Aceite: Host e Runtime trocam comandos, respostas e eventos, inclusive após reconexão.
- [ ] **MVP-004 — Implementar máquina de estados de sessão.** Aceite: estados válidos são rastreados e transições inválidas não causam alterações no Windows.
- [ ] **MVP-005 — Implementar persistência SQLite.** Aceite: biblioteca e configurações sobrevivem a reinicializações e migrações de esquema.
- [ ] **MVP-006 — Criar supervisor de processos.** Aceite: falha da UI não impede encerramento ou recuperação segura da sessão.

## Épico MVP-B — Full Screen Experience

- [ ] **MVP-007 — Investigar integração oficial FSE.** Validar registro e inicialização do launcher como aplicativo principal em configurações suportadas. Aceite: matriz documentada com builds, dispositivos, permissões, procedimento reproduzível e limitações.
- [ ] **MVP-008 — Detectar suporte e estado do FSE.** Aceite: ambientes suportados, indisponíveis e indeterminados são diferenciados sem falsos positivos.
- [ ] **MVP-009 — Implementar host compatível com FSE.** Aceite: demonstrar inicialização e retorno em pelo menos um ambiente com suporte oficial confirmado.
- [ ] **MVP-010 — Implementar Fullscreen convencional.** Aceite: usuários sem FSE abrem e utilizam o launcher normalmente.
- [ ] **MVP-011 — Implementar fallback para desktop.** Aceite: erro no modo Fullscreen oferece recuperação e não deixa o usuário preso em uma tela vazia.
- [ ] **MVP-012 — Garantir compatibilidade com entrada e saída do modo console.** Aceite: monitores, resolução e áudio retornam aos estados registrados antes da sessão.

**Gate técnico:** não declarar compatibilidade nativa FSE em Windows comuns até comprovar suporte. O MVP poderá ser distribuído como prévia Fullscreen caso a integração oficial com FSE continue restrita.

## Épico MVP-C — Theme Engine Playnite

- [ ] **MVP-013 — Criar tema padrão próprio.** Aceite: todas as operações essenciais funcionam sem temas externos.
- [ ] **MVP-014 — Importar pacotes `.pthm`.** Aceite: usuário seleciona arquivo e o sistema identifica o pacote sem Playnite instalado.
- [ ] **MVP-015 — Validar `theme.yaml`.** Aceite: nome, versão, modo, identificador e ThemeApiVersion são interpretados; temas incompatíveis recebem diagnóstico.
- [ ] **MVP-016 — Implementar carregador XAML restrito.** Aceite: tipos e recursos permitidos são carregados; componentes perigosos ou desconhecidos são rejeitados antes da interpretação.
- [ ] **MVP-017 — Implementar recursos de tema e herança.** Aceite: estilos, recursos, mídias e arquivos parciais herdam os componentes padrão definidos pelo adaptador.
- [ ] **MVP-018 — Implementar adaptadores de dados.** Aceite: nomes, capas, biblioteca, seleção e comandos essenciais funcionam com os bindings suportados.
- [ ] **MVP-019 — Implementar controles `PART_` prioritários.** Aceite: componentes de seleção, lista e detalhes possuem comportamento equivalente ao subset documentado.
- [ ] **MVP-020 — Gerenciar temas instalados.** Aceite: listar, ativar, desativar, remover e restaurar tema padrão.
- [ ] **MVP-021 — Implementar fallback de renderização.** Aceite: falha de tema não impede a inicialização nem causa loop de crash.
- [ ] **MVP-022 — Homologar temas reais.** Aceite: três temas de teste, incluindo um tema parcial, com matriz explícita de recursos suportados e não suportados.

## Épico MVP-D — Biblioteca e jogos

- [ ] **MVP-023 — Criar biblioteca local.** Aceite: jogos cadastrados são apresentados em uma grade navegável.
- [ ] **MVP-024 — Importar executáveis manualmente.** Aceite: adicionar `.exe`, definir nome, pasta de trabalho e argumentos opcionais.
- [ ] **MVP-025 — Criar tela de detalhes.** Aceite: capa, nome, informações disponíveis e botão Jogar aparecem no tema padrão e no tema homologado.
- [ ] **MVP-026 — Executar jogos e reconhecer encerramento.** Aceite: jogo inicia corretamente e o launcher detecta o término por estratégia configurada.
- [ ] **MVP-027 — Retomar a biblioteca após jogar.** Aceite: foco e seleção anterior são recuperados; sessão e áudio permanecem consistentes.

## Épico MVP-E — Controle e usabilidade

- [ ] **MVP-028 — Navegação integral por joystick.** Aceite: fluxo Início → Biblioteca → Detalhes → Jogar funciona sem mouse e teclado.
- [ ] **MVP-029 — Implementar gerenciamento consistente de foco.** Aceite: nenhuma tela essencial fica sem elemento selecionável ou com foco inacessível.
- [ ] **MVP-030 — Implementar ações de voltar e menu.** Aceite: usuário consegue retornar, abrir opções e sair do launcher usando o controle, sem colisão com atalhos existentes do Console Mode.
- [ ] **MVP-031 — Tratar desconexão de controle.** Aceite: reconexão não exige reiniciar o launcher e teclado permanece como alternativa de recuperação.

## Épico MVP-F — Qualidade e distribuição

- [ ] **MVP-032 — Criar instalador e versão portable.** Aceite: instalação limpa em ambiente Windows suportado, sem exigir Playnite.
- [ ] **MVP-033 — Criar logs de diagnóstico.** Aceite: falhas de FSE, UI, temas e execução de jogos podem ser identificadas sem expor dados sensíveis.
- [ ] **MVP-034 — Criar testes automatizados e checklists de hardware.** Aceite: CI executa testes de lógica e build; casos de monitores, áudio e controle são verificados em Windows real.
- [ ] **MVP-035 — Construir benchmark inicial.** Aceite: medir consumo de CPU/RAM, tempo de entrada, fluidez da UI e overhead com jogo em execução.
- [ ] **MVP-036 — Criar mecanismo de recuperação de configuração.** Aceite: saída inesperada, erro de tema ou falha de processo não deixam o PC sem um caminho funcional de volta ao desktop.

---

# 6. Backlog P1 — Experiência completa de sofá

## Épico UX — Interface e navegação

- [ ] **UX-001:** busca de jogos por texto com teclado virtual.
- [ ] **UX-002:** filtros por biblioteca, gênero, favorito e instalado.
- [ ] **UX-003:** sistema de coleções personalizadas.
- [ ] **UX-004:** ordenação por último jogado, nome e tempo de jogo.
- [ ] **UX-005:** atalhos rápidos de ações por controle.
- [ ] **UX-006:** configurações completas dentro da interface Fullscreen.
- [ ] **UX-007:** editor de configurações de cada jogo.
- [ ] **UX-008:** suporte a múltiplas resoluções, DPI e proporções de tela.
- [ ] **UX-009:** configurações acessíveis de som, animação e contraste.
- [ ] **UX-010:** tela inicial com jogos recentes e função Continuar Jogando.

**Aceite do épico:** os principais recursos são acessíveis sem mouse, preservam foco corretamente e funcionam com diferentes tamanhos de tela.

## Épico DESK — Desktop controlado por joystick

- [ ] **DESK-001:** integrar ControlFS como gerenciador de arquivos.
- [ ] **DESK-002:** integrar JoyChromium ou navegador equivalente.
- [ ] **DESK-003:** oferecer cursor virtual pelo analógico.
- [ ] **DESK-004:** configurar ações de clique esquerdo, direito e scroll.
- [ ] **DESK-005:** criar teclado virtual contextual.
- [ ] **DESK-006:** alternar entre aplicativos e janelas abertas.
- [ ] **DESK-007:** executar instaladores com apoio do cursor virtual.
- [ ] **DESK-008:** oferecer seleção, cópia, movimentação, extração e renomeação de arquivos.
- [ ] **DESK-009:** adicionar assistente de configuração de jogos standalone.
- [ ] **DESK-010:** criar um menu rápido acessível sobre aplicativos compatíveis.
- [ ] **DESK-011:** permitir encerrar aplicativos travados com confirmação.
- [ ] **DESK-012:** criar regras para impedir que cursor virtual interfira no input do jogo.

**Aceite do épico:** completar um fluxo de download autorizado, extração, configuração e inicialização de jogo standalone usando apenas o controle, excetuando diálogos de segurança do Windows que exijam interação especial.

## Épico STORE — Plataformas de jogos

- [ ] **STORE-001:** importar jogos instalados da Steam.
- [ ] **STORE-002:** integrar jogos da Epic Games.
- [ ] **STORE-003:** integrar GOG Galaxy ou biblioteca GOG.
- [ ] **STORE-004:** identificar jogos do Xbox / Microsoft Store, respeitando restrições do pacote.
- [ ] **STORE-005:** integrar Ubisoft Connect.
- [ ] **STORE-006:** integrar EA App.
- [ ] **STORE-007:** integrar Battle.net.
- [ ] **STORE-008:** integrar Amazon Games.
- [ ] **STORE-009:** importar catálogos existentes do Playnite, mediante suporte e consentimento.
- [ ] **STORE-010:** unificar jogos duplicados sem perder a identidade da loja.
- [ ] **STORE-011:** abrir instaladores e clientes oficiais quando exigidos.
- [ ] **STORE-012:** detectar se um cliente de loja está ausente ou desatualizado.

**Aceite do épico:** um jogo de cada integração homologada pode ser detectado, apresentado e aberto sem contornar autenticação, permissões ou mecanismos oficiais da plataforma.

## Épico SOURCE — Catálogos JSON e downloads

- [ ] **SOURCE-001:** criar formato versionado de manifesto JSON de catálogo.
- [ ] **SOURCE-002:** permitir adicionar fontes via URL HTTPS.
- [ ] **SOURCE-003:** importar listas JSON locais.
- [ ] **SOURCE-004:** suportar adaptadores para formatos de catálogos comunitários, incluindo estudos de compatibilidade com o modelo de fontes do Hydra.
- [ ] **SOURCE-005:** normalizar identificadores, nomes, versões e provedores.
- [ ] **SOURCE-006:** implementar busca unificada entre múltiplas fontes.
- [ ] **SOURCE-007:** exibir origem, licença e informações de procedência.
- [ ] **SOURCE-008:** oferecer permissões por fonte e avisos para conteúdo não verificado.
- [ ] **SOURCE-009:** implementar downloads HTTPS com pause/resume.
- [ ] **SOURCE-010:** oferecer fila, progresso, velocidade e limite de banda.
- [ ] **SOURCE-011:** validar checksum e integridade quando disponíveis.
- [ ] **SOURCE-012:** verificar espaço livre antes do download.
- [ ] **SOURCE-013:** importar arquivos de jogos já baixados.
- [ ] **SOURCE-014:** integrar downloads BitTorrent opcionais de conteúdo autorizado.
- [ ] **SOURCE-015:** oferecer extração de arquivos compatíveis.
- [ ] **SOURCE-016:** criar sistema de quarentena e confirmação antes de executar arquivos baixados.
- [ ] **SOURCE-017:** integrar metadados e capas com provedores licenciados.
- [ ] **SOURCE-018:** tratar fontes inválidas, indisponíveis e links quebrados.
- [ ] **SOURCE-019:** implementar atualização e cache de catálogos.
- [ ] **SOURCE-020:** configurar pasta padrão de downloads.

**Aceite do épico:** importar fonte confiável, encontrar item, baixar arquivo autorizado, validar integridade, extrair quando necessário e registrá-lo na biblioteca. Nenhuma fonte deve possuir permissão implícita para executar código, instalar programas ou desativar recursos de segurança.

---

# 7. Backlog P2 — Performance, upscaling e ferramentas

## Épico SCALE — Upscaling e Frame Generation

- [ ] **SCALE-001:** identificar fabricante e modelo de GPU.
- [ ] **SCALE-002:** identificar driver gráfico e capacidades disponíveis.
- [ ] **SCALE-003:** adicionar detector de instalação e versão do OptiScaler.
- [ ] **SCALE-004:** estudar integração por instalador e perfis do OptiScaler.
- [ ] **SCALE-005:** criar interface de configuração por jogo para integrações de upscaling suportadas.
- [ ] **SCALE-006:** permitir backup e restauração dos arquivos de configuração alterados.
- [ ] **SCALE-007:** apresentar avisos de compatibilidade com anti-cheat e jogos online.
- [ ] **SCALE-008:** identificar suporte nativo do jogo a DLSS, FSR e XeSS quando houver dados confiáveis.
- [ ] **SCALE-009:** integrar configurações compatíveis com Frame Generation existentes.
- [ ] **SCALE-010:** investigar integração com ferramentas equivalentes ao Lossless Scaling, respeitando APIs, licenças e limitações.
- [ ] **SCALE-011:** adicionar informações e detecção de compatibilidade com DLSS 5.
- [ ] **SCALE-012:** estudar integração com controles oficiais de DLSS 5 quando expostos por jogo, driver ou API.
- [ ] **SCALE-013:** criar perfis por jogo de renderização, upscaling e geração de quadros.
- [ ] **SCALE-014:** comparar frame rate renderizado, frame rate apresentado e latência quando tecnicamente possível.
- [ ] **SCALE-015:** validar compatibilidade com HDR, VRR e diferentes modos de apresentação.

**Aceite do épico:** o usuário enxerga somente opções compatíveis e consegue ativar, desativar e restaurar alterações suportadas. Não haverá promessa de aplicar DLSS 5 indiscriminadamente a qualquer jogo.

## Épico PERF — Desempenho do sistema

- [ ] **PERF-001:** monitorar consumo de RAM e CPU do launcher.
- [ ] **PERF-002:** suspender ou encerrar a UI principal durante o jogo.
- [ ] **PERF-003:** minimizar polling e trabalho de background.
- [ ] **PERF-004:** implementar cache de imagens e lazy loading.
- [ ] **PERF-005:** disponibilizar contador de FPS próprio, sem injeção, pelo PresentMon (ADR-0017; substitui a integração com o RTSS).
- [ ] **PERF-006:** registrar FPS médio e 1% lows em benchmarks.
- [ ] **PERF-007:** estudar métricas de frametime e latência de apresentação.
- [ ] **PERF-008:** permitir limitar FPS nos cenários compatíveis.
- [ ] **PERF-009:** integrar perfis de resolução e taxa de atualização.
- [ ] **PERF-010:** expor controles HDR e VRR suportados.
- [ ] **PERF-011:** oferecer presets por jogo e monitor.
- [ ] **PERF-012:** estudar impacto do Windows Game Mode e FSE.
- [ ] **PERF-013:** monitorar processos auxiliares do launcher.
- [ ] **PERF-014:** realizar testes comparativos contra desktop, Steam e Playnite.
- [ ] **PERF-015:** implementar diagnóstico de gargalos de CPU, memória e GPU usando APIs disponíveis.

**Aceite do épico:** métricas são reproduzíveis em cenários controlados; nenhuma alteração é aplicada automaticamente sem indicação clara e reversão possível.

## Épico MOD — Mods e jogos standalone

- [ ] **MOD-001:** criar gerenciador de mods por jogo.
- [ ] **MOD-002:** suportar importação manual de pacotes.
- [ ] **MOD-003:** organizar estrutura de pastas e perfis de mods.
- [ ] **MOD-004:** realizar backup antes de alterações.
- [ ] **MOD-005:** permitir ativar/desativar grupos de mods.
- [ ] **MOD-006:** oferecer instalação orientada por manifestos.
- [ ] **MOD-007:** detectar conflitos de arquivos conhecidos.
- [ ] **MOD-008:** criar atalhos de executáveis alternativos.
- [ ] **MOD-009:** permitir parâmetros de inicialização por perfil.
- [ ] **MOD-010:** estudar integração com ferramentas comunitárias de modding autorizadas.

**Aceite do épico:** configurar um jogo standalone e alternar perfis sem perder os arquivos originais, quando o formato do mod permitir.

## Épico EMU — Emulação

- [ ] **EMU-001:** detectar emuladores instalados.
- [ ] **EMU-002:** cadastrar emuladores personalizados.
- [ ] **EMU-003:** importar bibliotecas locais de ROMs do usuário.
- [ ] **EMU-004:** configurar parâmetros de inicialização por emulador.
- [ ] **EMU-005:** integrar RetroArch.
- [ ] **EMU-006:** criar perfis de controle por emulador.
- [ ] **EMU-007:** organizar capas e metadados.
- [ ] **EMU-008:** configurar inicialização e retorno ao launcher.

**Aceite do épico:** executar conteúdo legalmente obtido com um emulador configurado, preservando o retorno à interface principal.

---

# 8. Backlog P3 — Ecossistema e funcionalidades futuras

## Épico CLOUD — Sincronização e perfis

- [ ] **CLOUD-001:** múltiplos perfis locais.
- [ ] **CLOUD-002:** sincronização opcional de configurações.
- [ ] **CLOUD-003:** backup de dados da biblioteca.
- [ ] **CLOUD-004:** backup seletivo de saves.
- [ ] **CLOUD-005:** sincronização por WebDAV/S3 compatível ou provedor escolhido.
- [ ] **CLOUD-006:** exportar e importar perfis.
- [ ] **CLOUD-007:** resolver conflitos de sincronização.
- [ ] **CLOUD-008:** modo convidado.

## Épico STREAM — Streaming e controle remoto

- [ ] **STREAM-001:** integração com Apollo/Sunshine.
- [ ] **STREAM-002:** integração ou atalhos para Moonlight.
- [ ] **STREAM-003:** descoberta de computadores da rede.
- [ ] **STREAM-004:** gerenciamento de sessões remotas.
- [ ] **STREAM-005:** suporte a controle encaminhado no streaming.
- [ ] **STREAM-006:** interface web local autenticada.
- [ ] **STREAM-007:** controle remoto por smartphone.
- [ ] **STREAM-008:** suporte a automações e comandos externos.
- [ ] **STREAM-009:** integração com dispositivos de TV compatíveis.
- [ ] **STREAM-010:** integração opcional com serviços de cloud gaming permitidos.

## Épico SOCIAL — Recursos de comunidade

- [ ] **SOCIAL-001:** tempo de jogo por título.
- [ ] **SOCIAL-002:** histórico de sessões.
- [ ] **SOCIAL-003:** integração opcional com Discord Rich Presence.
- [ ] **SOCIAL-004:** favoritos e recomendações locais.
- [ ] **SOCIAL-005:** importação de conquistas suportadas.
- [ ] **SOCIAL-006:** comparação local de desempenho entre sessões.
- [ ] **SOCIAL-007:** exportação de estatísticas.

## Épico THEME+ — Comunidade e personalização

- [ ] **THEME-001:** gerenciador avançado de temas.
- [ ] **THEME-002:** galeria comunitária de temas autorizados.
- [ ] **THEME-003:** importação por URL de repositório.
- [ ] **THEME-004:** atualização automática opcional de temas.
- [ ] **THEME-005:** suporte progressivo a temas do Playnite 11, se viável.
- [ ] **THEME-006:** formato próprio de temas.
- [ ] **THEME-007:** documentação pública do SDK de temas.
- [ ] **THEME-008:** pré-visualização de temas antes de aplicar.
- [ ] **THEME-009:** edição de cores, fontes e componentes.
- [ ] **THEME-010:** suporte a extensões visuais com permissões limitadas.
- [ ] **THEME-011:** ferramenta de compatibilidade e diagnósticos para autores.
- [ ] **THEME-012:** criador visual de temas, como hipótese futura.

## Épico EXT — Extensibilidade e automações

- [ ] **EXT-001:** SDK de plugins.
- [ ] **EXT-002:** eventos `GameStarting`, `GameStarted`, `GameExited`.
- [ ] **EXT-003:** hooks de sessão com permissões.
- [ ] **EXT-004:** API local de integração.
- [ ] **EXT-005:** plugins de metadados e catálogos.
- [ ] **EXT-006:** ferramentas externas configuráveis por jogo.
- [ ] **EXT-007:** automação de pré-lançamento e pós-encerramento.
- [ ] **EXT-008:** execução restrita e registro de ações de plugins.
- [ ] **EXT-009:** importação e exportação de configurações em formato aberto.
- [ ] **EXT-010:** integração opcional com assistentes de IA para orientação de configuração, sem alterações automáticas não autorizadas.

---

# 9. Critérios de aceite do MVP

## CA-01 — Instalação independente

**Dado** um computador Windows compatível sem Playnite instalado,  
**Quando** o usuário instalar o launcher,  
**Então** deve conseguir abrir a interface, cadastrar um executável e iniciar um jogo.

## CA-02 — Navegação com controle

**Dado** o launcher aberto em Fullscreen,  
**Quando** o usuário conectar um controle suportado,  
**Então** deve conseguir navegar pela biblioteca, abrir detalhes, iniciar um jogo e voltar sem mouse.

## CA-03 — Importação de tema

**Dado** um pacote `.pthm` pertencente ao conjunto homologado,  
**Quando** o usuário importar e ativar o tema,  
**Então** os componentes e recursos previstos na matriz de compatibilidade devem funcionar sem depender do Playnite instalado.

## CA-04 — Tema inválido

**Dado** um tema com pacote corrompido ou recursos incompatíveis,  
**Quando** o usuário tentar carregá-lo,  
**Então** o launcher deve rejeitar ou degradar a importação com segurança, manter logs e permitir usar o tema padrão.

## CA-05 — FSE suportado

**Dado** um ambiente onde a integração oficial com FSE e a seleção do launcher sejam permitidas,  
**Quando** o usuário entrar no modo console,  
**Então** o launcher deve iniciar pela integração homologada e manter navegação, execução e retorno funcionais.

## CA-06 — FSE indisponível

**Dado** um computador sem suporte à integração FSE pretendida,  
**Quando** o launcher iniciar,  
**Então** deve oferecer Fullscreen convencional e comunicar corretamente a ausência do recurso.

## CA-07 — Execução de jogo

**Dado** um jogo local cadastrado corretamente,  
**Quando** o usuário selecionar Jogar,  
**Então** o executável deve receber seus argumentos e diretório de trabalho; o launcher deve acompanhar a execução e liberar recursos dispensáveis.

## CA-08 — Retorno automático

**Dado** um jogo iniciado pelo launcher,  
**Quando** o processo monitorado terminar,  
**Então** o usuário deve retornar ao launcher ou desktop, conforme a preferência configurada, com áudio, monitores e foco consistentes.

## CA-09 — Falha na UI

**Dado** um tema que cause encerramento inesperado da interface,  
**Quando** a UI falhar,  
**Então** o Host deve detectar a falha e permitir reiniciar com tema padrão ou recuperar o desktop sem perder o estado crítico da sessão.

## CA-10 — Segurança

**Dado** um tema externo com tentativa de carregar componentes não autorizados,  
**Quando** a validação ocorrer,  
**Então** os componentes devem ser bloqueados antes de executar funcionalidades privilegiadas.

## CA-11 — Consumo de recursos

**Dado** um jogo aberto,  
**Quando** a interface principal não for necessária,  
**Então** ela deve estar suspensa ou encerrada conforme o projeto e o runtime deve permanecer dentro do orçamento de recursos homologado.

## CA-12 — Compatibilidade visual

**Dado** um dos temas homologados,  
**Quando** visualizado em resoluções de teste,  
**Então** os elementos essenciais devem ser exibidos corretamente, mantendo seleção e navegação sem clipping funcional.

---

# 10. Requisitos não funcionais

### Desempenho

Metas iniciais de engenharia, sujeitas à validação após benchmarks:

| Métrica | Meta inicial |
|---|---|
| Abertura da UI em SSD com biblioteca local | Até 4 segundos no p95 |
| Resposta perceptível à navegação | Até 100 ms no p95 |
| Memória do processo UI em cenário de referência | Até 300 MB em idle |
| Memória do runtime essencial em jogo | Alvo inicial de 100 MB |
| Integridade da sessão após falhas | Nenhuma configuração de monitor ou áudio permanentemente perdida em testes homologados |
| Jogo iniciado pelo launcher vs diretamente | Overhead mensurável e documentado; sem alegar ganho de FPS sem evidências |

O benchmark deve usar máquinas de referência definidas, biblioteca representativa e várias repetições. As metas de memória não incluem processos dos próprios jogos nem componentes do sistema operacional.

### Compatibilidade

- Windows 11 x64 como alvo inicial.
- Windows 10 como possibilidade futura em modo Fullscreen convencional.
- Controles Xbox e PlayStation conforme capacidades homologadas.
- Resoluções 1080p e 4K obrigatórias na matriz de testes.
- Cenários com múltiplos monitores, TV, HDR e diferentes escalas de DPI.
- Temas Playnite 10 dentro de uma matriz explícita de suporte.

### Segurança

- Não executar arquivos baixados automaticamente.
- Não desabilitar Windows Defender ou anti-cheat.
- Não aplicar injeções gráficas de forma silenciosa.
- Não permitir execução arbitrária de código em temas por padrão.
- Sanitizar caminhos e impedir path traversal na extração de pacotes.
- Restringir Named Pipes ao usuário/sessão local apropriados.
- Não armazenar credenciais de lojas em texto puro.
- Validar atualizações e pacotes sempre que possível.
- Tratar URLs e catálogos JSON como dados não confiáveis.
- Não fornecer fontes para distribuição não autorizada de jogos como conteúdo oficial do launcher.

### Qualidade

- Testes unitários do Core.
- Testes de contrato de IPC.
- Testes de importação e validação de temas.
- Testes de regressão de sessão.
- Testes manuais com monitores, controles e áudio reais.
- Logs de diagnóstico exportáveis.
- Política de atualização e rollback.
- Versionamento de configuração e banco de dados.
- Revisão de licenças de software, temas e bibliotecas distribuídas.

---

# 11. Matriz de riscos

| Risco | Impacto | Mitigação |
|---|---|---|
| FSE não permitir launcher de terceiros em PCs comuns | Crítico | Spike técnico imediato e fallback Fullscreen |
| Temas Playnite exigirem componentes específicos | Alto | Adaptador versionado e homologação por tema |
| XAML não confiável executar comportamentos indevidos | Crítico | Validação, restrição de tipos e processo com privilégios mínimos |
| Alterações do Windows quebrarem gerenciamento de sessão | Alto | Testes de regressão, restauração e matriz de builds |
| UI consumir recursos durante jogos | Alto | Separação de processos e encerramento sob demanda |
| Tema travar a experiência | Alto | Supervisor, safe mode e tema padrão |
| Integrações externas mudarem APIs | Médio | Providers independentes e contratos versionados |
| OptiScaler causar problemas em jogos online | Alto | Avisos, bloqueio conservador e habilitação manual |
| Downloads externos serem inseguros | Alto | Validação de origem, hash, consentimento e isolamento |
| Problemas de licenças AGPL/temas/plugins | Alto | Revisão e atribuição antes de distribuição |
| Complexidade de features atrasar o MVP | Alto | Backlog separado e definição rígida do P0 |

---

# 12. Roadmap por marcos

## Marco 0 — Proof of Concept

Entregáveis:
- Launcher WPF inicial.
- Um tema Playnite renderizado.
- Biblioteca fictícia.
- Navegação com controle.
- Spike oficial de integração FSE.
- Relatório de riscos de XAML e compatibilidade.

**Gate:** comprovar as hipóteses técnicas ou registrar claramente as limitações que exigirão mudança de escopo.

## Marco 1 — MVP Alpha

Entregáveis:
- Biblioteca real de executáveis.
- Tema próprio e importador `.pthm`.
- Integração com Console Runtime.
- Início e retorno de jogos.
- FSE homologado onde disponível.
- Fallback convencional e recuperação segura.

**Gate:** executar a jornada central de ponta a ponta.

## Marco 2 — Public Preview

Entregáveis:
- Ampliar compatibilidade de temas.
- Melhorar navegação e gerenciamento.
- Integrar ControlFS e navegador.
- Iniciar conectores de lojas e catálogos JSON.
- Implantar diagnósticos e documentação pública.

**Gate:** testes com usuários externos, sem bloqueadores críticos de recuperação.

## Marco 3 — Gaming Toolkit

Entregáveis:
- OptiScaler e ferramentas gráficas compatíveis.
- Perfis de jogo.
- Downloads e gerenciamento avançado.
- Ferramentas de mods.
- Diagnósticos de desempenho.

**Gate:** estabilidade, reversibilidade e compatibilidade documentada das integrações.

## Marco 4 — v1.0 estável

Entregáveis:
- Experiência refinada de sofá.
- Documentação pública.
- Atualizações confiáveis.
- Recursos e integrações homologadas.
- Programa de temas comunitários.
- Políticas de segurança e compatibilidade estabelecidas.

---

# 13. Definition of Done

Uma história só poderá ser marcada como concluída quando:

- Seu comportamento tiver sido implementado e revisado.
- Existirem testes automatizados onde aplicáveis.
- Recursos de hardware tiverem checklist de testes reais.
- Erros e estados excepcionais possuírem tratamento adequado.
- A mudança não introduzir regressão conhecida no Console Runtime.
- Strings visíveis forem localizadas conforme o escopo da versão.
- O comportamento estiver documentado.
- Segurança, permissões e compatibilidade forem revisadas.
- Critérios de aceite forem demonstrados em ambiente homologado.

---

# 14. Decisões arquiteturais iniciais

| ADR | Decisão | Status |
|---|---|---|
| ADR-001 | Launcher separado do Console Mode, com runtime compartilhado | Proposta |
| ADR-002 | WPF + .NET 10 para UI e compatibilidade temática | Proposta |
| ADR-003 | FSE oficial onde suportado, com fallback Fullscreen | Proposta |
| ADR-004 | Adaptador Playnite 10 versionado e compatibilidade incremental | Proposta |
| ADR-005 | Host, UI e runtime separados por processos | Proposta |
| ADR-006 | SQLite local e arquitetura offline-first | Proposta |
| ADR-007 | Fontes JSON como providers não confiáveis, com permissões explícitas | Proposta |
| ADR-008 | Ferramentas de upscaling por adaptadores opcionais | Proposta |
| ADR-009 | Não modificar jogos ou drivers automaticamente | Proposta |
| ADR-010 | Diagnósticos, recuperação e rollback como requisitos obrigatórios | Proposta |

---

# 15. Referências técnicas

- Console Mode: https://github.com/lippdev/consolemode
- Playnite: https://github.com/JosefNemec/Playnite
- Documentação de temas Playnite: https://api.playnite.link/docs/tutorials/themes/introduction.html
- Playnite — Manifestos: https://api.playnite.link/docs/tutorials/themes/manifestFile.html
- Microsoft — Game launchers and handhelds: https://learn.microsoft.com/en-us/xbox/gdk/docs/gdk-dev/pc-dev/handheld/handheld-launchers
- Microsoft — Configuração GamingHomeApp: https://learn.microsoft.com/en-us/windows-hardware/customize/desktop/unattend/microsoft-windows-gaming-configuration-gaminghomeapp
- OptiScaler: https://github.com/optiscaler/OptiScaler
- NVIDIA — DLSS 5: https://www.nvidia.com/en-us/geforce/news/dlss-5-3d-guided-neural-rendering/
- Hydra Launcher: https://github.com/hydralauncher/hydra
- Suporte .NET: https://dotnet.microsoft.com/pt-br/platform/support/policy

---

**Estado do documento:** backlog proposto, não comprometido. A seleção final e a ordem de implementação devem seguir a validação técnica do Marco 0, especialmente FSE, importação de temas e recuperação segura do Windows.