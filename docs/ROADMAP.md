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

**Status (0.1.0-alpha.7):** plano, frentes e o que falta em [`docs/marco-1/plano.md`](marco-1/plano.md). Fora do alcance do código: LEGAL-001 (reuso do Console Mode, MVP-002), SCCD da Microsoft e hardware (MVP-007/009) e a validação em hardware real (checklist e números de referência do benchmark).

## Pendências antes de codar o runtime
- `LEGAL-001`: licença (repo MIT vs. AGPL do Console Mode) e reuso de código. Opções e recomendação em [`legal/LEGAL-001.md`](legal/LEGAL-001.md).
- Migração do runtime compartilhado de .NET 8 para .NET 10 (PRD §3).
- Definir máquinas de referência para benchmarks (MVP-035).
