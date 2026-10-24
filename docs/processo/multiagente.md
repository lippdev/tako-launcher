# Desenvolvimento com vários agentes (migração para a interface web)

Como a equipe de agentes trabalha na Fase 4 do PRD (interface web, épico INST). Vale junto com o
[`CONTRIBUTING.md`](../../CONTRIBUTING.md); onde os dois divergem, este documento vale **só para a migração**.

Plano técnico: [`docs/plans/2026-10-24-interface-web.md`](../plans/2026-10-24-interface-web.md).
Quadro de tarefas: [`docs/plans/web-quadro.md`](../plans/web-quadro.md).
Regras para quem implementa: [`AGENTS.md`](../../AGENTS.md).

## 1. Papéis

| Papel | Quem | Faz | Não faz |
|---|---|---|---|
| **Mantenedor** | Filipe | Decide o rumo, aprova o visual ao vivo, aprova os gates e os PRs para a `develop` | — |
| **Orquestrador** | Thread principal no T3 (Claude Opus 5.5, conta Alyx Medical) | Quebra o plano em tarefas, escreve o brief, escolhe nível, conta e modelo, revisa cada entrega, junta na branch de integração, mantém o quadro e o PRD, fala com o mantenedor | Implementar tarefa grande (gasta a cota de quem orquestra); aprovar o próprio código sem revisão cruzada nas tarefas N4 |
| **Implementador** | Agente filho, numa worktree própria | Implementa só o brief, testa, faz commits na própria branch e devolve o relatório | Push, PR, editar o PRD ou o quadro, mexer fora dos arquivos do brief sem avisar |
| **Revisor** | Agente filho de **outro provider** que o implementador | Revisa o diff contra o brief e os critérios; lista problemas com arquivo e linha | Corrigir o código (só aponta) |

## 2. Contas

| Conta no T3 | Provider | Uso |
|---|---|---|
| Alyx Medical (Claude) | `claudeAgent` | Orquestrador. Só tarefas N1 curtas além disso |
| Lipp InvestSmart (Claude) | `claudeAgent_lipp_investsmart` | Implementador e revisor Claude |
| Lipp (Codex) | `codex` | Implementador e revisor Codex |
| Alyx Medical (Codex) | `codex_alyx_medical` | Implementador e revisor Codex |

Regras de distribuição:
- **No máximo 4 agentes ao mesmo tempo**, e no máximo 2 com build Rust pesado (a máquina tem 16 núcleos; cada
  worktree compila o seu `target/`). Tarefas só de front (`npm`) são leves.
- Espalhar as tarefas paralelas entre as três contas de implementação; não acumular duas tarefas N4 na mesma conta.
- Revisão cruzada: código de Codex é revisado por Claude e vice-versa.
- O Antigravity (Gemini) também está disponível no T3; fica fora da rotação até o mantenedor pedir.

## 3. Níveis de tarefa e modelo

O orquestrador classifica cada tarefa antes de despachar. Na dúvida, sobe um nível.

| Nível | Quando | Exemplos | Claude | Codex | Revisão |
|---|---|---|---|---|---|
| **N1 — Mecânica** | Sem decisão; o resultado é óbvio pelo brief | Copiar assets, gerar `tokens.css`, renomear, texto, ajuste de config, doc | Haiku 5.5, esforço médio | GPT-6-Luna, low | Orquestrador olha o diff |
| **N2 — Simples** | Uma peça isolada, com referência pronta | Um componente pela prévia do design system, uma função Rust com testes, um script de medição | Sonnet 5.5, esforço médio | GPT-6-Luna, medium | Orquestrador |
| **N3 — Média** | Várias peças, algum desenho local, integração | Uma tela inteira, um comando da ponte com tipos e testes, variante WebView2, ferramenta de benchmark | Sonnet 5.5, esforço alto | GPT-6.1-Sol, medium | Orquestrador + mantenedor no visual |
| **N4 — Difícil** | Arquitetura, segurança, ciclo de vida, desempenho, `unsafe`, muitos módulos | Hospedeiro CEF, ponte e esquemas, interface descartável, extração da `tako-app`, instalador e atualização | Opus 5.5, esforço alto ou extra alto | GPT-6.1-Sol, high ou xhigh | **Revisor de outro provider** + orquestrador |

Os nomes dos modelos Codex seguem o catálogo do T3 (Luna = mais leve, 6.1-Sol = o mais recente de topo). Se uma
tarefa voltar ruim, ela é refeita **um nível acima**, não repetida no mesmo modelo.

## 4. Branches e worktrees

```
develop ──► feature/WEB-interface-web (integração da migração)
                 ├── worktree da tarefa A  (branch criada pelo T3)
                 ├── worktree da tarefa B
                 └── worktree da tarefa C
```

- **Branch de integração:** `feature/WEB-interface-web`, a partir da `develop`. É a exceção ao "branches vivem
  poucos dias" do `CONTRIBUTING.md`, só para a migração. O orquestrador a mantém em dia com a `develop` (merge da
  `develop` nela ao começar cada onda).
- **Cada tarefa roda numa worktree própria** (`delegate_task` com `isolation: "worktree"`). A worktree parte do
  **último commit local** da checkout do orquestrador: antes de despachar, tudo que a tarefa precisa (brief,
  contrato, design system) tem que estar commitado na integração. Edição não commitada não vai junto.
- O implementador faz commits pequenos na branch da worktree, em Conventional Commits com `Refs: <ID>`.
- **Entrada na integração:** depois de aprovada, o orquestrador faz `git merge --squash` da branch da tarefa na
  integração, com mensagem `tipo(escopo): … Refs: <ID>`, e remove a worktree.
- **PR para a `develop`** só nos marcos, aberto pelo orquestrador com aprovação do mantenedor: resultado do gate
  (MIG-W01), fundação (WEB-01 a WEB-06), cada lote de telas e o corte. Como a interface web só abre com uma opção
  de desenvolvimento até o corte (`--web`), a `develop` continua publicando o Slint sem risco.
- **Conflitos:** tarefas paralelas tocam pastas diferentes. Arquivos compartilhados (`Cargo.toml` e `Cargo.lock`
  da raiz, `web/package.json` e o lockfile, `locales/`) só mudam se o brief disser; o orquestrador resolve o
  conflito ao juntar.

## 5. Ciclo de uma tarefa

1. **Preparar** (orquestrador): ID no quadro, nível, conta e modelo, arquivos que a tarefa pode tocar,
   dependências prontas e commitadas na integração.
2. **Despachar:** `delegate_task` assíncrono, worktree, com o brief do §6. Anotar `taskId` e worktree no quadro.
3. **Implementar** (agente): seguir o brief e o `AGENTS.md`, verificar e devolver o relatório do §7.
4. **Revisar:** orquestrador lê o diff contra o brief e os critérios; N4 passa também por um revisor de outro
   provider (nova `delegate_task` com o brief, o diff e o pedido de revisão). Problemas voltam ao mesmo agente
   (nova rodada com o brief, os achados e as respostas) ou a um nível acima.
5. **Validar com o mantenedor** quando a tarefa tem visual: o orquestrador abre a worktree (servidor do Vite ou o
   Tako com `--web`), o mantenedor vê ao vivo e aprova ou pede ajuste. Ajuste pequeno o orquestrador faz na hora;
   ajuste grande vira nova rodada.
6. **Juntar:** squash na integração, quadro atualizado, worktree removida, item do PRD marcado quando a história
   inteira fecha.

## 6. Modelo de brief

O agente só sabe o que está no brief. Todo brief tem:

```
# <ID> — <título>                      Nível: N<1-4>

## Objetivo
<o que precisa existir ao final, em 2 a 4 frases>

## Contexto
- Leia: <arquivos e seções exatas: PRD §, ADR, plano, prévias do design system>
- Já decidido: <decisões que não se discutem nesta tarefa>
- Já descartado: <o que não fazer e por quê>

## Escopo
- Pode criar/alterar: <pastas e arquivos>
- Não toque: <pastas e arquivos>
- Dependências novas: <permitidas ou não; quais>

## Critérios de aceite
- [ ] <verificável, um por linha>

## Verificação
<comandos exatos que precisam passar e o que medir ou capturar>

## Entrega
Commits na branch desta worktree (sem push, sem PR) e o relatório no formato do AGENTS.md §4.
```

## 7. O que se espera de volta

O relatório do agente (formato no `AGENTS.md`) diz o que fez, quais commits, a saída das verificações, onde
divergiu do brief e o que ficou aberto. O orquestrador não aceita "pronto" sem a saída dos comandos de verificação.

## 8. Validação ao vivo e testes

- Telas são aprovadas pelo mantenedor, ao vivo, contra a prévia do `design/desktop/` a 1600×900.
- Código Rust segue o `CONTRIBUTING.md` §5 e §7 (fmt, clippy, testes) dentro do que a tarefa toca.
- O front tem lint e build passando; teste de componente onde houver regra (foco, navegação, formatação), não
  para pixel.
- Gates (MIG-W00, MIG-W01, WEB-60) seguem a §8.5 do PRD e passam pelo mantenedor.
