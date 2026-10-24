# Instruções para agentes

Você está no repositório do **Tako**, um launcher de jogos em Rust que está migrando a interface de Slint para
React sobre Chromium embutido (CEF). Processo completo: [`docs/processo/multiagente.md`](docs/processo/multiagente.md).

## 1. Ordem de verdade

1. O **brief** da sua tarefa.
2. [`docs/PRD.md`](docs/PRD.md), principalmente §2 (D-09 a D-12), §5, §7, §8.5 e a Fase 4.
3. [`docs/adr/0035-interface-web-rust-e-chromium.md`](docs/adr/0035-interface-web-rust-e-chromium.md) e o
   [plano](docs/plans/2026-10-24-interface-web.md).
4. [`CONTRIBUTING.md`](CONTRIBUTING.md) (padrões de código Rust, commits, testes).
5. Visual: [`design/desktop/`](design/desktop/) (README, `tokens.css`, `components/<Nome>/README.md` e
   `preview.html`). A prévia é o critério de aceite visual a 1600×900. O `Implementacao.md` de lá fala em Slint:
   vale a regra visual, não o caminho técnico.

Se o brief contradiz o PRD ou o ADR, **pare e diga no relatório**; não escolha sozinho.

## 2. Regras

- Trabalhe **só** nos arquivos que o brief permite. Precisou de outro? Diga no relatório por quê.
- Commits pequenos na branch desta worktree, em Conventional Commits, com `Refs: <ID>` no rodapé.
- **Não** faça push, não abra PR, não edite `docs/PRD.md` nem `docs/plans/web-quadro.md` (são do orquestrador).
- Não acrescente dependência que o brief não autorizou. Rust: dependência nova entra em `[workspace.dependencies]`.
- Núcleo Rust é a fonte da verdade; o React guarda só estado de tela (D-10). Sem porta de rede entre interface e
  núcleo; imagens só por `tako-img://`.
- Front: animação só de `transform` e `opacity` (nunca `width`, `height`, `top`, `left`, `font-size`); tokens do
  `design/desktop/tokens.css`, nada de cor ou medida solta; textos dos catálogos `locales/`, nunca escritos no
  componente; "Reduzir animações" zera durações.
- Tema não executa código: CSS, tokens e arquivos; sem JavaScript de tema.
- Comentários e documentação em português; nomes de código em inglês.

## 3. Ambiente (Windows)

- O SDL3 compila com `cmake`, que pode não estar no PATH. Use o do Visual Studio Build Tools:
  `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin`.
- Node 24 e npm estão instalados (sem pnpm). Python não está instalado: não use scripts Python.
- O primeiro `cargo build` numa worktree nova leva vários minutos; compile só o crate da tarefa
  (`cargo build -p <crate>`, `cargo test -p <crate>`).
- Rust: `cargo fmt --all`, `cargo clippy -p <crate> --all-targets -- -D warnings`, `cargo test -p <crate>`.

## 4. Relatório de entrega

Termine com este formato, sem enfeite:

```
## <ID> — relatório
Estado: pronto | parcial | bloqueado
Feito: <o que existe agora, em poucas linhas>
Commits: <hash curto e título de cada um>
Arquivos: <lista>
Verificação: <cada comando rodado e o resultado; números medidos, se houver>
Divergências do brief: <o que e por quê, ou "nenhuma">
Em aberto: <dúvidas, riscos, o que falta, ou "nada">
```

"Pronto" só com a saída da verificação. Se algo não pôde ser verificado, diga qual e por quê.
