# ADR-0025: Fronteira entre o Tako e o Console Mode

- Status: Aceita
- Data: 2026-10-16

## Contexto

Nas alphas (C#), o Tako absorveu boa parte do Console Mode: controle de TV, Steam na bandeja, contador e limite
de FPS, cortina de tela, troca de janelas, monitor de hardware, ControlFS e o menu "Modo Console" com assistente.
O PRD v0.2 (§3, D-03, D-04) desfaz isso: o Tako lança e organiza jogos; o resto é do Console Mode.

## Decisão

**Ficam no Tako** só quatro funções (D-03), mais a restauração delas no fim do jogo:

1. Troca de resolução e taxa por jogo ("tela de jogo").
2. Troca da saída de áudio por jogo.
3. Esconder o cursor durante a sessão.
4. Cursor pelo controle (analógico, cliques e rolagem).

**Saem do Tako** e continuam no Console Mode: controle de TV (ADB/Google TV, LG webOS, Samsung Tizen), Steam na
bandeja, contador e limite de FPS, cortina de tela, troca e fechamento de janelas, monitor de hardware, ControlFS,
o menu "Modo Console", o assistente de TV, "abrir direto na TV / voltar ao PC" e o atalho de emergência global.

A versão Rust simplesmente não tem essas funções. O changelog da primeira release Rust lista o que saiu e aponta
para o Console Mode.

### Contrato esperado da API do Console Mode (épico CMI)

O Tako não depende dela para funcionar. Quando existir, deve seguir isto:

- **Transporte:** o mesmo do supervisor do Tako: mensagens JSON com prefixo de tamanho (4 bytes little-endian)
  num socket local restrito ao usuário (named pipe `\\.\pipe\consolemode-api-<usuário>` no Windows; socket Unix
  `$XDG_RUNTIME_DIR/consolemode-api.sock` no Linux e `~/Library/Application Support/ConsoleMode/api.sock` no macOS).
- **Descoberta:** o Tako tenta conectar no endereço acima; sem resposta, o Console Mode não está rodando. Não há
  varredura nem rede.
- **Aperto de mão:** `{"type":"hello","client":"tako","api":1}` → `{"type":"hello","server":"consolemode","api":1,
  "version":"x.y.z","capabilities":["tv","fps","windows","console-session"]}`. Versão maior diferente = incompatível.
- **Comandos** (`{"type":"call","id":n,"method":"...","params":{...}}` → `{"type":"result","id":n,...}` ou
  `{"type":"error","id":n,"message":"..."}`): `tv.power`, `tv.input`, `fps.overlay.set`, `fps.limit.set`,
  `windows.list`, `windows.activate`, `windows.close`, `session.console.start`, `session.console.stop`.
- **Eventos** (`{"type":"event","name":"..."}`): `tv.state`, `session.console.changed`.
- **Tako como launcher do Console Mode (CMI-004):** o Console Mode abre `tako --console-session`, e o Tako avisa
  o fim da sessão pela API.

## Consequências

- O app Rust fica menor e igual nos três SOs. Usuários das alphas perdem TV e FPS até o Console Mode ter a API.
- As ADRs 0016 (TV e Steam na bandeja), 0017 (FPS) e 0018 (limite de FPS) passam a ser histórico.
- Configurações por jogo de FPS (`game_settings.fps_limit`, `show_fps`) ficam no banco, sem uso, para não perder
  dados das alphas.

## Detecção da API (CMI-002)

Socket Unix ou named pipe sem resposta resulta conservadoramente em `NaoInstalado`. Não há transporte alternativo por TCP nem arquivos locais de descoberta (`discovery.json`).
