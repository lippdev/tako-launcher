# Roadmap e ordem de ataque

Detalhes e gates em [PRD §12](PRD.md). Backlog completo: [PRD §5–8](PRD.md).

## Marco 0 — Proof of Concept
Objetivo: provar H1, H2, H3 ou registrar limitações. **Fechamento:** [`docs/marco-0/relatorio.md`](marco-0/relatorio.md).

1. ✅ **MVP-001** estrutura da solução em CI
2. ✅ **MVP-007 (spike)** FSE oficial — distribuível só com SCCD assinado pela Microsoft; experimento em hardware pendente
3. ✅ **MVP-016 (spike)** carregador XAML restrito — em uso pelo tema padrão; medir temas reais pendente
4. ✅ **MVP-013 / MVP-023 / MVP-028** tema padrão + biblioteca fictícia + navegação por controle (launcher WPF)
5. ✅ Relatório de riscos e escopo do Marco 1

## Marco 1 — MVP Alpha
Épicos do PRD §5 na ordem: A (fundação) → D (biblioteca) → B (FSE/fallback) → C (temas) → E (controle) → F (qualidade/distribuição).

**Status (0.1.0-alpha.10):** código do MVP concluído, exceto o FSE oficial (MVP-007/009, adiado) e a validação em hardware (MVP-034/035). Itens marcados com [x] no PRD têm o código entregue; a validação em hardware segue o [checklist](marco-1/checklist-hardware.md).

Histórico (0.1.0-alpha.7): plano, frentes e o que falta em [`docs/marco-1/plano.md`](marco-1/plano.md). LEGAL-001 decidido em 2026-10-15: o Tako passa a ser AGPL-3.0-only e reaproveita o código do Console Mode ([ADR-0014](adr/0014-licenca-agpl.md)). Fora do alcance do código: SCCD da Microsoft e hardware (MVP-007/009) e a validação em hardware real (checklist e números de referência do benchmark).

## Marco 2 — Backlog P1–P3 em ondas
Executado por frentes paralelas (uma PR por frente), sem release até os mantenedores pedirem.

- **Onda 1 (em andamento):** capas e metadados (SteamGridDB, seletor por fonte, IDs por fonte, capas animadas); EMU completo; lojas Ubisoft, EA, Battle.net, Amazon e importação do Playnite; DESK (desktop pelo controle, teclado virtual, standalone); SOCIAL e PERF restantes.
- **Onda 2:** STREAM (Sunshine/Apollo, Moonlight, interface web local, controle pelo celular), EXT (SDK de plugins, eventos, API local, automações), CLOUD (perfis locais, backup e sincronização por WebDAV/S3), UX-009 (acessibilidade).
- **Onda 3:** SCALE (OptiScaler e afins, com avisos de anti-cheat), MOD, THEME+ (gerenciador, galeria, formato próprio, editor de cores), SOURCE (catálogos JSON e downloads de conteúdo autorizado, com quarentena).

## Pendências antes de codar o runtime
- ~~`LEGAL-001`~~: decidido, AGPL-3.0-only ([`legal/LEGAL-001.md`](legal/LEGAL-001.md)).
- Migração do runtime compartilhado de .NET 8 para .NET 10 (PRD §3).
- Definir máquinas de referência para benchmarks (MVP-035).
