# Quadro da migração para a interface web

Mantido pelo orquestrador ([processo](../processo/multiagente.md)). Integração: `feature/WEB-interface-web`.

Estados: `fila` → `despachada` → `em revisão` → `validação` (mantenedor) → `integrada` | `refazer` | `bloqueada`.

## Onda 1 — Medir e provar (MIG-W00, MIG-W01)

Objetivo: chegar ao gate com números do Slint e do protótipo web lado a lado. Nada daqui vai para o app; o código
do spike mora em `crates/spike-web/` e `web-spike/` e é descartável.

| ID | Tarefa | Nível | Conta e modelo | Depende de | Pode tocar | Estado |
|---|---|---|---|---|---|---|
| ORQ-00 | Processo, branch de integração, design system em `design/desktop/` | — | Orquestrador | — | `docs/`, `design/`, `AGENTS.md` | integrada |
| W01-A | **Contrato da ponte do spike:** mensagens (consulta, comando, evento), formato JSON, esquemas `tako-app://` e `tako-img://`, tipos Rust e TS do spike | N4 | Orquestrador | ORQ-00 | `docs/spikes/MIG-W01-ponte.md`, `crates/spike-web/src/bridge.rs`, `web-spike/src/bridge/` | fila |
| W01-B | **Hospedeiro CEF mínimo (Windows):** janela, CEF baixado para cache compartilhado, `tako-app://` servindo o build, `tako-img://` servindo capas do cache, ponte do W01-A, fechar e reabrir o Chromium (D-10) com tempo medido | N4 | Codex Lipp · GPT-6.1-Sol xhigh. Revisão: Claude InvestSmart · Opus 5.5 | W01-A | `crates/spike-web/`, `Cargo.toml` (membro e deps do spike) | fila |
| W01-C | **Telas do spike em React** a partir do `design/desktop/`: TelaBiblioteca, BibliotecaGrade com 1.000 capas virtualizada, CampoDeSelecao abrindo o MenuDeContexto, Dialogo com vidro; foco por teclado e eventos de controle simulados; ponte falsa para rodar no navegador | N3 | Claude InvestSmart · Sonnet 5.5 alto | W01-A | `web-spike/` | fila |
| W01-D | **Variante WebView2** do hospedeiro, mesmo React e mesma ponte, para comparar no gate | N3 | Codex Alyx · GPT-6.1-Sol medium | W01-A, W01-B (estrutura) | `crates/spike-web/` (módulo `webview2`) | fila |
| W00-A | **Ferramenta de medição:** memória somada da árvore de processos (Private Working Set), quadros por tela, abertura a frio e volta do jogo; serve para o Slint (`tako --bench`) e para o spike | N3 | Codex Alyx · GPT-6.1-Sol medium | ORQ-00 | `crates/tako/src/bench.rs`, `crates/spike-web/src/measure.rs`, `docs/marco-web/` | fila |
| W00-B | **Linha de base do Slint** nas telas listadas pelo mantenedor, com a W00-A | N2 | Orquestrador + mantenedor | W00-A, lista de telas | `docs/marco-web/medicoes.md` | bloqueada (lista de telas) |
| W01-E | **Medição do spike** (CEF e WebView2) e registro no ADR-0035 com a recomendação do motor do Windows | N3 | Orquestrador | W01-B, W01-C, W01-D, W00-A | `docs/marco-web/`, `docs/adr/0035-*` | fila |
| W01-F | **Gate** com o mantenedor | — | Mantenedor | W00-B, W01-E | — | fila |

Paralelismo da onda: W01-A primeiro (curta, do orquestrador); depois W01-B, W01-C e W00-A juntas (três contas,
duas com Rust); W01-D entra quando a estrutura do W01-B existir.

## Próximas ondas (depois do gate)

- **Onda 2 — Fundação:** WEB-01 (`tako-app`), WEB-02 (`tako-web`), WEB-03 (`web/`), WEB-04 (design system),
  WEB-05 (foco), WEB-06 (textos).
- **Onda 3 — Telas:** WEB-10 a WEB-16, uma tela por tarefa, com validação ao vivo.
- **Em paralelo à 3 — Instalador:** INST-01 a INST-04.
- **Onda 4 — Temas, Chromium, corte, medição final:** WEB-40, WEB-41, WEB-50, WEB-60.
