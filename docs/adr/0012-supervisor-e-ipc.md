# ADR-0012: Supervisor da UI no próprio Tako.exe e IPC versionado por Named Pipes

- Status: Proposta
- Data: 2026-10-15
- Itens: MVP-003 (IPC), MVP-006 (supervisor)

## Contexto

O PRD (§4.1, ADR-005) separa Host, UI e runtime em processos para que uma falha da UI não deixe o usuário preso numa tela cheia vazia, e pede IPC por Named Pipes com JSON-RPC versionado (§4.3, CONTRIBUTING §6.4). Restrições:

- A distribuição é **um único `Tako.exe`**: o portátil é um .exe só, o instalador aponta para `Tako.exe`, o "iniciar com o Windows" grava o caminho desse executável e o atualizador troca esse arquivo.
- `ConsoleMode.Runtime` tem alvos `net8.0;net10.0` e só código sem dependência de plataforma (ADR-0011).
- O código do Console Mode (AGPL) não pode ser copiado enquanto o [LEGAL-001](../legal/LEGAL-001.md) estiver aberto. Tudo aqui foi escrito do zero.
- Já existem o modo seguro (`--safe-mode`, `SessionJournal`/`CrashLoopPolicy`) e a rede de segurança da thread da UI (`CrashGuard`).

## Decisão

### 1. Um executável, dois papéis

`Tako.exe` ganhou um `Main` próprio (`Launcher.UI.Wpf.Program`, `<StartupObject>`):

| Como foi aberto | Papel |
|---|---|
| Normalmente (atalho, instalador, "iniciar com o Windows", script do portátil) em versão publicada | **Supervisor**: sem janela e sem carregar WPF; abre o mesmo executável com `--ui --supervisor-pipe <nome>` |
| Com `--ui` | **UI** (WPF), ligada ao supervisor pelo pipe |
| `dotnet run`, testes, `--no-supervisor`, `TAKO_SUPERVISOR=0` | **UI** sozinha, sem supervisor (caminho de desenvolvimento e do teste de fumaça) |
| `TAKO_SUPERVISOR=1` | Supervisor mesmo fora da versão publicada (testar localmente) |

A decisão do papel é pura (`SupervisorArguments.ResolveRole`) e testada. Argumentos do usuário (ex.: `--safe-mode`) passam para a UI.

### 2. Supervisor (MVP-006)

- A UI manda `ui.heartbeat` a cada 2 s por um `DispatcherTimer` — só sai se a thread da UI estiver respondendo.
- O supervisor (`UiSupervisor`) considera a UI **travada** sem heartbeat por 20 s (60 s até o primeiro, porque a primeira abertura do .exe único extrai arquivos). Um buraco grande entre as próprias conferências (PC suspenso/hibernado) recomeça o prazo em vez de matar uma UI saudável (`HeartbeatMonitor`).
- UI travada é encerrada (só ela: `Kill(entireProcessTree: false)`, os jogos continuam) e conta como falha; código de saída ≠ 0 também (inclusive o "voltar ao desktop" do `CrashGuard`).
- Política (`SupervisorPolicy`, pura): até **3 reinícios em 5 minutos** no modo normal; na 4ª falha, **uma abertura em `--safe-mode`**; se ela falhar, o supervisor **desiste** (sai com código 1) e o usuário fica no desktop. Falhas fora da janela não somam. `Tako.exe --safe-mode` começa no último degrau.
- **Saída limpa** (código 0: fechar a janela, "Sair para o desktop") encerra o supervisor junto.
- **Saída anunciada** (`ui.closing`): a UI avisa antes de sair de propósito, e o supervisor não reabre mesmo com código ≠ 0 (o instalador pode forçar o fechamento):
  - `update`: enviado pelo `UpdateService` depois de iniciar o setup/script. Se a UI não sair em 30 s, o supervisor a encerra e sai.
  - `sessionEnding`: `Application.SessionEnding` (logoff/desligamento ou Restart Manager). O Windows pode cancelar o fim da sessão, então a UI continua vigiada; só não é reaberta quando sair.
- O supervisor registra tudo em `logs/supervisor/tako-AAAAMMDD.log` (pasta separada: dois processos não escrevem no mesmo arquivo). A UI registra no log dela quando foi reaberta (`supervisor.status`).
- A UI e o `SessionJournal` continuam como antes: a UI reaberta após duas saídas sujas já entra sozinha no modo seguro; o supervisor só garante que alguém a reabra e que a escada termine no desktop.

### 3. Atualizador, instalador e "iniciar com o Windows"

- **Portátil**: o script de troca espera **a UI e o supervisor** saírem (`PortableSwapScript`, PID do supervisor vem do handshake) antes de mover o arquivo; `start Tako.exe` reabre pelo supervisor.
- **Instalado**: o setup roda com `/CLOSEAPPLICATIONS /FORCECLOSEAPPLICATIONS` e o `Tako.iss` usa `CloseApplications=force`: o supervisor não tem janela, então o Restart Manager não consegue fechá-lo "educadamente" (na prática ele já saiu junto com a UI). O `[Run]` do instalador abre `Tako.exe` → supervisor.
- **Iniciar com o Windows**: o valor `Run` continua apontando para `Tako.exe` sem argumentos → abre o supervisor. Nada mudou no registro.

### 4. Protocolo IPC (MVP-003)

Contratos e implementação em `ConsoleMode.Runtime.Ipc` (`net8.0;net10.0`, só BCL: `System.IO.Pipes` e `System.Text.Json`), para o Console Mode poder falar o mesmo protocolo (H3). As mensagens do supervisor (`ui.*`, `supervisor.*`) são do Tako e ficam em `Launcher.Core.Supervision`.

**Transporte.** Named Pipe com `PipeOptions.CurrentUserOnly` (no Windows, o pipe só aceita o mesmo usuário e o cliente confere o dono do servidor; no macOS/Linux o .NET usa socket de domínio Unix). Nome aleatório por execução (`tako-sv-<pid>-<8 hex>`), passado à UI na linha de comando. O stream é abstraído (`IIpcClientTransport`/`IIpcServerTransport`): os testes rodam em memória e também com pipe real (Windows, macOS e Linux).

**Quadros.** `[tamanho Int32 little-endian][JSON UTF-8]`, máximo 1 MiB; tamanho inválido ou JSON inválido fecha a conexão (o outro lado é dado não confiável).

**Envelope** (inspirado em JSON-RPC 2.0, com tipo explícito e cabeçalho de versão):

```json
{ "v": 1, "kind": "request", "id": 7, "method": "supervisor.status", "payload": { } }
{ "v": 1, "kind": "response", "id": 7, "payload": { "processId": 1234, "restarts": 0, "mode": "normal" } }
{ "v": 1, "kind": "response", "id": 8, "error": { "code": "method_not_found", "message": "..." } }
{ "v": 1, "kind": "event", "method": "ui.heartbeat", "payload": { "processId": 4321, "sequence": 12 } }
```

- `kind`: `hello`, `welcome`, `reject` (handshake), `request`, `response`, `event`.
- `id`: correlação por conexão; as respostas podem chegar **em qualquer ordem** (comandos são atendidos em paralelo). Resposta depois do prazo, ou com id desconhecido, é descartada e contada.
- Eventos são entregues na ordem de chegada; evento sem tratador é ignorado (versões novas podem emitir eventos novos).
- Códigos de erro estáveis: `version_mismatch`, `invalid_message`, `method_not_found`, `handler_failed`.

**Negociação de versão.** O cliente envia `hello` com a faixa que aceita (`minVersion`–`maxVersion`), nome e PID; o servidor escolhe a **maior versão em comum** e responde `welcome` (com nome e PID dele). Sem versão em comum, responde `reject` com `version_mismatch` e uma mensagem que cita as duas faixas ("Versão do protocolo IPC incompatível: 'tako-supervisor' aceita 1–1 e 'tako-ui' aceita 2–3...") e fecha. Depois do handshake, todo quadro precisa ter `v` igual à versão negociada. Hoje: versão 1, mínima 1. Mudança incompatível de contrato incrementa `IpcProtocol.CurrentVersion` e exige teste de contrato (CONTRIBUTING §6.4).

**Prazos e reconexão.** Comandos têm prazo (padrão 5 s; `TimeoutException`), o handshake também (5 s). O `IpcClient` reconecta sozinho quando a conexão cai (servidor reiniciou), com espera crescente de 200 ms até 5 s, e avisa a cada conexão (`Connected`) para quem precisa refazer estado. Versão incompatível **para** as tentativas (`Faulted`). Eventos não ficam em fila enquanto desconectado (o próximo heartbeat substitui o perdido); comando em andamento quando a conexão cai falha com `IpcDisconnectedException`.

## Alternativas consideradas

- **`Launcher.Host.exe` separado como supervisor.** Mais simples de isolar, mas quebra o .exe único do portátil, muda o alvo do instalador e do "iniciar com o Windows" e obriga o atualizador a trocar dois arquivos. Rejeitada; o projeto `Launcher.Host` segue como placeholder.
- **Supervisor dentro do processo da UI (thread de watchdog).** Não sobrevive a um crash do processo nem detecta de forma confiável a thread da UI travada sem matar o próprio processo. Rejeitada.
- **Agendador de tarefas / serviço do Windows reiniciando a UI.** Exige permissões, não funciona no portátil e não decide o modo seguro. Rejeitada.
- **StreamJsonRpc (JSON-RPC 2.0 completo).** Dependência grande para dois processos do mesmo produto, sem negociação de versão pronta; o envelope próprio cabe em poucas centenas de linhas testadas e roda em `net8.0`. Se o runtime crescer para muitos comandos, reavaliar.
- **gRPC / sockets TCP locais.** Abrem porta de rede local e precisam de mais infraestrutura; Named Pipes com `CurrentUserOnly` já restringem ao usuário.
- **Heartbeat como comando com resposta.** Dobra o tráfego sem ganho: o supervisor só precisa saber que a UI está viva.

## Consequências

- Em versões publicadas há dois processos `Tako.exe` (supervisor leve + UI). Encerrar só a UI pelo Gerenciador de Tarefas faz o supervisor reabri-la (até a escada terminar no desktop); para fechar de vez, feche o Tako normalmente ou encerre o supervisor.
- A política e o monitor de heartbeat são puros e testados (`SupervisorPolicyTests`); o supervisor é testado de ponta a ponta com pipe real e processos simulados (`UiSupervisorTests`); o protocolo tem testes de contrato (`ConsoleMode.Runtime.Tests`: versão, prazo, reconexão, fora de ordem, mensagens inválidas). O teste de fumaça da UI confere os heartbeats saindo da thread da UI.
- Validação no Windows real (reinício após travar/cair, atualização portátil e instalada com supervisor, logoff) entra no checklist de hardware.
