# Quadro da migração para a interface web

Mantido pelo orquestrador ([processo](../processo/multiagente.md)). Integração: `feature/WEB-interface-web`.

Estados: `fila` → `despachada` → `em revisão` → `validação` (mantenedor) → `integrada` | `refazer` | `bloqueada`.

## Contas

| Conta | Provider | Situação |
|---|---|---|
| Claude Alyx Medical | `claudeAgent` | orquestrador + subagentes (plano 5x): WEB-02b, WEB-06a, WEB-01e |
| Antigravity | `antigravity` | WEB-04g, WEB-04h, WEB-04i (resetou; conferir tudo que ele entrega) |
| Claude Lipp InvestSmart | `claudeAgent_lipp_investsmart` | **rate limit**; reset às ~20:52 (aviso agendado para 20:55) |
| Codex Lipp | `codex` | sem limite; reset às ~20:52 |
| Codex Alyx Medical | `codex_alyx_medical` | sem limite; reset às ~20:52 |

## Onda 1 — Fundação (D-14: direto no código definitivo)

| ID | Tarefa | Nível | Conta e modelo | Depende de | Pode tocar | Estado |
|---|---|---|---|---|---|---|
| ORQ-00 | Processo, branch de integração, design system em `design/desktop/`, CI só manual | — | Orquestrador | — | `docs/`, `design/`, `.github/`, `AGENTS.md` | integrada |
| WEB-00 | **Contrato da ponte:** consultas, comandos e eventos (formato, erros, versão), esquemas `tako-app://` e `tako-img://`, geração dos tipos TS a partir do Rust; esqueleto dos crates `tako-app` e `tako-web` e da pasta `web/` | N4 | Orquestrador | ORQ-00 | `docs/web/ponte.md`, `crates/tako-app/`, `crates/tako-web/`, `web/src/bridge/`, `Cargo.toml`, `.cargo/config.toml` | integrada |
| WEB-02a | **Hospedeiro CEF no Windows:** janela com a barra própria, runtime do CEF num cache compartilhado de desenvolvimento, `tako-app://` (build do `web/` ou servidor do Vite em dev), `tako-img://` do cache de imagens, transporte da ponte, abrir com `tako --web`, fechar e reabrir o Chromium (D-10) | N4 | Codex Lipp · GPT-6.1-Sol xhigh. Rodada 2: Claude InvestSmart · Opus 5.5 (trocou de conta); re-revisão: Codex Alyx | WEB-00 | `crates/tako-web/`, `crates/tako/src/main.rs` (só a opção `--web`), `Cargo.toml` (deps do CEF) | integrada |
| WEB-03 | **Projeto `web/` e base do design system:** Vite + React + TypeScript, ESLint, build; `tokens.css`, fontes e ícones do `design/desktop/`; temas TAKO, Escuro, Meia-noite, Preto OLED e Liquid Glass por `data-theme`; hospedeiro falso (`web/src/bridge/fake.ts`) para rodar no navegador | N2 | Claude InvestSmart · Sonnet 5.5 médio | WEB-00 | `web/` (menos `web/src/bridge/`, que é do contrato) | integrada |
| WEB-01a | **`tako-app`, primeira fatia:** boot próprio do núcleo (o que o `build_ctx` da `tako-ui` faz, sem depender dela), `library.list` (filtros, busca, ordenação, página), `game.details` e `game.play` (pedido ao hospedeiro), com os testes; a `tako-ui` não é tocada | N4 | Codex Alyx · GPT-6.1-Sol high. Revisão: Claude InvestSmart · Opus 5.5 | WEB-00 | `crates/tako-app/`, `Cargo.toml` (deps da `tako-app`) | integrada |
| WEB-04a | **Componentes base, lote 1:** BotaoDesktop, Abas, CampoDeBusca, CampoDeSelecao + MenuDeContexto, Dialogo, Aviso | N3 | Claude InvestSmart · Sonnet 5.5 alto | WEB-03 | `web/src/components/` | integrada (aguarda validação visual) |
| WEB-04b | **Componentes base, lote 2:** GrupoDeBlocos, LinhaDeConfiguracao, ControleDeslizante, chave, PainelLateral, ListaDeJogos virtualizada | N3 | Claude InvestSmart · Sonnet 5.5 alto | WEB-03 | `web/src/components/` | integrada (aguarda validação visual) |
| WEB-06 | **Textos:** `locales/` servido à interface, troca de idioma em tempo de execução | N2 | a definir na hora | WEB-00, WEB-02a | `crates/tako-web/`, `web/src/i18n/` | fila |
| WEB-05 | **Foco e controle:** navegação espacial, foco visível, repetição, entrada do SDL3 pela ponte | N3 | a definir na hora | WEB-02a, WEB-04a | `web/src/focus/`, `crates/tako-web/` (eventos de entrada) | fila |
| WEB-01b | **`tako-app`, segunda fatia:** configurações, coleções (manuais e automáticas resolvidas pelo id no núcleo), tags e status do jogo, favoritos | N3 | Codex Alyx · GPT-6.1-Sol medium (trocou de conta) | WEB-01a | `crates/tako-app/` | integrada |
| WEB-10a | **Estrutura do app, barra de cima e janela:** BarraSuperior com BarraDaJanela (só BIBLIOTECA, sem loja), MenuDoTako, vistas com TrocaDeTela, lista recolhível, `ui.state`, atalhos, janela pequena | N3 | Claude InvestSmart · Sonnet 5.5 alto → continuação no Codex Alyx (limite de uso) | WEB-04a, WEB-04b | `web/src/app/`, `web/src/App.tsx` | integrada (aguarda validação visual) |
| WEB-01c | **Leitura das lojas na `tako-app`:** descoberta pela `tako-sources` numa thread própria, snapshot, `library.changed` ao terminar; hoje a versão web só vê o snapshot deixado pelo Slint | N4 | Codex Alyx · GPT-6.1-Sol high | WEB-01a | `crates/tako-app/` | integrada |
| WEB-11a | **Lista da biblioteca ligada à ponte:** `library.list` paginado, abas, busca, filtros e ordenação no PainelLateral, seletor de coleção, seleção múltipla com ações (favorito, status, tags, coleção), `library.changed`, estados vazios e carregando, hospedeiro falso respondendo as mesmas chamadas | N3 | Codex Alyx · GPT-6.1-Sol medium | WEB-10a, WEB-01b | `web/src/app/`, `web/src/bridge/fake.ts` | integrada (aguarda validação visual) |
| WEB-04c | **Componentes do jogo:** CapaDoJogo (com estado sem arte), PilulaDaLoja, StatusDoJogo, EtiquetasDoJogo | N2 | Antigravity · Gemini 3.8 Flash high | WEB-03 | `web/src/components/` (só as quatro pastas) | integrada (aguarda validação visual) |
| WEB-13a | **Configurações:** estrutura (categorias e conteúdo), Geral, Aparência (aplicada na hora e lida na abertura) e Bibliotecas (lojas com liga/desliga e "ler de novo"); outras categorias em breve | N3 | Claude Alyx · Sonnet 5.5 alto | WEB-10a, WEB-01b, WEB-01c | `web/src/app/settings/`, `web/src/app/`, `web/src/bridge/fake.ts` | integrada (aguarda validação visual) |
| WEB-13b | **Configurações, demais categorias:** Tema (cartões e cor de destaque), Tela e áudio, Controles, Capas e dados, Sistema etc.; busca nas configurações | N3 | Claude Alyx · Sonnet 5.5 alto | WEB-13a | `web/src/app/settings/` | integrada (aguarda validação visual) |
| WEB-12a | **Página do jogo:** destaque (FundoDoJogo, TituloDoJogo, Destaque, AcoesDoJogo) e cartões (Sobre, Detalhes, Sessões) como componentes, página ligada ao `game.details`, Jogar, favoritar, status, troca de jogo animada, estados jogando, não instalado e sem arte | N3 | Claude Alyx · Sonnet 5.5 alto (Claude InvestSmart ainda no limite) | WEB-04c, WEB-11a | `web/src/components/`, `web/src/app/` | integrada (aguarda validação visual) |
| WEB-10b | **Visão geral da biblioteca** (TelaBiblioteca: continuar jogando, recentes, fileiras de capas), reaproveitando o destaque do WEB-12a | N3 | Claude Alyx · Sonnet 5.5 alto (Claude InvestSmart ainda no limite) | WEB-12a | `web/src/app/` | integrada (aguarda validação visual) |
| WEB-04d | **EntradaDoApp** (componente; ligar à abertura depois) | N2 | Antigravity · Gemini 3.8 Flash high | WEB-03 | `web/src/components/EntradaDoApp/` | integrada (aguarda validação visual) |
| WEB-04e | **PainelDownloads e AtividadeNaBarra** (componentes; ligar ao núcleo no WEB-14) | N2 | Antigravity · Gemini 3.8 Flash high | WEB-04a | `web/src/components/` (duas pastas) | integrada (aguarda validação visual) |
| WEB-04f | **ErroDoJogo e AtualizacaoDoApp** (componentes; ligar no WEB-14) | N2 | Antigravity · Gemini 3.8 Flash high | WEB-04a | `web/src/components/` (duas pastas) | integrada (aguarda validação visual) |
| WEB-06a | **Idiomas, infraestrutura:** `i18n.catalog` e `i18n.changed` no núcleo, `t()`/`useT()` no React, prova num texto; a migração do `strings.ts` fica para o WEB-06b | N3 | Claude Alyx · Sonnet 5.5 alto | WEB-01b | `crates/tako-app/src/i18n.rs`, `web/src/i18n/` | despachada |
| WEB-06b | **Idiomas, migração:** textos do `strings.ts` e das telas para os catálogos `locales/` (chaves novas nos dois idiomas) | N2 | a definir | WEB-06a e telas estáveis | `web/src/`, `crates/tako-core/locales/` | fila |
| WEB-01e | **Downloads pela ponte:** `downloads.list/summary/pause/resume/cancel/sources/add` e `downloads.changed`, ligando o gerenciador que já existe no `tako-core`; desligado por padrão (ADR-0028) | N3 | Claude Alyx · Sonnet 5.5 alto | WEB-01a | `crates/tako-app/` | despachada |
| WEB-04g | **CampoDeTexto e SeletorDeImagens** (componentes; ligar no WEB-14) | N2 | Antigravity · Gemini 3.8 Flash high | WEB-04a | `web/src/components/` (duas pastas) | despachada |
| WEB-04h | **ConfiguracoesDoJogo** (componente; ligar no WEB-12b) | N2 | Antigravity · Gemini 3.8 Flash high | WEB-04b | `web/src/components/ConfiguracoesDoJogo/` | despachada |
| WEB-04i | **PerfilSessoes, PerfilColecoes, PerfilFamilia** (componentes; tela do perfil no WEB-14) | N2 | Antigravity · Gemini 3.8 Flash high | WEB-04b | `web/src/components/Perfil*/` | despachada |
| WEB-01d | **Arte pelo núcleo:** "id do jogo + tipo → arquivo no cache" na `tako-app`/`tako-core`, usado pelo `tako-img://`; hoje o hospedeiro lê o `found-games.json` na abertura e monta o URI da Steam à mão (arte de jogo descoberto depois não aparece) | N3 | a definir | WEB-01c, WEB-02a | `crates/tako-app/`, `crates/tako-web/src/resources.rs` | fila |
| WEB-02b | **Interface em processo próprio (D-10):** o CEF não reinicia no mesmo processo e, fechando só a janela, ficam 74,7 MiB e 4 processos do Chromium. O núcleo (`AppService`) fica no processo que vive; a interface CEF roda num processo filho descartável, que fecha durante o jogo, minimizado e na bandeja, e reabre restaurando o `ui.state`. Meta: ≤ 40 MB e zero Chromium sem interface | N4 | Claude Alyx · Opus 5.5 alto; revisão: Antigravity + orquestrador | WEB-02a | `crates/tako-web/`, `crates/tako/` | despachada |
| WEB-02c | **Sandbox do CEF no Windows:** religar o sandbox (desligado no WEB-02a porque a crate pede outro bootstrap) | N4 | a definir | WEB-02a | `crates/tako-web/`, `crates/tako/` | fila |

Paralelismo: WEB-00 primeiro (orquestrador). Depois WEB-02a, WEB-03 e WEB-01a juntas, uma por conta (duas com
Rust). WEB-04a e WEB-04b entram assim que o WEB-03 for integrado; WEB-06 e WEB-05 quando o hospedeiro existir.

## Próximas ondas

- **Onda 2 — Telas:** WEB-10 a WEB-16, uma tela por tarefa, com validação ao vivo; hospedeiro no Linux e no macOS.
- **Em paralelo — Instalador:** INST-01 a INST-04.
- **Onda 3 — Temas, Chromium, medição e otimização, corte:** WEB-40, WEB-41, WEB-60 (com MIG-W00 e MIG-W01), WEB-50.

## Escopo

- **Fora:** telas de loja do design system (TelaLoja, TelaLojaExplorar, TelaJogoNaLoja, TelaListaDeDesejos,
  PopupOferta, PopupCupom). Decisão do mantenedor, 24/10.
- **Depois:** design system da tela cheia (Big Picture), antes do WEB-15.

## Pendências registradas

- **Código de SO na `tako-app`** (revisão do WEB-01a, item 12): o boot copiou do `build_ctx` um `#[cfg(windows)]` (DPAPI das alphas) e o adaptador `Vault`. Pelo PRD §5, código de SO mora na `tako-platform`. Mover quando a `tako-ui` sair (WEB-50), ou antes se outra tarefa mexer ali.
- **`GameDetails` incompleto para a TelaJogo:** mídia (trailer, capturas) e conquistas ainda não existem no núcleo.
- **Loop do hospedeiro precisa atender `HostRequest::PollBackground`** chamando `AppService::poll()`, inclusive sem janela (WEB-02b).
- **Textos de erro em português dentro do núcleo** (`sources.rs`: "Falha ao ler a fonte"): trocar por código e catálogo no WEB-06.
- **Filtros da biblioteca, uma escolha por campo** (WEB-11a): por "escolhas são select", o painel aceita uma loja, um status e uma etiqueta por vez; várias ao mesmo tempo pedem um componente de seleção múltipla que o design system não tem. Decisão do mantenedor.
- **Etiqueta na barra de seleção** é campo com sugestões; o design pede chip "+ Etiqueta" com MenuDeContexto. Ajustar junto da tela de tags.
- **Paginação da lista** testada só com o hospedeiro falso (13 jogos); conferir rolando uma biblioteca grande no Tako de verdade.
- **Tamanho da interface 90%:** o design system tem 90/100/110/125, mas o núcleo só aceita 100/110/125 em `TextSizePercent` (herdado do app atual). Decidir se o núcleo passa a aceitar 90.
- **Página do jogo, dados que o núcleo não tem** (WEB-12a): "jogando" (a ponte não diz se o jogo está rodando; faltam também "voltar ao jogo" e "encerrar"), instalar, trailer e capturas, conquistas, parecidos, requisitos. A página já desenha o estado "jogando" quando o dado existir. Precisam de chamadas novas na `tako-app` ligadas à sessão do hospedeiro.
- **Descrição repetida** no destaque (3 linhas) e no Sobre: o núcleo manda um texto só. Decidir se o destaque mostra um resumo próprio.
- **Visão geral, partes da prévia fora** (WEB-10b): "Voltar a jogar" (fácil, com os dados atuais), "Sua semana" e "Precisa de atenção" (precisam de horas totais e métricas que a ponte não dá). Cabeçalho sem o nome (perfil é do WEB-14). Ao voltar da página do jogo a vista remonta e recarrega; guardar rolagem e fileiras fica para depois. A lógica de Jogar repete a da página do jogo (extrair um hook comum quando mexer em uma das duas).
- **Configurações, o que ainda não tem quem leia ou não tem chamada** (WEB-13b): o Sunshine sem a prévia de confirmação do app atual; Discord e Sunshine sem ninguém lendo as chaves no caminho web; cor de destaque e opções do Liquid Glass sem chave; troca de monitor sem restaurar os presets da tela (`GameScreenPresets`); testar a imagem na tela de jogo, exportar para o Sunshine, relatório, logs, backup e desempenho sem chamadas na ponte. "Sair do modo seguro" é ação do hospedeiro (`--safe-mode`), não chave.
