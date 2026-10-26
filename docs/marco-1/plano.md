# Marco 1 — plano e status (0.1.0-alpha.8)

Plano montado a partir do backlog do [PRD §5–8](../PRD.md) e executado por agentes em frentes paralelas (uma PR por frente), integradas no `develop`. Primeira rodada publicada na `v0.1.0-alpha.6`; segunda rodada (itens que estavam parciais ou bloqueados mas podiam ser feitos com código próprio) na `v0.1.0-alpha.7`; terceira rodada (reuso do Console Mode depois do LEGAL-001, desempenho e busca) na `v0.1.0-alpha.8`.

## Frentes executadas

| Frente | Itens | PR | Entregue |
|---|---|---|---|
| A · Persistência e diagnóstico | MVP-005, MVP-033 | #16 | SQLite (`tako.db`) com migrações versionadas, importação do `library.json`, configurações persistidas, arquivo corrompido não derruba o app; logs diários em `logs/` com 7 dias, sem dados sensíveis, e "Abrir pasta de logs" |
| B · Sessão de jogo | MVP-004, MVP-026, MVP-027, MVP-024 | #18 | Máquina de estados de sessão com transições explícitas; fim do jogo detectado por processo iniciado ou pela pasta do jogo (Steam); tempo jogado real; volta ao Tako com foco restaurado; editar nome, argumentos, pasta e estratégia |
| C · Temas | MVP-014, MVP-015, MVP-017, MVP-020, MVP-022 | #19 | `theme.yaml` validado (ThemeApiVersion 2.x); importação `.pthm` segura; Configurações › Temas; herança do tema padrão; homologação de 3 temas reais ([matriz](homologacao-temas.md)) |
| D · Resiliência e sistema | MVP-008, MVP-010, MVP-011, MVP-021, MVP-029, MVP-031, MVP-034, MVP-036 | #17 | Modo seguro após travamentos seguidos; tela "Algo deu errado" com saída para o desktop; iniciar com o Windows; detecção do Xbox mode sem falso positivo; foco verificado em todas as telas; reconexão de controle; [checklist de hardware](checklist-hardware.md) |

### Segunda rodada (0.1.0-alpha.7)

| Frente | Itens | PR | Entregue |
|---|---|---|---|
| E1 · IPC e supervisor | MVP-003, MVP-006 | #23 | IPC por named pipes com protocolo versionado, reconexão e só o usuário atual; `Tako.exe` vira supervisor sem janela que reinicia a UI travada/caída, depois modo seguro, depois desktop ([ADR-0012](../adr/0012-supervisor-e-ipc.md)) |
| E2 · Monitores e áudio | MVP-012, contratos do MVP-002 | #22 | Contratos próprios de monitores e áudio; snapshot antes do jogo e restauração só do que mudou ao terminar; recuperação se o Tako cair durante o jogo ([ADR-0013](../adr/0013-estado-de-monitores-e-audio.md)) |
| F · Tema Playnite na interface | MVP-017, MVP-018, MVP-019 | #24 | Tema Playnite aplicado à tela inicial inteira, com `{Api}` só-leitura, `PART_` e fallback sem loop |
| G · Benchmark e diagnóstico | MVP-035, MVP-034 | #21 | `tools/Launcher.Bench` + job no CI com linha de base; Configurações › Diagnóstico do sistema |
| H · Segurança de temas | MVP-016, MVP-018 | #25 | Guarda que bloqueia imagens/mídia de fora do tema, URLs e UNC montados em tempo de execução (fecha a questão 8 do spike) |

### Terceira rodada (0.1.0-alpha.8)

LEGAL-001 decidido em 2026-10-15: o Tako passa a ser AGPL-3.0-only e reaproveita o Console Mode ([ADR-0014](../adr/0014-licenca-agpl.md)). Assinaturas da Microsoft ficam para depois.

| Frente | Itens | PR | Entregue |
|---|---|---|---|
| Licença | LEGAL-001 | #27 | AGPL-3.0-only, CONTRIBUTING §10, ADR-0014 |
| R1 · Monitores do Console Mode | MVP-002, MVP-012 | #30 | Pilha de monitores do Console Mode (CCD, EDID, identidade, HDR, cortina preta) atrás dos contratos do ADR-0013; Tela de jogo com confirmação e reversão ([ADR-0015](../adr/0015-monitores-do-console-mode-e-tela-de-jogo.md)) |
| R2 · Áudio, Steam e TV | MVP-002, MVP-004 | #31 | Saída de áudio durante o jogo, volume, `RunningAppID` da Steam na sessão, Steam para a bandeja, Google/Android TV por ADB e Wake-on-LAN ([ADR-0016](../adr/0016-audio-steam-e-tv-na-sessao.md)) |
| R3 · Central de Controle | MVP-002 | #33 | Janelas abertas, desempenho real, RTSS (contador e limite), ponteiro pelo controle, ControlFS |
| R4 · Desempenho | MVP-035 | #29 | Grade virtualizada (Tako e temas Playnite), capas sob demanda com cache limitado; 1000 jogos: 507 → 170 MB parado, abrir a biblioteca 1852 → 77–167 ms |
| R5 · Busca e filtros | MVP-017, MVP-018, MVP-019 | #32 | Busca sem acento com teclado na tela, filtros e ordenação no Core, ligados também ao painel de filtros dos temas Playnite |
| Distribuição | MVP-032 | #28 | Portátil comprimido: 141 → 69 MB |

Antes das frentes, no mesmo ciclo: biblioteca real da Steam + jogos adicionados + metadados (#13, MVP-023/024/025), instalador e portátil (#14, MVP-032) e atualização automática (#15).

## Status do backlog do Marco 1

| Item | Status |
|---|---|
| MVP-001 estrutura · MVP-013 tema padrão · MVP-016 XAML restrito · MVP-023 biblioteca · MVP-025 detalhes · MVP-028 navegação | ✅ |
| MVP-004 · MVP-005 · MVP-010 · MVP-011 · MVP-014 · MVP-015 · MVP-020 · MVP-021 · MVP-024 · MVP-026 · MVP-027 · MVP-029 · MVP-031 · MVP-032 · MVP-033 · MVP-036 | ✅ (validação final em hardware pelo [checklist](checklist-hardware.md)) |
| MVP-008 detectar FSE | ✅ classificação sem falso positivo; "suportado" só confirmado em hardware |
| MVP-017 · MVP-018 · MVP-019 temas Playnite | ✅ tema Playnite aplicado à tela inicial (`Main.xaml`, `PART_`, `{Api}` só-leitura, `{PluginSettings}`, `{SettingsBinding}`, `{ThemeFileBinding}`, `HtmlTextView` seguro), com fallback (MVP-021) ([matriz](homologacao-temas.md)); validação em hardware pendente |
| MVP-022 homologação | ✅ 3 temas reais medidos e aplicados à interface inteira (Main.xaml do tema no Toggle e no Revisited; do padrão no PS5ish) |
| MVP-034 testes e checklists | 🟡 CI completo; checklist e Configurações › Diagnóstico do sistema prontos; execução no Windows real pendente |
| MVP-012 restaurar monitores/áudio | ✅ contratos e implementação próprios, sem código do Console Mode ([ADR-0013](../adr/0013-estado-de-monitores-e-audio.md)); validação em hardware pelo [checklist](checklist-hardware.md) §9 |
| MVP-003 IPC · MVP-006 supervisor | ✅ implementação própria ([ADR-0012](../adr/0012-supervisor-e-ipc.md)); validação em hardware pelo [checklist](checklist-hardware.md) §10 |
| MVP-002 extrair contratos do Console Runtime | ✅ código de monitores, áudio, Steam, TV e menu de sessão do Console Mode portado para `ConsoleMode.Runtime(.Windows)` atrás dos contratos (#30, #31, #33); validação em hardware pelo [checklist](checklist-hardware.md) |
| MVP-007 · MVP-009 FSE oficial | ⏸ adiado por decisão dos mantenedores; dependem do SCCD assinado pela Microsoft e do experimento em hardware ([spike](../spikes/MVP-007-fse.md)) |
| MVP-035 benchmark | 🟡 [`tools/Launcher.Bench`](../../tools/Launcher.Bench) e job `benchmark` no CI com a [linha de base do runner](benchmark.md); números de referência dependem do hardware real |

## O que não dá para concluir só com código
- **MVP-007 · MVP-009** (FSE oficial): o SCCD de produção é emitido pela Microsoft; o experimento precisa de hardware com Xbox mode.
- **Validação em hardware** de tudo marcado acima (checklist), números de referência do benchmark (MVP-035) e confirmação de "suportado" na detecção do FSE (MVP-008).

## Próximos passos
1. Rodar o [checklist de hardware](checklist-hardware.md) no Windows 11 com a `0.1.0-alpha.8`, incluindo as seções novas (tela de jogo, Central de Controle, áudio/Steam/TV), anexando o relatório de Configurações › Diagnóstico do sistema.
2. Confirmar o limite de FPS pela `RTSSHooks64.dll` e o ADB numa TV real (pontos mais incertos da rodada).
3. Pedir à Microsoft o SCCD de produção (MVP-007/009) quando o desenvolvimento estabilizar.
