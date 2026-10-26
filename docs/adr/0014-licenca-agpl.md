# ADR-0014: Licença AGPL-3.0-only e reuso do Console Mode

- Status: Aceita
- Data: 2026-10-15

## Contexto

O PRD §4.2 e o MVP-002 preveem reaproveitar do Console Mode os serviços de sessão, monitores e áudio. O Console Mode é AGPL-3.0-only e o Tako era MIT, então o reuso ficou bloqueado ([LEGAL-001](../legal/LEGAL-001.md)) e as frentes do Marco 1 escreveram contratos próprios (ADR-0011, ADR-0013).

Os dois projetos são dos mesmos mantenedores (@lippdev e @nextestudios).

## Decisão

O Tako passa a ser **AGPL-3.0-only** (opção B do LEGAL-001). O código do Console Mode pode ser trazido para o Tako, com um comentário de origem no topo de cada arquivo adaptado.

O foco agora é o desenvolvimento do Tako. A assinatura e as permissões da Microsoft para a experiência de tela cheia (SCCD, MVP-007/009) ficam para depois.

## Consequências

- O MVP-002 fica liberado: monitores (CCD, identidade de tela, HDR), áudio (Core Audio), Steam e controle de TV do Console Mode podem ser portados para `ConsoleMode.Runtime.Windows` atrás dos contratos atuais.
- Quem distribuir uma versão modificada do Tako, inclusive como serviço de rede, precisa oferecer o código-fonte nos termos da AGPL.
- Dependências e arquivos de terceiros precisam ter licença compatível (MIT, BSD, Apache-2.0, OFL). As atuais (fontes OFL, tema padrão do Playnite MIT, pacotes NuGet MIT) são compatíveis.
- Temas e extensões de terceiros carregados em tempo de execução mantêm a própria licença.
