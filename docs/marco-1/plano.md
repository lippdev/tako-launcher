# Marco 1 — plano e status (0.1.0-alpha.6)

Plano montado a partir do backlog do [PRD §5–8](../PRD.md) e executado em quatro frentes paralelas (uma PR por frente), integradas no `develop` e publicadas na release `v0.1.0-alpha.6`.

## Frentes executadas

| Frente | Itens | PR | Entregue |
|---|---|---|---|
| A · Persistência e diagnóstico | MVP-005, MVP-033 | #16 | SQLite (`tako.db`) com migrações versionadas, importação do `library.json`, configurações persistidas, arquivo corrompido não derruba o app; logs diários em `logs/` com 7 dias, sem dados sensíveis, e "Abrir pasta de logs" |
| B · Sessão de jogo | MVP-004, MVP-026, MVP-027, MVP-024 | #18 | Máquina de estados de sessão com transições explícitas; fim do jogo detectado por processo iniciado ou pela pasta do jogo (Steam); tempo jogado real; volta ao Tako com foco restaurado; editar nome, argumentos, pasta e estratégia |
| C · Temas | MVP-014, MVP-015, MVP-017, MVP-020, MVP-022 | #19 | `theme.yaml` validado (ThemeApiVersion 2.x); importação `.pthm` segura; Configurações › Temas; herança do tema padrão; homologação de 3 temas reais ([matriz](homologacao-temas.md)) |
| D · Resiliência e sistema | MVP-008, MVP-010, MVP-011, MVP-021, MVP-029, MVP-031, MVP-034, MVP-036 | #17 | Modo seguro após travamentos seguidos; tela "Algo deu errado" com saída para o desktop; iniciar com o Windows; detecção do Xbox mode sem falso positivo; foco verificado em todas as telas; reconexão de controle; [checklist de hardware](checklist-hardware.md) |

Antes das frentes, no mesmo ciclo: biblioteca real da Steam + jogos adicionados + metadados (#13, MVP-023/024/025), instalador e portátil (#14, MVP-032) e atualização automática (#15).

## Status do backlog do Marco 1

| Item | Status |
|---|---|
| MVP-001 estrutura · MVP-013 tema padrão · MVP-016 XAML restrito · MVP-023 biblioteca · MVP-025 detalhes · MVP-028 navegação | ✅ |
| MVP-004 · MVP-005 · MVP-010 · MVP-011 · MVP-014 · MVP-015 · MVP-020 · MVP-021 · MVP-024 · MVP-026 · MVP-027 · MVP-029 · MVP-031 · MVP-032 · MVP-033 · MVP-036 | ✅ (validação final em hardware pelo [checklist](checklist-hardware.md)) |
| MVP-008 detectar FSE | ✅ classificação sem falso positivo; "suportado" só confirmado em hardware |
| MVP-017 · MVP-018 · MVP-019 temas Playnite | 🟡 herança e adaptadores de estilos/templates prontos; telas `Main`/`GameDetails` dependem da API do Playnite (`{Api}`, `{PluginSettings}`, `{SettingsBinding}`) |
| MVP-022 homologação | ✅ 3 temas reais medidos; aplicar tema Playnite à interface inteira segue para MVP-018/019 |
| MVP-034 testes e checklists | 🟡 CI completo; checklist pronto, execução no Windows real pendente |
| MVP-012 restaurar monitores/áudio | 🟡 contratos e implementação próprios, sem código do Console Mode ([ADR-0013](../adr/0013-estado-de-monitores-e-audio.md)); validação em hardware pelo [checklist](checklist-hardware.md) §9 |
| MVP-002 · MVP-003 · MVP-006 runtime compartilhado, IPC, supervisor | ⛔ dependem da decisão [LEGAL-001](../legal/LEGAL-001.md) (reuso do Console Mode); MVP-002 já tem os contratos próprios de monitores e áudio (ADR-0013) |
| MVP-007 · MVP-009 FSE oficial | ⛔ dependem do SCCD assinado pela Microsoft e do experimento em hardware ([spike](../spikes/MVP-007-fse.md)) |
| MVP-035 benchmark | ⛔ precisa das máquinas de referência |

## Próximos passos
1. Rodar o [checklist de hardware](checklist-hardware.md) no Windows 11 com a `0.1.0-alpha.6` (a primeira que se atualiza sozinha).
2. Decidir o LEGAL-001 para destravar o runtime (MVP-002/003/006/012).
3. Cobertura de temas: implementar `{PluginSettings}`, `{SettingsBinding}`, `{ThemeFileBinding}` e liberar `SplineThicknessKeyFrame` (sugestões da homologação), depois aplicar o tema Playnite à interface inteira (MVP-018/019).
