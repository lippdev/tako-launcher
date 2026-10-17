# ADR-0027: Modo desktop principal e tela cheia como Big Picture

- Status: Aceita
- Data: 2026-10-17

## Contexto

O Tako também é usado no PC com mouse e teclado. Uma interface somente em tela cheia atrapalha a busca,
organização e configuração da biblioteca nesse uso. A tela cheia continua útil no sofá, com controle, e em
sessões iniciadas pelo Console Mode ou AnyFSE. A ADR-0026 descartava a interface durante o jogo para reduzir
memória; no desktop, desaparecer da barra de tarefas contraria a expectativa de uma janela de launcher.

## Decisão

- O modo **desktop** é o principal: uma janela com biblioteca, busca, abas, filtros e Configurações. A
  **tela cheia (Big Picture)** mantém a apresentação para controle e TV. Ambas usam o mesmo `Shell`/`Ctx`,
  biblioteca, configurações, sessões e ações. A troca descarta a janela anterior, preservando o jogo em foco.
- Sem argumentos, abre no desktop. Um controle conectado ao abrir oferece a tela cheia, com a opção
  persistente de não perguntar novamente. `--big-picture` e `--console-session` pulam a pergunta e abrem
  direto na tela cheia. F11 e a barra superior entram nela; a Central e Configurações › Geral voltam ao desktop.
  Temas de terceiros continuam limitados à tela cheia; o desktop usa o visual embutido do Tako.
- **Durante o jogo no desktop, a interface fica viva e minimizada.** O processo, seu estado e a janela na
  barra de tarefas permanecem. Quando a janela do jogo aparece, `SupMsg::GameShown` minimiza o Tako. Quando
  termina, o supervisor restaura o ambiente e envia `SupMsg::GameEnded`; a mesma interface registra a sessão,
  restaura a janela, solicita foco e seleciona o jogo na biblioteca. Mantém o aviso de volta ou de fim anormal,
  incluindo o código de saída quando disponível. O compositor do Wayland pode apenas destacar a janela.
- **Na tela cheia vale a ADR-0026:** após `GameShown`, a interface fecha e só o supervisor acompanha o jogo.
  A Central abre uma nova interface; voltar ao jogo fecha essa interface. O fim do jogo reabre a interface
  no último modo informado. Se a interface desktop fechar ou cair durante o jogo, o supervisor também
  continua acompanhando; a próxima abertura usa esse mesmo caminho de recuperação.
- O canal real entre os dois processos são pipes anônimos de entrada/saída padrão, herdados exclusivamente
  pelo filho, com uma mensagem JSON por linha (`crates/tako/src/protocol.rs`). Esta descrição atualiza o
  transporte descrito na ADR-0026. `UiMsg::Mode` comunica o modo inicial e cada troca; o supervisor remove
  flags de modo antigas e repassa apenas o modo atual na próxima abertura. Heartbeats continuam no desktop
  minimizado. `Track`, `EndGame`, `ForceEndGame` e `Pointer` mantêm seus contratos nos dois modos.
- Com a interface viva, ela ouve o atalho do controle e registra o atalho global de teclado durante a sessão,
  abrindo a mesma Central dentro da janela desktop. Voltar ao jogo minimiza novamente; Encerrar jogo continua
  sendo um pedido ao supervisor, que confirma o término pelo acompanhamento. Sem interface, o supervisor
  ouve os atalhos e abre a Central. O teclado global continua disponível apenas no Windows; os demais SOs
  mantêm o suporte de controle já existente. A janela desktop minimizada não navega com botões do jogo nem
  oculta seus controles só por continuar aberta.
- `--no-supervisor` continua um caminho de desenvolvimento: acompanha e restaura localmente, minimizando
  no desktop e escondendo na tela cheia. Não oferece a sobrevivência à queda do processo.

## Consequências

- A volta ao desktop preserva a janela e evita uma abertura a frio. Em troca, **o orçamento de ≤ 40 MB durante
  o jogo passa a valer para o caminho de tela cheia com somente o supervisor**. O desktop retém o custo da
  UI, cache de imagens e renderizador. A referência da ADR-0026 era 34 MB por software a mais de 140 MB com
  OpenGL, conforme o driver; não são medições novas nem um teto para a biblioteca desktop.
- Minimizar não equivale a encerrar: os timers de entrada, heartbeat e acompanhamento continuam ativos.
  O Slint não precisa desenhar uma janela minimizada; esta decisão não promete zero trabalho de CPU.
- O supervisor continua responsável por acompanhar o jogo, restaurar tela/áudio/cursor, sobreviver à queda
  da UI e recuperar o diário `session-environment.json`. Idempotência, isolamento entre processos e ausência
  de injeção nos jogos continuam valendo. A ADR-0026 é alterada apenas no ciclo de vida desktop, transporte
  documentado e respectivo orçamento de memória; a política de reinício e o caminho Big Picture permanecem.
