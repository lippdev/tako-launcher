# Hospedeiro CEF no Windows (WEB-02a, WEB-02b)

O CEF entra no binário apenas pela feature optativa `web`, desligada por padrão. Com ela, `tako --web`
abre a interface. Sem `--web`, o Slint continua. Sem a feature, `--web` termina com código 1 e a mensagem
“compilado sem a interface web”. Contrato: [ponte.md](ponte.md).

## Dois processos: núcleo e interface descartável (WEB-02b, D-10)

| Processo | Linha | Tem | Não tem |
|---|---|---|---|
| Núcleo | `tako --web [--data-dir …]` | `Core`, `AppService`, `ui.state`, leitura das lojas, ciclo de vida da interface (`core_process.rs`), bandeja do sistema (`tako-platform`) | CEF: a `libcef.dll` não é carregada |
| Interface | `tako --web-ui [--data-dir …]`, aberto só pelo núcleo | CEF, janela, esquemas `tako-app://` e `tako-img://`, ações de janela (`windows_host.rs`) | `AppService`, banco, lojas |
| Subprocessos do CEF | `tako.exe --type=…` (GPU, renderização, utilitários) | o que o Chromium põe neles | — |

O CEF não pode ser reiniciado no mesmo processo; por isso a interface é um processo que o núcleo abre, fecha e
reabre. O `tako.exe` é ligado com `/DELAYLOAD:libcef.dll` (`crates/tako/build.rs`): a DLL só carrega na
primeira chamada ao CEF, e o núcleo nunca faz uma (só confere que o arquivo existe). O `--web-ui` exige
`TAKO_WEB_CORE_PID` no ambiente, que só o núcleo põe; aberto à mão, termina com erro.

### Transporte

Pipes anônimos de **entrada e saída padrão do filho**, uma linha JSON por mensagem (`ipc.rs`), como o
supervisor do Slint (ADR-0026). Escolhido em vez de pipe nomeado porque os pipes anônimos só existem entre os
dois processos (não há nome para outro processo abrir nem ACL para acertar), a biblioteca padrão cuida deles sem
`unsafe`, e a queda de um lado fecha o pipe do outro. A saída de erro do filho é a do núcleo (logs); os dois
escrevem no mesmo log diário, com papéis `web` e `web-ui`. Nenhuma porta de rede.

| Direção | Mensagem | Quando |
|---|---|---|
| interface → núcleo | `request {page, text}` | `takoHost.post` aceito (frame principal, origem, até 1 MiB) |
| | `page {page}` | contexto JavaScript novo |
| | `closed {browser}` | navegador fechado (inclui DevTools) |
| | `user_closed` | a última janela fechou sem pedido do núcleo |
| núcleo → interface | `deliver {page, messages, actions}` | resposta e eventos, nessa ordem, e ações de janela |
| | `close` | fechar a interface |

O texto da página vai dentro de uma string JSON: uma quebra de linha da página não quebra o enquadramento.
A ponte não muda para a página: `window.takoHost.post` e `__takoDeliver` são os mesmos, com o mesmo JSON, a
mesma ordem (um só escritor por direção e fila FIFO até a thread UI) e os mesmos limites (1 MiB por pedido,
fila de 64 na interface; com a fila cheia, a interface responde `internal` na hora, como antes). Linha acima do
limite do transporte (cerca de 6 MiB vindo da interface, 64 MiB vindo do núcleo) é descartada sem guardar o
excesso; linha inválida é ignorada. O núcleo nunca espera a interface: a fila para o filho não tem limite e
cada filho tem um escritor próprio.

O `page` carrega navegador, frame e geração; o núcleo só os devolve. A conferência continua na interface
(`deliver` confere navegador, frame e geração, e o JavaScript confere a geração). Os pedidos do núcleo
(`library.*`, `game.*`, `ui.state`, `library.scan`) são atendidos no núcleo; `window.action` vira
`HostRequest::Window`, que o núcleo devolve como `actions` para a interface executar.
`HostRequest::PollBackground` chama `AppService::poll()` no núcleo também sem interface; sem interface, os
eventos são descartados de propósito (ao reabrir, a página consulta o núcleo).

### Ciclo de vida

- **Abrir:** o núcleo abre o filho primeiro e o `Core` depois, para a janela subir enquanto o banco carrega; os
  pedidos esperam na fila. Se o `Core` falhar, o núcleo fecha o filho e sai com o erro.
- **Usuário fecha (X ou `window.action` `close`):** a interface manda `user_closed` e termina. Sem a chave
  `CloseToTray`, o núcleo encerra o app (com um jogo aberto, não: ver [Sessão de jogo](#sessão-de-jogo-web-20)).
  Com `CloseToTray` ligada, a interface fecha, o filho CEF termina (zero processos Chromium) e o núcleo continua
  rodando na bandeja do sistema.
- **Núcleo pede (jogo, bandeja, teste):** `Command::Hide` → `close`; a interface fecha os navegadores e sai.
  Sem sair em **5 s**, o núcleo mata o processo. O núcleo segue sem interface até `Command::Show`, que abre um
  filho novo; a página lê o `ui.state` e volta onde estava. A sessão de jogo usa o mesmo caminho (`hide` e
  `show` em `core_process.rs`); `Command` continua sendo o gancho da bandeja e do teste.
- **Queda da interface:** saída com código diferente de 0, sem pedido e sem `user_closed`. O núcleo registra e
  reabre uma vez. Outra queda até **60 s** depois dessa reabertura encerra o app com “a interface caiu de novo
  logo depois de reabrir”; depois de 60 s estável, a próxima queda reabre de novo. Sem laço infinito.
- **Queda do núcleo:** a entrada padrão do filho chega ao fim. A interface fecha as janelas e, se não sair em
  5 s, termina à força (`process::exit(2)`). Os subprocessos do CEF terminam com o processo principal deles.
- O núcleo confere a saída do filho a cada 250 ms enquanto há interface; sem interface, só acorda por aviso
  das lojas, por `Command` ou por prazo.

Testes do ciclo, com um filho falso sobre `std::io::pipe`, em `core_process.rs`: fechar a pedido e reabrir com
o `ui.state` de volta, ação de janela depois da resposta, matar quem não fecha, queda dupla, mensagens de um
filho já substituído, falha ao abrir o núcleo e a interface fechando e voltando durante um jogo da demonstração.
O enquadramento é testado em `ipc.rs`.

## Sessão de jogo (WEB-20)

A sessão vive no **núcleo**, que sobrevive à interface como o supervisor sobrevive à interface Slint (ADR-0026). O
processo da interface nunca abre jogo. A regra é a mesma do app Slint e saiu da `tako-ui`:
`tako_core::session::launch` (preparar e abrir, tela e áudio do jogo, configurações efetivas por jogo, restaurar,
recuperar depois de uma queda) e `tako_core::session::running::GameSession` (acompanhar, encerrar, como terminou).
A `tako-ui` usa as mesmas funções; na `tako-app`, `session.rs` liga a sessão à ponte e ao núcleo.

| Passo | Quem | O quê |
|---|---|---|
| `game.play` | `AppService` | Valida (jogo instalado, nenhum outro aberto, senão `conflict`), responde e manda `HostRequest::Launch` |
| `HostRequest::Launch` | núcleo → `AppService::launch_game` | Grava o diário de tela e áudio, aplica a tela e a saída de áudio do jogo, esconde o cursor, abre pela loja ou pelo executável e registra a abertura. Evento `game.started` (`tracked` falso quando só dá para abrir, sem acompanhar) ou `game.ended` com `launch_failed` e o motivo |
| acompanhar | núcleo, a cada `session_deadline` | `poll_session` nesta mesma thread, sem bloquear: 0,5 s abrindo ou fechando, 2 s jogando. O laço acorda pelo prazo, não por espera ativa |
| janela do jogo na tela | `SessionUpdate::Shown` | O núcleo fecha a interface (`close`): **zero Chromium durante o jogo** (D-10) |
| `game.resume` | `HostRequest::Resume` | Fecha a interface; o jogo fica na frente |
| `game.end` (`force` opcional) | `AppService` | Pede para o jogo fechar ou o encerra à força; `game.running` passa a `closing`. `conflict` se o pedido não deu agora (a janela do jogo ainda não existe): tentar de novo ou forçar |
| fim do jogo | `SessionUpdate::Ended` | Comando de depois, ferramentas, cursor, tela e áudio restaurados (do diário), tempo jogado registrado, `library.changed` e `game.ended`. A interface reabre onde estava (`ui.state`) |

`game.running` responde `{ gameId, title, startedAt, state }` (`launching`, `playing` ou `closing`) ou `null`; é o
`running` que a página do jogo espera. `game.ended` traz `playedSeconds`, `end` (`normal`, `quick`,
`never_started`, `failed`, `launch_failed`), `exitCode` e `exitCodeText` (SESSION-003) e, em `launch_failed`,
`message`. Textos já vêm no idioma do usuário.

**O aviso de fim com a interface fechada.** Quase sempre o jogo termina com a interface fechada, e eventos sem
interface se perdem (D-10). O núcleo guarda o último `game.ended` não visto e o manda **logo depois da resposta do
próximo `game.running`**, na mesma entrega: a página que escuta `game.ended` e depois consulta `game.running` ao
abrir recebe o "bem-vindo de volta". Se havia página aberta no fim (o jogo caiu ainda em "Abrindo…", ou a
interface foi reaberta por cima dele), o evento vai na hora e não se repete.

**Interface durante o jogo.** Com um jogo aberto, qualquer saída da interface (pedido, usuário fechando, queda) só a
deixa fechada; o app não termina e ela não reabre sozinha até o jogo acabar, como o supervisor do Slint. Reabrir
por cima do jogo (atalho da Central, bandeja) fica para o WEB-22 e o WEB-36; a interface reaberta já sabe do jogo
(`game.running`) e pode voltar a ele ou encerrá-lo.

**Desktop × tela cheia.** No app Slint, durante o jogo, o modo desktop minimiza a janela e a tela cheia a fecha
(ADR-0027). Na versão web as duas fecham: a D-10 manda terminar o Chromium também minimizado, e a página web é a do
desktop. Para o usuário do desktop a diferença é a janela sumir da barra de tarefas durante o jogo em vez de ficar
minimizada. Como no Slint desktop, abrir um jogo sem situação (ou da lista de desejos) marca "Jogando".

**Queda do núcleo no meio do jogo.** O diário (`session_environment`) é gravado antes de mudar tela e áudio. Ao
abrir, o `Core` chama `recover_after_crash` antes de qualquer outra coisa: tela, áudio, cursor e controles
escondidos voltam. O tempo dessa sessão se perde (como no Slint).

**Demonstração (`simulate`).** A sessão usa as funções em memória do núcleo (processos, telas, áudio, cursor) e
uma pasta temporária, sem diário: nada abre e nada muda na máquina. O jogo simulado aparece "na tela" na primeira
leitura e fecha sozinho depois de 15 s, para o ciclo inteiro aparecer; `game.end` também funciona.

**Regras do acompanhamento vistas no teste com a Steam** (as mesmas do Slint, `GameRunTracker`, sem mudança
aqui): com o cliente da Steam fechado, o processo que abre a URI sai e a Steam leva mais que a folga de 4 s para
abrir o jogo, então a sessão termina como `quick` e o jogo abre fora dela; e um `game.end` antes de o processo do
jogo aparecer encerra a sessão sem nada para fechar. Com a Steam aberta, o jogo aparece em cerca de 2 s.

Ainda não há no núcleo web: presença no Discord, ponteiro pelo analógico (o controle já é lido no núcleo, WEB-05b;
o ponteiro é do WEB-34), Wine e Proton (Linux, fora da fase Windows, D-15) e esconder o controle do jogo com a
Central aberta (INPUT-005; só na tela cheia, veja "Controle").

Testes: `tako-core` (`session::launch`, `session::running`: abrir, restaurar, queda, encerrar), `tako-app`
(`tests/session_bridge.rs`: o ciclo pela ponte, `launch_failed`, a queda do núcleo, a demonstração, o formato) e
`tako-web` (`core_process.rs`: a interface fecha e volta com o jogo).

## Arte pelo núcleo (WEB-01d)

O `--web` monta as fontes por `tako_core::metadata::runtime`, compartilhado com o Slint: Steam, IGDB,
ScreenScraper e SteamGridDB, com a mesma prioridade do `MetadataAgent`, opções e chaves carregadas por
`IgdbSettings`, `ScreenScraperSettings` e `SteamGridDbSettings` do cofre. Mantém cache de respostas de sete dias,
cache de metadados (30 dias; sete sem correspondência), fila `MetadataWorker` com intervalo de 1,5 s, limites e
backoff dos clientes existentes, incluindo a opção de preenchimento automático do SteamGridDB. IDs e imagens
fixados no banco são aplicados antes da fila e novamente a cada leitura da biblioteca.

A fila de arte é separada da fila de metadados: capas já conhecidas não esperam uma consulta de metadados.
`ArtWorker` junta pedidos pendentes por jogo, baixa uma arte por vez com `ArtCache` (32 MiB por imagem,
cache de 1 GiB e faltas confirmadas por sete dias) e importa arquivos locais/Steam para o mesmo cache.
O transporte de imagens tem timeout de oito segundos; a mídia ScreenScraper usa seu cliente autenticado,
compartilhado com os metadados (20 s). A demonstração não inicia essas filas nem acessa o cofre. Não há nova
fonte nem implementação de cliente HTTP.

O núcleo publica um pequeno índice por jogo/tipo em `<perfil>/cache/art-index/`, apontando apenas para nomes
de arquivos em `DataPaths::art_cache()`. O índice é por perfil; as imagens e o limite continuam compartilhados
na máquina. `GameArt::resolve` é a função local usada no filho CEF: lê o índice a cada pedido, valida tamanho,
nome e caminho canônico e retorna só arquivos presos ao cache. Isso evita ida e volta na ponte e não prende
o esquema ao snapshot de abertura. Download e importação só acontecem no núcleo; sem arquivo, 404 imediato.

Resultados acordam o laço do núcleo por `HostRequest::PollBackground`, inclusive sem janela. `AppService::poll`
aplica metadados e enfileira suas novas URLs; quando uma arte é publicada, emite `art.changed { gameId }`.
O transporte existente entrega o evento à página aberta. Nenhum caminho, URL de origem ou segredo está nesse
evento. A página deve trocar a versão `v` das URLs `tako-img://` desse jogo (contrato em [ponte.md §4](ponte.md#4-esquemas-próprios)).
O tratamento no React fica para outra tarefa.

Testes: `metadata::game_art` no núcleo (resolução, limites, links para fora, fila, reaproveitamento e limpeza do
cache, arquivo local e ICO), `tests/art_bridge.rs` na aplicação (ponte livre durante download, URL descoberta
pelos metadados, publicação antes do evento e demonstração sem cofre/rede) e `resources` no hospedeiro
(índice atualizado sem reabrir e ICO convertido para PNG). Os transportes desses testes são falsos.

## Atualização do app (WEB-33)

A regra é a do Slint (`tako_core::updates`, SHA-256 publicado no GitHub); o estado vive no núcleo (`tako-app/src/updates.rs`).

| Chamada / pedido | Quem trata | O que faz |
|---|---|---|
| `updates.status`, `updates.check`, `updates.download`, `updates.apply` | `AppService` | Estado, procurar (canal de `updates.json`), baixar de novo depois de falha, aplicar. Evento `updates.changed` com o estado e o progresso |
| `updates.preferences`, `updates.preferences.set` | `AppService` | `AutoUpdate`, `Prereleases` e `Nightly` de `updates.json` |
| `HostRequest::Exit` | núcleo (`core_process.rs`) | A troca já foi iniciada (instalador ou script que espera este processo): fecha a interface e encerra o núcleo |

- Ao abrir o núcleo, `AppService::check_updates_on_open` procura se `AutoUpdate` estiver ligado (compilação de desenvolvimento e
  demonstração não procuram sozinhas) e avisa se o script de troca deixou `failed`.
- Achou e o pacote se atualiza sozinho: baixa sozinho, confere o SHA-256 (o que não confere é apagado) e fica `ready`. `updates.apply`
  confere de novo antes de executar. Com um jogo aberto (`game.running`) o reinício é recusado (`conflict`).
- "Depois" instala ao sair: ao terminar o app sem queda, `install_update_on_exit` roda o plano sem reabrir.
- Demonstração: versão de exemplo, download simulado e `updates.apply` que não faz nada.

Testes: `tako-core` (`updates::`, inclusive o download adulterado), `tako-app` (`tests/updates_bridge.rs`, com consulta falsa,
servidor HTTP local e instalador falso) e `tako-web` (`core_process.rs`: `Exit` encerra o núcleo).

## Controle (WEB-05b)

O SDL3 roda no **núcleo**, numa thread própria (`core_process::spawn_pads`, ligada pela feature `cef`), e nunca no
filho CEF. Como o núcleo vive durante o jogo, é ele que vê o atalho da Central com a interface fechada, como o
supervisor do Slint. A leitura pelo SDL (`Pads`) saiu da `tako-ui` para a `tako-platform` (feature `gamepad`); as
regras são as do Slint e ficam no `tako-core`:

| Regra | Onde | Valor |
|---|---|---|
| Zona morta do analógico, repetição da direção mantida | `input::GamepadInterpreter` | 0,5; 380 ms e depois a cada 110 ms |
| Botões de cada controle pelo seu perfil, combinação da Central | `input::profiles`, `input::shortcut` | INPUT-001; View + Y por 500 ms no padrão |
| Família dos botões, estado somado, avisos de conexão | `input::pad` | Xbox, PlayStation, Nintendo; `PadNotices` |
| Bateria fraca | `input::low_battery_warning` | 15 %, um aviso por conexão |

A thread do SDL entrega cada leitura (`PadFrame`) a um `tako_app::input::InputBridge`, que diz o que vai para a
interface. O laço do núcleo (`InputSync`) repassa e conta à thread onde a interface está (`InputScene`) e, depois
de cada pedido da página, os perfis de botões das configurações do perfil em uso.

| Ponte | O quê |
|---|---|
| evento `input.nav` `{ action, repeat }` | Comando já traduzido: `up`, `down`, `left`, `right`, `accept` (A), `back` (B), `favorite` (X), `search` (Y), `previous_section` e `next_section` (LB e RB), `menu`. `repeat` só em direção mantida |
| evento `input.controller` | `{ connected, count, style, name, battery }`: `style` é `xbox`, `playstation` ou `nintendo` (o último controle que conectou ou apertou um botão); `battery` em % só sem fio |
| chamada `input.controller` | O mesmo estado, para a página que abre (ou reabre depois do jogo) |
| evento `input.notice` `{ kind, pad, percent }` | `connected`, `disconnected` ou `battery_low`; o texto é `status.pad_connected`, `status.pad_disconnected` e `status.battery_low` do catálogo |

**Quando vai algo para a interface.** Comandos e avisos só com uma página aberta; fechada (durante o jogo), nada
(D-10). Com um jogo aberto o controle é do jogo: nem a interface que ainda mostra "Abrindo…" recebe comandos (no
Slint desktop é igual). Avisos de conexão não saem durante o jogo, e "desconectado" só para quem usava o controle;
como o núcleo não vê teclado nem mouse, conectar já conta como "usando o controle" (no Slint o teclado desfaz).
"Conectado" só aparece depois de o último controle cair com aviso. A bateria é lida a cada 30 s só com a interface
aberta (e logo depois de um controle entrar ou sair), então o aviso de bateria fraca espera a interface, como no
Slint.

**Atalho da Central durante o jogo.** A combinação do perfil (View + Y por padrão) com a interface fechada a reabre
por cima do jogo, e os comandos voltam a valer; a mesma combinação com ela aberta assim volta ao jogo. A tela da
Central é do WEB-22. O que estiver apertado quando a interface volta só vale depois de solto. O SDL só acorda pelos
eventos com a interface fechada (2 s de espera, 50 ms segurando a combinação); aberta, uma volta a cada 16 ms.

**HidHide (INPUT-005).** O Slint só esconde o controle do jogo na tela cheia (o supervisor, com a interface por cima
do jogo); no desktop o jogo continua com ele (ADR-0027). A versão web é só desktop até o WEB-15, então não esconde.

**No React** (`web/src/focus/`): `controller.ts` liga `input.nav` ao mesmo `dispatchNavAction` do teclado, com a
ação oferecida antes ao componente focado como a tecla equivalente (com `repeat`, para lista e deslizante não
pularem a borda segurando). Y abre a busca geral pelo mesmo Ctrl+K; X e Menu chegam ao componente focado como
`tako:command` (`detail.command` é `favorite` ou `menu`), e quem não escuta ignora. `data-pad` na raiz e
`useController()` dão a família dos botões para ícones e dicas; `AvisosDoControle` mostra os avisos.

**Hospedeiro falso.** `?controle[=xbox|playstation|nintendo]` e `?bateria=12` simulam um controle conectado;
Ctrl+Shift+F8 desconecta e reconecta, Ctrl+Shift+F9 troca a família, e `window.takoFakePad.press("down")` manda um
comando como se viesse do controle.

Testes: `tako-core` (`input::pad`), `tako-app` (`tests/input_bridge.rs`: zona morta, repetição, perfis, cena,
Central, avisos, bateria, formato) e `tako-web` (`core_process.rs`: comandos para a página aberta, nada com ela
fechada, a Central reabre por cima do jogo e volta a ele).

## Discord, API local e links tako:// (WEB-34)

Três integrações do Slint que vivem no processo do **núcleo** (`tako --web`), nunca no filho CEF. A regra que não
dependia de Slint foi para o `tako-core`; o Slint usa a mesma.

| Integração | Regra (compartilhada) | No núcleo web |
|---|---|---|
| Discord Rich Presence (EXT-004) | `tako_core::session::discord` (IPC, `Presence`) | `session.rs`: com a chave `DiscordRichPresence` ligada e o id do aplicativo definido, `game.play` mostra o jogo; o fim da sessão solta a presença e limpa a atividade |
| API local (EXT-002, ADR-0033) | `tako_core::api` (servidor) e `api::runtime` (ligar/desligar pela chave, token, estado pelos eventos de sessão) | `tako_app::local_api`: `AppService::sync_local_api` a cada volta do laço |
| Links `tako://` (EXT-005) | `tako_core::links` (`parse`, o arquivo de ativação) | `tako_app::links`: `AppService::open_link` |

**Discord.** O recurso só roda com `DISCORD_CLIENT_ID` (id do aplicativo no portal do Discord, do mantenedor), que
hoje está **vazio** no código: sem ele nem o Slint mostra a presença, e a opção não faz efeito. Com o Discord aberto
o IPC responde (`\\.pipediscord-ipc-0`); a demonstração (`--simulate`) nunca fala com o Discord.

**API local.** A mesma porta (`LocalApiPort`, padrão 47774), o mesmo token (`LocalApiToken`), a mesma autenticação
`Bearer`, `Host` e limites do ADR-0033, sem código duplicado. Fica **desligada** com `LocalApiEnabled` desligada
(o padrão): nenhuma porta abre e nenhum evento é assinado. Ligada, o token é criado e guardado nas configurações na
primeira vez, e trocado se o guardado for inválido. As chaves não estão na lista da ponte: a interface não lê nem
grava o token (D-09: esta porta é para integrações externas, não é a ponte da interface), e ligar ou desligar vale
no próximo `tako --web`. `open` e `play` chegam ao laço do núcleo (`Command::Link`) e seguem o mesmo caminho do
link `tako://`; sem o laço, `503`.

**Links.** Um link válido (só as telas e IDs de jogo que o Slint aceita; nunca caminho, comando ou outra URL) vira o
estado de tela (`ui.state`, D-10): `activeGame`/`view`/`library.filter`, sem mexer no resto. A interface aberta
noutro lugar é fechada e reaberta já no destino (não há ação de janela para navegar uma página aberta); uma que ainda
está subindo lê o estado sozinha. `tako://play/<id>` abre o jogo direto **só** quando o Slint também abriria sem
diálogo (`details::primary_action`): instalado, sem outro jogo aberto, cliente da loja achado e comandos aprovados;
o mesmo jogo já aberto volta para a frente; em qualquer outro caso a página do jogo aparece e a confirmação é do
usuário. Jogo que a biblioteca não conhece não faz nada (e, se a primeira leitura das lojas ainda corre, o link
espera por ela). Sem aviso na tela para o jogo desconhecido: a ponte ainda não tem toast.

**Instância única.** `tako --web` pega o mesmo `tako.lock` do Slint (um Tako por usuário, web ou Slint). O segundo
só deixa um pedido em `tako.activate` (escrito por `links::request_activation`, com a troca de nome atômica) e sai,
sem abrir janela nem outro núcleo. O núcleo confere o arquivo a cada 400 ms (`core_process::watch_activation`) e
executa o link, ou só traz a interface para a frente (hoje isso é fechar e reabrir a janela). Com o Tako fechado, o
link entra pelo mesmo arquivo, antes de a interface ler o estado. Um Tako Slint aberto depois do web entrega pelo
mesmo caminho. Pedido inválido ou grande demais é descartado.

**Argumentos.** `tako --web --open-url "<url>"` ou `tako --web "<url>"`. Em release, qualquer outra forma com link
falha com código 2 (uma URL do navegador não injeta `--data-dir` pelas aspas da linha do Windows); em debug valem
outras opções, para testar com `--data-dir`. O comando do protocolo no registro (`HKCU\Software\Classes\tako\shell\open\command`)
pertence ao instalador (INST-*): para abrir a versão web deve ser `"Tako.exe" --web --open-url "%1"`; o instalado hoje
ainda aponta para o Slint (`--open-url "%1"`), que entrega o link ao núcleo web aberto pelo mesmo arquivo.

Testes: `tako-core` (`session::discord` com o Discord simulado por pipes, `api::runtime`, `links`), `tako-app`
(`tests/local_api_bridge.rs`, `tests/links_bridge.rs`) e `tako-web` (`core_process.rs`: o link reabre a interface no
destino, o segundo Tako traz a janela, o arquivo de ativação chega ao laço).

## Central rápida (WEB-22)

Durante um jogo aberto pela sessão (WEB-20), a Central rápida abre por cima do jogo como no Slint (PRD D-15, `docs/plans/web-paridade-windows.md`). Sem jogo aberto, o atalho não faz nada.

### Abertura e foco

- **Como abre:**
  - Pela combinação do controle (View + Y por padrão), detectada pelo interpretador de controle no núcleo (`InputSync` em `core_process.rs`);
  - Pelo atalho global de teclado do Slint (`ControlCenterKey`, padrão `Ctrl+Shift+Tab`), registrado via `RegisterHotKey` na thread dedicada da plataforma (`tako_platform::control_center::register`), sem instalar hook global de teclado.
- **Foco e sobreposição:**
  - Ao acionar o atalho com a interface fechada, o núcleo aciona `host.show = true` e marca `service.set_central_open(true)`;
  - Quando a janela da interface reabre (`UiMsg::Page`), o núcleo traz a janela do CEF para a frente com `focus_processes(&[ui.process.id()])`;
  - Ao voltar ao jogo (`game.resume` ou repetição do atalho), a interface fecha (`host.hide`) e o foco retorna à janela do processo do jogo com `focus_processes(&service.central_game_pids())`.

### Ponte e controles do sistema

| Chamada / Evento | Tipo | Descrição |
|---|---|---|
| `control_center.state` | Chamada | Devolve o estado da Central: `open`, `audio` (dispositivo padrão, volume, mudo, saídas disponíveis), `displays` (telas ativas e modos), `displayManaged`, `controller`, `memoryMib` e `cpuSeconds`. |
| `control_center.set` | Chamada | Aplica alterações nos dispositivos (`volume`, `mute`, `audio_output`, `display_mode`) e devolve o estado atualizado. Exige jogo ativo (`conflict` sem jogo). |
| `control_center.open` | Evento | Notifica a interface quando a Central abre (`true`) ou fecha (`false`). |

### Interface React e hospedeiro falso

- `CentralRapida.tsx` (`web/src/app/central/`) exibe o diálogo modal sobre a tela:
  - Cabeçalho com o nome do jogo, tempo jogado e bateria do controle conectado;
  - Botão "Voltar ao jogo", que chama `game.resume` e fecha a interface;
  - Botões para alternar os painéis rápidos de Áudio (volume, mudo, saída), Tela (resolução e taxa de atualização) e Desempenho (CPU e memória);
  - Botão "Encerrar jogo", com confirmação e transição direta para o fluxo "Fechando…" da WEB-21 (`web/src/app/sessao/`);
  - Suporte a Escape (fecha o painel aberto ou volta ao jogo) e aprisionamento de foco (`useFocusTrap`).
- **Hospedeiro falso:** abrir com `?central` inicia uma sessão simulada já na Central rápida, permitindo testar controles, atalho de teclado (`Ctrl+Shift+Tab`) e botões de voltar/encerrar no navegador.

Testes: `crates/tako-app/tests/control_center_bridge.rs` (ciclo pela ponte, validações, atalho customizado e envelope de rede), `crates/tako-web/src/core_process.rs` (abertura com a interface fechada e retorno do foco ao jogo).

## Seletor de arquivo e de pasta nativo (WEB-12e)

Paridade com os seletores nativos do Slint no Windows (`rfd`). A interface web (`tako-web`) dispara diálogos modais nativos por meio da ponte para escolher executáveis, pastas de trabalho ou pastas de ROMs, sem bloquear a thread principal do núcleo nem a interface CEF.

| Chamada / Evento | Tipo | Descrição |
|---|---|---|
| `system.pick` | Chamada | Solicita a abertura de um seletor nativo: `{ kind: "file" \| "folder", purpose: string, filters?: PickFilter[] }`. Responde imediatamente com `{ id: number }`. |
| `HostRequest::Pick` | Pedido ao hospedeiro | Emitido pelo `AppService` ao hospedeiro com `{ id, kind, purpose, filters }`. |
| `system.picked` | Evento | Notifica a página com `{ id: number, path: string \| null }`. O caminho é `null` se o usuário cancelar o diálogo. |

### Isolamento de threads e segurança

- **Thread própria:** `rfd::FileDialog` é síncrono e modal. Para nunca bloquear o laço de eventos do núcleo (`core_process.rs`) nem a renderização do Chromium, o diálogo é executado em uma thread dedicada (`std::thread::spawn`). Ao concluir, a thread envia um comando de volta à thread do núcleo (`Command::Picked { id, path }`).
- **Casamento de IDs:** o `PickerState` no `AppService` registra o pedido pendente e confere o `id` quando o resultado retorna. Resultados para IDs desconhecidos ou expirados são rejeitados silenciosamente.
- **Validação mantida:** o caminho escolhido retorna à interface apenas como sugestão de preenchimento dos campos de texto existentes. A gravação final e a validação de acesso continuam ocorrendo através das rotas de negócio existentes (`game.edit.set`, `library.add_game`, `emulators.folder.add`).
- **Hospedeiro falso:** em ambiente de desenvolvimento sem hospedeiro nativo, `fakeSystemPick` responde na hora com um id e emite `system.picked` após um pequeno atraso (150 ms) com caminhos simulados (`C:\Jogos\Exemplo\game.exe` ou `C:\Roms`).

Testes: `crates/tako-app/src/picker.rs` (ciclo do id, limite e cancelamento), `crates/tako-app/tests/picker_bridge.rs` (ciclo completo pela ponte, resposta imediata, evento, cancelamento e rejeição de id inválido) e `crates/tako-web/src/core_process.rs` (entrega de `Command::Picked` e emissão do evento para a interface).

## Bandeja (WEB-36)

No Windows, o Tako exibe um ícone na área de notificação (bandeja do sistema) gerenciado exclusivamente pelo processo do **núcleo** (`crates/tako-web/src/core_process.rs` e `crates/tako-platform/src/windows/tray.rs`), com paridade de comportamento em relação à versão Slint (PRD D-10 e D-15). O processo filho CEF nunca toca na bandeja.

### Arquitetura na plataforma (`tako-platform`)

- Janela oculta dedicada a mensagens Win32 (`Shell_NotifyIconW`, `NIM_ADD`, `NIM_DELETE`) rodando em thread de segundo plano.
- Ícone carregado a partir do binário (`IDI_APPLICATION`) dimensionado para as métricas da bandeja (`SM_CXSMICON`, `SM_CYSMICON`).
- Tratamento de `TaskbarCreated` via `RegisterWindowMessageW`, garantindo restauração do ícone caso o Windows Explorer reinicie.
- Menu de contexto suspenso (`TrackPopupMenuEx`) acionado por clique com o botão direito, com as seguintes ações e textos localizados (`tray.*` em `crates/tako-core/locales/`):
  1. **Abrir o Tako** (`tray.open`): item padrão (também acionado por clique com o botão esquerdo / duplo clique no ícone).
  2. **Jogos recentes** (até 5 títulos obtidos via `AppService::recent_games`), acionando `TrayAction::Launch(id)`.
  3. **Central de Controle** (`tray.central`): exibido apenas se houver sessão de jogo ativa (`tray_state.game_running == true`).
  4. **Sair do Tako** (`tray.quit`): encerra o núcleo e todos os subprocessos.

### Fechar para a bandeja e ciclo de vida

- **Configuração `CloseToTray`:** chave booleana em `Settings` (configurações do perfil).
- **Comportamento ao fechar a janela:** ao clicar no botão de fechar ('X') da janela, o processo de interface CEF notifica o núcleo (`Exit::UserClosed`). Com `CloseToTray` ativado:
  - O processo da interface é encerrado por completo (`host.watch` descarta o filho e limpa o estado).
  - Todos os subprocessos Chromium são finalizados, garantindo consumo de memória $\le 40$ MB e **zero processos Chromium** ativos enquanto minimizado para a bandeja (D-10).
  - O núcleo Rust permanece em execução com o ícone na bandeja.
- **Reabertura:** um clique no ícone da bandeja envia `Command::Tray(TrayAction::Open)`, fazendo o núcleo iniciar um novo processo filho CEF, restaurando o estado da interface (`ui.state`) exatamente onde estava.
- **Ação "Sair do Tako":** envia `Command::Tray(TrayAction::Quit)`. Caso haja jogo aberto na sessão gerenciada, o encerramento restaura tela, áudio, cursor, ferramentas e presença antes de sair, com o mesmo comportamento limpo do Slint.

Testes: `crates/tako-platform` (`tray`), `crates/tako-app` (`tests/tray_tests.rs`) e `crates/tako-web` (`core_process.rs`: ciclo de fechar para a bandeja e reabertura, saída da bandeja com sessão ativa).

## Preparar e compilar

Windows x64, Rust de `rust-toolchain.toml`, Visual Studio 2022 Build Tools com C++ e Windows SDK.

```powershell
# Não depende de CEF, exportador, wrapper C++ ou Ninja.
cargo build -p tako
# Dot-source: as variáveis precisam permanecer nesta sessão.
. .\scripts\cef-setup.ps1
cargo build -p tako --features web   # liga também a feature `cef` da `tako-web`; sem ela, nada do Chromium entra no build
```

O script lê a versão completa da crate `cef` no **Cargo.lock** e deriva dela as versões do runtime e do
exportador, sem outra versão para atualizar no script. Nesta rodada: crate `156.1.0+156.0.8`, CEF
`156.0.8+g426920c`, Chromium `156.0.8078.48`, API CEF 15600.

O exportador oficial `export-cef-dir` fica em `%LOCALAPPDATA%\tako-dev\cef-tools\<versão-da-crate>`.
Baixa a distribuição **minimal**, confere o SHA-1 do índice oficial e extrai em
`%LOCALAPPDATA%\tako-dev\cef\<versão-do-runtime>\cef_windows_x86_64`. O script define `CEF_PATH`, inclui
CMake/Ninja do Visual Studio no PATH e serializa o preparo por um lock exclusivo. As worktrees reutilizam
o cache. O exportador é ferramenta de desenvolvimento, sem dependência no aplicativo.

Com a feature, `cef-dll-sys` compila o wrapper C++ e copia DLLs, `.pak`, `icudtl.dat`, snapshots e `locales/`
para o diretório do executável. **Feche o Tako antes de build, clippy ou test:** DLL em uso impede a cópia.
Nenhum binário CEF vai para o git. Antes da primeira chamada CEF, inclusive nos subprocessos, o hospedeiro
verifica `libcef.dll` ao lado do executável: se faltar, devolve erro e registra no log sem atingir a carga
adiada. A existência do arquivo não substitui validar a integridade do pacote na instalação.

Referências: [cef-rs](https://github.com/tauri-apps/cef-rs),
[crate usada](https://docs.rs/cef/156.1.0+156.0.8/cef/),
[exportador](https://docs.rs/crate/export-cef-dir/156.1.0+156.0.8).

## Página de diagnóstico

```powershell
. .\scripts\cef-setup.ps1
$env:TAKO_WEB_TEST_ROOT = (Resolve-Path crates/tako-web/testpage).Path
$env:TAKO_WEB_TEST_ART = Join-Path $env:TAKO_WEB_TEST_ROOT 'fixture.png'
$env:TAKO_WEB_RECYCLE_MS = '20000'
.\target\debug\tako.exe --web --data-dir "$PWD\target\web02b\manual"
```

A página técnica é separada do React. Mostra `app.info`, faz `ui.state.set` → `ui.state`, aumenta `visits`,
carrega a fixture por `tako-img://` e pede leitura das lojas. Eventos aparecem no diagnóstico. Seus textos
ficam nos catálogos da fixture. Os botões permitem minimizar, maximizar/restaurar, fechar, repetir a leitura,
abrir um link HTTP externo e armar `onbeforeunload` após um `alert` para testar o fechamento.

A janela tem título **Tako**, tamanho inicial **1600×900**, limitado à área de trabalho em unidades DIP do
CEF, e mínimo **960×600**. O arraste usa apenas `-webkit-app-region: drag` na barra superior, repassado pelo
`DragHandler` ao CEF Views. Não existe `start_drag` na ponte nem chamada Win32 de arraste na `tako-web`.
Arquivos arrastados para a janela são recusados. Links HTTP/HTTPS fora da origem abrem pelo
`Platform.processes.launch` da `tako-platform`, sem navegar a janela; outros esquemas externos são recusados.
Popups não criam outra janela de aplicação. Alertas são suprimidos e `onbeforeunload` permite encerrar.

A fixture PNG local de 120×180 é copiada para `test-cover.img` no cache do núcleo e publicada no índice
`GameArt` sob o ID `test:cover`.
A página pede `tako-img://cover?id=test%3Acover&w=120&h=180`; nenhum caminho local é exposto a ela.
`TAKO_WEB_TEST_ROOT`, `TAKO_WEB_TEST_ART`, `TAKO_WEB_RECYCLE_MS` e o ID `test:cover` só valem em debug.

Com `TAKO_WEB_RECYCLE_MS` (100 a 300000 ms), o núcleo fecha a interface esse tempo depois de abri-la, como
faria durante o jogo, e a reabre o mesmo tempo depois, uma vez. Na página técnica, `visits` sobe: o
`ui.state` atravessou o processo descartado. O `AppService` conserva `ui.state` enquanto o núcleo vive; não
promete persistência entre execuções. Fechar o último navegador termina o processo da interface.
**DevTools também entra na contagem** até `on_before_close`; ao fechar a janela principal, seu host pede o
fechamento do DevTools. O CEF inicializa uma vez por processo da interface e recebe `shutdown` ao sair. O
registro de objetos Views é local à thread UI para as tarefas acessarem só objetos dessa thread.

## Servidor de desenvolvimento

Em debug, sem fixture, o pacote vem de `web/dist/` desta checkout; na distribuição, de `web/dist/` junto ao
executável. `TAKO_WEB_DEV_URL=http://localhost:5173/` troca o pacote por um servidor local. Só HTTP e loopback
(`localhost`, `127.0.0.1`, `[::1]`) são aceitos. Esquema e host aceitam maiúsculas, a porta é normalizada e a
origem continua exata. Essas variáveis são ignoradas em release. A URL concede a ponte ao código daquela
origem: use um servidor sob seu controle.

```powershell
# Em um terminal, na raiz desta worktree:
npx -y http-server crates/tako-web/testpage -a 127.0.0.1 -p 5192 -c-1
# Em outro:
$env:TAKO_WEB_DEV_URL = 'http://127.0.0.1:5192/'
$env:TAKO_WEB_TEST_ART = (Resolve-Path crates/tako-web/testpage/fixture.png).Path
.\target\debug\tako.exe --web --data-dir "$PWD\target\web02b\dev"
```

O servidor é responsável por sua CSP. `testpage/vite.config.mjs` exemplifica uma origem exata, com WebSocket
apenas para HMR. Pode ser iniciado com
`npm exec --yes --package=vite@8.3.5 -- vite crates/tako-web/testpage --config crates/tako-web/testpage/vite.config.mjs`.
F12 ou Ctrl+Shift+I abre DevTools apenas no debug; o hospedeiro não acrescenta porta de depuração.

## Ponte e segurança

- `main` reconhece `--type=...` antes de allocator, logs, SDL e supervisor, usando o próprio `tako.exe`
  como subprocesso. A integração CEF só é compilada com a feature `web`.
- Em release, `Settings.command_line_args_disabled = 1`: o próprio CEF descarta as opções externas do Chromium
  antes de aplicar `Settings`. O callback `on_before_command_line_processing` só acrescenta opções do
  hospedeiro; **não limpa a linha**, porque nela o CEF já pôs `--no-sandbox` e os caminhos de `Settings`
  (limpá-la deixa o release com a página em branco, sem renderer). Testes:
  `only_debug_accepts_external_chromium_options` e `host_options_keep_what_cef_already_applied`.
  Conferido no binário release com `--remote-debugging-port`, `--disable-web-security`, `--renderer-cmd-prefix`
  e `--js-flags=--expose-gc`: nenhuma porta escuta, a página abre com renderer normal e `typeof gc` é
  `undefined`; no debug, a mesma `--js-flags` vale.
- O renderer instala `window.takoHost.post` antes dos scripts, apenas no frame principal da origem autorizada.
  O processo principal repete a validação. A geração de página vem de contador atômico por instância do app
  no renderer, com PID para distinguir processos de renderização.
- A fila da página limita 64 mensagens. O processo do núcleo abre `Core` e `AppService::with_core`, usa
  `tako_app::protocol::parse_request` por meio de `bridge::handle_message` e atende `HostRequest::PollBackground`
  chamando `AppService::poll()`. Um relay acorda esse mesmo laço com avisos das lojas; não entrega eventos.
- Depois de cada mensagem, o núcleo drena o canal de eventos em ordem. Resposta e eventos causados por ela vão
  no mesmo `deliver`, com a resposta primeiro, e entram juntos na fila UI da interface. Ambas as entregas
  conferem navegador, frame e geração na interface, mais a geração no JavaScript. Mensagem e geração usam
  escape JSON. Sem página ou sem interface, os eventos são descartados; ao reabrir, a interface consulta o núcleo.
- A lógica de ponte, entrega e recursos saiu dos callbacks `wrap_*` para funções comuns: `accept_post`,
  `install_bridge`, `deliver_script`, `read_resource` e outras. O único bloco `unsafe` escrito na `tako-web`
  é a cópia exigida por `ResourceHandler::read`, com `#[allow]` local e `// SAFETY:`;
  `resource_reads_in_chunks_and_head_has_no_body` exercita essa cópia em pedaços, com `HEAD` e ponteiro nulo.
- `tako-app` e `tako-img` são esquemas standard, seguros, com CORS/fetch e sem bypass de CSP. O pacote recebe
  a CSP de [ponte.md](ponte.md), com `object-src`, `base-uri`, `frame-src` e `frame-ancestors` bloqueados.
  Não existe servidor HTTP de comunicação com o núcleo.
- Arquivos usam autoridade exata `app` e os validadores de `schemes`; só rotas sem extensão recebem fallback
  para `index.html`. HTML, JS e CSS têm `charset=utf-8`; há MIME para GIF, AVIF e webmanifest, além dos tipos
  anteriores, e `nosniff`. Arquivos do pacote têm limite próprio de 32 MiB; o JSON de descoberta tem limite
  próprio de 64 MiB, compatível com o núcleo.
- Imagens só saem do hash em `ArtCache::cache_path`, validado sob `cache/art`: entrada de até 32 MiB, decode
  de até 128 MiB e saída de até 16 milhões de pixels. Nenhum `locate`, caminho externo ou download.
  A resolução temporária por `found-games.json`/`steam-art://` passa ao núcleo na WEB-01d.
- O sandbox continua desativado no Windows (WEB-02c). A crate cef (v156.1.0) não expõe cef_sandbox_info_create/destroy para Windows (módulo sandbox restrito ao macOS via dylib), o pacote minimal do CEF (scripts/cef-setup.ps1) não contém cef_sandbox.lib (presente apenas na distribuição standard), e a inicialização do CEF sem windows_sandbox_info força a injeção de --no-sandbox pelo próprio runtime do Chromium. A ativação no Windows exigirá empacotar cef_sandbox.lib e fornecer bindings/bootstrap nativo.

## Verificação

```powershell
. .\scripts\cef-setup.ps1
cargo fmt --all
cargo clippy -p tako-web -p tako --all-targets -- -D warnings
cargo clippy -p tako-web -p tako --all-targets --features tako/web -- -D warnings
cargo test -p tako-web -p tako-app
cargo test -p tako-web --features cef        # testes do hospedeiro CEF
cargo build -p tako
cargo build -p tako --features web
(cd web && npm run typecheck)   # confere os tipos gerados da ponte junto com o resto do front
```

`cargo tree -p tako --edges normal,build` deve conter zero entradas `cef`/`cef-dll-sys` sem a feature.
Os tipos TypeScript são regenerados por `cargo test -p tako-app`, sem edição manual. Saídas, capturas e
outros dados locais desta rodada ficam em `target/web02b/` (ignorado pelo git).

### Testar o ciclo à mão

Com `TAKO_WEB_RECYCLE_MS` definido (seção da página de diagnóstico), e o PID do núcleo em `$core`:

| Caso | Como | Esperado |
|---|---|---|
| Fechar a pedido e reabrir | esperar o intervalo | a janela some, só o núcleo fica; reabre e a página volta (`visits` sobe; no React, a mesma tela) |
| Queda da interface | `taskkill /F /PID <pid do --web-ui>` | log `a interface caiu`, reabre em menos de 1 s, página restaurada |
| Queda dupla | repetir em menos de 60 s | o núcleo sai com código 1 e “a interface caiu de novo…”; nenhum `tako.exe` fica |
| Queda do núcleo | `taskkill /F /PID $core` (só ele) | log `transporte do núcleo fechado`; a árvore inteira some em menos de 1 s |
| Usuário fecha | botão Fechar ou X | `interface fechada … exit=User`; o núcleo sai com 0 |
| CEF fora do núcleo | `(Get-Process -Id $core).Modules \| ? ModuleName -like 'libcef*'` | vazio |

Memória: Private Working Set (`Win32_PerfRawData_PerfProc_Process.WorkingSetPrivate`) somado do núcleo e de
todos os descendentes (`ParentProcessId` recursivo). Medição desta rodada, build debug, biblioteca real de
18 jogos da Steam, 1600×900; o `conhost.exe` (1,5 MiB) só existe no debug, que tem console:

| Momento | Página técnica | React (`web/dist`) | Processos |
|---|---|---|---|
| Interface aberta | 83,46 MiB | 115,22 MiB | núcleo + `web-ui` + GPU + 2 renderização + 2 utilitários |
| Logo depois de fechar a pedido | 6,85 MiB | 6,61 MiB | núcleo (5,1 a 5,4 MiB) + `conhost` |
| 15 s depois | 6,68 MiB | 6,61 MiB | idem, zero do Chromium |
| Reaberta | 105,58 MiB | 112,01 MiB | como aberta |

Sem interface, a meta da D-10 (≤ 40 MB e zero Chromium) fica com folga; o número com a interface aberta não
é desta tarefa (WEB-60). Antes da WEB-02b, fechar a janela deixava 74,71 MiB e 4 processos do Chromium.

## Limitações mantidas

A sessão de jogo já fecha e reabre a interface (acima). O descarte depois de um tempo minimizado (a interface avisa “minimizada”, o núcleo
conta o tempo e manda `Hide`) fica para próxima etapa. Sem Job Object, a queda do núcleo depende do fim da entrada padrão do filho; os
subprocessos do CEF somem com o processo principal deles, conferido acima.

A resolução de arte ainda é um snapshot de abertura, não cobre arte fixada fora do cache nem descobertas
posteriores e gera PNG por pedido sem cache de redimensionamento. O snapshot é lido pelo processo da
interface a cada abertura. O empacotamento do runtime continua fora desta tarefa. O log CEF é WARNING no debug e desativado em release; ainda não passa
pelo redator/rotação do núcleo. O build debug emite LNK4098 pela mistura
de runtimes C++ (SDL3, e também CEF com a feature `web`).
Linux e macOS precisam de entradas próprias; `--web` retorna erro nesses sistemas nesta etapa.

## Instalador de prévia

O script `build/rust/Package-Web.ps1` automatiza a geração do pacote e do instalador Inno Setup (`dist/Tako-Web-Preview-Setup-x64.exe`) para testes da versão web no Windows:

1. Compila os arquivos estáticos da interface web (`npm ci` e `npm run build` em `web/`), gerando `web/dist`.
2. Configura o ambiente CEF (`scripts/cef-setup.ps1`) e compila o binário em release com `cargo build -p tako --release --features web --locked`.
3. Monta uma pasta de estágio independente (`artifacts/rust/web-preview-x64/`) contendo o executável `tako.exe`, o runtime completo do CEF (`libcef.dll`, `chrome_elf.dll`, `d3dcompiler_47.dll`, `icudtl.dat`, `v8_context_snapshot.bin`, arquivos `.pak`, `locales/`, etc.) e a interface compilada em `web/dist/`.
4. Executa um teste automatizado na pasta montada iniciando `tako.exe --web` com um `--data-dir` temporário, validando a subida da interface e encerrando apenas o PID de teste iniciado.
5. Invoca o Inno Setup (`build/rust/tako-web.iss`) para produzir o instalador por usuário `dist/Tako-Web-Preview-Setup-x64.exe`. O instalador utiliza AppId dedicado e pasta de destino própria (`%LOCALAPPDATA%\Programs\Tako Web (prévia)`), mantendo atalhos apontando para `tako.exe --web` sem interferir na instalação existente do Tako Slint. Caso o Inno Setup não esteja disponível, pode ser instalado via `winget install --id JRSoftware.InnoSetup -e --scope user` ou compactado diretamente como pacote portátil (`dist/Tako-Web-Preview-Portable-x64.zip`).
