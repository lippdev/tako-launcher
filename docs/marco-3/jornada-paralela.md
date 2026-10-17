# Jornada paralela: dois agentes no mesmo PC

Protocolo da rodada noturna de 16/10/2026. Dois agentes trabalham ao mesmo tempo no repositório, cada um com uma fila do
backlog (`plano.md`), e integram na `develop` local sem esperar o CI do GitHub. Vale o `CONTRIBUTING.md` em tudo que
não estiver dito aqui.

## Quem é quem

| Agente | Fila | Worktree |
|---|---|---|
| **A** (orquestrador) | Sistema, lojas e sessão | `C:\Users\LIPP\tako-wt\a` |
| **B** | Interface, biblioteca e experiência | criada pela T3 ao delegar |

O checkout principal (`C:\Users\LIPP\tako-launcher`) fica **sempre limpo na `develop`**: ninguém edita nele. Ele só
recebe merges pelo `scripts/bancada.sh integrar`.

## A bancada (uma coisa por vez)

O PC é um só, então compilar, testar, abrir o Tako e integrar passam por uma trava (`scripts/bancada.sh`). Quem chega
com a trava ocupada espera a vez.

```bash
scripts/bancada.sh run B cargo test -p tako-ui      # qualquer cargo build/test/clippy/run
scripts/bancada.sh run B target/release/tako.exe    # abrir o Tako
scripts/bancada.sh ci B                              # CI local completo
scripts/bancada.sh integrar B /caminho/msg.txt       # rebase + CI local + merge na develop
scripts/bancada.sh status                            # quem está com a trava
```

- **Todo** comando `cargo` (inclusive `check`) passa pela bancada. Editar código, ler e usar o git não precisam.
- As worktrees dividem o mesmo `target/` (o disco é curto). Por isso a regra acima não tem exceção.
- Comandos longos (CI, integrar) rodam em segundo plano no terminal do agente: a espera pode passar de 10 minutos.
- Segure a trava o mínimo. Nada de abrir o Tako e deixar aberto.
- Trava com mais de 90 minutos é considerada abandonada e liberada sozinha.

## CI local

`scripts/bancada.sh ci` roda o mesmo que o `rust.yml` roda no Windows, na ordem: `cargo fmt --all --check`, `cargo
clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked` e `cargo build -p tako
--release --locked`. Se a branch mexeu no código C# (`src/`, `tests/`, `tools/`), roda também o `dotnet` do `ci.yml`.

O que o CI local não cobre: código atrás de `#[cfg(target_os = "linux")]` ou `"macos"`. Por isso os itens só de
Linux ou macOS ficam fora desta rodada. Código novo de SO, mesmo em item de Windows, fica atrás de `cfg` com versão em
memória para os testes (CONTRIBUTING §6).

## Ciclo de um item

1. `git switch -c <tipo>/<ID>-<slug> develop` na sua worktree (ex.: `feat/SRC-003-store-client-missing`).
2. Leia o item no `docs/PRD.md` e no `docs/backlog/ideias.md`. Item grande: entregue a menor fatia útil e registre o
   resto como pendência no texto do merge.
3. Implemente com testes. Commits em Conventional Commits, com `Refs: <ID>` no rodapé.
4. Escreva a mensagem do merge num arquivo **fora do repositório** (ex.: `$TEMP/merge-<ID>.txt`), no lugar da
   descrição do PR:

   ```
   feat(sources): <título do item> (<ID>)

   O que mudou e por quê, em poucas linhas.

   Testes: o que cobre e o que rodou.
   Teste manual pendente: o que só dá pra conferir com a pessoa na frente do PC (ou "nenhum").
   Pendências: o que ficou fora (ou "nenhuma").

   Refs: <ID>
   ```

5. `scripts/bancada.sh integrar <A|B> $TEMP/merge-<ID>.txt`. Se der conflito no rebase, resolva fora da trava (`git
   rebase develop`) e rode de novo. Se o CI ficar vermelho, nada entra na `develop`: corrija e repita.
6. Depois do merge, a worktree fica destacada na `develop`. Comece o próximo item no passo 1.

Sem `push`, sem PR no GitHub e sem tag. O A faz um push só da `develop` no fim da rodada, depois de revisar.

## Arquivos que os dois mexem

- **`main.slint` e `crates/tako-ui/src/shell/`**: a estrutura é da Fila B. A Fila A só acrescenta (um botão, um
  aviso, um campo) e não reorganiza.
- **Migrações (`persistence/migrator.rs`)**: quem integra depois renumera a sua no rebase. Uma migração por item.
- **Catálogos de texto (`locales/*/*.toml`)**: só acrescentar chaves, nos dois idiomas.
- **`Cargo.toml` e `Cargo.lock`**: dependência nova só se for indispensável, e citada no texto do merge.
- **`docs/PRD.md` e changelogs**: só o A, no fim da rodada, a partir dos textos de merge. Não marque itens nem
  escreva changelog no seu item.

## Quando parar e avisar o A

- O item depende de decisão de produto, conta, certificado, servidor ou hardware que não existe aqui.
- Para fazer o item, você precisaria mudar uma arquitetura (CONTRIBUTING §6) ou criar ADR.
- Você ficou mais de 1 hora preso no mesmo erro.

Nesses casos, registre no texto do item o que descobriu, pule pro próximo e avise na sua resposta final.

## Filas

Em ordem de prioridade. Cada agente segue a sua até o tempo acabar.

**A:** bug do bench (`--simulate`) · SRC-003 · SESSION-006 · SESSION-003 · SRC-004 · PLAT-006 · PLAT-008 ·
SESSION-004 · SRC-005 · PLAT-007 · INPUT-005 · SRC-001 · DL-001/DL-002 · estudo SESSION-001 + INPUT-002.

**B:** UX-003 · LIB-002 · LIB-005 · LIB-003 · EXT-005 · ART-002 · testes de interface (§8.4) · UX-001 · PERF-001 ·
PERF-002 · INPUT-001 · THEME-004 · THEME-001 · UX-004 · ART-001 · VOICE-001 · estudo AI-001 + THEME-010.

Fora desta rodada: design (THEME-002/003/006/007/008, BRAND-001/002, UX-002); itens só de Linux ou macOS (atalho da
Central no X11/macOS, PLAT-002, SESSION-005, SRC-002); itens que dependem de algo externo (THEME-005, ART-003,
DEALS-001, DATA-001).
