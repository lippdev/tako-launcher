# Marco 0 — Relatório de fechamento (gate)

- Data: 2026-10-14
- Escopo: [PRD §12 › Marco 0](../PRD.md) e [ROADMAP](../ROADMAP.md)
- Gate: comprovar as hipóteses técnicas ou registrar as limitações que mudam o escopo

## Entregáveis

| Entregável (PRD §12) | Situação | Onde |
|---|---|---|
| Launcher WPF inicial | ✅ Implementado | `src/Launcher.UI.Wpf` |
| Biblioteca fictícia | ✅ 19 jogos (dados reais, só URLs de arte) | `src/Launcher.Core/Library/SampleLibrary.json` |
| Navegação com controle | ✅ XInput + teclado + mouse; navegação espacial testada | `Launcher.Core/Navigation`, `Launcher.UI.Wpf/Navigation` |
| Tema padrão (MVP-013) | ✅ Tokens carregados pelo carregador XAML restrito | `src/Launcher.UI.Wpf/Themes/Default` |
| Spike FSE oficial (MVP-007) | ✅ Pesquisa concluída; experimento no Windows pendente | [`docs/spikes/MVP-007-fse.md`](../spikes/MVP-007-fse.md) |
| Relatório de riscos XAML e compatibilidade (MVP-016) | ✅ Protótipo + 53 testes | [`docs/spikes/MVP-016-xaml-restrito.md`](../spikes/MVP-016-xaml-restrito.md) |
| Um tema Playnite renderizado | ✅ Tema Fullscreen padrão do Playnite 10 renderizado sem o Playnite | Configurações › Prévia de tema Playnite 10; `src/Launcher.PlayniteCompat.P10` |

## Hipóteses

**H1 — Fullscreen próprio e FSE onde suportado.** *Parcialmente comprovada.*
O launcher roda em tela cheia própria, com navegação completa por controle. A integração como home app do Xbox mode existe tecnicamente (`windows.gamingApp` + capability `gamingHome`), mas **só é distribuível com SCCD assinado pela Microsoft**; sem isso exige Modo de Desenvolvedor. Decisão: o MVP sai como **prévia Fullscreen** (como o gate do épico MVP-B já previa) e a integração FSE vira trilho de parceria.

**H2 — Temas Playnite sem o Playnite.** *Comprovada para estilos e templates; views completas no Marco 1.*
O tema Fullscreen padrão do Playnite 10 (MIT, incluído em `Themes/Playnite-Default`) é carregado pelo carregador XAML restrito com o adaptador P10 e **renderizado com a biblioteca do launcher, sem o Playnite instalado**: grade de capas com `ListGameItemStyle`/`ListGameItemTemplate`, cores, fundos e botões do tema.

| Etapa (medida no Windows CI) | Arquivos aceitos |
|---|---|
| Validador padrão, sem adaptação | 18 de 35 |
| + declarações de namespace não usadas toleradas (e removidas antes do parser) | 22 de 35 |
| + política do adaptador P10 (validação) | 32 de 35 |
| **Carregados e aplicados pelo WPF** (composição na ordem do Playnite) | **27 de 33** |

Recusas restantes: `Popup` e `HtmlTextView` (bloqueados por segurança), `{Api}` (API do Playnite, Marco 1) e 3 views que dependem do estilo de ComboBox recusado. Achado de robustez: o WPF adia a validação de estilos/templates até o uso; o carregador agora os materializa na carga, e um erro tardio vira tela de recuperação em vez de derrubar o launcher (MVP-021).

**H3 — Runtime compartilhado com o Console Mode.** *Limitação registrada: bloqueada por decisão de licença.*
Reutilizar código do Console Mode exige resolver `LEGAL-001` (repositório MIT × Console Mode AGPL-3.0-only), decisão que cabe aos mantenedores e não pode ser tomada no código. O lado do launcher está pronto para a integração: o contrato `ConsoleMode.Runtime` compila para .NET 8 e .NET 10 (ADR-0011) e a sessão de jogo já percorre os estados de `SessionState` (`LaunchingGame → Playing → Returning → Restoring → Browsing`). Pelo gate do Marco 0, a hipótese segue para o Marco 1 condicionada a `LEGAL-001`.

## O que o Marco 0 simula (e o Marco 1 troca pelo real)

| Simulado no Marco 0 | Real no Marco 1 |
|---|---|
| "Jogar" abre uma tela com a arte do jogo | Iniciar e acompanhar o processo (MVP-026) |
| Desempenho (FPS, CPU, GPU) gerado | Leitura via PresentMon/ETW |
| Biblioteca e preferências em memória | SQLite com migrações (MVP-005) |
| Restauração de monitores/áudio descrita na UI | Console Runtime (MVP-012) |
| Controle Xbox via XInput | GameInput, PlayStation, bateria (MVP-031) |

## Riscos atualizados

| Risco | Situação |
|---|---|
| FSE não permitir launcher de terceiros | **Confirmado** sem parceria; mitigado pela prévia Fullscreen |
| XAML não confiável executar código | Mitigado: validação antes do parser, lista de permissões, 53 testes |
| Licença AGPL × MIT | **Aberto e bloqueante** para H3 (decisão dos mantenedores) |
| Temas Playnite com controles/views próprios | Mitigado: adaptador P10 cobre 27 de 33 arquivos do tema padrão; restante mapeado |
| Overlay sobre jogo em tela cheia exclusiva | Aberto: sem injeção (PRD §10), a Central de Controle precisa de janela topmost; validar em hardware |
| Navegação por controle em hardware real | CI cobre teclado/comandos; falta teste com controle físico e TV |

## Validação

- **CI (Windows):** build com warnings como erro, 36 testes do Core, 56 do carregador XAML e um **teste de fumaça da UI** que monta o app real, percorre todas as telas com comandos de controle, verifica foco e estado, falha com qualquer erro de binding e publica capturas de cada tela (artefato `ui-screenshots`).
- **Pendente em hardware:** checklist abaixo.

### Checklist no Windows (mantenedor)

1. `dotnet run --project src/Launcher.UI.Wpf -c Release`.
1. Configurações › Prévia de tema Playnite 10: a grade usa o visual do Playnite (fundo azul, seleção laranja).
2. Com um controle Xbox: navegar Início → Biblioteca → Detalhes → Jogar → Guia (Central de Controle) → Encerrar, sem mouse.
3. Botão Guia: se o Windows/Game Bar capturar o botão, usar **View**.
4. Mouse: passar por cima foca, clique ativa, roda navega, clique fora fecha painéis.
5. Configurações › Carregar tema inválido → tela de recuperação → "Usar tema padrão".
6. TV 4K e monitor 1080p: o canvas 1920×1080 escala sem cortes.

## Escopo proposto para o Marco 1

Na ordem do ROADMAP: fundação (IPC, máquina de estados, SQLite, supervisor) → biblioteca real (importar executáveis, iniciar e acompanhar processos) → Fullscreen convencional + fallback para desktop → adaptador Playnite 10 com homologação de 3 temas → controle (GameInput, desconexão) → instalador e logs. Condição: resolver `LEGAL-001` antes de qualquer código do Console Mode.
