# ADR-0017: Contador de FPS próprio pelo PresentMon, sem RTSS

- Status: Proposta
- Data: 2026-10-15

## Contexto

A Central de Controle (PR #33) mostrava o contador e o limite de FPS pelo RivaTuner Statistics Server (RTSS). Isso
conflita com o MSI Afterburner (que usa o mesmo RTSS e o mesmo OSD), exige que o usuário instale um programa à parte
e, como todo OSD do RTSS, injeta código no jogo. O PRD (§10) proíbe injeção gráfica pelo Tako.

O CapFrameX mede o FPS sem injeção: roda o PresentMon da Intel (MIT), que lê os eventos de apresentação do Windows
(ETW), e lê o CSV no stdout. A sessão ETW exige administrador ou o grupo local "Usuários de log de desempenho"
(`BUILTIN\Performance Log Users`, SID `S-1-5-32-559`).

## Decisão

1. **PresentMon 2.6.0 x64** (release oficial `PresentMon-2.6.0-x64.exe`, MIT) vai junto com o Tako na pasta
   `PresentMon\`. Não fica no repositório: `build\Publish-Tako.ps1` (usado pelo CI e pela release) baixa da release
   do GitHub da Intel, confere o SHA-256 fixado e passa `-p:PresentMonExe` ao `dotnet publish`. Hash diferente para
   o build. No portátil, ele vai dentro do .exe e é extraído com o resto do conteúdo. Não há build ARM64 do
   PresentMon: lá o contador aparece como indisponível. Licenças em `NOTICE.md`.
2. **Captura** (`PresentMonFpsCounter`, `ConsoleMode.Runtime.Windows`): `--output_stdout --no_console_stats
   --session_name Tako_FPS --stop_existing_session --no_track_input --exclude Tako.exe --exclude dwm.exe`.
   - Sessão com **nome próprio**: CapFrameX, FrameView e o PresentMon avulso usam outros nomes, então os dois
     convivem. `--stop_existing_session` só derruba uma sessão `Tako_FPS` que sobrou de uma queda.
   - **Sem filtro de processo no PresentMon**: o jogo troca de processo (launcher → jogo, Steam) e o
     `--process_id` aceita um só. O Tako filtra pelos processos da sessão atual (`GameRunTracker.GameProcessIds`,
     lidos a cada segundo), como o modo em tempo real do CapFrameX faz. Com mais de um processo do jogo apresentando,
     vale o de maior FPS.
   - Leitura pura em `ConsoleMode.Runtime` (`PresentMonReader`): colunas pelo nome do cabeçalho (`FrameTime` no
     2.x, `MsBetweenPresents` no 1.x/`--v1_metrics`), números invariantes, linhas ruins ignoradas.
   - Estatística (`FrameMath`/`FrameStatsWindow`, adaptadas do CapFrameX): FPS = quadros ÷ tempo somado do último
     segundo de quadros; 1% low = FPS da média dos quadros a partir do percentil 99 (R-8) nos últimos 10 s, com ao
     menos 30 quadros. As janelas usam tempo de quadro somado, não hora de chegada, porque o PresentMon entrega em
     rajadas. Sem quadros há 2 s, o contador mostra "--".
   - **Vida do PresentMon**: começa com a sessão de jogo (se "Mostrar FPS" estiver ligado e permitido) e fecha no fim
     dela. Ele roda num Job Object com `KILL_ON_JOB_CLOSE`, então morre junto se o Tako cair. Matar o processo não
     fecha a sessão ETW: o Tako a fecha com `ControlTrace(STOP)` no fim e, se um marcador (`fps-capture-running`)
     sobrou de uma queda, na abertura seguinte.
3. **Permissão: grupo "Usuários de log de desempenho"**, pelo SID (vale em qualquer idioma do Windows), só para
   quem liga "Mostrar FPS".
   - `Tako.exe --grant-perf-log-access <SID>` / `--revoke-perf-log-access <SID>` é o passo elevado
     (`NetLocalGroupAddMembers`/`DelMembers` com `LOCALGROUP_MEMBERS_INFO_0`), aberto pelo Tako com o verbo `runas`.
   - **O SID é lido no Tako não elevado** e passado como argumento: um usuário padrão pode digitar no UAC a senha de
     outro administrador, e um passo elevado que lesse "o usuário atual" poria o administrador no grupo. O passo
     elevado só aceita SIDs de conta de usuário (`S-1-5-21-…`, `S-1-12-1-…`); grupos e SIDs conhecidos (Todos,
     Usuários, Administradores) são recusados.
   - Estado: pronto se o token tem o grupo (ou o Tako está elevado); se a conta está no grupo mas o token não
     (`NetLocalGroupGetMembers`), "Saia e entre no Windows de novo para o contador funcionar."
   - **Só com teclado e mouse**: o UAC não aceita o controle. Em Configurações, "Mostrar FPS" (e "Remover
     permissão") esmaecem quando a última entrada foi o controle e dizem "Use teclado e mouse para ativar". Antes do
     UAC o Tako explica: "Para medir o FPS sem tocar no jogo, o Windows precisa liberar o Tako uma única vez."
   - Na Central de Controle, sem o contador pronto, as opções de FPS ficam esmaecidas com "Ative o contador de FPS
     nas Configurações do Tako, com teclado e mouse."; pronto, estilo e posição mudam pelo controle.
   - **Instalador**: tarefa opcional, desmarcada, "Ativar contador de FPS (o Windows vai pedir permissão)". Marcada,
     o fim da instalação roda `Tako.exe --setup-fps-counter` como o usuário que instalou (o instalador continua por
     usuário, sem administrador): o Tako lê o SID dele, pede o UAC para o passo elevado e liga a preferência.
     Atualizações silenciosas não repetem o pedido.
4. **Overlay próprio** (`FpsOverlayWindow`): janela WPF transparente, sempre no topo, com `WS_EX_LAYERED |
   WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` (sem clique, sem foco, fora do Alt+Tab), no canto
   escolhido do monitor da janela do jogo, posicionada em pixels físicos (`OverlayPlacement`, puro) com margem
   escalada pelo DPI do monitor; o WPF (PerMonitorV2) escala o texto, então 2K/4K ficam corretos. Estilos Só FPS,
   Compacto e Detalhado (`FpsOverlay`, puro), posição, tamanho e fundo escuro, como antes. Some enquanto o Tako está
   na frente (Central aberta). **Tela cheia exclusiva**: o Windows não compõe janelas sobre o jogo, então o contador
   não aparece; a medição continua e a Central mostra "FPS do jogo".
5. **RTSS removido do Tako**: contador, escrita de perfil e "Baixar o RTSS" saíram. Quem usa RTSS/Afterburner
   continua usando; o Tako não lê nem altera nada deles. Estilos e posições salvos no tempo do RTSS são convertidos
   ("O do RTSS" → Só FPS, "Personalizado" → Detalhado, posição do RTSS → superior esquerda).
6. **Limite de FPS** é do driver de vídeo ([ADR-0018](0018-limite-de-fps-pelo-driver.md)), por
   `IFrameRateLimiter`; o contador não depende dele e vice-versa.

## Consequências

- Sem conflito com o Afterburner e sem nada carregado no processo do jogo (melhor para anti-cheat; validar EAC e
  BattlEye no checklist de hardware, seção 14).
- O contador custa um UAC uma vez e um novo login. Quem não liga "Mostrar FPS" não recebe pedido nenhum.
- O pacote cresce ~1 MB (PresentMon). O build depende do download da release da Intel; o hash fixado impede troca
  silenciosa do binário.
- Pertencer ao grupo deixa a conta abrir sessões de rastreamento do Windows em geral (não só pelo Tako); por isso o
  pedido é explícito, opcional e reversível em "Remover permissão".
- Tela cheia exclusiva fica sem contador sobre o jogo; o FPS fica na Central.
