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
| Um tema Playnite renderizado | ⚠️ Não feito | depende do adaptador P10 (ver H2) |

## Hipóteses

**H1 — Fullscreen próprio e FSE onde suportado.** *Parcialmente comprovada.*
O launcher roda em tela cheia própria, com navegação completa por controle. A integração como home app do Xbox mode existe tecnicamente (`windows.gamingApp` + capability `gamingHome`), mas **só é distribuível com SCCD assinado pela Microsoft**; sem isso exige Modo de Desenvolvedor. Decisão: o MVP sai como **prévia Fullscreen** (como o gate do épico MVP-B já previa) e a integração FSE vira trilho de parceria.

**H2 — Temas Playnite sem o Playnite.** *Parcialmente comprovada.*
O caminho seguro está pronto e em uso: o tema padrão é carregado pelo mesmo carregador restrito que temas da comunidade usarão, e um tema inválido leva à tela de recuperação sem derrubar o app (MVP-021). Falta medir temas reais com `tools/Launcher.ThemeCheck` e construir o adaptador P10 (remapear `xmlns` do Playnite, composição de `MergedDictionaries`). Nenhum tema Playnite foi renderizado ainda.

**H3 — Runtime compartilhado com o Console Mode.** *Não iniciada.*
Bloqueada pela revisão de licença (`LEGAL-001`, MIT × AGPL). O contrato `ConsoleMode.Runtime` já compila para .NET 8 e .NET 10 (ADR-0011) e o launcher usa sua máquina de estados de sessão (`SessionState`), mas nenhum código do Console Mode foi reutilizado.

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
| Licença AGPL × MIT | **Aberto e bloqueante** para H3 |
| Overlay sobre jogo em tela cheia exclusiva | Aberto: sem injeção (PRD §10), a Central de Controle precisa de janela topmost; validar em hardware |
| Navegação por controle em hardware real | CI cobre teclado/comandos; falta teste com controle físico e TV |

## Validação

- **CI (Windows):** build com warnings como erro, 36 testes do Core, 53 do carregador XAML e um **teste de fumaça da UI** que monta o app real, percorre todas as telas com comandos de controle, verifica foco e estado, falha com qualquer erro de binding e publica capturas de cada tela (artefato `ui-screenshots`).
- **Pendente em hardware:** checklist abaixo.

### Checklist no Windows (mantenedor)

1. `dotnet run --project src/Launcher.UI.Wpf -c Release`.
2. Com um controle Xbox: navegar Início → Biblioteca → Detalhes → Jogar → Guia (Central de Controle) → Encerrar, sem mouse.
3. Botão Guia: se o Windows/Game Bar capturar o botão, usar **View**.
4. Mouse: passar por cima foca, clique ativa, roda navega, clique fora fecha painéis.
5. Configurações › Carregar tema inválido → tela de recuperação → "Usar tema padrão".
6. TV 4K e monitor 1080p: o canvas 1920×1080 escala sem cortes.

## Escopo proposto para o Marco 1

Na ordem do ROADMAP: fundação (IPC, máquina de estados, SQLite, supervisor) → biblioteca real (importar executáveis, iniciar e acompanhar processos) → Fullscreen convencional + fallback para desktop → adaptador Playnite 10 com homologação de 3 temas → controle (GameInput, desconexão) → instalador e logs. Condição: resolver `LEGAL-001` antes de qualquer código do Console Mode.
