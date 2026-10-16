# Roadmap e ordem de ataque

Backlog, decisões e critérios de aceite no [PRD](PRD.md) (versão 0.2, multiplataforma em Rust). Este arquivo só define a ordem.

## Agora: migração para Rust (PRD §9, fases 0 a 3)

1. **Fase 0 — Decisão técnica.** MIG-001 (spike Slint, com gate de desempenho) e MIG-002 (spike de plataforma) em paralelo; depois MIG-003 (ADR da fronteira com o Console Mode) e MIG-004 (marcar ADRs antigos). **Nada da Fase 1 começa antes do gate do MIG-001.**
2. **Fase 1 — Fundação Rust.** MIG-010 primeiro; depois MIG-011 a MIG-016 em paralelo (uma PR por história).
3. **Fase 2 — Paridade.** MIG-020 a MIG-023 (funções de plataforma) e MIG-025 a MIG-028 (fontes e metadados) em paralelo; MIG-024, MIG-029 e MIG-031 conforme as bases ficam prontas; MIG-030 e MIG-032 por último.
4. **Fase 3 — Distribuição e corte.** MIG-040 a MIG-042, uma beta Rust nos três SOs e então MIG-043 (remoção do C#/WPF).

Durante as fases 0 a 3, a versão C#/WPF continua sendo a publicada e só recebe correções críticas (PRD D-08).

## Depois do corte

- **P1:** PLAT (sessão gamescope, AUR/Flatpak, assinatura e notarização), SRC (emuladores, Bottles, itch.io), UX (acessibilidade, arte animada, primeira execução) e CMI, quando o Console Mode tiver a API.
- **P2:** THEME e PERF.
- **P3:** EXT, STREAM, CLOUD e SOURCE.

## Histórico

O plano anterior (Marcos 0 a 2, WPF e Windows) está em [`docs/marco-0/`](marco-0/relatorio.md), [`docs/marco-1/`](marco-1/plano.md) e na versão 0.1 do PRD (histórico do git).
