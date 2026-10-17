# Jornada paralela: dois agentes no mesmo PC

Protocolo da rodada noturna de 16 e 17/10/2026. Um orquestrador (não escreve código) e dois agentes executores, cada
um com uma fila do backlog (`plano.md`), trabalhando ao mesmo tempo no mesmo PC. Vale o `CONTRIBUTING.md` em tudo
que não estiver dito aqui.

## Quem é quem

| Papel | Quem | Fila |
|---|---|---|
| Orquestrador | Claude Fable 5.1 (esta conversa) | Revisa e faz o merge dos PRs, resolve dúvidas, marca o PRD e o changelog |
| **Agente A** | Claude Opus 5.5 (conta Alyx Medical) | Sistema, lojas, sessão, entrada, temas |
| **Agente B** | Claude Opus 5.5 (conta Lipp InvestSmart) | Interface, biblioteca e experiência |

Cada agente trabalha na **própria worktree** (criada pela T3 ao ser lançado). O checkout principal
(`C:\Users\LIPP\tako-launcher`) é do orquestrador: ninguém edita nele.

## A bancada (uma coisa por vez)

O PC é um só, então compilar, testar e abrir o Tako passam por uma trava (`scripts/bancada.sh`). Quem chega com a
trava ocupada espera a vez.

```bash
scripts/bancada.sh run A cargo test -p tako-ui      # qualquer cargo build/test/clippy/run
scripts/bancada.sh run A target/release/tako.exe    # abrir o Tako
scripts/bancada.sh ci A                              # CI local completo (obrigatório antes do PR)
scripts/bancada.sh status                            # quem está com a trava
```

- **Todo** comando `cargo` (inclusive `check`) passa pela bancada. Editar código, ler e usar o git não precisam.
- As worktrees dividem o mesmo `target/` (o disco é curto). Por isso a regra acima não tem exceção.
- Comandos longos (CI) rodam em segundo plano no terminal do agente: a espera pode passar de 10 minutos.
- Segure a trava o mínimo. Nada de abrir o Tako e deixar aberto.
- Trava com mais de 90 minutos é considerada abandonada e liberada sozinha.

`scripts/bancada.sh ci` roda o mesmo que o `rust.yml` roda no Windows (fmt, clippy com `-D warnings`, testes e build
de release) e, se a branch mexeu no C#, o `dotnet` do `ci.yml`. O que ele não cobre: código atrás de
`#[cfg(target_os = "linux")]` ou `"macos"`. O CI do GitHub roda depois do merge, nos três SOs, como segurança.

## Ciclo de um item

1. `git fetch origin && git switch -c <tipo>/<ID>-<slug> origin/develop` (ex.: `feat/EXT-005-tako-links`).
2. Leia o item no `docs/PRD.md` e no `docs/backlog/ideias.md`. Item grande: entregue a menor fatia útil e registre o
   resto como pendência no PR.
3. Implemente com testes. Commits em Conventional Commits, com `Refs: <ID>` no rodapé e
   `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
4. `scripts/bancada.sh ci <A|B>` verde. Vermelho: corrija antes de seguir.
5. `git push -u origin <branch>` e abra o PR contra a `develop` com `gh pr create`, título em Conventional Commits e
   o corpo neste formato:

   ```
   ## O que mudou
   Poucas linhas: o que o usuário ganha e como foi feito.

   ## Testes
   O que cobre e o que rodou (CI local verde).

   ## Teste manual pendente
   O que só dá para conferir com a pessoa na frente do PC (ou "nenhum").

   ## Pendências
   O que ficou fora (ou "nenhuma").

   Refs: <ID>
   ```

6. **Não faça o merge.** O orquestrador revisa cada PR: aprova e faz o merge (squash), ou deixa comentários.
7. Siga para o próximo item da fila (passo 1) sem esperar a revisão.
8. **Antes de começar cada item**, confira os seus PRs abertos: `gh pr list --author @me --state open` e
   `gh pr view <n> --comments`. PR com comentário do orquestrador tem prioridade: corrija na mesma branch, rode o
   CI local, faça push e responda no PR o que mudou.

Sem tag, sem changelog, sem mexer no `docs/PRD.md`: isso é do orquestrador, a partir dos PRs.

## Arquivos que os dois mexem

- **`main.slint` e `crates/tako-ui/src/shell/`**: a estrutura é da Fila B. A Fila A só acrescenta (um botão, um
  aviso, um campo) e não reorganiza.
- **Migrações (`persistence/migrator.rs`)**: a última é a 11. Quem abre o PR depois renumera a sua ao rebasear.
  Uma migração por item.
- **Catálogos de texto (`locales/*/*.toml`)**: só acrescentar chaves, nos dois idiomas.
- **`Cargo.toml` e `Cargo.lock`**: dependência nova só se for indispensável, e citada no PR.
- Conflito no rebase com a `develop`: resolva na sua branch (`git rebase origin/develop`), rode o CI local de novo
  e faça `git push --force-with-lease`.

## Quando parar e avisar

- O item depende de decisão de produto, conta, certificado, servidor ou hardware que não existe aqui.
- Para fazer o item, você precisaria mudar uma arquitetura (CONTRIBUTING §6) ou criar ADR.
- Você ficou mais de 1 hora preso no mesmo erro.

Nesses casos, registre no PR (ou numa issue, se não houver PR) o que descobriu, pule para o próximo e cite na
resposta final.

## Filas

Em ordem de prioridade. Cada agente segue a sua até o tempo acabar.

**A (Alyx Medical):** PR do PLAT-007 (branch `feat/PLAT-007-anyfse`, já com commit) · INPUT-005 · PERF-002 ·
INPUT-001 · THEME-004 · THEME-001 · SRC-001 · VOICE-001 · estudo SESSION-001 + INPUT-002.

**B (InvestSmart):** LIB-003 (continuar a branch `feat/LIB-003-smart-collections`, WIP sem testar) · EXT-005 ·
ART-002 · testes de interface (§8.4) · UX-001 · PERF-001 · UX-004 · ART-001 · estudo AI-001 + THEME-010.

Já integrados nesta rodada: bug do bench, SRC-003, SESSION-006, SESSION-003, SRC-004, PLAT-008, SESSION-004,
SRC-005, UX-003, LIB-002, LIB-005. PLAT-006 já estava pronto no código.

Fora desta rodada: design (THEME-002/003/006/007/008, BRAND-001/002, UX-002); itens só de Linux ou macOS (atalho da
Central no X11/macOS, PLAT-002, SESSION-005, SRC-002); itens que dependem de algo externo ou de decisão do
mantenedor (THEME-005, ART-003, DEALS-001, DATA-001, DL-001, DL-002).
