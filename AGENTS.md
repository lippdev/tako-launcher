# Instruções para agentes

Você está no repositório do **Tako**, um launcher de jogos em Rust que está migrando a interface de Slint para
React sobre Chromium embutido (CEF). Processo completo: [`docs/processo/multiagente.md`](docs/processo/multiagente.md).

**Pedido de design** (ajustar o visual, montar telas do `design/desktop/Backlog.md`)? Vá direto à §3: trabalha-se
no navegador, ao vivo, sem tarefa nem bateria de testes.

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
- **Nunca encerre processos que você não iniciou** (`Stop-Process -Name node`, `taskkill /IM node.exe`, `pkill`
  e afins). Outros agentes rodam builds, testes e servidores em paralelo na mesma máquina. Encerre só pelo PID do
  que você abriu, e use portas próprias (a 5173 é do orquestrador).

## 3. Trabalho de design (via rápida)

Vale quando o pedido é visual (ajustar ou montar telas, componentes, tokens, itens de `design/desktop/Backlog.md`)
e a mudança fica em `web/src/**` (`.tsx` de apresentação, `.module.css`), `design/desktop/` ou `locales/`. Aqui
**não** há brief, tarefa, worktree nem bateria de testes:

1. **Abra no navegador primeiro.** Se nada responde em `http://localhost:5173`, rode `npm run dev` em `web/` (em
   segundo plano) e mostre a tela a quem pediu (ferramenta de prévia do navegador, se houver; senão, passe o
   endereço). Rotas: `#/` o app, `#/componentes` a galeria com os estados de cada componente, `#/dev` a base
   visual (temas, tipografia, cores, movimento).
2. **Itere ao vivo.** Quem pede aponta o que mudar; você edita e o Vite recarrega sozinho. Confira com captura
   da tela a 1600×900 (a medida do design) e, quando couber, em 960×600. Não espere uma lista completa: um ponto
   por vez.
3. **Só o que é rápido de conferir:** `npm run typecheck` e `npm run lint` em `web/` antes de commitar.
4. **Commit direto** na branch em que se está, em Conventional Commits (`style(web): …`, `feat(web): …`).
5. **O Tako de verdade fica para a release.** O navegador e o CEF são o mesmo Chromium; a conferência no app
   instalado (com a biblioteca real, janela sem moldura, DPI) acontece na bateria de teste da próxima alpha.

As regras de front da §2 continuam valendo (tokens, catálogos `locales/`, só `transform`/`opacity`, Reduzir
animações). Saiu do visual (ponte, núcleo Rust, dados novos que o hospedeiro falso não tem)? Volte ao processo
normal: pare, diga o que precisa e siga o [`multiagente.md`](docs/processo/multiagente.md).

## 4. Ambiente (Windows)

- O SDL3 compila com `cmake`, que pode não estar no PATH. Use o do Visual Studio Build Tools:
  `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin`.
- Node 24 e npm estão instalados (sem pnpm). Python não está instalado: não use scripts Python.
- O primeiro `cargo build` numa worktree nova leva vários minutos; compile só o crate da tarefa
  (`cargo build -p <crate>`, `cargo test -p <crate>`).
- Rust: `cargo fmt --all`, `cargo clippy -p <crate> --all-targets -- -D warnings`, `cargo test -p <crate>`.

## 5. Relatório de entrega

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
