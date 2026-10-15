# ADR-0016: Áudio, Steam e TV durante a sessão de jogo

- Status: Proposta
- Data: 2026-10-15

## Contexto

O MVP-002 (parte 2) pede que o Tako faça, em volta da sessão de jogo, o que o Console Mode já faz: trocar a saída de áudio para a TV e devolver depois, mandar a Steam para a bandeja, ligar a TV e trocar para a entrada do PC. Também pede um controle de volume que a Central de Controle possa usar.

Com o Tako sob AGPL-3.0-only (ADR-0014), o código do Console Mode (mesmos donos, mesma licença) pode ser portado. A [ADR-0013](0013-estado-de-monitores-e-audio.md) já definiu contratos próprios de áudio (`IAudioEndpointService`) com snapshot e restauração; nada aqui pode quebrar isso.

O acompanhamento de jogos da Steam (MVP-026) olha só os processos dentro da pasta de instalação. Isso erra quando o executável fica fora da pasta, quando o acesso ao processo é negado ou quando sobram processos (relatório de falha, launcher) depois do jogo fechar. Jogos da Steam sem pasta conhecida nem eram acompanhados.

## Decisão

1. **Onde fica cada coisa** (portado do Console Mode, adaptado ao estilo do Tako):
   - `ConsoleMode.Runtime` (puro, `net8.0;net10.0`): `Audio/AudioOutputSwitch` e `AudioVolumeControl` (em cima de `IAudioEndpointService`), `Steam/SteamClientRules` e o contrato `ISteamClient`, `Tv/*` (protocolo ADB, cliente ADB, Wake-on-LAN, `AndroidTvControl`, `AdbKeyStore`), simulações para testes.
   - `ConsoleMode.Runtime.Windows`: `WindowsSteamClient` (registro e janela da Steam) e `DpapiSecretProtector` (`CryptProtectData` direto, sem pacote novo).
   - `Launcher.Core`: `Session/Devices/GameSessionDevices` (orquestra início e fim, com log por `[LoggerMessage]`) e o sinal da Steam no `GameRunTracker`.
2. **Saída de áudio durante o jogo** (Configurações, padrão "Não alterar"): guarda o id do dispositivo e o nome (se o id sumir, acha pelo nome, desde que só um dispositivo tenha esse nome). No início do jogo, depois do snapshot da ADR-0013, a saída escolhida vira a padrão dos três papéis; se a TV está sendo ligada, espera até 15 s o dispositivo aparecer. No fim, antes da restauração da ADR-0013, cada papel volta para o dispositivo anterior **só se ainda estiver na saída do jogo** (uma troca feita pelo usuário durante o jogo fica). Assim a troca funciona com a opção "Restaurar monitores e áudio" ligada ou desligada, e a restauração da ADR-0013 não encontra diferença para corrigir.
3. **Volume**: `AudioVolumeControl` lê e ajusta a saída padrão (porcentagem, mudo, passos de 5%, trocar saída). Fica exposto em `ShellViewModel.AudioVolume` para a Central de Controle (frente C).
4. **Steam**:
   - `TrackingPlan` ganha `SteamAppId` para jogos da Steam. A cada leitura dos processos, o monitor também lê `HKCU\Software\Valve\Steam\RunningAppID`. Quando a Steam informa o jogo, ele conta como rodando mesmo sem processo visível; depois da primeira confirmação, a Steam decide o fim (processos que sobram na pasta não seguram a sessão). Se o registro não puder ser lido, volta para os processos.
   - Nova estratégia `SteamApp` para jogos da Steam sem pasta conhecida (antes ficavam sem acompanhamento).
   - Opção "Mandar a Steam para a bandeja ao sair do jogo" (padrão: desligada): no fim de um jogo da Steam, fecha só a janela do cliente com `WM_CLOSE` (nunca o processo, nunca uma janela de jogo) se a Steam está aberta e nenhum **outro** jogo está rodando. O próprio jogo ainda no registro não conta, porque a Steam demora alguns segundos para zerar o valor.
5. **TV** (padrão: desligado): Google TV / Android TV pela rede com ADB na porta 5555 ("Depuração USB" / "Depuração pela rede"), sem adb.exe. No início: conecta (Wake-on-LAN se houver MAC e a TV não responder), tecla de acordar e tecla "HDMI n" (ou um comando próprio). No fim, opcionalmente, tecla de espera. A tela das Configurações tem IP, MAC, entrada HDMI, comando avançado, espera ao sair e **Testar**, que faz o pareamento na primeira vez (a TV pergunta "Permitir depuração deste computador?").
6. **Segurança da TV**: o pareamento ADB dá um shell na TV a quem tem a chave. A chave RSA-2048 deste PC fica em `adbkey.pem` na pasta de dados, cifrada com DPAPI do usuário atual (a pasta do Tako portátil pode ser copiada). Chave ilegível (outro usuário ou PC) gera uma nova, e a TV pede permissão de novo. Só os comandos fixos (`input keyevent …`) e o comando avançado escrito pelo próprio usuário são enviados. A "Depuração sem fio" do Android 11+ (TLS com código de pareamento) não é suportada: a TV responde `STLS` e o Tako mostra como ativar o modo clássico.
7. **Convivência com a "Tela de jogo"** ([ADR-0015](0015-monitores-do-console-mode-e-tela-de-jogo.md)): com as duas ligadas, a TV é ligada (com prazo) antes de focar a tela de jogo, porque a tela só aparece com a TV na entrada do PC; depois disso a espera da própria tela de jogo cobre a imagem, sem segunda espera. A TV não é ligada de novo no início em segundo plano. No fim a ordem é: tela de jogo, saída de áudio/Steam/TV, restauração da ADR-0013.
8. **Nada disso atrasa ou derruba a sessão**: TV e áudio do início rodam fora da thread da UI com prazo (TV 45 s, espera 15 s); falhas vão para o log. O fim cancela o que ainda estiver rodando e desfaz o áudio já trocado.

## Consequências

- O comportamento real (Core Audio com `IPolicyConfig`, `RunningAppID`, janela da Steam, ADB numa TV de verdade, Wake-on-LAN) só é validado em hardware: [checklist](../marco-1/checklist-hardware.md), seção 12. O CI usa `SimulatedAudioEndpoints`, `SimulatedSteamClient` e `SimulatedTvControl`.
- Se o Tako cair durante o jogo, a saída de áudio do jogo fica; com "Restaurar monitores e áudio" ligada, a pergunta da ADR-0013 na próxima abertura cobre isso.
- Fora desta decisão (ficam para depois): ligar uma saída de áudio desativada no Windows, a escolha automática "a que aparecer ao conectar" do Console Mode, TVs que não são Android (CEC, webOS, Tizen) e a "Depuração sem fio" com TLS.
