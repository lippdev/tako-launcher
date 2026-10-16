# Guia de contribuição e regras de desenvolvimento

Fonte de verdade de como trabalhamos. Contexto de produto: [`docs/PRD.md`](docs/PRD.md).

## 1. Git Flow (modelo Atlassian)

| Branch | Origem | Destino (merge) | Finalidade |
|---|---|---|---|
| `main` | — | — | Somente código de release. Cada commit é uma versão com tag `vX.Y.Z`. Protegida (`scripts/setup-branch-protection.sh`, roda um admin). |
| `develop` | `main` | — | Integração do próximo release. Protegida. |
| `feature/<id>-<slug>` | `develop` | `develop` | Uma história do backlog. Ex.: `feature/MVP-005-sqlite-persistence`. |
| `release/X.Y.Z` | `develop` | `main` **e** `develop` | Estabilização: só correções, versão, changelog. |
| `hotfix/X.Y.Z-<slug>` | `main` | `main` **e** `develop` | Correção urgente de produção. |
| `spike/<id>-<slug>` | `develop` | descartada ou convertida | Investigação com prazo (ex.: `spike/MVP-007-fse`). Resultado vira ADR/doc, não merge direto. |

Regras:

1. **Nunca** commitar direto em `main` ou `develop`; tudo entra por Pull Request.
2. `feature/*` → `develop`: **squash merge**. `release/*` e `hotfix/*` → `main`: **merge commit** (`--no-ff`), depois tag e merge de volta em `develop`.
3. Branches de feature vivem poucos dias; histórias grandes são quebradas. Faça rebase em `develop` antes do PR.
4. Apague a branch após o merge.
5. Tags: `vMAJOR.MINOR.PATCH` (SemVer; pré-lançamentos `-alpha.N`, `-beta.N`, `-rc.N`). Builds `-alpha.N` para teste podem ser marcadas direto em `develop`; `beta`, `rc` e versões finais saem de `release/*` → `main`. Releases seguem o processo do Console Mode (seção 8).

## 2. Commits — Conventional Commits

`<tipo>(<escopo>): <descrição no imperativo, até 72 caracteres>`

- Tipos: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.
- Escopos: `host`, `core`, `ui`, `themes`, `compat-p10`, `runtime`, `ipc`, `fse`, `docs`, `ci`.
- Referencie a história no rodapé: `Refs: MVP-005`. Breaking change: `feat(ipc)!:` + rodapé `BREAKING CHANGE:`.

## 3. Pull Requests

- Título segue Conventional Commits (validado no CI); descrição usa o template.
- Um PR = uma história. Preferir menos de 400 linhas alteradas.
- Para merge: CI verde, **1 aprovação** de um code owner (ver `.github/CODEOWNERS`), conversas resolvidas, DoD cumprida.
- PRs que tocam `runtime`, `ipc`, `themes` ou segurança: o revisor confirma explicitamente o item "Segurança e permissões revisadas" do checklist. Com três ou mais mantenedores, passam a exigir **2 aprovações**.
- O autor não aprova o próprio PR.
- **Mantenedores com permissão de push e merge em `main`:** @lippdev e @nextestudios.

## 4. Definition of Done

Vale a DoD do [PRD §12](docs/PRD.md); o checklist está no template de PR.

## 5. Padrões de código (C# / .NET 10)

> Valem para o código C#/WPF até o corte (PRD MIG-043). Os padrões do código Rust entram aqui no MIG-010; até lá, vale a [PRD §5](docs/PRD.md).

- `.editorconfig` + analyzers; **warnings são erros**. Rode `dotnet format` antes do commit.
- `Nullable` habilitado; sem `!` sem comentário justificando.
- Namespaces file-scoped; campos privados `_camelCase` (constantes e `static readonly` em PascalCase); métodos assíncronos terminam em `Async` e aceitam `CancellationToken`.
- **MVVM** na UI: sem lógica de negócio em code-behind.
- Injeção de dependência (`Microsoft.Extensions.DependencyInjection`); sem singletons estáticos.
- Logs estruturados com `Microsoft.Extensions.Logging`. **Nunca** logar credenciais ou tokens.
- Não engolir exceções; tratar nos limites (UI, IPC, processo) e levar a sessão a `Error → Recovery`.

## 6. Regras arquiteturais (código C#)

1. Dependências fluem para dentro: `UI.Wpf`/`Host` → `Core`/`Themes` → `ConsoleMode.Runtime` (contratos). `Core` **não** referencia WPF.
2. Temas e `PlayniteCompat` nunca acessam serviços privilegiados do runtime.
3. A UI não restaura monitores/áudio; o runtime restaura e sobrevive à falha da UI.
4. IPC: Named Pipes restritos ao usuário local + JSON-RPC **versionado**; mudança de contrato exige teste de contrato.
5. Máquina de estados de sessão com transições idempotentes; transição inválida não altera o Windows.
6. Biblioteca 100% offline. Fontes JSON, URLs e pacotes de tema são **dados não confiáveis**.
7. Proibido: executar arquivos baixados, desativar Defender/anti-cheat, injetar gráficos silenciosamente.
8. Decisões relevantes viram ADR em `docs/adr/`.

## 7. Testes

- Unitários (xUnit) para Core/Themes; contrato para IPC; regressão para a máquina de estados.
- Meta de 80% de cobertura em `Launcher.Core`.
- Testes que exigem monitores, áudio, controle ou FSE reais não rodam no CI: marque com `[Trait("Category","Hardware")]` (excluídos no CI via `--filter-not-trait "Category=Hardware"`) e use checklists em `docs/testing/`.

## 8. Releases

SemVer, `0.x` durante o MVP. Fluxo: `develop` → `release/X.Y.Z` → PR para `main` → tag → merge de volta em `develop`. O processo e os arquivos são os mesmos do Console Mode:

1. Adicione a seção `## [X.Y.Z]` (ou `## [X.Y.Z-alpha.N]`) **nos dois changelogs**: `CHANGELOG.md` (português) e `CHANGELOG.en-US.md` (inglês). Um `(#NN)` no fim do item vira link para a PR. O workflow falha se faltar alguma seção.
2. Publique de um destes jeitos (versão com hífen = pré-release):
   - push de uma tag `vX.Y.Z` / `vX.Y.Z-alpha.N`; ou
   - Actions › Release › Run workflow na `main`, com a versão e "publish" marcado (a tag é criada ali). Sem "publish", só gera os arquivos como artefato.
3. A release sai com o nome `Tako X.Y.Z`, notas em inglês tiradas do `CHANGELOG.en-US.md` mais o link para o changelog em português, e os arquivos `Tako-Setup-x64.exe` (instalador Inno Setup, por usuário) e `Tako-Portable-x64.exe` (um único .exe; dados em `Tako_Data` ao lado). O kit do spike FSE (`Tako-FseKit-x64.zip`) acompanha enquanto o MVP-007 estiver aberto.
4. Assinatura (SignPath) e envio ao winget ficam prontos no workflow e ligam quando as variáveis/segredos existirem (`SIGNPATH_*`, `WINGET_TOKEN`).

Para gerar localmente: `.\build\Publish-Tako.ps1 -Runtime x64 -Target All -Version X.Y.Z`.

## 9. Issues

Cada item do PRD (ex.: `MVP-005`) vira uma issue com o ID no título. Labels: `epic:*`, `priority:P0..P3`, `type:story|bug|spike`, `blocked`. P0 primeiro, na ordem do [`docs/ROADMAP.md`](docs/ROADMAP.md).

## 10. Licença

O Tako é **AGPL-3.0-only** ([`LICENSE`](LICENSE)), a mesma licença do Console Mode ([ADR-0014](docs/adr/0014-licenca-agpl.md), [`LEGAL-001`](docs/legal/LEGAL-001.md)). Ao contribuir, você licencia sua contribuição nos mesmos termos.

- **Código do Console Mode** pode ser reaproveitado. Mantenha um comentário no topo do arquivo com a origem (`// Adaptado de lippdev/consolemode: src/ConsoleMode/...`).
- **Código de terceiros** só entra com licença compatível com a AGPL-3.0 (MIT, BSD, Apache-2.0, OFL para fontes). Registre a origem e a licença num `NOTICE.md` ao lado.
- **Temas e extensões** de terceiros carregados em tempo de execução não fazem parte do Tako e mantêm a própria licença.
