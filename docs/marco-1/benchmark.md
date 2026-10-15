# Benchmark inicial (MVP-035)

Mede o Tako de forma reprodutível: abertura até a primeira tela interativa (a frio e a quente), memória, CPU em repouso, fluidez da UI numa navegação roteirizada e o custo do Tako com um jogo em execução. Ferramenta: [`tools/Launcher.Bench`](../../tools/Launcher.Bench). O CI roda o benchmark em toda PR e todo push em `develop`/`main` (job `benchmark`) e publica o relatório como artefato **`benchmark`** e no resumo do job.

> **Os números de referência vêm do hardware real.** O runner do GitHub é uma VM sem GPU (renderização por software, tela virtual de 1024×768) e muda de processador entre execuções; os números dele servem para comparar PRs entre si e para pegar regressões grandes, não para dizer como o Tako roda num PC de jogos. Os números que valem para o produto são os da tabela [Máquinas de referência](#máquinas-de-referência), preenchida pelos mantenedores e testadores no hardware deles.

## Como rodar localmente

No Windows 10/11 (o benchmark abre a UI real):

```powershell
dotnet build tools/Launcher.Bench -c Release
.\tools\Launcher.Bench\bin\Release\net10.0-windows\Launcher.Bench.exe --label "Meu PC (Ryzen 7 7800X3D / RTX 4070)"
```

- Leva cerca de 10 minutos com os padrões. `--quick` faz uma rodada curta (só 20 jogos, poucos segundos por etapa) para conferir que tudo funciona.
- Opções: `--sizes 20,200,1000,5000` (tamanhos da biblioteca), `--warm 5` (aberturas a quente por tamanho), `--idle-seconds 10`, `--load-seconds 10`, `--out pasta` (padrão `.\benchmark`), `--label "nome"`.
- Saída: `benchmark.md` (tabelas abaixo), `benchmark.json` (tudo, inclusive cada abertura e as distribuições completas) e `runs\` (relatório de cada processo).
- Rode pelo `.exe`, não por `dotnet Launcher.Bench.dll`: o benchmark abre a si mesmo em processos novos.
- Feche jogos, navegador e downloads; deixe o PC na tomada e no plano de energia de sempre. Não mexa no mouse/teclado durante a rodada (a janela do Tako abre em tela cheia várias vezes).
- O benchmark não muda nada no sistema: usa uma pasta de dados própria (`Tako_Data` ao lado do `Launcher.Bench.exe`), biblioteca fictícia, artes geradas localmente (sem rede) e monitores/áudio **simulados** (a restauração de monitores e áudio do MVP-012 não toca na máquina).

## Metodologia

Cada medição roda num **processo novo**. O processo filho monta o Tako com a mesma composição do app (`App.BuildServices`, o mesmo tema pelo carregador XAML restrito, a mesma `MainWindow` em tela cheia) e só troca a biblioteca e os serviços de monitores/áudio. Ele mede a UI do Tako diretamente, sem o supervisor do `Tako.exe` (equivale a abrir com `--ui`, ver [ADR 0012](../adr/0012-supervisor-e-ipc.md)).

**Biblioteca fictícia.** 20, 200, 1000 e 5000 jogos, sempre iguais para o mesmo tamanho (semente fixa): lojas variadas, 15 jogados recentemente, favoritos, descrições de tamanho realista. Cada jogo tem a própria capa JPEG 600×900 (decodificação real por jogo); os fundos 1920×1080 se repetem a partir de 12 imagens. Tudo gerado na primeira rodada em `benchmark\art`.

| Métrica | Como é medida |
|---|---|
| **Abertura até a primeira tela interativa** | Do `Process.Start` no orquestrador até o filho avisar "pronto": primeiro quadro desenhado (`ContentRendered`), um elemento com foco e a fila do dispatcher vazia (`ApplicationIdle`). Inclui criar o processo, carregar o .NET e o JIT. O filho também registra as fases internas (serviços prontos, janela mostrada, 1º quadro, interativa) em ms desde o início do processo. |
| **A frio** | Primeira abertura de cada tamanho, com a pasta de dados apagada (banco `tako.db` e logs criados do zero). Não esvazia o cache de disco do Windows (exigiria reiniciar ou ser administrador): é "a frio" para o Tako, não para o sistema operacional. |
| **A quente** | Mediana e p95 de N aberturas seguidas (padrão 5), com a pasta de dados já criada. |
| **Memória** | Conjunto de trabalho (RAM física em uso), bytes privados e heap gerenciado do .NET: logo após abrir, após o repouso e 2 s depois da navegação roteirizada. |
| **CPU em repouso** | Tempo de CPU do processo durante N segundos (padrão 10) com o Tako parado na Início, 3 s depois de abrir. Em % da máquina (todos os núcleos) e % de um núcleo. Inclui a leitura de controles a cada 16 ms e o relógio. |
| **Fluidez** | `CompositionTarget.Rendering` durante uma navegação roteirizada com os comandos do controle (104 comandos, 120 ms entre eles, como o direcional segurado): 24 × → e 12 × ↓ na grade da biblioteca, detalhes e volta, fileira da Início, 20 linhas das Configurações e uma descida de 25 linhas na grade. O quadro esperado é a mediana dos intervalos com a UI parada (a taxa de atualização efetiva). **Quadros lentos** = intervalos acima de 1,5× o esperado; **quadros perdidos** = soma dos quadros que cabiam nesses intervalos. **Comando → próximo quadro** = do comando até o próximo quadro desenhado; **tratamento do comando** = tempo síncrono do comando na thread da UI. **Abrir a biblioteca** = do comando de abrir a grade "Todos" até o primeiro quadro com a grade desenhada (o maior das duas aberturas do roteiro): é o travamento que o usuário vê em bibliotecas grandes. |
| **Jogo em execução** | O benchmark se abre como "jogo" adicionado (uma carga que ocupa todos os núcleos com contas de ponto flutuante e conta o trabalho feito por segundo). O **Tako inicia o processo e acompanha até ele fechar**, como faz com um jogo de verdade. Mede a CPU e a memória do Tako nos 10 s de carga (depois de 2 s de aquecimento) e compara o trabalho do jogo com a carga rodando sozinha (melhor de duas rodadas: uma antes e outra depois da rodada completa, porque o runner ainda está "frio" no começo do job). |

**O que reprova no CI:** só travar (processo sem terminar em 5 min), quebrar (código de saída diferente de 0, exceção, tela "Algo deu errado"), não chegar à tela interativa em 90 s ou o Tako não acompanhar o jogo de carga. Nenhum número reprova.

**Fora do benchmark (por enquanto):** varredura da Steam e leitura do `tako.db` com a biblioteca real (a fictícia fica em memória), downloads de artes e metadados, o supervisor (`Tako.exe` sem `--ui`), GPU real e telas acima de 1080p no CI, FPS de jogos reais (PresentMon/ETW, PERF-006/007).

## Linha de base: runner do GitHub

Execução do CI de 15/10/2026 no commit `40982fd` ([run 38698112228](https://github.com/lippdev/ultralauncher/actions/runs/38698112228)), Tako 0.1.0, 5 aberturas a quente por tamanho, repouso 10 s, jogo 10 s.

| Máquina | CPU | GPU | RAM | Tela | Windows |
|---|---|---|---|---|---|
| GitHub Actions windows-latest | AMD EPYC 9V45 (4 threads) | Microsoft Hyper-V Video (sem GPU) | 16 GB | 1024×768, 64 Hz, 100% | Windows Server 2025 Datacenter 24H2 (26100.33438) |

### Abertura até a primeira tela interativa

| Jogos | A frio (ms) | A quente, mediana (ms) | A quente, p95 (ms) | Fases a quente: serviços / janela / 1º quadro / interativa (ms) |
|---:|---:|---:|---:|---|
| 20 | 1588 | 1012 | 1036 | 478 / 708 / 987 / 1010 |
| 200 | 903 | 968 | 971 | 497 / 704 / 946 / 965 |
| 1000 | 1272 | 1245 | 1257 | 510 / 1009 / 1226 / 1239 |

### Memória e CPU

| Jogos | Ao abrir: WS / privada (MB) | Em repouso: WS / privada (MB) | Após navegar: WS / privada / heap .NET (MB) | CPU em repouso: % da máquina (% de 1 núcleo) |
|---:|---:|---:|---:|---:|
| 20 | 230 / 160 | 168 / 96 | 302 / 214 / 13 | 0,23% (0,9%) |
| 200 | 284 / 221 | 231 / 159 | 495 / 407 / 22 | 0,00% (0,0%) |
| 1000 | 525 / 468 | 511 / 444 | 857 / 768 / 56 | 0,04% (0,2%) |

### Fluidez na navegação roteirizada

| Jogos | Quadro esperado (ms) | FPS médio | Tempo de quadro p50 / p95 / p99 / máx (ms) | Quadros lentos | Quadros perdidos | Comando → quadro p50 / p95 (ms) | Tratamento do comando p95 (ms) |
|---:|---:|---:|---|---:|---:|---|---:|
| 20 | 15,6 | 56,2 | 15,6 / 31,3 / 36,3 / 93,8 | 171 de 1369 | 190 | 10,9 / 23,0 | 3,1 |
| 200 | 15,6 | 51,0 | 15,6 / 31,3 / 46,9 / 265,6 | 233 de 1088 | 277 | 10,6 / 31,6 | 8,2 |
| 1000 | 15,6 | 43,6 | 15,6 / 31,3 / 60,9 / 1125,0 | 223 de 911 | 427 | 20,7 / 43,8 | 28,1 |

### Com um jogo em execução

| Jogos | CPU do Tako: % da máquina (% de 1 núcleo) | RAM do Tako: WS / privada (MB) | Jogo sozinho (ops/s) | Jogo com o Tako (ops/s) | Perda do jogo |
|---:|---:|---:|---:|---:|---:|
| 20 | 0,78% (3,1%) | 301 / 210 | 800646 | 790856 | 1,2% |
| 200 | 0,74% (3,0%) | 408 / 314 | 810970 | 790185 | 2,6% |
| 1000 | 0,47% (1,9%) | 817 / 724 | 797910 | 787902 | 1,3% |

### Leitura da linha de base

- **Ruído do runner.** Duas execuções seguidas caíram em processadores diferentes (EPYC 7763 e 9V45) e a abertura a quente mudou ~33% só por isso. Compare PRs pelo artefato da mesma época e olhe tendências, não diferenças de 10%.
- **Biblioteca grande.** Com 1000 jogos a RAM após navegar chegava a ~860 MB e abrir a biblioteca travava a UI por ~1 s (quadro máximo de 1125 ms; ~3 s no runner mais lento): a grade criava e decodificava a capa de todos os jogos de uma vez. Resolvido pela frente D (abaixo).

### Bibliotecas grandes: antes e depois da virtualização (frente D, PERF-004)

A grade da biblioteca, as fileiras da Início e a grade dos temas Playnite passaram a ser **virtualizadas** (só existem as capas perto da tela, com duas linhas de folga; os elementos são reaproveitados ao rolar), e as capas passaram a ser **carregadas sob demanda**: só quando aparecem, decodificadas fora da thread da UI no tamanho exibido (`DecodePixelWidth` na densidade real da tela, 2K/4K incluídos), congeladas e guardadas num cache LRU limitado em bytes (32 MB de capas, 32 MB de fundos).

Mesmo runner (GitHub Actions windows-latest, 1024×768), PR #29: "antes" = commit `64d02cc` (só o cenário novo de 5000 jogos), "depois" = commit `b521bc9`. Processadores diferentes entre as execuções (Xeon 8573C × EPYC 9V45): compare ordens de grandeza.

| Jogos | Abertura a quente, mediana (ms) | RAM em repouso: WS (MB) | RAM após navegar: WS (MB) | Abrir a biblioteca (ms) | Maior quadro na navegação (ms) | Comando → quadro p95 (ms) |
|---:|---:|---:|---:|---:|---:|---:|
| 1000, antes | 1701 | 507 | 888 | 1852 | 1828 | 52,5 |
| 1000, **depois** | **885** | **165** | **406** | **77** | **78** | **25,5** |
| 5000, antes | 3642 | 2038 | 2651 | 6115 | 6094 | 134,4 |
| 5000, **depois** | **990** | **175** | **433** | **102** | **78** | **26,1** |

- Memória e tempos passam a quase não depender do tamanho da biblioteca: 20 jogos ficam em 164 MB em repouso e 276 MB após navegar.
- A meta do PRD (até 300 MB em repouso) vale com folga em 1000 e 5000 jogos. Após a navegação roteirizada o conjunto de trabalho fica em ~400 MB no runner: são as capas e fundos ainda na tela ou no cache e memória de imagens já descartadas que o GC ainda não devolveu (a heap .NET fica abaixo de 20 MB).
- Os quadros acima de 100 ms somem; os quadros lentos que sobram (~1,5× o esperado) são da renderização por software do runner, como com 20 jogos.
- **Repouso** fica abaixo de 1% de um núcleo em todos os tamanhos; **com o jogo aberto**, o Tako usa ~3% de um núcleo (relógio, leitura de controles, acompanhamento do processo) e o jogo perde 1–3%, perto do ruído.
- Os quadros lentos no runner vêm da renderização por software sem GPU; em hardware real espere quadros perdidos perto de zero com 20 jogos.

## Máquinas de referência

Preencha uma linha por máquina com o `benchmark.md` gerado no próprio hardware (cole o arquivo completo na PR ou issue). CPU, GPU, RAM e tela saem do cabeçalho do relatório, igual ao [diagnóstico do sistema](checklist-hardware.md). Valores com 200 jogos, a não ser que indicado.

| Máquina | CPU | GPU | RAM | Tela (resolução, Hz, escala) | Windows (build) | Tako (versão/commit) | Abertura a frio / a quente (ms) | RAM em repouso / após navegar (MB) | CPU em repouso (% 1 núcleo) | FPS médio / quadros perdidos | Perda do jogo | Quem / data |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Desktop de referência | | | | | | | | | | | | |
| Notebook de referência | | | | | | | | | | | | |
| Portátil de jogos (ROG Ally, Legion Go…) | | | | | | | | | | | | |
| PC de entrada (iGPU, 8 GB) | | | | | | | | | | | | |
