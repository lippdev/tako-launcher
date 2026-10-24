//! Hospedeiro da interface web do Tako (PRD D-09, D-10; ADR-0035). Contrato em `docs/web/ponte.md`.
//!
//! Aqui fica o que não depende do motor: a ponte (`bridge`) e a validação dos esquemas próprios (`schemes`). O
//! hospedeiro CEF (WEB-02a) e a variante WebView2 (MIG-W01) usam estas peças e só cuidam de janela, processos e
//! transporte das strings.

pub mod bridge;
pub mod schemes;
