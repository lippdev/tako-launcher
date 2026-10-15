# ADR-0013: Estado de monitores e áudio da sessão

- Status: Proposta
- Data: 2026-10-15

## Contexto

O MVP-012 pede que monitores, resolução e áudio voltem ao estado registrado antes da sessão de jogo. O PRD (§4.2, MVP-002) previa reaproveitar os serviços de monitores e áudio do Console Mode, mas o Console Mode é AGPL-3.0-only e a decisão sobre reuso ([LEGAL-001](../legal/LEGAL-001.md)) ainda está pendente. O CONTRIBUTING proíbe copiar código do Console Mode até lá.

Ao abrir um jogo, ele pode trocar a resolução, a taxa de atualização, o monitor principal, a saída de áudio ou o volume, e nem sempre desfaz isso ao fechar. Se o Tako cair durante o jogo, ninguém restaura.

## Decisão

1. **Contratos próprios em `ConsoleMode.Runtime`**, escritos do zero, sem consultar o código do Console Mode. Em conceito são compatíveis com ele: um snapshot do estado antes da sessão e a restauração depois.
   - `Display`: `DisplayTopology` (monitores ativos), `MonitorState` (id estável, principal, posição, resolução, `RefreshRate` como fração, rotação) e `IDisplayConfigurationService` (`Capture`/`Apply`). `PlatformData` é opcional e opaco: a implementação da plataforma guarda ali o que precisa para uma restauração exata.
   - `Audio`: `AudioState` (dispositivos ativos, padrão por papel `Console`/`Multimedia`/`Communications`, volume/mudo de cada dispositivo) e `IAudioEndpointService`.
   - `SessionEnvironmentSnapshot` com versão de esquema, serializado em JSON (`SessionEnvironmentSnapshotJson`; versão desconhecida ou arquivo incompleto são recusados), e comparação por equivalência (ignora ordem, nomes, arredondamento de taxa e volume dentro de 1%).
   - `Simulation`: implementações em memória para testes, teste de fumaça e CI.
   - Sem dependência de plataforma, com alvos `net8.0;net10.0` (ADR-0011).
2. **Lógica pura no Core** (`Launcher.Core.Session.Restore`): `EnvironmentDiff` (o que mudou), `RestorePlanner` (o que restaurar) e `SessionEnvironmentGuard` (registra, persiste, restaura, verifica e registra no log). Regras do plano:
   - restaura só o que mudou; se nada mudou, nenhuma chamada ao Windows;
   - monitores primeiro, num passo só (topologia inteira), depois dispositivo padrão, depois volume e mudo;
   - dispositivo desconectado ou troca de padrão indisponível: não restaura e registra o motivo;
   - para voltar ao mudo, silencia antes de mexer no volume; para tirar do mudo, ajusta o volume antes (sem pico de som);
   - leitura que falha numa parte não impede a outra; parte ausente nunca conta como diferença;
   - depois de aplicar, lê de novo e compara o que foi planejado (verificação no log).
3. **Implementação Windows em projeto separado** `ConsoleMode.Runtime.Windows` (`net10.0-windows`, como previsto na ADR-0011), também escrita do zero a partir da documentação pública:
   - Monitores: `QueryDisplayConfig`/`SetDisplayConfig` (CCD). A configuração ativa inteira (caminhos e modos) vai para `PlatformData` e é devolvida ao `SetDisplayConfig` (`SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_APPLY | SDC_SAVE_TO_DATABASE | SDC_ALLOW_CHANGES`). Se falhar (ex.: depois de reiniciar o PC os LUIDs dos adaptadores mudam), restaura monitor a monitor com `EnumDisplaySettingsEx`/`ChangeDisplaySettingsEx`; nesse caminho, um monitor desligado não pode ser religado e a falha vai para o log.
   - Áudio: Core Audio (`IMMDeviceEnumerator`, `IAudioEndpointVolume`) com `[GeneratedComInterface]` e P/Invoke por `LibraryImport`.
   - **Risco aceito:** trocar o dispositivo padrão só é possível pela interface **não documentada** `IPolicyConfig` (usada há anos por ferramentas de troca de saída de áudio). Ela fica isolada em `AudioNative.cs`, é criada sob demanda e, se não existir ou falhar, `CanSetDefaultDevice` fica falso: o plano deixa de trocar o padrão (com motivo no log) e volume/mudo continuam. Uma atualização do Windows pode quebrá-la; o caso está no checklist de hardware.
4. **Ciclo da sessão** (`ShellViewModel`): ao iniciar um jogo real (antes de `LaunchingGame` abrir o processo) registra o snapshot e grava `session-environment.json` na pasta de dados; no estado `Restoring` restaura e apaga o arquivo. Falha ao abrir, jogo sem como ser acompanhado ou que não iniciou no prazo descartam o registro. Se o arquivo sobrar (o Tako caiu), a próxima abertura compara com o estado atual e, havendo diferença, pergunta "Restaurar monitores e áudio como estavam antes do jogo"; nada muda sem confirmação.
5. Opção **"Restaurar monitores e áudio ao sair do jogo"** em Configurações, ligada por padrão e salva no `tako.db`.

## Consequências

- O MVP-012 funciona sem depender do LEGAL-001. **A integração com o Console Mode (MVP-002/003) continua dependendo do LEGAL-001**: quando a decisão sair, os contratos daqui podem ser adotados pelo Console Mode ou substituídos pelos extraídos dele; o formato JSON tem versão para essa migração.
- A restauração roda hoje dentro do processo da UI, no estado `Restoring`. A regra "a UI não restaura monitores/áudio; o runtime restaura e sobrevive à falha da UI" (CONTRIBUTING §6.3) é atendida em parte: o snapshot em disco cobre a queda da UI, mas a restauração automática sem a UI só vem com o supervisor/runtime fora de processo (MVP-003/006).
- A restauração de monitores roda na thread da UI e pode congelar a tela por até alguns segundos enquanto o Windows troca o modo.
- O CI nunca altera monitores ou áudio: testes e teste de fumaça usam `Simulation`. O comportamento real (CCD, Core Audio, `IPolicyConfig`) só é validado em hardware ([checklist](../marco-1/checklist-hardware.md), seção "Restaurar monitores e áudio").
