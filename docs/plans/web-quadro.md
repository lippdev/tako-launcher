# Quadro da migração para a interface web

Mantido pelo orquestrador ([processo](../processo/multiagente.md)). Integração: `feature/WEB-interface-web`.

Estados: `fila` → `despachada` → `em revisão` → `validação` (mantenedor) → `integrada` | `refazer` | `bloqueada`.

## Contas

| Conta | Provider | Situação |
|---|---|---|
| Claude Alyx Medical | `claudeAgent` | orquestrador |
| Claude Lipp InvestSmart | `claudeAgent_lipp_investsmart` | livre |
| Codex Lipp | `codex` | livre |
| Codex Alyx Medical | `codex_alyx_medical` | livre |

## Onda 1 — Fundação (D-14: direto no código definitivo)

| ID | Tarefa | Nível | Conta e modelo | Depende de | Pode tocar | Estado |
|---|---|---|---|---|---|---|
| ORQ-00 | Processo, branch de integração, design system em `design/desktop/`, CI só manual | — | Orquestrador | — | `docs/`, `design/`, `.github/`, `AGENTS.md` | integrada |
| WEB-00 | **Contrato da ponte:** consultas, comandos e eventos (formato, erros, versão), esquemas `tako-app://` e `tako-img://`, geração dos tipos TS a partir do Rust; esqueleto dos crates `tako-app` e `tako-web` e da pasta `web/` | N4 | Orquestrador | ORQ-00 | `docs/web/ponte.md`, `crates/tako-app/`, `crates/tako-web/` (esqueleto), `web/` (esqueleto), `Cargo.toml` | fila |
| WEB-02a | **Hospedeiro CEF no Windows:** janela com a barra própria, runtime do CEF num cache compartilhado de desenvolvimento, `tako-app://` (build do `web/` ou servidor do Vite em dev), `tako-img://` do cache de imagens, transporte da ponte, abrir com `tako --web`, fechar e reabrir o Chromium (D-10) | N4 | Codex Lipp · GPT-6.1-Sol xhigh. Revisão: Claude InvestSmart · Opus 5.5 | WEB-00 | `crates/tako-web/`, `crates/tako/src/main.rs` (só a opção `--web`), `Cargo.toml` (deps do CEF) | fila |
| WEB-03 | **Projeto `web/` e base do design system:** Vite + React + TypeScript, ESLint, build; `tokens.css`, fontes e ícones do `design/desktop/`; temas TAKO, Escuro, Meia-noite, Preto OLED e Liquid Glass por `data-theme`; ponte falsa para rodar no navegador | N2 | Claude InvestSmart · Sonnet 5.5 médio | WEB-00 | `web/` | fila |
| WEB-01a | **`tako-app`, primeira fatia:** biblioteca (lista, filtros, busca, ordenação), detalhes do jogo e Jogar, extraídos da `Shell` com os testes; a `tako-ui` passa a chamar a `tako-app` nesses pontos | N4 | Codex Alyx · GPT-6.1-Sol high. Revisão: Claude InvestSmart · Opus 5.5 | WEB-00 | `crates/tako-app/`, `crates/tako-ui/src/shell/` (só os pontos extraídos) | fila |
| WEB-04a | **Componentes base, lote 1:** BotaoDesktop, Abas, CampoDeBusca, CampoDeSelecao + MenuDeContexto, Dialogo, Aviso | N3 | a definir na hora (conta livre) | WEB-03 | `web/src/components/` | fila |
| WEB-04b | **Componentes base, lote 2:** GrupoDeBlocos, LinhaDeConfiguracao, ControleDeslizante, chave, PainelLateral, ListaDeJogos virtualizada | N3 | a definir na hora (conta livre) | WEB-03 | `web/src/components/` | fila |
| WEB-06 | **Textos:** `locales/` servido à interface, troca de idioma em tempo de execução | N2 | a definir na hora | WEB-00, WEB-02a | `crates/tako-web/`, `web/src/i18n/` | fila |
| WEB-05 | **Foco e controle:** navegação espacial, foco visível, repetição, entrada do SDL3 pela ponte | N3 | a definir na hora | WEB-02a, WEB-04a | `web/src/focus/`, `crates/tako-web/` (eventos de entrada) | fila |

Paralelismo: WEB-00 primeiro (orquestrador). Depois WEB-02a, WEB-03 e WEB-01a juntas, uma por conta (duas com
Rust). WEB-04a e WEB-04b entram assim que o WEB-03 for integrado; WEB-06 e WEB-05 quando o hospedeiro existir.

## Próximas ondas

- **Onda 2 — Telas:** WEB-10 a WEB-16, uma tela por tarefa, com validação ao vivo; hospedeiro no Linux e no macOS.
- **Em paralelo — Instalador:** INST-01 a INST-04.
- **Onda 3 — Temas, Chromium, medição e otimização, corte:** WEB-40, WEB-41, WEB-60 (com MIG-W00 e MIG-W01), WEB-50.
