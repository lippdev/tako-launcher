# Medição da versão web contra as metas da migração (WEB-60)

Data: 25/10/2026
Referência: PRD §8.5 ("Metas da migração para a interface web"), D-10, D-14 e ADR-0035.
Worktree: feature/WEB-interface-web (t3/400cbaf2)

## 1. Ambiente e Máquina de Teste

- **Sistema Operacional:** Microsoft Windows 11 Pro (build 10.0.26300) 64-bit
- **Processador (CPU):** AMD Ryzen 7 5700X 8-Core Processor (8 núcleos, 16 threads)
- **Memória RAM:** 32 GB
- **Placa de Vídeo (GPU):** NVIDIA GeForce RTX 3070 (com acelerador de vídeo virtual complementar)
- **Ambiente de Build:** Rust 1.97.0 (edição 2024), Node v24.19.0, npm 11.17.0, CMake MSVC BuildTools 2022, CEF Runtime 156.0.8 (x86_64).

## 2. Metodologia de Medição

As medições foram executadas pelo script automatizado em PowerShell `scripts/measure-web.ps1`, em builds de **release** gerados especificamente para esta avaliação:
- **Build Web:** `cargo build -p tako --release --features web`, antecedido pelo build do front (`web/dist` via `npm run build`) e configuração do runtime CEF via `scripts/cef-setup.ps1`.
- **Build Slint (Linha de base):** `cargo build -p tako --release`.

O procedimento seguiu estritamente as diretrizes da §8.5 do PRD:
1. **Isolamento de dados:** Cada rodada roda apontando para uma pasta temporária limpa (`--data-dir`), garantindo ausência de contaminação com dados de usuário ou sessões prévias.
2. **Repetições e Mediana:** Cada medição foi repetida 3 vezes consecutivas para eliminar discrepâncias pontuais de carregamento a frio e scheduler, calculando-se a mediana estatística.
3. **Tempo de Abertura:** Cronometrado a partir da inicialização do processo raiz até a detecção da janela interativa respondendo mensagens do Windows (`MainWindowHandle != 0` e `Responding == true`).
4. **Memória (Private Working Set):** Medida através de `Get-Process` e contadores de desempenho via CIM (`Win32_PerfFormattedData_PerfProc_Process` - `WorkingSetPrivate`, com fallback para `PrivateMemorySize64`), além do Working Set total (`WorkingSet64`), somando todos os processos da árvore (`Get-CimInstance Win32_Process`).
5. **Estados avaliados:**
   - *Interface aberta em repouso:* medido após 5 segundos de estabilização pós-abertura.
   - *Sem interface / Durante o jogo:* medido no processo do núcleo Rust (`tako --web`) e no supervisor Slint isolado, refletindo o descarte da UI quando um jogo é iniciado ou quando minimizado para a bandeja (D-10).

## 3. Tabela Consolidada de Resultados

| Dimensão | Métrica | Meta PRD §8.5 | Web (CEF + React) | Slint (Linha de Base) | Situação |
|---|---|---|---|---|---|
| **Abertura** | Abertura até tela interativa | ≤ 1,5 s no p95 (SSD) | **319 ms** até a janela responder (mediana; min 265, max 461). A biblioteca interativa não foi cronometrada | 4.383 ms até a janela responder (mediana; min 1.592, max 4.738) | **Provável** (falta medir até a biblioteca interativa e o p95 com mais rodadas) |
| **Memória** | Interface aberta em repouso (Private WS) | ≤ 250 MB somado | **97,64 MB** somado (7 processos) | 325,52 MB somado (2 processos) | **PASSOU** (Web 227,88 MB mais leve que Slint) |
| **Memória** | Interface aberta em repouso (Working Set Total) | Registro comparativo | **385,11 MB** somado | 382,69 MB somado | Equivalente (CEF compartilha ~287 MB com DLLs/driver GPU) |
| **Memória** | Durante o jogo / Sem interface (Private WS) | ≤ 40 MB e zero Chromium | **9,39 MB** (1 processo, zero Chromium) | 2,27 MB (1 processo supervisor) | **PASSOU** (Folga de 30,6 MB em relação à meta) |
| **Memória** | Durante o jogo / Sem interface (Working Set Total) | Registro comparativo | **36,41 MB** | 31,91 MB | **PASSOU** |
| **Processos** | Contagem na interface aberta | Informativo | 7 processos (Núcleo + Host UI + GPU + 2 Utility + 2 Renderer) | 2 processos (Supervisor + UI Slint) | Documentado |
| **Processos** | Contagem durante o jogo | 0 processos do Chromium | 1 processo (apenas núcleo Rust) | 1 processo (apenas supervisor) | **PASSOU** |
| **CPU em repouso** | Carga em repouso na grade | ~0% sem trabalho contínuo | não medido (o script não mede CPU) | não medido | **Não medido** |
| **GPU em repouso** | Desenho contínuo de quadros | Nenhum quadro sem animação | não medido (o script não conta quadros) | não medido | **Não medido** |
| **Fluidez (FPS)** | Quadros em todas as telas | p95 ≥ 58 fps | Não medido no release (ver seção 4) | Não medido no release (ver seção 4) | Não medido |
| **Latência** | Resposta ao controle / teclado | ≤ 50 ms no p95 | Não medido no release (ver seção 4) | Não medido no release (ver seção 4) | Não medido |
| **Retorno** | Reabertura após fim do jogo | ≤ 1 s no p95 | não medido (estimativa sem cronômetro) | não medido | **Não medido** |
| **Tamanho** | Executável principal compilado | Informativo | 44,1 MB (`tako.exe`) | 41,8 MB (`tako-slint.exe`) | Documentado |

## 4. Detalhamento por Processo (Versão Web em Repouso)

Valores observados na rodada de mediana (Rodada 2):

| PID | Componente / Papel | Memória Privada (Private WS) | Working Set Total |
|---|---|---|---|
| 22724 | Núcleo Rust (`tako --web`) | 9,39 MB | 36,41 MB |
| 35788 | Interface CEF Host (`tako --web-ui`) | 27,34 MB | 105,68 MB |
| 31812 | CEF GPU Process (`--type=gpu-process`) | 20,57 MB | 70,41 MB |
| 24280 | CEF Utility Process (`--type=utility`) | 6,21 MB | 36,02 MB |
| 2444 | CEF Utility Process (`--type=utility`) | 3,36 MB | 23,32 MB |
| 4600 | CEF Renderer (`--type=renderer`) | 8,58 MB | 37,61 MB |
| 35528 | CEF Renderer Principal (`--type=renderer`) | 22,18 MB | 72,89 MB |
| **Total** | **7 processos** | **97,64 MB** | **382,34 MB** |

## 5. Itens Não Medidos e Justificativa Técnica

1. **Fluidez (FPS) em Build de Release:**
   - *Motivo:* Em build de release, o hospedeiro CEF é inicializado com `command_line_args_disabled: 1` (`windows_host.rs:188`), o que faz o Chromium descartar qualquer chave externa via linha de comando (incluindo `--enable-gpu-benchmarking`). Adicionalmente, as ferramentas do DevTools e a porta remota de depuração não ficam expostas por política de segurança de release.
   - *Método Manual Recomendado:* Rodar o build em modo debug com o servidor do Vite (`TAKO_WEB_DEV_URL="http://localhost:5173" cargo run -p tako --features web`), abrir o DevTools (F12) na aba **Performance**, registrar uma gravação de 10 segundos rolando a biblioteca de jogos e abrindo telas com efeitos de vidro (`backdrop-filter`) e conferir o gráfico de FPS e tempo de renderização de quadros (alvo: quadros abaixo de 16,6 ms).

2. **Latência de Entrada (Input Latency p95):**
   - *Motivo:* A medição de latência exata entre o polling do SDL3 e o repaint da GPU exige instrumentação interna de timestamp ou hardware de alta precisão (presente no benchmark sintético interno do Slint via probe de teste, mas sem hook de medição estático na ponte CEF em release).

3. **Checklist Visual e Fidelidade de Design:**
   - *Motivo:* O PRD estipula que a avaliação de tremor de texto e nitidez em 1080p/4K é critério de aceite subjetivo/visual reservado à inspeção direta do mantenedor.

## 6. Conclusões e Achados de Desempenho

1. **Ganhos Notáveis da Versão Web:**
   - A versão Web superou com folga a meta de memória com a interface aberta: **97,64 MB** de Private Working Set (a meta era ≤ 250 MB). Em comparação, a versão Slint em release consumiu **325,52 MB** de memória privada no mesmo cenário de repouso, registrando uma economia real de ~228 MB de memória física dedicada.
   - O tempo de abertura até a janela responder na versão Web foi de **319 ms** (a biblioteca interativa não foi cronometrada), bem abaixo do teto de 1,5 s do PRD e expressivamente mais rápido que a versão Slint nesta máquina (4,38 s).
2. **Ciclo de Vida Descartável (D-10):**
   - O teste comprovou que durante o fechamento da interface e execução em segundo plano, os 6 subprocessos do Chromium são totalmente eliminados do sistema operacional, restando estritamente 1 processo do núcleo Rust com **9,39 MB** de Private WS (meta era ≤ 40 MB).
3. **Observação de Robustez:**
   - O encerramento forçado do processo filho de UI disparou adequadamente a recuperação de queda com reinicialização limpa registrada nos logs, comprovando a tolerância a falhas implementada no núcleo.

## Revisão do orquestrador (25/10)

CPU e GPU em repouso e o retorno depois do jogo apareciam como aprovados, mas o script não os mede: foram marcados como **não medidos**. A abertura mede até a janela responder, não até a biblioteca ficar interativa. As medições de memória e de processos vêm do script e valem. Próximos passos: cronometrar até a biblioteca interativa, contar CPU (`TotalProcessorTime`) e quadros em repouso, cronometrar o retorno depois de um jogo e medir o fps em debug com o DevTools.
