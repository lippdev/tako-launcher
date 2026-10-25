# Medição da versão web contra as metas da migração (WEB-60 e WEB-60b)

Data: 25/10/2026
Referência: PRD §8.5 ("Metas da migração para a interface web"), D-10, D-14 e ADR-0035.
Worktrees: feature/WEB-interface-web (WEB-60, t3/400cbaf2) e t3/c934ee33 (WEB-60b).

A WEB-60 mediu memória, processos e a abertura até a janela responder. A WEB-60b mediu o que tinha ficado de fora:
abertura até a biblioteca interativa, CPU e GPU em repouso, fps, latência de teclado e retorno depois do jogo. O que
não deu para medir está na §7, cada um com o motivo. Nada aqui foi estimado: onde não há número, está "não medido".

## 1. Ambiente e máquina de teste

- **Sistema operacional:** Microsoft Windows 11 Pro (build 10.0.26300) 64-bit
- **Processador (CPU):** AMD Ryzen 7 5700X 8-Core Processor (8 núcleos, 16 threads)
- **Memória RAM:** 32 GB
- **Placa de vídeo (GPU):** NVIDIA GeForce RTX 3070
- **Monitor onde a janela abriu:** 1920×1080 a **239 Hz** (o `requestAnimationFrame` mede intervalos de 4,2 ms)
- **Ambiente de build:** Rust 1.97.0 (edição 2024), Node v24.19.0, npm 11.17.0, CMake MSVC BuildTools 2022, CEF 156.0.14
  (WEB-60b; a WEB-60 usou 156.0.8), x86_64.
- **A máquina é compartilhada** com outros agentes que compilam e testam em paralelo. Antes de cada rodada o script
  espera o PC ficar abaixo de 20% de CPU (média de 3 s) e registra a carga vista; as rodadas finais tiveram 0,3% a
  19,7% e nenhuma passou do limite de espera (`rodadasComPcRuidoso: 0`). A WEB-60b só passou a ler essa carga depois
  de corrigir um nome de contador; rodadas de teste anteriores a isso não têm carga confiável e não entram nas tabelas.

## 2. Metodologia da WEB-60 (memória, processos, janela)

As medições foram executadas pelo script `scripts/measure-web.ps1` (métrica `memoria`, o comportamento de sempre), em
builds de **release**:
- **Build web:** `cargo build -p tako --release --features web`, antecedido pelo build do front (`web/dist` via
  `npm run build`) e pela configuração do runtime CEF com `scripts/cef-setup.ps1`.
- **Build Slint (linha de base):** `cargo build -p tako --release`, copiado para `tako-slint.exe`.

1. **Isolamento de dados:** cada rodada roda com uma pasta limpa (`--data-dir`), sem dados de usuário.
2. **Repetições e mediana:** 3 rodadas, mediana.
3. **Tempo de abertura:** do início do processo raiz até a janela existir e responder (`MainWindowHandle != 0` e
   `Responding`). **Não é** a biblioteca interativa: ver §5.1.
4. **Memória (Private Working Set):** soma de todos os processos da árvore (`Win32_PerfFormattedData_PerfProc_Process`,
   `WorkingSetPrivate`), além do Working Set total.
5. **Estados:** interface aberta em repouso (5 s depois da abertura) e núcleo sozinho (sem interface, durante o jogo).

Em pasta limpa o Tako lê as lojas do PC (aqui, 18 jogos da Steam), mostra o assistente da primeira vez e verifica e
baixa atualização em segundo plano; é um estado de "primeira abertura", não o de quem já usa o Tako.

## 3. Metodologia da WEB-60b

Tudo roda por `scripts/measure-web.ps1 -Metrics <métrica>` (ou `-Metrics tudo`). O script não compila Rust; os
pré-requisitos estão no cabeçalho dele. Código de página só existe em `web/src/dev/medicao/` e o `npm run build` normal
não o importa (conferido: nenhum `__medicao` nem marcador em `web/dist`).

### 3.1 A bancada

- **Biblioteca de medição.** Os 18 jogos da Steam do PC mais 300 cópias de um executável de teste ("Jogo 001" a "Jogo
  300"), adicionadas pela própria interface (`library.add_game`), com o assistente da primeira vez concluído e a busca de
  capas desligada. São 318 jogos, **sem capas** (a meta do PRD fala em 1.000 jogos com capas em cache). Cada rodada
  usa uma cópia da pasta de dados, sem os logs e sem o cache do CEF, mas com o cache de metadados.
- **Fila de metadados assentada.** O núcleo consulta metadados jogo a jogo (a cada ~1,8 s) e cada resposta dispara
  `library.changed`, que a interface redesenha. Com a fila ligada o repouso não é repouso (§6, achado 5). Por isso a
  semente roda uma vez até a fila esvaziar (585 s para 318 jogos) e o resultado vai no cache copiado.
- **Dois executáveis web.** (a) **Release de verdade** (`target\release\tako.exe` + `web\dist`): CPU, GPU, memória.
  Nele não há código de medição na interface. (b) **Executável "de medição"**: o mesmo release otimizado, mas com
  asserções de debug ligadas (`CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true`, outro `CARGO_TARGET_DIR`). Só assim o
  hospedeiro aceita `TAKO_WEB_DEV_URL` (em release de verdade ele é ignorado, por segurança). Ele carrega o **pacote de
  produção do app** (o mesmo JS e CSS do release) mais `web/src/dev/medicao/`, servido por HTTP em loopback na porta
  5199. Diferenças para o release real: a origem é `http://127.0.0.1:5199` e não `tako-app://app`, e há asserções de
  debug no Rust. Fica de fora (e é dito em cada número): o CSP e a leitura de arquivos do `tako-app://`.
- **Coletor.** A página manda eventos por `POST /__medicao` ao servidor local, que grava uma linha JSON por evento.
  Os horários são `performance.timeOrigin + performance.now()`, o mesmo relógio (ms desde 1970) que o script usa.
- **Teclado e mouse reais.** Quando a medição precisa de entrada, o script traz a janela para a frente (e confere) e
  manda teclas e cliques pelo sistema (`keybd_event`, `mouse_event`). Nunca clica fora da janela do Tako.

### 3.2 Como cada meta foi medida

| Meta | Método | O que o número não cobre |
|---|---|---|
| Abertura até a biblioteca interativa | `performance.mark` e um `MutationObserver`: a lista (`role=listbox`) tem itens, não está carregando, aceita foco (`focus()` e `activeElement`) e o primeiro item é o elemento do topo naquele ponto (a entrada do app, o polvo de ~1 s, não o cobre). O script marca o início logo antes de abrir o processo. 10 e 20 rodadas, cada uma com pasta nova. | Executável de medição (§3.1); biblioteca de 318 jogos sem capas; CEF com cache frio (cache do CEF apagado a cada rodada). |
| CPU em repouso | `TotalProcessorTime` de cada processo da árvore (núcleo, host da interface, GPU, renderers, utilities) no começo e no fim de 30 s, sem ninguém mexer; em % de um núcleo e % do PC (16 threads). O repouso só começa depois de o Tako assentar (CPU < 3% de um núcleo em 3 janelas de 2 s). Slint idem, com a mesma biblioteca. | Processos que nascem e morrem dentro da janela (não aparecem). |
| GPU em repouso | Contadores do Windows `GPU Engine` (`Utilization Percentage`) de cada PID do Tako, 1 amostra por segundo durante 30 s, somando os motores. | Não é uma contagem de quadros; o DWM (`dwm.exe`) compõe a janela fora destes PIDs. |
| "Nenhum quadro desenhado" em repouso | Indício, não contagem: na página, número de pedidos de `requestAnimationFrame` do app, animações CSS em curso (`document.getAnimations()`, a cada 250 ms), mutações do DOM e tarefas longas em 30 s. Sem laço próprio de quadros (isso forçaria quadros). | O número exato de quadros que o Chromium desenhou: exigiria tracing ou DevTools, que o release não expõe e o debug só abre à mão (§7). |
| fps | Laço de `requestAnimationFrame` durante o cenário; p50, p95, p99 e máximo do tempo entre quadros, quadros acima de 17,24 ms (1000/58) e "perdidos" (> 25,9 ms). Cenários: rolagem suave da biblioteca (percorreu 5.600 px de 5.280), troca de jogo (8 cliques, 8 páginas abertas), busca geral com vidro (Ctrl+K, 4 aberturas, 4 confirmadas) e vista Perfil (3 aberturas, 3 confirmadas). Cada cenário confirma na própria página que a ação aconteceu. 3 rodadas. | Cadência do navegador, não a varredura do monitor; PC com 239 Hz, RTX 3070 (§7). |
| Latência de teclado | 100 teclas (setas) pelo sistema, uma a cada 200 ms, na lista. A página mede com Event Timing (`duration`, do `timeStamp` do evento até a pintura seguinte, em passos de 8 ms, só eventos acima de 16 ms aparecem), o atraso de entrada e o tempo até o primeiro quadro depois do evento. | Varredura do monitor e controle (§7). |
| Retorno depois do jogo | Um jogo de teste (janela comum que fecha sozinha 16 s depois e anota o instante da saída). A interface o abre pelo `game.play`; o núcleo fecha a interface quando a janela do jogo aparece, acompanha o jogo e reabre a interface. Horários: saída do jogo (o próprio jogo), `game.ended` (linha "jogo terminou" do log do núcleo, ms), interface aberta (log) e página (marcos acima). 5 rodadas. | Executável de medição; o jogo de teste é leve. |

## 4. Tabela consolidada de resultados

| Dimensão | Métrica | Meta PRD §8.5 | Web (CEF + React) | Slint (linha de base) | Situação |
|---|---|---|---|---|---|
| **Abertura** | Até a janela responder (release real, pasta limpa) | (parte da meta abaixo) | **319 ms** (WEB-60); **329 ms** no reteste final (346 e 535 ms em invocações anteriores, mediana de 3 cada) | 4.383 ms (WEB-60) | Documentado |
| **Abertura** | Até a **biblioteca interativa** | ≤ 1,5 s no p95 (SSD) | **1.691 ms** de mediana, p95 1.709 ms (20 rodadas). Sem a entrada do app (lista na tela): 959 ms, p95 983 ms. Outras três invocações de 10 rodadas: medianas 1.503, 1.904 e 2.183 ms, p95 1.623, 3.526 e 2.246 ms | não medido (§7) | **Não passa com a entrada do app** (~0,73 s em cima); passa sem ela. Ver §5.1 |
| **Memória** | Interface aberta em repouso (Private WS, somado) | ≤ 250 MB | **97,6 MB** (WEB-60, pasta limpa); **132,3 MB** no reteste de pasta limpa; **102 a 145 MB** com a biblioteca de 318 jogos (3 invocações, 7 processos) | 325,5 MB (WEB-60); 335 a 339 MB com a biblioteca de teste | **PASSOU** |
| **Memória** | Interface aberta em repouso (Working Set total) | Registro | 385 MB (WEB-60); 379 a 441 MB | 383 MB; 402 a 406 MB | Equivalente |
| **Memória** | Sem interface / durante o jogo (Private WS) | ≤ 40 MB e zero Chromium | **9,4 MB** (WEB-60); **9,5 MB** no reteste, 1 processo, zero Chromium | 2,27 MB (supervisor) | **PASSOU** |
| **Processos** | Contagem na interface aberta / durante o jogo | Informativo / 0 do Chromium | 7 processos / 1 processo (só o núcleo) | 2 / 1 | **PASSOU** |
| **CPU em repouso** | Interface aberta, parada (Visão geral), 30 s | ~0%, sem trabalho contínuo | **0,98%** de um núcleo (0,06% do PC) de mediana de 3 invocações; faixa 0,73 a 1,19% | **0,36%** de um núcleo; faixa 0,21 a 0,52% | **Passa na prática, mas não é zero**: há trabalho residual (§6, achado 6) |
| **CPU em repouso** | Interface aberta, parada, na página de um jogo | ~0%, sem trabalho contínuo | **1,61%** de um núcleo (mediana); faixa 0,83 a 2,23% | não medido (§7) | Idem |
| **GPU em repouso** | Utilização dos motores da GPU, 30 s | Nenhum quadro desenhado | **0,00%** (média por segundo; 0 a 0,004%; máximo de 1 s: 0,11%, e 5,4% numa rodada da página do jogo) | 0,00% | **Passa (indício)** |
| **GPU em repouso** | Quadros desenhados sem animação | Nenhum | Página: 0 pedidos de quadro do app, 0 animações, 0 mutações do DOM em 30 s nos dois estados (fila de metadados assentada). Contagem exata de quadros: **não medida** | não medido | Indício favorável; **contagem não medida** |
| **Fluidez** | Tempo de quadro em 4 cenários (rolagem, troca de jogo, vidro na busca, vista Perfil), 3 rodadas | p95 ≥ 58 fps (iGPU de referência) | **p95 4,3 ms** em todos (cadência do monitor de 239 Hz), 0 a 2 quadros acima de 17,24 ms por cenário, pior quadro **41,7 ms** (vista Perfil, em todas as rodadas) | não medido (§7) | **Passa nesta máquina**; iGPU de referência e linha de base do Slint **não medidas** |
| **Latência** | Teclado (setas na lista), 100 teclas | ≤ 50 ms no p95 | **p95 abaixo de 16 ms** (Event Timing; 2 de 100 eventos em 16 ms, 0 acima de 50 ms); atraso de entrada p95 5,9 ms; até o quadro seguinte p95 1,1 ms | não medido | **Passa (teclado)** |
| **Latência** | Controle | ≤ 50 ms no p95 | **não medido**: exige um controle físico ou virtual | não medido | **Não medido** (§7) |
| **Retorno** | `game.ended` até a biblioteca interativa | ≤ 1 s no p95 | **309 a 378 ms** (5 rodadas; a rodada mais lenta, 378 ms, é o p95 de 5) | não medido | **PASSOU** |
| **Retorno** | Saída do jogo até a biblioteca interativa | ≤ 1 s no p95 (se a meta for contada da saída) | **2,8 a 5,4 s** (o núcleo só dá o jogo por terminado 2,5 a 5,0 s depois da saída) | não medido | **Não passa, se a meta for contada da saída**: decisão do mantenedor (§6, achado 2) |
| **Tamanho** | Executável principal compilado | Informativo | 44,1 MB (`tako.exe`) | 41,8 MB (`tako-slint.exe`) | Documentado (WEB-60) |

## 5. Resultados da WEB-60b

### 5.1 Abertura até a biblioteca interativa

Marcos a partir do início do processo (executável de medição, 20 rodadas, pasta nova a cada rodada, PC entre 0,7% e
19,7% de CPU antes de cada uma): a página carrega em **697 ms** (mediana; min 686, max 722), a lista aparece em **959 ms**
(p95 983) e fica livre, sem a entrada do app por cima, em **1.691 ms** (p95 1.709, max 1.713). Do começo da navegação
da página até a lista interativa são 1.130 ms.

A diferença entre "na tela" e "livre" é de 731 ms: a `EntradaDoApp` (800 ms de permanência e 200 ms de saída, pulável
por clique ou tecla) cobre a lista, que já está montada por baixo. Sem ela a abertura passa da meta com folga; com ela
não. Quem decide se a meta conta a entrada é o mantenedor.

Variação entre invocações: três invocações anteriores de 10 rodadas deram medianas de 1.503, 1.904 e 2.183 ms (p95
1.623, 3.526 e 2.246 ms). A invocação de 1.904 ms teve duas rodadas lentas (a lista só apareceu depois de 1,9 s). O
melhor p95 que vi foi 1.623 ms, acima de 1,5 s. Com 10 rodadas o p95 é o próprio máximo.

### 5.2 CPU e GPU em repouso

Três invocações do script (`-Metrics cpu-gpu`, 30 s por janela, Tako assentado em 10 a 17 s). A mediana é das três.

| | Web (release real) | Slint (release) |
|---|---|---|
| CPU, Visão geral | 0,73 / 0,98 / 1,19% de um núcleo (mediana 0,98%, 0,06% do PC) | 0,21 / 0,36 / 0,52% (mediana 0,36%) |
| CPU, página de um jogo | 1,61 / 2,23 / 0,83% | não medido |
| GPU, média por segundo | 0 / 0,003 / 0,004% (máx de 1 s: 0,11%) | 0 / 0 / 0% |
| GPU, página de um jogo | 0,179% (um pico de 5,4% em 1 s) / 0 / 0% | não medido |
| Memória privada, somada | 102 / 144 / 145 MB (7 processos) | 339 / 338 / 335 MB (2 processos) |

Por processo, na Visão geral da primeira invocação (% de um núcleo): núcleo Rust 0,31, host da interface 0,21, renderer
0,16, GPU 0,05, utility 0. O núcleo sozinho gasta de 0,31 a 0,57% na Visão geral e de 0,31 a 1,24% na página do jogo.

O estado "página de um jogo" foi aberto com um clique de mouse do sistema no primeiro item da lista. No executável de
medição a página confirma que o jogo abriu (`clique-recebido`, na primeira tentativa nas três invocações); no release
real não há como confirmar, porque não há código de medição nele. O clique usa a mesma posição e o mesmo procedimento.

Com a página de medição (pacote de produção, executável de medição) o mesmo repouso deu 1,19 a 1,66% de um núcleo na
Visão geral e 1,24 a 1,45% na página do jogo, GPU ~0%, e na página: **0 pedidos de quadro do app, 0 animações CSS em
curso, 0 mutações do DOM e 0 tarefas longas em 30 s** nos dois estados. Parte desse CPU a mais é o próprio observador
da página.

### 5.3 Fluidez (fps)

Três rodadas, 4 cenários cada (executável de medição, pacote de produção). Monitor de 239 Hz; todos os cenários
rodaram no ritmo do monitor.

| Cenário | Quadros por rodada | p50 | p95 | p99 | Pior quadro | Acima de 17,24 ms | Perdidos (> 25,9 ms) | fps médio |
|---|---|---|---|---|---|---|---|---|
| Rolagem da biblioteca (6 s) | ~1.440 | 4,2 ms | 4,3 ms | 4,3 ms | 4,4 ms | 0 | 0 | 239,9 |
| Troca de jogo (8,5 s) | ~2.028 | 4,2 ms | 4,3 ms | 4,3 ms | 20,8 a 25,0 ms | 1 | 0 | 238,5 |
| Vidro: busca geral (8 s) | ~1.909 | 4,2 ms | 4,3 ms | 4,3 ms | 24,9 a 37,5 ms | 1 | 0 a 1 | 238,4 |
| Vista Perfil (6 s) | ~1.424 | 4,2 ms | 4,3 ms | 4,3 ms | **41,6 a 41,7 ms** | 2 | 1 | 237,1 |

O p95 (4,3 ms) é a cadência do monitor, muito abaixo dos 17,24 ms da meta. O que sobra são quadros isolados: 1 a 2 por
cenário acima de 17,24 ms e, na vista Perfil, **um quadro de ~42 ms em todas as rodadas** (a abertura da vista).

### 5.4 Latência de teclado

100 teclas (setas na lista, uma a cada 200 ms, com a janela na frente): 2 eventos em 16 ms (o limiar do Event
Timing), nenhum acima de 50 ms. **p95 abaixo de 16 ms.** Atraso de entrada (do evento ao início do tratamento) p95 de
5,9 ms; até o primeiro quadro depois do evento, p50 0,9 ms, p95 1,1 ms, máximo 7,8 ms. A medida vai até a pintura da
página; não inclui a varredura do monitor (no máximo mais um quadro: 4,2 ms a 239 Hz, 16,7 ms a 60 Hz).

### 5.5 Retorno depois do jogo

Cinco rodadas, jogo de teste de 16 s. Medianas (e faixa):

| Trecho | Tempo |
|---|---|
| Saída do jogo → `game.ended` (núcleo percebe o fim) | 2.491 ms (2.484 a 2.980; em rodadas anteriores também 4.974 a 4.992) |
| `game.ended` → processo da interface aberto | 9 ms |
| Processo da interface aberto → página carregada | 200 ms (194 a 260) |
| Página carregada → biblioteca na tela / interativa | 104 / 106 ms |
| **`game.ended` → biblioteca interativa** | **314 ms (309 a 378)** |
| **Saída do jogo → biblioteca interativa** | **2.870 ms (2.797 a 3.294)**; 5.281 a 5.363 ms em rodadas anteriores |
| Fechar a interface antiga (quando a janela do jogo aparece) | 234 ms (218 a 236) |

A página reaberta recebe o `game.ended` guardado 233 a 328 ms depois do evento e mostra a página do jogo de onde o
usuário saiu (o estado de tela volta do núcleo). Em 25 rodadas de teste ao todo, uma teve a reabertura em 10,6 s (§6,
achado 3).

## 6. Achados

1. **A entrada do app é metade da abertura.** São ~0,73 s dos 1,69 s até a biblioteca interativa, e a lista já está
   montada por baixo. Decisão do mantenedor: contar ou não a entrada, encurtá-la, ou liberar a lista por baixo dela.
2. **O retorno só é rápido depois de o núcleo perceber o fim do jogo.** O rastreador de sessão (`tako-core`, `GameRunTracker`)
   espera 4 s depois de o processo sumir (`DEFAULT_EXIT_GRACE`, para o caso de launcher que passa o jogo adiante) e consulta
   a cada 2 s (`GameSession::POLL`): daí 2,5 a 5,0 s entre a saída do jogo e o `game.ended`. A reabertura em si (309 a
   378 ms) cabe com folga na meta de 1 s. Se a meta do PRD conta da saída do jogo, ela não passa e a causa não é a
   interface web. O Slint usa o mesmo rastreador (não medi).
3. **Uma rodada de teste teve a reabertura em 10,6 s** (de 25): a interface antiga não saiu dentro do prazo de 5 s depois
   do pedido, o núcleo registrou "a interface não fechou no prazo; matando", e só deu o processo por encerrado cerca de
   25 s depois do pedido. Foi a primeira rodada de teste, antes de o script ter a sonda de carga; nas outras rodadas medidas a
   interface antiga fechou em 218 a 303 ms. Não reproduzi e não sei a causa.
4. **Abrir a vista Perfil custa um quadro de ~42 ms** em todas as rodadas (a vista tem blocos de vidro). É o pior
   quadro medido e o único que passa de 2 quadros de 60 Hz.
5. **A fila de metadados mexe no repouso.** Com a fila ainda trabalhando (318 jogos, uma consulta a cada ~1,8 s, cada
   resposta emite `library.changed`), o mesmo repouso mediu 1,7 e 2,5% de um núcleo no release real e 2,1 a 4,3% com a página de
   medição (contra 0,7 a 1,2% assentado), e a página teve 246 mutações do DOM em 30 s (as maiores: os atributos `name` e `type` de um `input`, 140 e 70 vezes, e o texto de um
   contador, 36 vezes). Assentada, a página fica em zero mutações. Vale conferir por que um `input` tem atributos
   reescritos a cada atualização da biblioteca. O Slint, na mesma situação, ficou em 5,4 e 5,45% de um núcleo (duas invocações), e em 0,2 a 0,5% assentado. Sem chave do
   IGDB, a pasta limpa registrou um WARN a cada ~0,5 s ("Twitch respondeu 400 … InvalidClientId") enquanto havia jogos na
   fila.
6. **O núcleo web gasta 0,3 a 0,6% de um núcleo parado** (até 1,2% com a página de um jogo aberta). Pelo código, o laço do núcleo acorda a cada 250 ms
   (`Timings::tick`) para conferir a interface; não medi se é essa a causa. Para a meta "~0%" é pouco, mas não é zero.
7. **A primeira abertura baixa uma atualização** (`TakoUpdate\Tako-Portable-x64.exe`, verificada) nos primeiros segundos,
   e lê as lojas. Por isso o repouso só é medido depois de o Tako assentar (10 a 17 s).

## 7. O que não foi medido, e por quê

| Item | Por quê |
|---|---|
| **Contagem exata de quadros desenhados em repouso** | O release descarta opções de linha de comando do Chromium (`command_line_args_disabled`, `windows_host.rs`) e não abre porta de depuração; o núcleo abre a interface com argumentos fixos, então nem o debug aceita `--remote-debugging-port` nem tracing por linha de comando. O DevTools (F12) só abre à mão no debug, e a gravação do Performance exige interação. No lugar: utilização da GPU por PID e contadores na página (§3.2). |
| **fps no iGPU de referência (Steam Deck ou equivalente)** | Não há esse hardware aqui. Esta máquina tem RTX 3070 e monitor de 239 Hz: o p95 de 4,3 ms prova que o app acompanha 239 Hz nesta GPU, não num iGPU. |
| **fps "nenhuma tela abaixo da linha de base do Slint"** | O Slint não tem contador de quadros que este método alcance. Só medi 4 cenários da web (rolagem, troca de jogo, busca com vidro, vista Perfil); Configurações, Downloads, diálogos e menus ficaram de fora. |
| **Latência de controle** | Exige um controle físico ou virtual (a entrada passa pelo SDL3 no núcleo). Não há controle virtual instalado e não criei um. |
| **Latência até a luz do monitor** | A varredura do monitor não é medida por software. O que está medido vai até a pintura da página. |
| **Abertura até a biblioteca interativa no release real** | A página do release real não tem código de medição e o CSP dela bloqueia rede (`connect-src 'none'`), então ela não consegue reportar o marco. Mediu-se com o executável de medição (§3.1). Estimativa da diferença: não medida. A captura de tela do Windows devolve a janela do CEF em branco, então não houve conferência visual do release. |
| **Abertura até a biblioteca e fps/latência no Slint** | Sem marcadores nem contadores no Slint. Linha de base do Slint só em memória, CPU e GPU em repouso. |
| **Repouso do Slint na página de um jogo** | O clique do script é calibrado para a página web. |
| **Biblioteca de 1.000 jogos com capas em cache (cenário do PRD)** | Medi 318 jogos sem capas. A memória e o repouso com capas podem ser maiores. |
| **Memória do jogo/bandeja depois do tempo de descarte** | Mantida a medida da WEB-60 (núcleo sozinho: 9,4 a 9,5 MB). |
| **Checklist visual e fidelidade ao design** | Critério do mantenedor, não de script. |

## 8. Como repetir

```powershell
. .\scripts\cef-setup.ps1
cargo build -p tako --release --features web                     # target\release\tako.exe
# Slint: cargo build -p tako --release  e copiar tako.exe para target\release\tako-slint.exe (antes do build web)
$env:CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS = 'true'; $env:CARGO_TARGET_DIR = 'target\medicao'
cargo build -p tako --release --features web                     # target\medicao\release\tako.exe
Remove-Item Env:CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS, Env:CARGO_TARGET_DIR
Push-Location web; npm ci; npm run build; Pop-Location
.\scripts\measure-web.ps1 -Metrics tudo            # ~40 min; o resultado em JSON fica em $env:TEMP\tako-web60b\resultados.json
.\scripts\measure-web.ps1 -Metrics cpu-gpu         # ou abertura-biblioteca, fps, latencia, retorno
```

A bancada (jogos de teste, biblioteca semeada, pacote de medição) fica em `$env:TEMP\tako-web60b`. A primeira execução
semeia a biblioteca e espera a fila de metadados esvaziar (~10 min). O script mexe no mouse e no teclado e traz a janela
do Tako para a frente: não use o PC durante as rodadas de `cpu-gpu`, `latencia` e `fps`.

## 9. Detalhamento por processo (versão web em repouso, WEB-60)

Valores da rodada de mediana (rodada 2), pasta limpa:

| PID | Componente / Papel | Memória privada (Private WS) | Working Set total |
|---|---|---|---|
| 22724 | Núcleo Rust (`tako --web`) | 9,39 MB | 36,41 MB |
| 35788 | Interface CEF Host (`tako --web-ui`) | 27,34 MB | 105,68 MB |
| 31812 | CEF GPU Process (`--type=gpu-process`) | 20,57 MB | 70,41 MB |
| 24280 | CEF Utility Process (`--type=utility`) | 6,21 MB | 36,02 MB |
| 2444 | CEF Utility Process (`--type=utility`) | 3,36 MB | 23,32 MB |
| 4600 | CEF Renderer (`--type=renderer`) | 8,58 MB | 37,61 MB |
| 35528 | CEF Renderer Principal (`--type=renderer`) | 22,18 MB | 72,89 MB |
| **Total** | **7 processos** | **97,64 MB** | **382,34 MB** |

## 10. Conclusões da WEB-60 que seguem valendo

1. A memória com a interface aberta ficou muito abaixo da meta (97,6 a 145 MB contra 250 MB) e a do Slint em 325 a
   339 MB. A abertura até a janela é de 0,3 s, contra 4,4 s do Slint nesta máquina.
2. Ciclo descartável (D-10): ao fechar a interface os subprocessos do Chromium somem e fica só o núcleo, com 9,4 a
   9,5 MB (meta ≤ 40 MB).
3. O encerramento forçado do processo de UI dispara a recuperação de queda com reinício limpo registrado nos logs.

## Revisão do orquestrador (25/10)

CPU e GPU em repouso e o retorno depois do jogo apareciam como aprovados, mas o script não os mede: foram marcados como **não medidos**. A abertura mede até a janela responder, não até a biblioteca ficar interativa. As medições de memória e de processos vêm do script e valem. Próximos passos: cronometrar até a biblioteca interativa, contar CPU (`TotalProcessorTime`) e quadros em repouso, cronometrar o retorno depois de um jogo e medir o fps em debug com o DevTools.

_Atendido na WEB-60b: ver as §3 a §7. O fps não saiu do DevTools (que não dá para automatizar), e sim de um laço de quadros na página; a contagem exata de quadros em repouso continua não medida._
