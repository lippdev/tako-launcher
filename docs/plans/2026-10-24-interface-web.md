# Interface web (React sobre Chromium embutido): plano de transição

> **For agentic workers:** siga as etapas em ordem. Nenhuma tela é portada antes de o gate da Etapa 1 passar e o
> mantenedor aprovar os números.

**Goal:** trocar a interface Slint por React + TypeScript desenhados por um Chromium embutido, no modelo do Steam,
mantendo o núcleo Rust, e entregar o instalador por componentes com onboarding.
**Architecture:** núcleo Rust como fonte da verdade (`tako-core` e demais crates); nova camada de serviços
`tako-app`; hospedeiro `tako-web` (CEF) com ponte tipada e interface descartável; `web/` em React.
**Tech Stack:** Rust 2024, CEF (crate `cef`; WebView2 no Windows se empatar no gate), React + TypeScript + Vite,
SDL3 no núcleo.
**Spec:** PRD v0.3 §2 (D-09 a D-12), §2.1, §5, §7, §8.5, Fase 4 e épico INST; ADR-0035.

## Global Constraints

- O React não guarda dado de domínio; fechar a interface não pode perder nada (D-10).
- Sem porta de rede entre a interface e o núcleo; imagens só pelo `tako-img://`.
- Tema não executa código: CSS, tokens e arquivos sob CSP sem scripts (§7).
- O Slint segue publicado e congelado (D-12) até o corte (WEB-50). Funções novas entram no núcleo.
- Animações só de `transform` e `opacity`; nada anima `width`, `height`, `top`, `left` nem `font-size`.
- Todo texto vem de `locales/` (pt-BR e en-US).

## Como fica o desenho

```
┌──────────────── processo do núcleo (sempre vivo) ────────────────┐
│ tako (binário)  supervisor · sessão · single-instance · update     │
│ tako-web        janela · ciclo de vida do Chromium · ponte · esquemas│
│ tako-app        comandos · consultas · eventos                     │
│ tako-core · tako-sources · tako-platform · tako-themes · downloads │
│ SDL3 (controle) · cache de imagens                                 │
└───────────────▲──────────────────────────────▲─────────────────────┘
                │ mensagens do CEF (tipadas)   │ tako-img://, tako-app://
┌───────────────┴─ processos do Chromium (só com a janela aberta) ───┐
│ web/  React + TypeScript: design system, telas, foco, animação      │
└─────────────────────────────────────────────────────────────────────┘
```

- **Comandos** (React → Rust): `play(gameId)`, `setSetting(key, value)`, `editGame(...)`…; retornam resultado ou
  erro tipado.
- **Consultas** (React → Rust): biblioteca filtrada e paginada, detalhes do jogo, configurações…
- **Eventos** (Rust → React): biblioteca mudou, progresso de download, sessão começou ou terminou, controle
  conectou, bateria fraca…
- Os tipos Rust da ponte geram os tipos TypeScript no build (ts-rs ou specta, escolhido no WEB-02); quebra de
  contrato falha no CI.

## Review Focus

- Interface reaberta precisa voltar à mesma tela, foco e rolagem (estado de tela que importa é salvo no Rust).
- Nenhum processo do Chromium vivo durante o jogo (verificar no gerenciador de tarefas e no `ps`).
- Esquemas próprios não podem servir arquivo fora do pacote da interface ou do cache (path traversal).
- Entrada do controle: nenhum eco duplicado entre SDL3 e a Gamepad API do Chromium (desligar a do Chromium).

---

## Etapa 0 — Linha de base do Slint (MIG-W00)

- [ ] Mantenedor lista as telas onde vê texto granulado e quadros baixos.
- [ ] Definir as máquinas de referência (um iGPU tipo Steam Deck e um PC Windows) e o método de memória da §8.5.
- [ ] Estender `tako --bench` para medir quadros por tela, memória somada dos processos e volta do jogo.
- [ ] Rodar no Slint atual (release) e publicar em `docs/marco-web/medicoes.md`.

## Etapa 1 — Spike e gate (MIG-W01)

Código descartável em `crates/spike-web` e `web-spike/`; não entra no app.

- [ ] Hospedeiro CEF mínimo em Rust: janela, `tako-app://` servindo o build do Vite, `tako-img://` servindo capas
  do cache atual.
- [ ] Ponte mínima: uma consulta (biblioteca), um comando (abrir jogo simulado), um evento (progresso).
- [ ] Telas montadas com o CSS do `tako-design-desktop`: TelaBiblioteca (visão geral), grade virtualizada de
  1.000 capas (BibliotecaGrade), CampoDeSelecao abrindo o MenuDeContexto e um Dialogo com vidro e desfoque. Assim o
  gate compara o mesmo visual que hoje fica granulado no Slint.
- [ ] Controle: SDL3 no Rust → eventos → foco no React.
- [ ] Interface descartável: fechar todos os processos do Chromium e reabrir na mesma tela; medir o tempo.
- [ ] Mesmo React sobre WebView2 no Windows.
- [ ] Rodar em Windows e Linux (X11, Wayland KDE, gamescope); macOS se o empacotamento permitir.
- [ ] Medir pela §8.5, comparar com a Etapa 0 e registrar no ADR-0035, com a recomendação do motor do Windows.
- [ ] **Gate:** mantenedor aprova os números. Se reprovar, parar aqui.

## Etapa 2 — Fundação (WEB-01 a WEB-06)

- [ ] **WEB-01 `tako-app`:** levantar, em `crates/tako-ui/src/shell/`, o que é regra do app e o que é tela.
  Extrair primeiro o que as telas da Etapa 3 usam: biblioteca e filtros (`library.rs`, `smart.rs`), jogo
  (`details.rs`, `edit_game.rs`, `game_settings.rs`, `art.rs`, `source_ids.rs`), sessão (`launch_splash.rs`,
  `closing_game.rs`), configurações (`settings*.rs`), downloads, perfis, temas. Portar os testes de
  `ui_tests.rs` e `settings_tests.rs` que testam regra (não pixel) para a `tako-app`. A `tako-ui` passa a chamar
  a `tako-app`.
- [ ] **WEB-02 `tako-web`:** hospedeiro do spike promovido a crate, com barra de janela própria, ciclo de vida
  (D-10), restauração de tela, esquemas com validação de caminho e geração de tipos.
- [ ] **WEB-03 `web/`:** Vite + React + TypeScript, ESLint, testes de componentes, build entrando no pacote; modo
  dev em que o Tako carrega o servidor do Vite (hot reload) com DevTools aberto.
- [ ] **WEB-04 design system:** partir do `tako-design-desktop` (o mesmo HTML/CSS do protótipo). `tokens.css` →
  `web/src/styles/tokens.css` (gerado do `tokens.json`); fontes (Poppins, Plus Jakarta Sans) e ícones SVG de
  `assets/Icones`; CSS do `bundle.css` quebrado por componente, sem as imagens em base64 e sem o escopo `.tk`.
  Componentes base primeiro (BotaoDesktop, CampoDeSelecao, MenuDeContexto, Dialogo, Aviso, GrupoDeBlocos,
  LinhaDeConfiguracao, ControleDeslizante, ListaDeJogos virtualizada), com os tempos do MovimentoBase,
  "Reduzir animações", "Efeitos de vidro" e "Tamanho da interface". O `Implementacao.md` do design system fala
  em Slint: vale a regra visual, não o caminho técnico.
- [ ] **WEB-05 foco:** navegação espacial com as regras de `tako-core`, foco visível, repetição ao segurar,
  atalhos e remapeamento (INPUT-001) vindos do núcleo.
- [ ] **WEB-06 textos:** `locales/` servido à interface, com troca de idioma em tempo de execução.

## Etapa 3 — Porte das telas (WEB-10 a WEB-16)

Fluxo por tela, com o mantenedor validando ao vivo (hot reload do Vite):

1. Visual: a prévia da tela em `design/desktop/` (nada vem dos `.slint`). Comportamento: a `Shell` em Rust
   (`crates/tako-ui/src/shell/`), que diz o que a tela faz (estados, ações, textos).
2. Garantir comandos, consultas e eventos na `tako-app`.
3. Montar a tela no React com os componentes base.
4. Mantenedor aprova o visual; checar quadros no DevTools.
5. Marcar o item no PRD.

Ordem: barra e visão geral (WEB-10) → biblioteca (WEB-11) → página do jogo (WEB-12) → configurações (WEB-13) →
camadas (WEB-14) → Big Picture (WEB-15) → primeira execução no onboarding (WEB-16, com o INST-03).

Até o corte, a interface web abre por uma opção de desenvolvimento (por exemplo `--web`); o padrão continua Slint.

## Etapa 4 — Instalador e onboarding (INST-01 a INST-04)

Em paralelo à Etapa 3, a partir do WEB-02:

- [ ] Formato dos componentes no GitHub Releases (núcleo, runtime do Chromium, interface), cada um com SHA-256 e
  versão; manifesto da release lista as versões compatíveis entre si.
- [ ] `tako-installer`: baixa, confere, retoma e instala; abre o onboarding. Desenho das telas com o mantenedor.
- [ ] Atualizador por componentes (INST-02) sobre o `ReleaseFeed` atual.
- [ ] Pacotes completos e portable (INST-04); Linux e macOS pelos pacotes.

## Etapa 5 — Temas, segurança do Chromium, corte e medição (WEB-40 a WEB-60)

- [ ] **WEB-40:** formato de tema web, validação do CSS, CSP, layouts oficiais (Fita), prévia, edição e criação.
- [ ] **WEB-41:** job do CI que avisa nova estável do CEF; procedimento para publicar o runtime em poucos dias.
- [ ] **WEB-50:** beta web nos três SOs; remover `tako-ui`, `.slint`, dependências do Slint e o modo dev de hot
  reload do Slint; changelog sobre temas `.slint`.
- [ ] **WEB-60:** medir a §8.5 no app completo contra a linha de base e publicar antes da estável.

## Fora deste plano

- O desenho visual do instalador e do onboarding (vem depois, com o mantenedor).
- Funções novas de produto: entram no núcleo e ganham tela já na web, cada uma com sua história.
