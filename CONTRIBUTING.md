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
- Escopos: `core`, `sources`, `platform`, `themes`, `ui`, `app`, `docs`, `ci`, `build` (e, até o corte, os do C#: `host`, `compat-p10`, `runtime`, `ipc`, `fse`).
- Referencie a história no rodapé: `Refs: MVP-005`. Breaking change: `feat(ipc)!:` + rodapé `BREAKING CHANGE:`.

## 3. Pull Requests

- Título segue Conventional Commits (validado no CI); descrição usa o template.
- O CI roda só em PR com código (PRs só de docs, design ou imagens não rodam), fora de rascunho, e cancela a execução anterior a cada push. Instalador e portable rodam em PR para a `main` ou com o label `package`; o benchmark, com o label `benchmark`. Os dois também podem ser pedidos em Actions › CI › Run workflow.
- Um PR = uma história. Preferir menos de 400 linhas alteradas.
- Para merge: CI verde, **1 aprovação** de um code owner (ver `.github/CODEOWNERS`), conversas resolvidas, DoD cumprida.
- PRs que tocam `runtime`, `ipc`, `themes` ou segurança: o revisor confirma explicitamente o item "Segurança e permissões revisadas" do checklist. Com três ou mais mantenedores, passam a exigir **2 aprovações**.
- O autor não aprova o próprio PR.
- **Mantenedores com permissão de push e merge em `main`:** @lippdev e @nextestudios.

## 4. Definition of Done

Vale a DoD do [PRD §12](docs/PRD.md); o checklist está no template de PR.

## 5. Padrões de código (Rust)

Vale para o Tako em Rust (PRD v0.2, §4 e §5). O código C#/WPF continua com as regras da seção 5.1 até o corte (MIG-043).

- Rust edição 2024, toolchain fixada em `rust-toolchain.toml`. Antes do commit: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` e `cargo test --workspace`. O CI roda os três nos três SOs.
- Nomes em inglês; comentários e documentação em português, curtos, explicando o **porquê** (o que o código não mostra).
- O menor código que resolve: sem abstração, opção ou dependência "para depois". Prefira a biblioteca padrão e as dependências já listadas em `[workspace.dependencies]`; dependência nova entra lá.
- **Dados externos não são confiáveis** (arquivos das lojas, JSON de APIs, temas, arquivos do usuário): limite de tamanho, nada de `unwrap`/`expect`/pânico em entrada inválida e teste com entrada quebrada. `unwrap` só em testes.
- Erros: `thiserror` nos tipos públicos; `Result<_, String>` onde for mais simples. Nunca engolir um erro sem log.
- Logs com `tracing`. **Nunca** logar credenciais ou tokens (a redação do `tako_core::diagnostics` é a última barreira, não a primeira).
- Textos da interface sempre por `tako_core::i18n` (`t`, `tf`, `tn`), com a chave nos dois catálogos (`crates/tako-core/locales/{pt-BR,en-US}/<módulo>.toml`); o teste `catalogs_match` reprova chave faltando.

### 5.1 Padrões de código (C# / .NET 10, até o corte)

- `.editorconfig` + analyzers; **warnings são erros**. Rode `dotnet format` antes do commit.
- `Nullable` habilitado; sem `!` sem comentário justificando.
- Namespaces file-scoped; campos privados `_camelCase` (constantes e `static readonly` em PascalCase); métodos assíncronos terminam em `Async` e aceitam `CancellationToken`.
- **MVVM** na UI: sem lógica de negócio em code-behind.
- Logs estruturados com `Microsoft.Extensions.Logging`. **Nunca** logar credenciais ou tokens.

## 6. Regras arquiteturais

1. Crates (PRD §5.1): `tako` → `tako-ui` → `tako-core`; `tako-sources`, `tako-platform` e `tako-themes` dependem de `tako-core`, nunca o contrário. `tako-core` não conhece UI nem SO.
2. Os contratos das funções de plataforma ficam em `tako_core::devices` (com versões em memória para testes); as implementações por SO ficam em `tako-platform`, atrás de `#[cfg(target_os = "...")]`. Código de SO só existe em `tako-platform` e `tako-sources`.
3. `unsafe` só em `tako-platform`, e cada bloco com `// SAFETY:` explicando por que é seguro.
4. A interface não restaura tela nem áudio: o supervisor restaura e sobrevive à queda da interface (ADR-0026). Durante o jogo, o processo da interface termina.
5. Sessão: transições idempotentes; restaurar duas vezes não desfaz nada a mais. Transição inválida não mexe no sistema.
6. Temas são dados: só apresentação, sem rede, disco ou processos; validados antes de carregar e com volta ao tema padrão (PRD §7).
7. Biblioteca 100% offline. Proibido: executar arquivos baixados, desativar antivírus/anti-cheat, injetar qualquer coisa em jogos.
8. Decisões relevantes viram ADR em `docs/adr/` (a partir da ADR-0023 para o Rust).

## 7. Testes

- Testes de unidade ao lado do código (`#[cfg(test)]`), portados dos testes C# quando o módulo vem de lá, com fixtures fictícias.
- Parsers de loja e de API testados em todos os SOs (são código puro); só a descoberta de pastas e o registro dependem do SO.
- Funções que exigem monitor, áudio, controle ou permissões reais não rodam no CI: use as versões em memória (`tako_core::devices::fake`) nos testes e os checklists de hardware em `docs/testing/` (`windows.md`, `linux.md`, `macos.md`) na PR.
- O tema padrão é compilado nos testes do `tako-ui`; um tema quebrado precisa cair no padrão sem loop de falha.

## 8. Releases

SemVer. O Tako em Rust sai pelo workflow `Release` (`.github/workflows/release.yml`, regras em `build/resolve-release.sh`): push na `main` publica a estável do topo do `CHANGELOG.md`; na `develop`, *Run workflow* com o canal alpha ou beta publica a próxima pré-release. Pacotes: `build/rust/Package-Tako.ps1` (Windows) e `build/rust/package.sh` (Linux e macOS). A versão C#/WPF tem o workflow manual `release-wpf.yml` até o corte. O processo anterior, ainda válido para a WPF:

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
