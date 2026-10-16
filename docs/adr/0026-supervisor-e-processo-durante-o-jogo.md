# ADR-0026: Supervisor e processo do Tako durante o jogo

- Status: Aceita
- Data: 2026-10-16

## Contexto

O PRD §5.2 pede um executável só (`tako`) com um modo supervisor que garanta a restauração de tela e áudio
mesmo se a interface cair, e §8.1 pede ≤ 40 MB e zero quadros durante o jogo. O spike (ADR-0023) mostrou que
manter a interface viva e escondida custa de 34 MB (renderizador por software) a mais de 140 MB (OpenGL),
conforme o driver.

## Decisão

- `tako` sem argumentos roda o **supervisor**, que abre a interface (`tako --ui`) e a reinicia se cair (até 3
  vezes em 5 minutos; depois, modo seguro). `--no-supervisor` roda só a interface (desenvolvimento).
- A comunicação é JSON com prefixo de tamanho num socket local restrito ao usuário (named pipe com
  `CurrentUserOnly` no Windows; socket Unix com permissão 0600 em `$XDG_RUNTIME_DIR` ou na pasta de dados).
- **Durante o jogo, quem fica é o supervisor.** Ao jogar, a interface manda a sessão para o supervisor
  (jogo, configurações efetivas de tela, áudio e cursor) e fecha quando o jogo começa. O supervisor aplica tela,
  áudio e cursor, abre e acompanha o jogo, ouve o atalho da Central (controle e teclado) e, no fim, restaura tudo,
  grava o tempo de jogo e reabre a interface com o jogo em foco.
- A Central rápida durante o jogo abre uma interface nova só com a Central; "Voltar ao jogo" fecha de novo.
- O diário da sessão (`session-environment.json`) é gravado antes de qualquer mudança, então uma queda do
  supervisor ainda é restaurada na próxima execução.

## Consequências

- A memória durante o jogo é a do supervisor (sem janela nem GPU), em qualquer SO e driver.
- Voltar do jogo custa uma abertura a frio da interface.
- Substitui a ADR-0012 (supervisor por Named Pipes na implementação C#).
