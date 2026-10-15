# LEGAL-001 — Licença do UltraLauncher e reuso do Console Mode

**Status:** Decidido em 2026-10-15: **opção B** (o Tako passa a ser AGPL-3.0-only). Registro em [ADR-0014](../adr/0014-licenca-agpl.md).
**Bloqueia:** H3 (runtime compartilhado), MVP-002 e MVP-012

> Este documento organiza as opções para a decisão. Não é parecer jurídico.

## Situação

| | UltraLauncher | Console Mode |
|---|---|---|
| Repositório | `lippdev/ultralauncher` (público) | `lippdev/consolemode` (público) |
| Licença | MIT (`LICENSE`, © Filipe Moreira) | AGPL-3.0-only |
| Autores com commits (out/2026) | lippdev, nextestudios | lippdev (241), nextestudios (16) e bots (codex, dependabot, pullfrog) |

O PRD §4.2 prevê extrair serviços do Console Mode (sessão, monitores, áudio) para bibliotecas compartilhadas. Copiar esse código para um repositório MIT sem autorização dos autores não é permitido pela AGPL. Hoje o launcher só tem o **contrato** `ConsoleMode.Runtime` (ADR-0011), escrito do zero, sem código copiado.

**Fato que muda as opções:** os autores humanos do Console Mode são os próprios mantenedores do UltraLauncher. Quem tem os direitos autorais pode licenciar o mesmo código também sob outra licença. A AGPL só obriga quem recebe o código, não o autor.

## Opções

### A. Liberar em MIT o código extraído do Console Mode (recomendada)
Os dois autores declaram, num commit assinado por ambos (ou num comentário de PR de cada um), que os arquivos extraídos para `ConsoleMode.Runtime` passam a ter licença dupla: AGPL-3.0-only no Console Mode e MIT no UltraLauncher.
- **A favor:** mantém o UltraLauncher MIT e permite o reuso planejado no PRD. O Console Mode continua AGPL.
- **Contra:** antes de extrair, é preciso conferir a autoria de cada arquivo. Commits de bots não geram direitos autorais próprios, mas código de terceiros (PRs externas, trechos copiados de outros projetos) não pode ser relicenciado assim.
- **Para executar:** criar `src/ConsoleMode.Runtime/NOTICE.md` com a declaração e a lista de arquivos de origem; registrar o ADR; liberar o MVP-002.

### B. Mudar o UltraLauncher para AGPL-3.0-only (escolhida)
- **A favor:** o reuso fica livre, sem conferir autoria.
- **Contra:** todo o launcher passa a ser AGPL, inclusive para quem distribui versões modificadas. Isso pode afastar contribuições e integrações. Temas e plugins de terceiros carregados em tempo de execução precisariam de análise própria.
- **Para executar:** trocar `LICENSE`, atualizar o README e o CONTRIBUTING §10 e conferir a compatibilidade das dependências (Inter OFL, tema Playnite MIT: ambos compatíveis).

### C. Manter MIT sem reutilizar código
- **A favor:** não precisa de nenhuma decisão de licença.
- **Contra:** o runtime precisa ser reescrito do zero, sem consultar o código AGPL, e a H3 deixa de testar "compartilhar" o runtime. Pelo PRD, é o caminho de maior custo.

## Recomendação
A opção **A** atende ao PRD (launcher MIT e runtime compartilhado) com o menor custo, desde que a autoria dos arquivos extraídos seja conferida. Se houver código de terceiros no Console Mode, esses arquivos ficam de fora ou são reescritos.

## Como registrar a decisão
1. Marcar a opção escolhida neste arquivo e mudar o **Status** para "Decidido em AAAA-MM-DD".
2. Criar o ADR correspondente em `docs/adr/` a partir do template.
3. Atualizar o CONTRIBUTING §10 e o ROADMAP ("Pendências antes de codar o runtime").
