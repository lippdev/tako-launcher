# ADR-0015: Monitores vindos do Console Mode e "Tela de jogo"

- Status: Proposta
- Data: 2026-10-15
- Complementa: [ADR-0013](0013-estado-de-monitores-e-audio.md) (a parte de áudio não muda)
- Depende de: [ADR-0014](0014-licenca-agpl.md) (Tako sob AGPL-3.0-only)

## Contexto

A ADR-0013 escreveu do zero os contratos e a implementação Windows de monitores porque o código do Console Mode (AGPL-3.0-only) não podia ser copiado. Com o Tako também AGPL-3.0-only (ADR-0014) e os mesmos donos, o reuso previsto no PRD (§4.2, MVP-002) ficou liberado.

O Console Mode tem uma pilha de monitores testada em campo (`Native/NativeDisplays.cs`, `Native/CcdHelper.cs`, `Native/BlackCurtain.cs`, `Services/DisplayIdentity.cs`, `Services/DisplayBackend.cs`, `Services/MonitorService.cs`, `Services/VideoFeaturesService.cs`) e o recurso que mais pedem ao Tako: escolher a tela onde se joga (a TV) e deixar só ela ligada durante o jogo.

## Decisão

1. **Contratos da ADR-0013 mantidos.** `DisplayTopology`, `MonitorState`, `IDisplayConfigurationService` e o snapshot/restauração do `SessionEnvironmentGuard` continuam iguais, com o mesmo formato JSON (versão 1). O `Id` do monitor continua sendo o caminho do dispositivo.
2. **Lógica pura portada para `ConsoleMode.Runtime`** (testável no Mac/CI): `DisplayIdentity` (caminho do dispositivo, ID no formato do Console Mode, EDID, escolha de fonte de vídeo para religar um monitor com o mesmo `\\.\DISPLAYn`). Contratos novos: `IDisplayControlService` (listar conectados, inclusive desligados; ligar; desconectar; principal; resolução/taxa; HDR), `IScreenCurtain`, `DisplayMode`, `ConnectedDisplay`, `GameScreenSettings` e `GameScreenPlanner`. Simulação em `SimulatedDisplayDesk`/`SimulatedScreenCurtain`. Os testes de lógica pura do Console Mode (`DisplayIdentityTests`) vieram junto; o formato `.cfg` de backup da versão 1.5 do Console Mode não foi portado (o Tako guarda o estado pela ADR-0013).
3. **Implementação Windows portada para `ConsoleMode.Runtime.Windows`**, no estilo do Tako (`LibraryImport`, estruturas por ponteiro, falhas como `RuntimeEnvironmentException`):
   - `WindowsDisplayControl`: listagem por `QDC_ALL_PATHS` com a escolha de fontes do Console Mode, ligar (caminho inativo + `SetDisplayConfig`; último recurso: estender para todos), desconectar (recusa deixar o PC sem tela), principal por deslocamento das fontes, modo por `ChangeDisplaySettingsEx` e HDR por `DisplayConfigSetDeviceInfo` (cor avançada).
   - `WindowsScreenCurtain`: a cortina preta do Console Mode, numa thread própria com fila de mensagens e janelas que não roubam o foco.
   - `WindowsDisplayConfiguration` (ADR-0013) **reforçado**: a restauração exata pela configuração CCD guardada continua sendo o primeiro caminho; se falhar, antes do caminho GDI monitor a monitor, religa os monitores que estavam ativos e desconecta os que não estavam (ordem do `RestoreBackup` do Console Mode). Antes, um monitor desligado não voltava nesse caminho.
   - Consultas CCD compartilhadas em `Ccd` (antes duplicadas).
4. **"Tela de jogo"** (`Launcher.Core.Session.GameScreen.GameScreenSession`), desligada por padrão, em Configurações: tela, "Outras telas" (desconectar ou cobrir de preto), resolução e taxa (ou manter a atual) e ligar HDR. Ao iniciar um jogo real, depois do snapshot do MVP-012 e antes de abrir o processo, na ordem do Console Mode: liga a tela escolhida e espera ela aparecer (até 8 s, com nova tentativa; sem ela, volta tudo e nunca apaga as outras), torna-a principal, esconde as outras, aplica o modo (recusado: segue no modo atual) e liga o HDR. O Tako vai para a tela de jogo. Ao terminar (ou se o jogo não abrir, não puder ser acompanhado ou não iniciar no prazo), tira a cortina, desliga o HDR que ligou e devolve a topologia lida antes, conferindo o resultado (até duas tentativas). Só depois o `SessionEnvironmentGuard` compara e restaura o resto.
5. **Confirmação "Você consegue ver esta tela?"** na primeira vez com uma combinação tela + modo (`id|LxA@Hz` ou `id|current`, a mesma chave do Console Mode). 15 s para responder com controle, teclado ou mouse; sem resposta, "Voltar ao normal" ou B/Esc, as telas voltam e o jogo não abre. A combinação confirmada fica salva no `tako.db`.

## Consequências

- O MVP-002 (parte de monitores) passa a usar o código do Console Mode; cada arquivo adaptado diz de onde veio. Correções feitas lá podem ser trazidas para cá arquivo a arquivo.
- Fica fora desta versão: abrir o Tako direto "em modo console" na tela de jogo (hoje ela só vale durante o jogo), desligar telas por DDC/CI, VRR, ligar a TV pela rede e mover a janela do jogo para a tela de jogo (o jogo abre na principal, que já é a tela de jogo).
- Se o Tako cair com a tela de jogo em foco, a volta depende da oferta de restauração da ADR-0013 (opção "Restaurar monitores e áudio ao sair do jogo", ligada por padrão); com ela desligada, as telas ficam como estão até o usuário arrumar. A cortina preta some com o processo.
- A tela de jogo roda fora da thread da UI ao entrar; a volta ainda roda na thread da UI (como a restauração da ADR-0013) e pode congelar a tela por alguns segundos.
- O CI usa só a simulação. O comportamento real (CCD, cortina, HDR, TV que demora a acordar, drivers que reorganizam as telas) só é validado em hardware ([checklist](../marco-1/checklist-hardware.md), seção "Tela de jogo").
