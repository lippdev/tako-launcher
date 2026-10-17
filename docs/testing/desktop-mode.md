# Modo desktop e Big Picture (ADR-0027)

Validação local em 17/10/2026, Windows, na bancada B. Os dados usados foram exclusivamente
`C:\Users\LIPP\tako-test\desktop-mode\data`. Não usar a pasta de dados principal nestes testes.

## Verificação automatizada

`scripts/bancada.sh ci B`: fmt, clippy do workspace com `-D warnings`, testes do workspace e build de release.
As regressões de `tako-ui` renderizam pelo backend de software do Slint, nos dois idiomas. Cobrem seleção,
busca sem/com resultado, abas, coleções, filtros, Jogar, favoritar, mais ações, pergunta do controle e sua
preferência persistida. Os testes de `app.rs` trocam as janelas nos dois sentidos, verificam minimização,
Central, pedido de encerramento e retorno normal/anormal com seleção do jogo. Supervisor/protocolo cobrem
as flags de modo na reabertura, mensagens e restauração com a UI viva.

## Verificação na janela nativa

Foi usado `target/release/tako.exe --data-dir C:/Users/LIPP/tako-test/desktop-mode/data`, sempre por
`scripts/bancada.sh run B ...`, com supervisor. Um executável WinForms local e descartável serviu como jogo:
abre uma janela, atende WM_CLOSE e, com `--fail`, sai com código 3 após dois segundos. Não é uma validação
de compatibilidade com jogos comerciais.

- Desktop: o mesmo PID da UI permaneceu vivo e `IsIconic` confirmou a janela minimizada; a janela da fixture
  permaneceu em primeiro plano. O atalho global `Ctrl+Shift+Tab` abriu a Central. Voltar ao jogo minimizou
  novamente. Encerrar pela Central terminou a fixture, restaurou a biblioteca e mostrou Bem-vindo de volta.
- Fim anormal: o aviso exibiu o código de saída `0x3 (3)` e selecionou a fixture correta.
- Big Picture: aberto pelo botão do desktop. Ao jogar, o PID da UI terminou, mantendo supervisor e fixture.
  O mesmo atalho abriu uma UI nova somente com a Central; Encerrar terminou o jogo e mostrou a volta.
- Configurações › Geral voltou do Big Picture ao desktop. A instância de teste foi fechada ao final,
  incluindo o supervisor, e a bancada ficou livre.
- Biblioteca, filtro de instalação, Coleções e Configurações foram inspecionados na janela real.

Capturas e evidências ficam em `C:\Users\LIPP\tako-test\desktop-mode\final-*`. As seis capturas principais
são `final-library.png`, `final-filter.png`, `final-collections.png`, `final-settings.png`,
`final-controller-question.png` e `final-big-picture.png`. Há capturas adicionais da Central e dos retornos,
logs e os estados dos processos. A captura da pergunta usa a mesma UI nativa com evento de controle
simulado por um harness temporário, salvo como `question-harness.rs` nessa pasta; não havia controle físico
conectado. O harness não faz parte do produto.

## Teste manual pendente

1. Conectar um controle físico e abrir pelo comando da bancada com a pasta de teste. Confirmar a pergunta,
   escolher Não perguntar de novo, fechar e reabrir. Confirmar que `--big-picture` e `--console-session`
   entram direto em tela cheia.
2. Abrir um jogo real no desktop. Confirmar minimização, atalho do controle, Voltar ao jogo e Encerrar pela
   Central. Repetir na tela cheia e confirmar que a UI fecha durante o jogo. Conferir HidHide quando instalado.
3. Com hardware de tela/áudio, conferir restauração após o fim e após queda da UI. No Linux e macOS,
   compilar e repetir o ciclo nativo; o teclado global nesses SOs continua fora do suporte atual.
4. Fechar o Tako de teste e conferir `scripts/bancada.sh status`. Não deixar a bancada presa.
