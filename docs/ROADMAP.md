# Roadmap e ordem de ataque

Detalhes e gates em [PRD §12](PRD.md). Backlog completo: [PRD §5–8](PRD.md).

## Marco 0 — Proof of Concept (comece aqui)
Objetivo: provar H1, H2, H3 ou registrar limitações.

1. **MVP-001** estrutura da solução em CI (esqueleto já criado)
2. **MVP-007 (spike)** FSE oficial — maior risco do projeto; fazer cedo
3. **MVP-016 (spike)** carregador XAML restrito — segundo maior risco (segurança)
4. **MVP-013 / MVP-023 / MVP-028** tema padrão + biblioteca fictícia + navegação por controle
5. Relatório de riscos define o escopo do Marco 1

## Marco 1 — MVP Alpha
Épicos do PRD §5 na ordem: A (fundação) → D (biblioteca) → B (FSE/fallback) → C (temas) → E (controle) → F (qualidade/distribuição).

## Pendências antes de codar o runtime
- `LEGAL-001`: licença (repo MIT vs. AGPL do Console Mode) e reuso de código.
- Migração do runtime compartilhado de .NET 8 para .NET 10 (PRD §3).
- Definir máquinas de referência para benchmarks (MVP-035).
