# Hospedeiro CEF no Windows (WEB-02a, WEB-02b)

O CEF entra no binário apenas pela feature optativa `web`, desligada por padrão. Com ela, `tako --web`
abre a interface. Sem `--web`, o Slint continua. Sem a feature, `--web` termina com código 1 e a mensagem
“compilado sem a interface web”. Contrato: [ponte.md](ponte.md).

## Dois processos: núcleo e interface descartável (WEB-02b, D-10)

| Processo | Linha | Tem | Não tem |
|---|---|---|---|
| Núcleo | `tako --web [--data-dir …]` | `Core`, `AppService`, `ui.state`, leitura das lojas, ciclo de vida da interface (`core_process.rs`) | CEF: a `libcef.dll` não é carregada |
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
- **Usuário fecha (X ou `window.action` `close`):** a interface manda `user_closed` e termina, e o núcleo encerra
  o app. Saída com código 0 sem a mensagem conta igual. Ainda não há bandeja: quando houver, “fechar para a
  bandeja” é o núcleo mandar `Command::Hide` em vez de encerrar.
- **Núcleo pede (jogo, bandeja, teste):** `Command::Hide` → `close`; a interface fecha os navegadores e sai.
  Sem sair em **5 s**, o núcleo mata o processo. O núcleo segue sem interface até `Command::Show`, que abre um
  filho novo; a página lê o `ui.state` e volta onde estava. `Command` é o gancho da sessão de jogo e da bandeja
  (`HostRequest::Launch` ainda só registra no log).
- **Queda da interface:** saída com código diferente de 0, sem pedido e sem `user_closed`. O núcleo registra e
  reabre uma vez. Outra queda até **60 s** depois dessa reabertura encerra o app com “a interface caiu de novo
  logo depois de reabrir”; depois de 60 s estável, a próxima queda reabre de novo. Sem laço infinito.
- **Queda do núcleo:** a entrada padrão do filho chega ao fim. A interface fecha as janelas e, se não sair em
  5 s, termina à força (`process::exit(2)`). Os subprocessos do CEF terminam com o processo principal deles.
- O núcleo confere a saída do filho a cada 250 ms enquanto há interface; sem interface, só acorda por aviso
  das lojas, por `Command` ou por prazo.

Testes do ciclo, com um filho falso sobre `std::io::pipe`, em `core_process.rs`: fechar a pedido e reabrir com
o `ui.state` de volta, ação de janela depois da resposta, matar quem não fecha, queda dupla, mensagens de um
filho já substituído e falha ao abrir o núcleo. O enquadramento é testado em `ipc.rs`.

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

A fixture PNG local de 120×180 é copiada para `ArtCache::cache_path("tako-test://cover")` no cache do núcleo.
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
- O sandbox continua desativado conforme a implementação anterior. O bootstrap/empacotamento fica para a
  WEB-02c; esta rodada não muda essa decisão.

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

O núcleo ainda não decide sozinho quando fechar a interface: o gancho (`Command::Hide`/`Show`) existe, mas só
o `TAKO_WEB_RECYCLE_MS` o usa. Faltam a sessão de jogo (`HostRequest::Launch` → `Hide` ao abrir o jogo e
`Show` ao voltar, com o supervisor de sessão), a bandeja (ícone, “fechar para a bandeja” → `Hide`, clique →
`Show`) e o descarte depois de um tempo minimizado (a interface avisa “minimizada”, o núcleo conta o tempo e
manda `Hide`). Um segundo `tako --web` ainda não traz a janela do primeiro (a trava de instância do supervisor
Slint não vale aqui). Sem Job Object, a queda do núcleo depende do fim da entrada padrão do filho; os
subprocessos do CEF somem com o processo principal deles, conferido acima.

A resolução de arte ainda é um snapshot de abertura, não cobre arte fixada fora do cache nem descobertas
posteriores e gera PNG por pedido sem cache de redimensionamento. O snapshot é lido pelo processo da
interface a cada abertura. O empacotamento do runtime continua fora desta tarefa. O log CEF é WARNING no debug e desativado em release; ainda não passa
pelo redator/rotação do núcleo. O build debug emite LNK4098 pela mistura
de runtimes C++ (SDL3, e também CEF com a feature `web`).
Linux e macOS precisam de entradas próprias; `--web` retorna erro nesses sistemas nesta etapa.
