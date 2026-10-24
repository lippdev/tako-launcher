# Hospedeiro CEF no Windows (WEB-02a)

O CEF entra no binário apenas pela feature optativa `web`, desligada por padrão. Com ela, `tako --web`
abre a interface. Sem `--web`, o Slint continua. Sem a feature, `--web` termina com código 1 e a mensagem
“compilado sem a interface web”. Contrato: [ponte.md](ponte.md).

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
$env:TAKO_WEB_REOPEN_MS = '20000'
.\target\debug\tako.exe --web --data-dir "$PWD\target\web02a-round2\manual"
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
`TAKO_WEB_TEST_ROOT`, `TAKO_WEB_TEST_ART`, `TAKO_WEB_REOPEN_MS` e o ID `test:cover` só valem em debug.

Com `TAKO_WEB_REOPEN_MS` (100 a 300000 ms), o primeiro fechamento agenda uma reabertura; o segundo termina.
O `AppService` conserva `ui.state` entre janelas do mesmo processo; não promete persistência entre execuções.
Sem a variável, fechar o último navegador encerra o processo. **DevTools também entra na contagem** até
`on_before_close`; ao fechar a janela principal, seu host pede o fechamento do DevTools. O CEF inicializa uma
vez e recebe `shutdown` ao sair. Não se deve reinicializá-lo neste processo; a guarda global `CEF_STARTED`
foi removida. O registro de objetos Views é local à thread UI para as tarefas acessarem só objetos dessa thread.

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
.\target\debug\tako.exe --web --data-dir "$PWD\target\web02a-round2\dev"
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
- A fila da página limita 64 comandos. A thread do núcleo abre `Core` e `AppService::with_core`, usa
  `tako_app::protocol::parse_request` por meio de `bridge::handle_message` e atende `HostRequest::PollBackground`
  chamando `AppService::poll()`. Um relay acorda essa mesma thread com avisos das lojas; não entrega eventos.
- Depois de cada `handle` e `poll`, o núcleo drena o canal de eventos em ordem. Resposta e eventos causados
  por ela entram no mesmo lote da fila UI, com a resposta primeiro. Ambas as entregas conferem navegador,
  frame e geração no hospedeiro, mais a geração no JavaScript. Mensagem e geração usam escape JSON.
  Sem página, os eventos são descartados; ao reabrir, a interface consulta o núcleo.
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
outros dados locais desta rodada ficam em `target/web02a-round2/` (ignorado pelo git).

## Limitações mantidas

A WEB-02b fará o processo CEF descartável. Fechar a janela nesta entrega mantém o runtime no processo
principal durante a espera por reabertura: ainda não cumpre zero Chromium/≤40 MB da D-10. O desenho desse
ciclo não mudou. A medição anterior, com a fixture antiga, foi 89,02 MiB aberta, 74,71 MiB fechada e 87,16 MiB
reaberta (Private Working Set de toda a árvore); não é uma nova medição desta rodada.

A resolução de arte ainda é um snapshot de abertura, não cobre arte fixada fora do cache nem descobertas
posteriores e gera PNG por pedido sem cache de redimensionamento. A sessão de jogo (`HostRequest::Launch`)
ainda não está conectada neste hospedeiro. A trava de instância do supervisor Slint e o empacotamento do
runtime continuam fora desta tarefa. O log CEF é WARNING no debug e desativado em release; ainda não passa
pelo redator/rotação do núcleo. O build debug emite LNK4098 pela mistura
de runtimes C++ (SDL3, e também CEF com a feature `web`).
Linux e macOS precisam de entradas próprias; `--web` retorna erro nesses sistemas nesta etapa.
